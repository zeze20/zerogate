#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

ALLOWED_FILES=(
  "zerogate-ebpf/src/parser.rs"
  "zerogate-agent/src/umem.rs"
  "zerogate-agent/src/sys.rs"
)

is_allowed_file() {
  local rel="$1"
  local allowed
  for allowed in "${ALLOWED_FILES[@]}"; do
    if [[ "$rel" == "$allowed" ]]; then
      return 0
    fi
  done
  return 1
}

UNSAFE_PATTERN='(^|[^A-Za-z0-9_])unsafe[[:space:]]*({|fn|impl|trait)([^A-Za-z0-9_]|$)'

violations=0

while IFS= read -r file; do
  rel="$(realpath --relative-to="$REPO_ROOT" "$file" 2>/dev/null || python3 -c 'import os,sys; print(os.path.relpath(sys.argv[1], sys.argv[2]))' "$file" "$REPO_ROOT")"

  if is_allowed_file "$rel"; then
    continue
  fi

  while IFS=: read -r line_no line; do
    line_without_comment="${line%%//*}"
    if [[ "$line_without_comment" =~ $UNSAFE_PATTERN ]]; then
      echo "unsafe violation: ${rel}:${line_no}: ${line}"
      violations=$((violations + 1))
    fi
  done < <(grep -nE 'unsafe[[:space:]]*({|fn|impl|trait)' "$file" || true)

done < <(find "$REPO_ROOT" \
  -type f \
  -name '*.rs' \
  -not -path '*/target/*' \
  -not -path '*/.cargo/*' \
  | sort)

if [[ "$violations" -gt 0 ]]; then
  echo "FAILED: found $violations unsafe occurrence(s) outside allowed modules."
  echo "Allowed modules:"
  printf '  - %s\n' "${ALLOWED_FILES[@]}"
  exit 1
fi

echo "PASSED: No unsafe found outside allowed modules."
exit 0
