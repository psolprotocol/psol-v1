require('dotenv').config();

const rawBs58 = require('bs58');
const { Keypair } = require('@solana/web3.js');

// Match config.ts import style as close as possible
const bs58 = rawBs58.default || rawBs58;

const v = process.env.RELAYER_PRIVATE_KEY;

console.log('ENV key length:', v ? v.length : 'MISSING');

try {
  const decoded = bs58.decode(v);
  console.log('decoded length:', decoded.length);
  const kp = Keypair.fromSecretKey(decoded);
  console.log('Keypair.fromSecretKey OK, pubkey =', kp.publicKey.toBase58());
} catch (e) {
  console.error('TEST SCRIPT FAILED:', e);
}
