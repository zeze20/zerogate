#!/usr/bin/env bash
set -euo pipefail

cmd="${1:-shell}"

case "$cmd" in
  build)
    docker compose build dev
    ;;

  shell)
    docker compose run --rm dev
    ;;

  check)
    docker compose run --rm dev bash -lc '
      cargo fmt --all -- --check
      cargo check --workspace
      cargo clippy --workspace --all-targets -- -D warnings
      cargo test --workspace
      ./scripts/audit_no_unsafe.sh
    '
    ;;

  ebpf)
    docker compose run --rm ebpf
    ;;

  verifier)
    docker compose run --rm verifier
    ;;

  clean)
    docker compose down
    ;;

  *)
    echo "Usage: $0 {build|shell|check|ebpf|verifier|clean}"
    exit 1
    ;;
esac
