#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ -x "./scripts/verify_ebpf_load.sh" ]]; then
  echo "[verify-ebpf] delegating to scripts/verify_ebpf_load.sh"
  exec ./scripts/verify_ebpf_load.sh "$@"
fi

echo "[verify-ebpf] ERROR: scripts/verify_ebpf_load.sh not found or not executable"
exit 1
