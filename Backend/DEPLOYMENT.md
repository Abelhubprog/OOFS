# OOF Backend Docker Deployment

This directory contains Docker configurations and deployment scripts for the OOF Backend system.

## 🏗️ Architecture

The OOF Backend consists of the following services:

- **API Service** (`api`) - REST API endpoints and authentication
- **Indexer Service** (`indexer`) - Webhook processing and transaction indexing
- **Workers Service** (`workers`) - Background job processing and computations
- **PostgreSQL** (`postgres`) - Primary database
- **Redis** (`redis`) - Caching and pub/sub
- **MinIO** (`minio`) - S3-compatible object storage
- **Prometheus** (`prometheus`) - Metrics collection
- **Grafana** (`grafana`) - Observability dashboards
- **Nginx** (`nginx`) - Reverse proxy and load balancer (production)

## 🚀 Quick Start (Development)

### Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- Git
- 8GB+ RAM recommended

### 1. Clone and Setup

```bash
git clone <repository-url>
cd oof-backend
```

### 2. Configure Environment

```bash
# Copy environment template
cp .env.example .env

# Edit configuration (database passwords, API keys, etc.)
nano .env
```

### 3. Deploy Development Environment

```bash
# Run deployment script
chmod +x scripts/deploy-dev.sh
./scripts/deploy-dev.sh

# Or manually with docker-compose
docker-compose up -d
```

### 4. Verify Deployment

```bash
# Check service health
curl http://localhost:8080/health
curl http://localhost:8081/health

# View logs
docker-compose logs -f api
```

## 🏭 Production Deployment

### Prerequisites

- Production server with Docker
- SSL certificates (for HTTPS)
- Dedicated RPC endpoints
- S3 bucket or object storage
- Domain names configured

### 1. Production Configuration

```bash
# Copy production environment template
cp .env.production .env.prod

# Edit with production values
nano .env.prod
```

### 2. Setup Secrets

```bash
# Create secrets directory
mkdir -p secrets

# Add database password
echo "your_secure_postgres_password" > secrets/postgres_password.txt

# Add Grafana password
echo "your_secure_grafana_password" > secrets/grafana_password.txt

# Set proper permissions
chmod 600 secrets/*.txt
```

### 3. Deploy to Production

```bash
# Run production deployment
chmod +x scripts/deploy-prod.sh
./scripts/deploy-prod.sh production

# Or with Docker Compose
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### 4. Configure Reverse Proxy

For production, use the included Nginx configuration or your own reverse proxy:

```bash
# Enable nginx profile for production
docker-compose --profile production up -d nginx
```

## 📊 Monitoring and Observability

### Prometheus Metrics

- **URL**: http://localhost:9090
- **Metrics endpoint**: http://localhost:8080/metrics
- **Targets**: API, Indexer, Database, Redis

### Grafana Dashboards

- **URL**: http://localhost:3001
- **Default credentials**: admin/admin (change in production)
- **Dashboards**: Pre-configured for OOF Backend metrics

### Health Checks

```bash
# API health
curl http://localhost:8080/health

# Indexer health
curl http://localhost:8081/health

# Database health
docker-compose exec postgres pg_isready -U oof_user

# Redis health
docker-compose exec redis redis-cli ping
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DATABASE_URL` | PostgreSQL connection string | - | ✅ |
| `REDIS_URL` | Redis connection string | - | ❌ |
| `ASSET_BUCKET` | S3 bucket for assets | - | ✅ |
| `API_BIND` | API service bind address | 0.0.0.0:8080 | ❌ |
| `INDEXER_BIND` | Indexer service bind address | 0.0.0.0:8081 | ❌ |
| `HELIUS_WEBHOOK_SECRET` | Helius webhook HMAC secret | - | ✅ |
| `JWKS_URL` | JWT public keys URL | - | ✅ |
| `RPC_PRIMARY` | Primary Solana RPC endpoint | - | ✅ |
| `JUPITER_BASE_URL` | Jupiter API base URL | - | ✅ |

### Resource Requirements

#### Development
- **CPU**: 4 cores minimum
- **RAM**: 8GB minimum
- **Storage**: 20GB minimum

#### Production
- **CPU**: 8 cores recommended
- **RAM**: 16GB recommended
- **Storage**: 100GB+ recommended (for data growth)

## 🛠️ Maintenance

### Database Backup

```bash
# Create database backup
docker-compose exec postgres pg_dump -U oof_user oof_backend > backup.sql

# Restore from backup
docker-compose exec -T postgres psql -U oof_user oof_backend < backup.sql
```

### Log Management

```bash
# View logs
docker-compose logs -f [service_name]

# Log rotation (production)
docker system prune -af --volumes  # WARNING: Removes all unused data
```

### Updates and Rollbacks

```bash
# Update to latest version
git pull
./scripts/deploy-prod.sh production force_rebuild=true

# Rollback (if backup was created)
./scripts/rollback.sh [backup_timestamp]
```

## 🔒 Security Considerations

### Production Security Checklist

- [ ] Change all default passwords
- [ ] Use secrets management for sensitive data
- [ ] Enable SSL/TLS for external connections
- [ ] Configure firewall rules
- [ ] Regular security updates
- [ ] Monitor access logs
- [ ] Use dedicated RPC endpoints
- [ ] Implement rate limiting
- [ ] Regular backup testing

### Network Security

- Services communicate over internal Docker network
- Only necessary ports exposed to host
- Nginx handles SSL termination
- Rate limiting configured per endpoint

## 🐛 Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check logs
docker-compose logs [service_name]

# Check resource usage
docker stats

# Restart service
docker-compose restart [service_name]
```

#### Database Connection Issues

```bash
# Check database status
docker-compose exec postgres pg_isready -U oof_user

# Check database logs
docker-compose logs postgres

# Verify credentials
echo $DATABASE_URL
```

#### High Memory Usage

```bash
# Check memory usage
docker stats

# Reduce worker concurrency
# Edit .env: WORKER_CONCURRENCY=2

# Restart workers
docker-compose restart workers
```

### Performance Tuning

#### Database Optimization

```sql
-- Check connection usage
SELECT count(*) FROM pg_stat_activity;

-- Check slow queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
ORDER BY mean_exec_time DESC LIMIT 10;
```

#### Redis Optimization

```bash
# Check Redis memory usage
docker-compose exec redis redis-cli info memory

# Check cache hit rate
docker-compose exec redis redis-cli info stats
```

## 📚 Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [PostgreSQL Tuning](https://wiki.postgresql.org/wiki/Tuning_Your_PostgreSQL_Server)
- [Redis Configuration](https://redis.io/topics/config)
- [Prometheus Configuration](https://prometheus.io/docs/prometheus/latest/configuration/configuration/)
- [Grafana Documentation](https://grafana.com/docs/)

## 🆘 Support

For issues with the deployment:

1. Check the troubleshooting section above
2. Review service logs: `docker-compose logs [service]`
3. Check resource usage: `docker stats`
4. Verify configuration: Review `.env` file
5. Create an issue with full error details

## 📝 License

[License information]
