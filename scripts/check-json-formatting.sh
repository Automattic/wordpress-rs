#!/bin/bash
#
# Check that JSON test fixtures are pretty-printed (2-space indent via jq).
# Usage:
#   ./scripts/check-json-formatting.sh          # check mode (exit 1 on failure)
#   ./scripts/check-json-formatting.sh --fix    # rewrite files in place

set -euo pipefail

fix=false
if [[ "${1:-}" == "--fix" ]]; then
    fix=true
fi

failed=false

while IFS= read -r -d '' file; do
    # Strip BOM if present — jq handles it, but we want consistent output
    contents=$(sed '1s/^\xef\xbb\xbf//' "$file")

    formatted=$(echo "$contents" | jq .) || {
        echo "Invalid JSON: $file"
        failed=true
        continue
    }

    actual=$(cat "$file")
    if [[ "$actual" != "$formatted" ]]; then
        if $fix; then
            echo "$formatted" > "$file"
            echo "Fixed: $file"
        else
            echo "Not properly formatted: $file"
            failed=true
        fi
    fi
done < <(find wp_api/tests test-data -name '*.json' -print0 | sort -z)

if $failed; then
    echo
    echo "Some JSON files are not properly formatted."
    echo "Run 'make fmt-json' to fix them."
    exit 1
elif ! $fix; then
    echo "All JSON test fixtures are properly formatted."
fi
