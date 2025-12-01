# pSol Relayer - Mainnet Deployment Checklist

## Pre-Deployment Requirements

### ✅ Infrastructure Setup

- [ ] **Redis Server**
  - [ ] Redis 6+ installed and running
  - [ ] Persistence enabled (`appendonly yes`)
  - [ ] Password configured (`requirepass`)
  - [ ] Memory limits set (`maxmemory` + `maxmemory-policy`)
  - [ ] Bind to localhost or private network only
  - [ ] Backup strategy configured

- [ ] **Server/VM**
  - [ ] Linux server (Ubuntu 22.04+ recommended)
  - [ ] Minimum 2 vCPU, 4GB RAM
  - [ ] SSD storage for Redis persistence
  - [ ] Firewall configured (allow only 80/443)
  - [ ] Automatic security updates enabled

- [ ] **TLS/SSL Certificates**
  - [ ] Valid certificates from Let's Encrypt or CA
  - [ ] Certificates in `./certs/` directory
  - [ ] Auto-renewal configured (certbot)

### ✅ Wallet & Keys Setup

- [ ] **Relayer Hot Wallet**
  - [ ] Dedicated keypair generated (`solana-keygen new`)
  - [ ] Address recorded: `___________________________`
  - [ ] Funded with SOL (recommend 5+ SOL for mainnet)
  - [ ] Private key stored in secrets manager or secure env

- [ ] **API Keys Generated**
  - [ ] Admin API key: `openssl rand -hex 32`
  - [ ] Client API keys (one per frontend/service)
  - [ ] Keys stored securely (never in git!)

### ✅ Configuration Review

- [ ] **Environment Variables Set**
  ```bash
  # Verify these are configured in .env or container env:
  SOLANA_RPC_URL=https://<paid-rpc-provider>
  SOLANA_NETWORK=mainnet-beta
  PSOL_PROGRAM_ID=<mainnet-program-id>
  RELAYER_PRIVATE_KEY=<base58-encoded>
  RELAYER_MIN_BALANCE=1.0
  
  REDIS_HOST=localhost
  REDIS_PASSWORD=<strong-password>
  
  REQUIRE_AUTH_FOR_WRITE=true
  RELAYER_API_KEYS=<key1>,<key2>
  ADMIN_API_KEY=<admin-key>
  
  NODE_ENV=production
  LOG_LEVEL=info
  ```

- [ ] **Fee Configuration**
  - [ ] `BASE_FEE_BPS` set appropriately for mainnet (e.g., 50 = 0.5%)
  - [ ] `MIN_FEE_LAMPORTS` covers at least 2x expected gas costs
  - [ ] `MAX_FEE_BPS` caps maximum fee (e.g., 500 = 5%)

- [ ] **Rate Limits Configured**
  - [ ] `RATE_LIMIT_MAX_REQUESTS` set (e.g., 60/minute)
  - [ ] `MAX_PENDING_PER_IP` set (e.g., 5)
  - [ ] `MAX_PENDING_GLOBAL` set (e.g., 1000)

### ✅ Security Hardening

- [ ] **Network Security**
  - [ ] Relayer port (3000) not exposed publicly
  - [ ] Only Nginx (80/443) exposed
  - [ ] Redis port (6379) not exposed publicly
  - [ ] SSH key-based auth only (no passwords)

- [ ] **Secrets Management**
  - [ ] `RELAYER_PRIVATE_KEY` not in git
  - [ ] API keys not in git
  - [ ] Using secrets manager or encrypted env
  - [ ] `.env` file has 600 permissions

- [ ] **Nginx Configuration**
  - [ ] TLS 1.2+ only
  - [ ] Strong cipher suite
  - [ ] HSTS enabled
  - [ ] Rate limiting configured

### ✅ Monitoring Setup

- [ ] **Alerting Configured**
  - [ ] Low balance alert (< 1 SOL)
  - [ ] High error rate alert
  - [ ] Queue depth alert
  - [ ] Redis connection alert

- [ ] **Logging**
  - [ ] Logs forwarded to aggregator (optional)
  - [ ] Log rotation configured
  - [ ] Debug logs disabled in production

- [ ] **Metrics**
  - [ ] `/metrics` endpoint accessible internally
  - [ ] Prometheus scraping configured (optional)
  - [ ] Grafana dashboards (optional)

## Deployment Steps

### 1. Clone and Build

```bash
git clone <repo>
cd relayer
npm install
npm run build
```

### 2. Configure Environment

```bash
cp .env.example .env
# Edit .env with production values
chmod 600 .env
```

### 3. Setup TLS Certificates

```bash
# Using Let's Encrypt
sudo certbot certonly --standalone -d relayer.yourdomain.com
mkdir -p certs
sudo cp /etc/letsencrypt/live/relayer.yourdomain.com/fullchain.pem certs/
sudo cp /etc/letsencrypt/live/relayer.yourdomain.com/privkey.pem certs/
sudo chown $USER:$USER certs/*
```

### 4. Start Services

```bash
# Using Docker Compose
docker-compose --profile production up -d

# Or manually
redis-server &
npm start &
nginx -c $(pwd)/nginx.conf
```

### 5. Verify Deployment

```bash
# Health check
curl https://relayer.yourdomain.com/health

# Info endpoint
curl https://relayer.yourdomain.com/info

# Test withdrawal (with API key)
curl -X POST https://relayer.yourdomain.com/withdraw \
  -H "Authorization: Bearer <API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"poolAddress": "...", ...}'
```

## Post-Deployment Monitoring

### Daily Checks

- [ ] Relayer balance sufficient
- [ ] Queue depth normal
- [ ] Success rate acceptable
- [ ] No unusual error patterns

### Weekly Tasks

- [ ] Review metrics trends
- [ ] Check certificate expiration
- [ ] Review access logs
- [ ] Update dependencies if needed

## Rollback Plan

If issues occur:

1. **Stop accepting new requests:**
   ```bash
   # Remove from load balancer or
   docker-compose stop relayer
   ```

2. **Allow in-flight jobs to complete:**
   ```bash
   # Wait for pending jobs (check Redis)
   redis-cli KEYS 'psol:relayer:pending:*' | wc -l
   ```

3. **Rollback to previous version:**
   ```bash
   git checkout <previous-tag>
   npm install && npm run build
   docker-compose up -d
   ```

4. **Investigate and fix:**
   - Check logs: `docker-compose logs relayer`
   - Check Redis: `redis-cli MONITOR`
   - Check RPC: test Solana connectivity

## Emergency Contacts

- Relayer Operator: _______________
- pSol Protocol Team: _______________
- RPC Provider Support: _______________

---

**Last Updated:** _______________
**Deployed By:** _______________
**Version:** 1.0.0
