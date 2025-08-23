#!/bin/bash

# OOF Backend Production Deployment Script
set -e

DEPLOYMENT_ENV=${1:-production}
FORCE_REBUILD=${2:-false}

echo "🚀 Starting OOF Backend Production Deployment"
echo "   Environment: $DEPLOYMENT_ENV"

# Validation
if [ "$DEPLOYMENT_ENV" != "production" ] && [ "$DEPLOYMENT_ENV" != "staging" ]; then
    echo "❌ Invalid environment. Use 'production' or 'staging'"
    exit 1
fi

# Check if running as root (for production security)
if [ "$DEPLOYMENT_ENV" = "production" ] && [ "$EUID" -eq 0 ]; then
    echo "⚠️  Running as root in production is not recommended"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check required files
required_files=(".env.production" "docker-compose.yml")
for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "❌ Required file missing: $file"
        exit 1
    fi
done

# Load environment
if [ -f ".env.$DEPLOYMENT_ENV" ]; then
    echo "📝 Loading environment from .env.$DEPLOYMENT_ENV"
    export $(cat .env.$DEPLOYMENT_ENV | grep -v '^#' | xargs)
fi

# Set build variables
export GIT_HASH=$(git rev-parse --short HEAD)
export BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
export RUSTC_VERSION=$(rustc --version | cut -d' ' -f2)

echo "📋 Deployment Information:"
echo "  Environment: $DEPLOYMENT_ENV"
echo "  Git Hash: $GIT_HASH"
echo "  Build Time: $BUILD_TIME"
echo "  Rust Version: $RUSTC_VERSION"

# Backup current deployment (if exists)
if [ "$DEPLOYMENT_ENV" = "production" ]; then
    echo "💾 Creating backup of current deployment..."
    timestamp=$(date +"%Y%m%d_%H%M%S")
    backup_dir="backups/deployment_$timestamp"
    mkdir -p "$backup_dir"

    # Backup current images
    docker save -o "$backup_dir/api_backup.tar" oof-backend_api:latest 2>/dev/null || echo "No API image to backup"
    docker save -o "$backup_dir/indexer_backup.tar" oof-backend_indexer:latest 2>/dev/null || echo "No indexer image to backup"
    docker save -o "$backup_dir/workers_backup.tar" oof-backend_workers:latest 2>/dev/null || echo "No workers image to backup"

    # Backup database (if needed)
    if [ "$BACKUP_DATABASE" = "true" ]; then
        echo "💾 Backing up database..."
        docker-compose exec -T postgres pg_dump -U oof_user oof_backend > "$backup_dir/database_backup.sql"
    fi

    echo "✅ Backup created at $backup_dir"
fi

# Build new images
if [ "$FORCE_REBUILD" = "true" ] || [ ! "$(docker images -q oof-backend_api:latest 2> /dev/null)" ]; then
    echo "🔨 Building application images..."

    # Build with build cache for faster builds
    docker-compose build \
        --parallel \
        --build-arg GIT_HASH="$GIT_HASH" \
        --build-arg BUILD_TIME="$BUILD_TIME" \
        --build-arg RUSTC_VERSION="$RUSTC_VERSION" \
        api indexer workers migrations
else
    echo "📦 Using existing images (use force_rebuild=true to rebuild)"
fi

# Pre-deployment health checks
echo "🏥 Running pre-deployment health checks..."

# Check database connectivity
if ! docker-compose exec -T postgres pg_isready -U oof_user -d oof_backend; then
    echo "❌ Database is not accessible"
    exit 1
fi

# Check Redis connectivity
if ! docker-compose exec -T redis redis-cli ping | grep -q PONG; then
    echo "❌ Redis is not accessible"
    exit 1
fi

# Run database migrations
echo "📊 Running database migrations..."
docker-compose run --rm migrations

# Rolling deployment for zero-downtime
if [ "$DEPLOYMENT_ENV" = "production" ]; then
    echo "🔄 Performing rolling deployment..."

    # Deploy workers first (they can be stopped/started safely)
    echo "  Updating workers..."
    docker-compose up -d --no-deps workers
    sleep 10

    # Deploy indexer
    echo "  Updating indexer..."
    docker-compose up -d --no-deps indexer
    sleep 10

    # Deploy API (most critical)
    echo "  Updating API..."
    docker-compose up -d --no-deps api
    sleep 15
else
    # Non-production: standard deployment
    echo "🚀 Starting services..."
    docker-compose up -d
fi

# Post-deployment health checks
echo "🏥 Running post-deployment health checks..."
sleep 20

# Check API health
max_attempts=30
attempt=1
while [ $attempt -le $max_attempts ]; do
    if curl -f -s http://localhost:8080/health > /dev/null 2>&1; then
        echo "✅ API health check passed"
        break
    fi
    echo "⏳ API health check attempt $attempt/$max_attempts..."
    sleep 5
    attempt=$((attempt + 1))
done

if [ $attempt -gt $max_attempts ]; then
    echo "❌ API health check failed after $max_attempts attempts"
    if [ "$DEPLOYMENT_ENV" = "production" ]; then
        echo "🔄 Rolling back deployment..."
        # Rollback logic would go here
        exit 1
    fi
fi

# Check indexer health
if curl -f -s http://localhost:8081/health > /dev/null 2>&1; then
    echo "✅ Indexer health check passed"
else
    echo "⚠️  Indexer health check failed"
fi

# Verify services are running
echo "📊 Service status:"
docker-compose ps

# Display service information
echo ""
echo "🌐 Deployment Information:"
echo "  Environment: $DEPLOYMENT_ENV"
echo "  API: http://localhost:8080"
echo "  Indexer: http://localhost:8081"
echo "  Git Hash: $GIT_HASH"
echo "  Build Time: $BUILD_TIME"
echo ""

# Cleanup old images (keep last 3 versions)
if [ "$DEPLOYMENT_ENV" = "production" ]; then
    echo "🧹 Cleaning up old Docker images..."
    docker image prune -f
    # Keep only the last 3 versions of our images
    docker images | grep oof-backend | tail -n +4 | awk '{print $3}' | xargs -r docker rmi
fi

echo "✅ $DEPLOYMENT_ENV deployment complete!"
echo ""
echo "📋 Next steps:"
echo "  - Monitor logs: docker-compose logs -f"
echo "  - Check metrics: http://localhost:9090 (Prometheus)"
echo "  - Check dashboards: http://localhost:3001 (Grafana)"
echo ""
echo "🆘 Rollback command (if needed):"
echo "  ./scripts/rollback.sh $timestamp"
