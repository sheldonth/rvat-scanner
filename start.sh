#!/opt/homebrew/bin/bash

check_bin() {
  local binary_name=$1
  if ! command -v "$binary_name" >/dev/null 2>&1; then
    echo "Binary '$binary_name' is missing."
    exit 1
  fi
}

check_bin "node"

node build_cache.js
cargo run --release

