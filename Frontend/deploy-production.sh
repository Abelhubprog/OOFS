#!/bin/bash

# 🚀 OOF Platform Production Deployment Script
# This script sets up and deploys the OOF Platform to Railway.com

set -e  # Exit on any error

echo "🚀 OOF Platform Production Deployment"
echo "=================================="

# Check if Railway CLI is installed
if ! command -v railway &> /dev/null; then
    echo "❌ Railway CLI is not installed. Please install it first:"
    echo "   npm install -g @railway/cli"
    echo "   or visit: https://railway.app/cli"
    exit 1
fi

# Check if user is logged in to Railway
if ! railway whoami &> /dev/null; then
    echo "🔐 Please login to Railway..."
    railway login
fi

echo "📋 Deployment Checklist:"
echo "========================"

# Environment variables checklist
echo "✅ Checking required environment variables..."

required_vars=(
    "DYNAMIC_ENVIRONMENT_ID"
    "DYNAMIC_API_KEY"
    "DYNAMIC_JWKS_URL"
    "JWT_SECRET"
    "HELIUS_API_KEY"
    "BIRDEYE_API_KEY"
    "OPENAI_API_KEY"
    "R2_ACCOUNT_ID"
    "R2_ACCESS_KEY_ID"
    "R2_SECRET_ACCESS_KEY"
    "CDN_BASE_URL"
    "SESSION_SECRET"
)

echo "Please ensure you have set the following environment variables in Railway:"
for var in "${required_vars[@]}"; do
    echo "  - $var"
done

read -p "Have you configured all required environment variables in Railway? (y/n): " confirm
if [[ $confirm != "y" && $confirm != "Y" ]]; then
    echo "❌ Please configure the environment variables first."
    echo "   You can set them using: railway variables set KEY=value"
    exit 1
fi

echo "✅ Environment variables confirmed"

# Build check
echo "🔧 Preparing build..."

# Check if we're in the right directory
if [[ ! -f "package.json" ]]; then
    echo "❌ Error: package.json not found. Please run this script from the project root."
    exit 1
fi

# Install dependencies if needed
if [[ ! -d "node_modules" ]]; then
    echo "📦 Installing dependencies..."
    npm ci
fi

# Build frontend
echo "🎨 Building frontend..."
if [[ -d "client" ]]; then
    cd client
    if [[ ! -d "node_modules" ]]; then
        npm ci
    fi
    npm run build
    cd ..
else
    echo "⚠️  Frontend directory not found, skipping frontend build"
fi

# Build backend
echo "⚙️  Building backend..."
npm run build

echo "✅ Build preparation completed"

# Deploy to Railway
echo "🚂 Deploying to Railway..."

# Create railway project if it doesn't exist
if ! railway status &> /dev/null; then
    echo "📝 Creating new Railway project..."
    railway init oof-platform
fi

# Add PostgreSQL service
echo "🗄️  Setting up PostgreSQL..."
railway add postgresql

# Add Redis service
echo "📦 Setting up Redis..."
railway add redis

# Deploy the application
echo "🚀 Deploying application..."
railway up

# Wait for deployment
echo "⏳ Waiting for deployment to complete..."
sleep 30

# Get the deployment URL
RAILWAY_URL=$(railway domain)
if [[ -n "$RAILWAY_URL" ]]; then
    echo "🌐 Deployment URL: https://$RAILWAY_URL"
else
    echo "🌐 Getting deployment status..."
    railway status
fi

# Run database migrations
echo "🗄️  Running database migrations..."
railway run npm run db:migrate

# Health check
echo "🏥 Performing health check..."
if [[ -n "$RAILWAY_URL" ]]; then
    if curl -f "https://$RAILWAY_URL/health" &> /dev/null; then
        echo "✅ Health check passed!"
    else
        echo "⚠️  Health check failed, but deployment may still be starting..."
    fi
fi

echo ""
echo "🎉 Deployment Complete!"
echo "======================"
echo "🌐 Application URL: https://$RAILWAY_URL"
echo "📊 Monitor: railway logs"
echo "⚙️  Manage: https://railway.app/dashboard"
echo ""
echo "🔧 Useful Commands:"
echo "   railway logs            # View application logs"
echo "   railway shell           # Access application shell"
echo "   railway variables       # Manage environment variables"
echo "   railway status          # Check deployment status"
echo ""
echo "✨ Your OOF Platform is now live in production! ✨"
