# pSol Relayer - Privacy-Preserving Withdrawal Service

A hardened, mainnet-ready relayer service for the pSol Privacy Pool protocol.

## Overview

The relayer enables private withdrawals by:
1. Accepting ZK proof submissions from users
2. Validating proofs locally (optional) and on-chain
3. Submitting withdrawal transactions on behalf of users
4. Charging a small fee for the service

**Without a relayer, privacy is broken:** Users would pay gas from their withdrawal address, linking deposits to withdrawals on-chain.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Request                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Nginx (Rate Limiting)                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Express API Server                         │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┐ │
│  │   Auth MW    │  Rate Limit  │  Validation  │   Metrics    │ │
│  └──────────────┴──────────────┴──────────────┴──────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    BullMQ Job Queue                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Redis Persistence                      │  │
│  │   - Job Store (survives restarts)                        │  │
│  │   - Rate Limit Counters                                  │  │
│  │   - Nullifier Deduplication                              │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Worker Process                               │
│  1. Validate proof (snarkjs)                                   │
│  2. Check nullifier not spent                                  │
│  3. Verify merkle root                                         │
│  4. Build and submit transaction                               │
│  5. Wait for confirmation                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Solana RPC                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Node.js 18+
- Redis 6+
- Solana CLI (for key generation)

### 1. Clone and Install

```bash
git clone <repo>
cd relayer
npm install
```

### 2. Generate Relayer Keypair

```bash
# Generate a new keypair (hot wallet - use limited funds!)
solana-keygen new --no-bip39-passphrase -o relayer-keypair.json

# Get base58 private key
cat relayer-keypair.json | node -e "
  const fs = require('fs');
  const bs58 = require('bs58');
  const data = JSON.parse(fs.readFileSync('/dev/stdin', 'utf-8'));
  console.log(bs58.encode(Buffer.from(data)));
"
```

### 3. Configure Environment

```bash
cp .env.example .env
# Edit .env with your settings
```

**Minimum required:**
- `SOLANA_RPC_URL` - Your RPC endpoint
- `PSOL_PROGRAM_ID` - pSol program address
- `RELAYER_PRIVATE_KEY` - Base58 encoded keypair

### 4. Fund Relayer Wallet

```bash
# Devnet
solana airdrop 2 <RELAYER_ADDRESS> --url devnet

# Mainnet - Transfer SOL to relayer address
```

### 5. Start Services

```bash
# Development
npm run dev

# Production
npm run build
npm start

# Docker
docker-compose up -d
```

## API Reference

### Health & Info

```
GET /health
```
Returns relayer health status, balance, and Redis connectivity.

```
GET /info
```
Returns relayer address, fee configuration, and statistics.

### Metrics

```
GET /metrics
GET /metrics?format=prometheus
```
Returns metrics in JSON or Prometheus format.

### Fee Quote

```
POST /fee/quote
Content-Type: application/json

{
  "poolAddress": "...",
  "amount": "1000000000"
}
```

### Submit Withdrawal

```
POST /withdraw
Authorization: Bearer <API_KEY>
Content-Type: application/json

{
  "poolAddress": "...",
  "tokenMint": "...",
  "proofData": "...",        // Hex encoded
  "merkleRoot": "...",       // 64 hex chars
  "nullifierHash": "...",    // 64 hex chars
  "recipient": "...",
  "amount": "...",
  "relayerFee": "..."
}
```

Response:
```json
{
  "success": true,
  "data": {
    "jobId": "...",
    "status": "queued",
    "estimatedTime": 30
  },
  "meta": {
    "requestId": "...",
    "timestamp": 1234567890
  }
}
```

### Check Job Status

```
GET /withdraw/:jobId
```

Response:
```json
{
  "success": true,
  "data": {
    "jobId": "...",
    "status": "succeeded",
    "txSignature": "...",
    "createdAt": 1234567890,
    "updatedAt": 1234567890
  }
}
```

### Validate Nullifier

```
POST /validate/nullifier
Content-Type: application/json

{
  "poolAddress": "...",
  "nullifierHash": "..."
}
```

