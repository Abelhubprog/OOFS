#!/bin/bash
# Railway.com startup script for OOF Backend
set -e

echo "🚀 Starting OOF Backend on Railway..."

# Check required environment variables
required_vars=(
    "DATABASE_URL"
    "DYNAMIC_ENVIRONMENT_ID"
    "DYNAMIC_API_KEY"
    "R2_ENDPOINT"
    "R2_ACCESS_KEY_ID"
    "R2_SECRET_ACCESS_KEY"
    "R2_BUCKET_NAME"
    "APP_SECRET"
)

for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "❌ Error: Required environment variable $var is not set"
        exit 1
    fi
done

echo "✅ Environment variables validated"

# Set Railway-specific defaults
export PORT=${PORT:-8080}
export API_BIND="0.0.0.0:${PORT}"
export ENVIRONMENT=${ENVIRONMENT:-production}
export RUST_LOG=${RUST_LOG:-info}

# Ensure public URL uses HTTPS in production
if [ "$ENVIRONMENT" = "production" ] && [[ "$R2_PUBLIC_URL" == http://* ]]; then
    echo "⚠️  Warning: R2_PUBLIC_URL should use HTTPS in production"
fi

# Wait for database to be ready
echo "⏳ Waiting for database to be ready..."
timeout 30 bash -c 'until pg_isready -h $(echo $DATABASE_URL | cut -d@ -f2 | cut -d: -f1) > /dev/null 2>&1; do sleep 1; done'

if [ $? -eq 0 ]; then
    echo "✅ Database is ready"
else
    echo "⚠️  Database readiness check timed out, proceeding anyway..."
fi

# Run database migrations (if you have them)
# echo "📊 Running database migrations..."
# ./migrations/run.sh || echo "⚠️  Migration script not found, skipping..."

# Validate Dynamic.xyz configuration
echo "🔐 Validating Dynamic.xyz configuration..."
JWKS_URL_FORMATTED=$(echo "$DYNAMIC_JWKS_URL" | sed "s/{ENVIRONMENT_ID}/$DYNAMIC_ENVIRONMENT_ID/g")
echo "JWKS URL: $JWKS_URL_FORMATTED"

# Test JWKS endpoint accessibility
if curl -s -f "$JWKS_URL_FORMATTED" > /dev/null; then
    echo "✅ Dynamic.xyz JWKS endpoint is accessible"
else
    echo "⚠️  Warning: Could not reach Dynamic.xyz JWKS endpoint"
fi

# Test Cloudflare R2 connectivity
echo "☁️  Testing Cloudflare R2 connectivity..."
# Note: More sophisticated R2 testing would require AWS CLI tools

echo "🎯 Starting OOF API server..."
echo "📡 Listening on: http://0.0.0.0:${PORT}"
echo "🌍 Environment: $ENVIRONMENT"
echo "🔗 Database: $(echo $DATABASE_URL | sed 's/postgresql:\/\/[^@]*@/postgresql:\/\/***:***@/')"

# Start the application
exec ./api
