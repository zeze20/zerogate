#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ARTIFACT_DIR="$REPO_ROOT/artifacts"
mkdir -p "$ARTIFACT_DIR"

VERIFIER_LOG="$ARTIFACT_DIR/verifier.log"

echo "== ZeroGate eBPF verifier/load smoke test =="

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "ERROR: eBPF verifier/load test requires Linux."
  exit 1
fi

if ! command -v bpftool >/dev/null 2>&1; then
  echo "ERROR: bpftool is required."
  echo "Install with your distro package manager, e.g. apt install linux-tools-common linux-tools-generic"
  exit 1
fi

if [[ "${EUID}" -ne 0 ]]; then
  echo "ERROR: verifier/load test should be run as root or with required BPF capabilities."
  echo "Required capabilities typically include CAP_BPF and CAP_NET_ADMIN."
  exit 1
fi

./scripts/build_ebpf.sh

echo "Searching for eBPF object..."
OBJ="$(find target/bpfel-unknown-none -type f \( -name '*.o' -o -name '*.elf' \) | head -n 1 || true)"

if [[ -z "$OBJ" ]]; then
  echo "WARNING: No .o or .elf object found under target/bpfel-unknown-none."
  echo "The Rust/cargo eBPF build produces an ELF binary, not a .o file."
  echo "Searching for any ELF binary..."
  OBJ="$(find target/bpfel-unknown-none -type f -executable | head -n 1 || true)"

  if [[ -z "$OBJ" ]]; then
    OBJ="$(find target/bpfel-unknown-none/debug -type f -name 'zerogate-ebpf' 2>/dev/null | head -n 1 || true)"
  fi

  if [[ -z "$OBJ" ]]; then
    echo "ERROR: no eBPF object found under target/bpfel-unknown-none"
    echo "Expected path: target/bpfel-unknown-none/debug/zerogate-ebpf"
    echo "TODO: If the build system produces a different artifact path, update this script."
    exit 1
  fi
fi

echo "Found eBPF object: $OBJ"
echo "Attempting verifier load..."

set +e
bpftool prog load "$OBJ" /sys/fs/bpf/zerogate_verifier_smoke 2>"$VERIFIER_LOG"
STATUS=$?
set -e

if [[ "$STATUS" -ne 0 ]]; then
  echo "ERROR: BPF verifier rejected the program."
  echo "Verifier log:"
  cat "$VERIFIER_LOG" || true
  echo ""
  echo "NOTE: The current build produces a Rust ELF binary which may not have"
  echo "the correct BPF program section naming (e.g. SEC(\"xdp\")) expected by bpftool."
  echo "A future MR may introduce an Aya-based loader or explicit section annotations."
  exit "$STATUS"
fi

echo "BPF verifier load succeeded."

rm -f /sys/fs/bpf/zerogate_verifier_smoke || true

echo "Verifier log written to: $VERIFIER_LOG"
