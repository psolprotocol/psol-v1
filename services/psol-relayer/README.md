pSol Relayer

A hardened withdrawal relayer for the pSol privacy pool.
It submits withdrawals on behalf of users so their identity and wallet activity stay unlinkable on-chain.

Purpose

A withdrawal without a relayer leaks privacy because users must pay gas from the withdrawing wallet.
This relayer solves that by:

Receiving user ZK proofs

Verifying the proof and merkle root

Checking the nullifier is unused

Preparing and submitting the withdrawal transaction

Charging a configurable fee

The relayer never learns secrets. It only receives already-generated proofs.

Architecture
Client → Nginx → Express API → BullMQ → Redis → Worker → Solana RPC


Key components:

Express API: public endpoints, optional API key auth

BullMQ + Redis: queue for job processing and persistence

Worker: proof verification, nullifier checks, transaction execution

Nginx (optional): TLS termination and rate limiting

Setup
Requirements

Node.js 18 or newer

Redis 6 or newer

Solana CLI for keypair creation

A dedicated relayer wallet

1. Install
git clone <repo>
cd services/psol-relayer
npm install

2. Create relayer wallet
solana-keygen new --no-bip39-passphrase -o relayer-keypair.json


Convert to base58:

cat relayer-keypair.json | node -e "
  const fs=require('fs');
  const bs58=require('bs58');
  const k=JSON.parse(fs.readFileSync(0,'utf8'));
  console.log(bs58.encode(Buffer.from(k)));
"


Set this value in .env.

3. Configure environment
cp .env.example .env


Minimum required:

SOLANA_RPC_URL=...
PSOL_PROGRAM_ID=...
RELAYER_PRIVATE_KEY=...

4. Fund the relayer wallet

Devnet:

solana airdrop 2 RELAYER_ADDRESS --url devnet


Mainnet: transfer SOL manually.

5. Start

Development:

npm run dev


Production:

npm run build
npm start


Docker:

docker-compose up -d

API
Health
GET /health


Connectivity, balance, redis state.

Info
GET /info


Address, fee settings, and configuration.

Fee quote
POST /fee/quote
{
  "poolAddress": "...",
  "amount": "1000000000"
}

Submit withdrawal
POST /withdraw
Authorization: Bearer <API_KEY>
{
  "poolAddress": "...",
  "tokenMint": "...",
  "proofData": "...",
  "merkleRoot": "...",
  "nullifierHash": "...",
  "recipient": "...",
  "amount": "...",
  "relayerFee": "..."
}


Returns a queued job ID.

Job status
GET /withdraw/:jobId


Returns pending, processing, succeeded, or failed, plus transaction signature if complete.

Nullifier validation
POST /validate/nullifier
{
  "poolAddress": "...",
  "nullifierHash": "..."
}

Environment variables
Variable	Required	Description
SOLANA_RPC_URL	yes	RPC endpoint
PSOL_PROGRAM_ID	yes	Program ID
RELAYER_PRIVATE_KEY	yes	Relayer hot wallet
REDIS_HOST	no	Default: localhost
BASE_FEE_BPS	no	Default: 50 (0.5 percent)
REQUIRE_AUTH_FOR_WRITE	no	Enforce API key
RELAYER_API_KEYS	no	Comma-separated keys

See .env.example for complete list.

Security
Before mainnet

Use a dedicated hot wallet

Store keys in a secrets manager

Enforce API key auth

Run behind Nginx or Cloudflare

Enable TLS

Restrict allowed origins

Protect Redis (password + private network)

Monitor balance and error rates

Test everything on devnet

What is never logged

Proof data

Private keys

Full nullifiers

Client IPs (only hashed in production mode)

Deployment
Docker
docker-compose up -d


Scale workers:

docker-compose up -d --scale relayer=3

TLS

Use real certificates in production.

Example self-signed for testing:

openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
-keyout certs/privkey.pem -out certs/fullchain.pem

Monitoring

Health:

curl http://localhost:3000/health


Prometheus metrics:

curl http://localhost:3000/metrics?format=prometheus


Key metrics:

worker failure rate

relayer wallet balance

job queue depth

transaction confirmation latency

Troubleshooting

Redis errors
Check Redis is running and reachable.

Low balance
Fund the relayer address and monitor regularly.

Nullifier already spent
Withdrawal already executed.

Rate limited
Respect Retry-After or adjust limits.

Invalid API key
Ensure correct comma-separated list in .env.

Logs:

docker-compose logs -f relayer

Development
npm install
npm run dev
npm run build
npm test
npm run lint

License

MIT