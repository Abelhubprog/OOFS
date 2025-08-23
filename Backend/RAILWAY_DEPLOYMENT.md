# OOF Backend - Railway Deployment Guide

This guide covers deploying the OOF backend to Railway.com with Dynamic.xyz authentication and Cloudflare R2 storage.

## 🚀 Quick Deploy to Railway

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/template/your-template-id)

## 📋 Prerequisites

1. **Railway Account**: Sign up at [railway.app](https://railway.app)
2. **Dynamic.xyz Account**: Get your environment from [dynamic.xyz](https://dynamic.xyz)
3. **Cloudflare R2**: Set up R2 storage at [cloudflare.com](https://cloudflare.com)
4. **Helius Account**: For Solana RPC access at [helius.xyz](https://helius.xyz)

## 🔧 Environment Configuration

### Required Environment Variables

Set these in your Railway project settings:

#### Database & Redis (Railway Add-ons)
```bash
# Automatically provided by Railway PostgreSQL add-on
DATABASE_URL=postgresql://username:password@host:port/database

# Automatically provided by Railway Redis add-on
REDIS_URL=redis://username:password@host:port
```

#### Dynamic.xyz Authentication
```bash
# Get these from your Dynamic.xyz dashboard
DYNAMIC_ENVIRONMENT_ID=your_dynamic_environment_id
DYNAMIC_API_KEY=your_dynamic_api_key
DYNAMIC_JWKS_URL=https://app.dynamic.xyz/api/v0/environments/{ENVIRONMENT_ID}/keys
DYNAMIC_WEBHOOK_SECRET=your_webhook_secret_for_user_events
```

#### Cloudflare R2 Storage
```bash
# Get these from your Cloudflare R2 dashboard
R2_ENDPOINT=https://your_account_id.r2.cloudflarestorage.com
R2_ACCESS_KEY_ID=your_r2_access_key_id
R2_SECRET_ACCESS_KEY=your_r2_secret_access_key
R2_BUCKET_NAME=oof-storage
R2_PUBLIC_URL=https://your-custom-domain.com
```

#### Solana Configuration
```bash
# Solana RPC endpoints (use your Helius endpoints)
RPC_PRIMARY=https://rpc.helius.xyz/?api-key=your-api-key
RPC_SECONDARY=https://api.mainnet-beta.solana.com

# Webhook secret from Helius dashboard
HELIUS_WEBHOOK_SECRET=your_helius_webhook_secret
```

#### External APIs
```bash
# Jupiter Price API (default values)
JUPITER_BASE_URL=https://price.jup.ag/v3

# Pyth Network SSE endpoint
PYTH_HERMES_SSE=wss://hermes.pyth.network/ws
```

#### Security & Server
```bash
# Generate a secure random string for JWT signing
APP_SECRET=your_secure_random_string_min_32_chars

# Server configuration
API_BIND=0.0.0.0:8080
INDEXER_BIND=0.0.0.0:8081
ALLOW_ORIGIN=https://your-frontend-domain.com

# Deployment environment
ENVIRONMENT=production
```

## 🏗️ Railway Setup Steps

### 1. Create Railway Project

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login to Railway
railway login

# Create new project
railway new oof-backend
```

### 2. Add Database Services

In the Railway dashboard:
- Add **PostgreSQL** add-on
- Add **Redis** add-on
- Both will automatically set `DATABASE_URL` and `REDIS_URL`

### 3. Configure Environment Variables

Set all the required environment variables in Railway:

```bash
# Using Railway CLI
railway variables set DYNAMIC_ENVIRONMENT_ID=your_value
railway variables set DYNAMIC_API_KEY=your_value
railway variables set R2_ENDPOINT=your_value
# ... set all other variables
```

### 4. Deploy

```bash
# Connect to Railway project
railway link

# Deploy the backend
railway up
```

## 🔐 Dynamic.xyz Setup

### 1. Create Dynamic Environment

1. Go to [Dynamic.xyz Dashboard](https://app.dynamic.xyz)
2. Create a new environment for production
3. Configure supported wallets (Phantom, Solflare, etc.)
4. Enable social providers (Google, Twitter, etc.)
5. Copy the Environment ID and API Key

### 2. Configure Webhook (Optional)

Set up webhooks for user events:
- Webhook URL: `https://your-railway-domain.railway.app/webhooks/dynamic`
- Events: `user.created`, `user.updated`, `wallet.connected`

## ☁️ Cloudflare R2 Setup

### 1. Create R2 Bucket

```bash
# Using Cloudflare CLI (wrangler)
npx wrangler r2 bucket create oof-storage
```

### 2. Configure Custom Domain

1. Add custom domain in Cloudflare R2 settings
2. Set up DNS records
3. Update `R2_PUBLIC_URL` to use your custom domain

### 3. Set CORS Policy (Optional)

```json
[
  {
    "AllowedOrigins": ["https://your-frontend-domain.com"],
    "AllowedMethods": ["GET", "PUT", "POST"],
    "AllowedHeaders": ["*"],
    "MaxAgeSeconds": 3600
  }
]
```

## 📊 Health Checks

The backend includes comprehensive health checks:

- **Basic Health**: `GET /health`
- **Detailed Health**: `GET /health/detailed`
- **Readiness**: `GET /health/ready`

Railway will automatically use these for monitoring.

## 🔧 Troubleshooting

### Common Issues

1. **Database Connection Errors**
   - Verify `DATABASE_URL` is set correctly
   - Check PostgreSQL add-on status in Railway

2. **Authentication Failures**
   - Verify Dynamic.xyz environment ID and API key
   - Check JWKS URL accessibility
   - Ensure JWT tokens are valid

3. **Storage Issues**
   - Verify R2 credentials and endpoint
   - Check bucket permissions
   - Ensure custom domain is configured

4. **RPC Connection Issues**
   - Verify Helius API key is valid
   - Check RPC endpoint accessibility
   - Consider fallback RPC endpoints

### Logs

View logs in Railway:
```bash
# Using Railway CLI
railway logs
```

### Environment Verification

Test environment setup:
```bash
# Check health endpoint
curl https://your-railway-domain.railway.app/health

# Check detailed health
curl https://your-railway-domain.railway.app/health/detailed
```

## 🚀 Production Considerations

### Security
- Use strong `APP_SECRET` (min 32 characters)
- Enable HTTPS only in production
- Configure proper CORS origins
- Regular security updates

### Monitoring
- Set up Railway alerts
- Monitor health endpoints
- Track error rates and response times

### Scaling
- Railway auto-scales based on traffic
- Monitor resource usage
- Consider database connection pooling

### Backup
- Railway handles PostgreSQL backups automatically
- Consider additional backup strategies for critical data

## 📞 Support

- **Railway Support**: [Railway Discord](https://discord.gg/railway)
- **Dynamic.xyz Support**: [Dynamic Docs](https://docs.dynamic.xyz)
- **Cloudflare Support**: [Cloudflare Community](https://community.cloudflare.com)

## 🔗 Useful Links

- [Railway Documentation](https://docs.railway.app)
- [Dynamic.xyz Documentation](https://docs.dynamic.xyz)
- [Cloudflare R2 Documentation](https://developers.cloudflare.com/r2)
- [Solana RPC Documentation](https://docs.solana.com/developing/clients/jsonrpc-api)
