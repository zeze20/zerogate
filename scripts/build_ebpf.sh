#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "== ZeroGate eBPF build =="
echo "Repo: $REPO_ROOT"

if ! command -v rustup >/dev/null 2>&1; then
  echo "ERROR: rustup is required for eBPF build."
  exit 1
fi

if ! rustup toolchain list | grep -q '^nightly'; then
  echo "ERROR: Rust nightly toolchain is required."
  echo "Install with: rustup toolchain install nightly"
  exit 1
fi

if ! command -v bpf-linker >/dev/null 2>&1; then
  echo "ERROR: bpf-linker is required for eBPF builds."
  echo "Install with: cargo +nightly install bpf-linker"
  echo "Note: bpf-linker requires Linux and LLVM development libraries."
  exit 1
fi

rustup component add rust-src --toolchain nightly

echo "Building zerogate-ebpf for bpfel-unknown-none..."

cargo +nightly build \
  -Z build-std=core \
  --target bpfel-unknown-none \
  -p zerogate-ebpf

echo "eBPF build completed."
echo "Artifacts are under target/bpfel-unknown-none/"
