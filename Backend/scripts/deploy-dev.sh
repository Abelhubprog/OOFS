#!/bin/bash

# OOF Backend Development Deployment Script
set -e

echo "🚀 Starting OOF Backend Development Deployment"

# Check if Docker and Docker Compose are installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Set build variables
export GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
export BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
export RUSTC_VERSION=$(rustc --version | cut -d' ' -f2 2>/dev/null || echo "unknown")

echo "📋 Build Information:"
echo "  Git Hash: $GIT_HASH"
echo "  Build Time: $BUILD_TIME"
echo "  Rust Version: $RUSTC_VERSION"

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env file from template..."
    cp .env.example .env
    echo "⚠️  Please review and update .env file with your configuration"
fi

# Create necessary directories
echo "📁 Creating necessary directories..."
mkdir -p logs
mkdir -p data/postgres
mkdir -p data/redis
mkdir -p data/minio

# Pull latest images
echo "📥 Pulling latest base images..."
docker-compose pull postgres redis minio prometheus grafana nginx

# Build the application images
echo "🔨 Building application images..."
docker-compose build --parallel api indexer workers migrations

# Create MinIO buckets
echo "🪣 Setting up MinIO buckets..."
docker-compose up -d minio
sleep 5

# Create bucket using MinIO client
docker run --rm --network oof-backend_oof-network \
    -e MC_HOST_local=http://oof_minio_user:oof_minio_password@minio:9000 \
    minio/mc:latest \
    sh -c "mc mb local/oof-assets || echo 'Bucket already exists'"

# Start all services
echo "🚀 Starting all services..."
docker-compose up -d

# Wait for services to be healthy
echo "⏳ Waiting for services to be healthy..."
sleep 10

# Check service health
echo "🏥 Checking service health..."
services=("postgres" "redis" "minio" "api" "indexer")
for service in "${services[@]}"; do
    echo -n "  $service: "
    if docker-compose ps $service | grep -q "healthy\|Up"; then
        echo "✅ Healthy"
    else
        echo "❌ Unhealthy"
    fi
done

# Show service URLs
echo ""
echo "🌐 Service URLs:"
echo "  API: http://localhost:8080"
echo "  Indexer: http://localhost:8081"
echo "  MinIO Console: http://localhost:9001"
echo "  Prometheus: http://localhost:9090"
echo "  Grafana: http://localhost:3001 (admin/admin)"
echo ""

# Show logs command
echo "📊 To view logs, run:"
echo "  docker-compose logs -f [service_name]"
echo ""

# Show stop command
echo "⏹️  To stop all services, run:"
echo "  docker-compose down"
echo ""

echo "✅ Development deployment complete!"
echo "🔗 API Health Check: curl http://localhost:8080/health"
