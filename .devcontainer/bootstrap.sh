#!/bin/bash
set -e

echo ">>> Updating system"
sudo apt update -y
sudo apt install -y build-essential pkg-config libssl-dev curl git cmake

echo ">>> Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustup default stable

echo ">>> Installing Solana 1.18.20"
sh -c "$(curl -sSfL https://release.solana.com/v1.18.20/install)"

echo ">>> Installing Anchor 0.30.1"
cargo install --git https://github.com/coral-xyz/anchor --tag v0.30.1 anchor-cli --locked

echo ">>> Cleaning old Solana caches"
rm -rf ~/.cache/solana || true

echo ">>> Verifying installs"
solana --version
anchor --version
rustc --version
cargo-build-sbf --version

echo ">>> Environment setup complete"
