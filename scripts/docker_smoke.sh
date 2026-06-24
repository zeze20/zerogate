#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[docker-smoke] repo root: $ROOT_DIR"

echo "[docker-smoke] docker version"
docker version

echo "[docker-smoke] docker compose version"
docker compose version

echo "[docker-smoke] compose services"
docker compose config --services

echo "[docker-smoke] build dev"
docker compose build dev

echo "[docker-smoke] cargo check"
docker compose run --rm dev cargo check --workspace

echo "[docker-smoke] cargo test"
docker compose run --rm dev cargo test --workspace

if [[ "${RUN_AUDIT:-0}" == "1" ]]; then
  echo "[docker-smoke] unsafe audit"
  docker compose run --rm dev ./scripts/audit_no_unsafe.sh
else
  echo "[docker-smoke] unsafe audit skipped"
  echo "[docker-smoke] run with: RUN_AUDIT=1 ./scripts/docker_smoke.sh"
fi

echo "[docker-smoke] build ebpf"
docker compose build ebpf

echo "[docker-smoke] run eBPF build"
docker compose run --rm ebpf ./scripts/build_ebpf.sh

if [[ "${RUN_VERIFIER:-0}" == "1" ]]; then
  echo "[docker-smoke] run verifier"
  docker compose run --rm verifier ./scripts/verify_ebpf.sh
else
  echo "[docker-smoke] verifier skipped"
  echo "[docker-smoke] run with: RUN_VERIFIER=1 ./scripts/docker_smoke.sh"
fi

echo "[docker-smoke] done"
