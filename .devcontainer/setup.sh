#!/usr/bin/env bash

# Setup script for the flo devcontainer.

set -euo pipefail

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

echo "==> rustup: wasm32 target"
rustup target add wasm32-unknown-unknown

echo "==> rustup: fmt + clippy components"
rustup component add rustfmt clippy 2>/dev/null || true

has_bin() { command -v "$1" >/dev/null 2>&1; }

if ! has_bin just; then
  echo "==> installing just"
  curl -LsSf https://just.systems/install.sh | sh -s -- --to "${HOME}/.local/bin"
fi

if ! has_bin cargo-deny; then
  echo "==> installing cargo-deny"
  curl -LsSf https://raw.githubusercontent.com/EmbarkStudios/cargo-deny/main/install-cargo-deny.sh \
    | sh -s -- --to "${HOME}/.local/bin"
fi

if ! has_bin wasm-pack; then
  echo "==> installing wasm-pack"
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

echo "==> toolchain versions"
rustc --version
cargo --version
wasm-pack --version
just --version
cargo-deny --version
node --version
python3 --version
echo "==> flo devcontainer ready"