## Configuration Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SOLANA_RPC_URL` | Yes | - | Solana RPC endpoint |
| `PSOL_PROGRAM_ID` | Yes | - | pSol program address |
| `RELAYER_PRIVATE_KEY` | Yes | - | Base58 encoded keypair |
| `REDIS_HOST` | No | localhost | Redis host |
| `BASE_FEE_BPS` | No | 50 | Fee in basis points (0.5%) |
| `REQUIRE_AUTH_FOR_WRITE` | No | false | Require API key for POST |
| `RELAYER_API_KEYS` | No | - | Comma-separated API keys |

See `.env.example` for full configuration options.

## Security Checklist

### Before Mainnet Deployment

- [ ] **Generate dedicated hot wallet** - Don't use your main wallet
- [ ] **Use secrets manager** - Never commit `RELAYER_PRIVATE_KEY` to git
- [ ] **Configure API keys** - Set `REQUIRE_AUTH_FOR_WRITE=true`
- [ ] **Set IP allowlist** - If only serving your own frontend
- [ ] **Enable TLS** - Use Nginx with SSL certificates
- [ ] **Configure alerts** - Monitor balance and error rates
- [ ] **Test on devnet first** - Verify everything works
- [ ] **Review rate limits** - Tune for expected traffic

### Sensitive Data Handling

The relayer is designed to protect privacy:

- **Proof data is never logged** - Only truncated nullifier hashes
- **Private keys are loaded once** - Never printed or serialized
- **Client IPs are hashed** - In production logs
- **API keys are compared timing-safe** - Prevents timing attacks

### Network Security

1. **Firewall rules:**
   - Only expose ports 80/443 (through Nginx)
   - Block direct access to port 3000
   - Allow Redis only from local network

2. **Redis security:**
   - Set a password (`REDIS_PASSWORD`)
   - Bind to localhost or Docker network only
   - Enable persistence for job data

## Production Deployment

### Docker Compose

```bash
# Start all services
docker-compose up -d

# With Nginx (requires TLS certs)
docker-compose --profile production up -d

# Scale relayer workers
docker-compose up -d --scale relayer=3
```

### TLS Certificates

```bash
# Create certs directory
mkdir -p certs

# Option 1: Let's Encrypt (recommended)
certbot certonly --standalone -d relayer.yourdomain.com
cp /etc/letsencrypt/live/relayer.yourdomain.com/fullchain.pem certs/
cp /etc/letsencrypt/live/relayer.yourdomain.com/privkey.pem certs/

# Option 2: Self-signed (testing only)
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout certs/privkey.pem -out certs/fullchain.pem
```

### Monitoring

1. **Health checks:**
   ```bash
   curl http://localhost:3000/health
   ```

2. **Prometheus metrics:**
   ```bash
   curl http://localhost:3000/metrics?format=prometheus
   ```

3. **Key metrics to monitor:**
   - `psol_relayer_jobs_failed_total` - Failure rate
   - `psol_relayer_relayer_balance_sol` - Balance alerts
   - `psol_relayer_pending_jobs` - Queue depth
   - `psol_relayer_processing_time_p95_ms` - Latency

## Troubleshooting

### Common Issues

**"Redis connection error"**
- Check Redis is running: `redis-cli ping`
- Verify `REDIS_HOST` and `REDIS_PORT`

**"Relayer balance below minimum"**
- Fund the relayer wallet with SOL
- Check balance: `solana balance <ADDRESS>`

**"Invalid API key"**
- API keys must be 32+ characters
- Check `RELAYER_API_KEYS` format (comma-separated)

**"Nullifier already spent"**
- The deposit has already been withdrawn
- Check on-chain state

**"Rate limited"**
- Wait and retry after `Retry-After` header
- Or configure higher limits

### Logs

```bash
# Docker logs
docker-compose logs -f relayer

# Log levels: trace, debug, info, warn, error
LOG_LEVEL=debug npm run dev
```

## Development

```bash
# Install dependencies
npm install

# Run in development mode (hot reload)
npm run dev

# Build for production
npm run build

# Run tests
npm test

# Lint
npm run lint
```

## License

MIT

## Support

- GitHub Issues: [link]
- Discord: [link]
- Documentation: [link]
