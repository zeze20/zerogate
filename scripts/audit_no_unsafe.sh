#!/bin/bash
# audit_no_unsafe.sh — Fails if "unsafe" appears outside allowed modules.
#
# Allowed unsafe locations:
#   - zerogate-ebpf/src/parser.rs
#   - zerogate-agent/src/umem.rs
#   - zerogate-agent/src/sys.rs
#
# Usage: ./scripts/audit_no_unsafe.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

ALLOWED_FILES=(
    "zerogate-ebpf/src/parser.rs"
    "zerogate-agent/src/umem.rs"
    "zerogate-agent/src/sys.rs"
)

# Build grep exclusion pattern.
EXCLUDE_ARGS=()
for f in "${ALLOWED_FILES[@]}"; do
    EXCLUDE_ARGS+=("--exclude=${f##*/}")
done

# Search all .rs files for "unsafe" outside allowed locations.
VIOLATIONS=0

while IFS= read -r file; do
    # Get path relative to repo root.
    rel="$(realpath --relative-to="$REPO_ROOT" "$file" 2>/dev/null || echo "$file")"

    # Check if file is in the allowed list.
    allowed=false
    for a in "${ALLOWED_FILES[@]}"; do
        # Normalize path separators.
        norm_rel="${rel//\\//}"
        norm_a="${a//\\//}"
        if [[ "$norm_rel" == "$norm_a" ]] || [[ "$norm_rel" == *"/$norm_a" ]]; then
            allowed=true
            break
        fi
    done

    if $allowed; then
        continue
    fi

    # Skip test code (cfg(test) modules).
    # We only flag non-test unsafe.
    if grep -n '\bunsafe\b' "$file" | grep -v '//.*unsafe' | grep -v '#\[cfg(test)\]' | grep -v 'cfg(not(test))' | grep -qv '^\s*$'; then
        matches=$(grep -n '\bunsafe\b' "$file" | grep -v '//.*unsafe' | grep -v 'mod tests' || true)
        if [ -n "$matches" ]; then
            echo "VIOLATION: $rel contains 'unsafe':"
            echo "$matches" | head -5
            echo ""
            VIOLATIONS=$((VIOLATIONS + 1))
        fi
    fi
done < <(find "$REPO_ROOT" -name "*.rs" -not -path "*/target/*" -not -path "*/.cargo/*")

if [ "$VIOLATIONS" -gt 0 ]; then
    echo "FAILED: Found $VIOLATIONS file(s) with unsafe outside allowed modules."
    echo "Allowed modules: ${ALLOWED_FILES[*]}"
    exit 1
fi

echo "PASSED: No unsafe found outside allowed modules."
exit 0
