#!/bin/bash
#
# Validates that UniFFI checksums embedded in the Kotlin bindings JAR match
# the checksum functions exported by each Android ABI's .so file.
#
# Usage:
#   # From published S3 artifacts (by version string):
#   ./validate-uniffi-checksums.sh trunk-9edcee430afd18d7d440baf497a763f9b1bb83d9
#
#   # From local build artifacts (Buildkite CI):
#   ./validate-uniffi-checksums.sh --local
#
# Requirements: curl, unzip, llvm-objdump, python3

set -euo pipefail

S3_BASE="https://a8c-libs.s3.amazonaws.com/android/rs/wordpress/api"
ABIS=("armeabi-v7a" "arm64-v8a" "x86" "x86_64")
WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

# --- Helpers ---

find_objdump() {
    for candidate in \
        llvm-objdump \
        /Library/Developer/CommandLineTools/usr/bin/llvm-objdump \
        /usr/bin/llvm-objdump \
        "$(xcrun --find llvm-objdump 2>/dev/null || true)"; do
        if [ -n "$candidate" ] && command -v "$candidate" &>/dev/null; then
            echo "$candidate"
            return
        fi
    done
    echo "ERROR: llvm-objdump not found" >&2
    exit 1
}

find_nm() {
    for candidate in \
        llvm-nm \
        /Library/Developer/CommandLineTools/usr/bin/llvm-nm \
        /usr/bin/llvm-nm \
        "$(xcrun --find llvm-nm 2>/dev/null || true)"; do
        if [ -n "$candidate" ] && command -v "$candidate" &>/dev/null; then
            echo "$candidate"
            return
        fi
    done
    echo "ERROR: llvm-nm not found" >&2
    exit 1
}

OBJDUMP=$(find_objdump)
NM=$(find_nm)

extract_kotlin_checksums() {
    local jar_or_dir="$1"
    local out_file="$2"

    # Extract checksum expectations from Kotlin source:
    #   if (lib.uniffi_wp_api_checksum_func_foo() != 12345.toShort()) {
    # Output: function_name expected_value (sorted)
    if [ -d "$jar_or_dir" ]; then
        find "$jar_or_dir" -name "*.kt" -print0 | xargs -0 cat
    else
        unzip -p "$jar_or_dir" '*.kt' 2>/dev/null
    fi | python3 -c "
import re, sys
pattern = re.compile(r'lib\.(uniffi_\w*checksum_\w+)\(\)\s*!=\s*(\d+)\.toShort\(\)')
for line in sys.stdin:
    for m in pattern.finditer(line):
        print(f'{m.group(1)} {m.group(2)}')
" | sort > "$out_file"
}

# Maps ABI name to the llvm-objdump --triple needed for correct disassembly.
# armv7 .so files contain Thumb2 code, which llvm-objdump misinterprets as
# ARM mode unless told otherwise, producing garbage disassembly.
objdump_triple_for_abi() {
    case "$1" in
        armeabi-v7a) echo "thumbv7-linux-androideabi" ;;
        arm64-v8a)   echo "aarch64-linux-android" ;;
        x86)         echo "i686-linux-android" ;;
        x86_64)      echo "x86_64-linux-android" ;;
        *)           echo "" ;;
    esac
}

extract_so_checksums() {
    local so_file="$1"
    local out_file="$2"
    local abi="${3:-}"

    # Get checksum function names from dynamic symbol table (survives stripping).
    local symbols
    symbols=$("$NM" -D "$so_file" 2>/dev/null | grep 'checksum' | awk '{print $3}' | sort)

    if [ -z "$symbols" ]; then
        # Fall back to regular symbol table (unstripped local builds)
        symbols=$("$NM" "$so_file" 2>/dev/null | grep 'checksum' | awk '{print $NF}' | sed 's/^_//' | sort)
    fi

    if [ -z "$symbols" ]; then
        : > "$out_file"
        return
    fi

    # Determine the right triple for correct disassembly
    local triple_args=()
    if [ -n "$abi" ]; then
        local triple
        triple=$(objdump_triple_for_abi "$abi")
        if [ -n "$triple" ]; then
            triple_args=(--triple="$triple")
        fi
    fi

    # Disassemble checksum functions in batches to avoid command-line length limits.
    local script_dir
    script_dir="$(cd "$(dirname "$0")" && pwd)"
    local batch_size=200
    local sym_array
    IFS=$'\n' read -r -d '' -a sym_array <<< "$symbols" || true

    : > "$out_file.raw"
    local i=0
    while [ "$i" -lt "${#sym_array[@]}" ]; do
        local batch=()
        local j=0
        while [ "$j" -lt "$batch_size" ] && [ "$((i + j))" -lt "${#sym_array[@]}" ]; do
            batch+=("${sym_array[$((i + j))]}")
            j=$((j + 1))
        done
        local sym_list
        sym_list=$(IFS=,; echo "${batch[*]}")

        "$OBJDUMP" -d "${triple_args[@]}" --disassemble-symbols="$sym_list" "$so_file" 2>/dev/null \
        >> "$out_file.raw"

        i=$((i + batch_size))
    done

    python3 "$script_dir/parse-uniffi-checksums.py" < "$out_file.raw" | sort > "$out_file"
    rm -f "$out_file.raw"
}

compare_checksums() {
    local kotlin_file="$1"
    local so_file="$2"
    local abi="$3"

    local kotlin_count so_count
    kotlin_count=$(wc -l < "$kotlin_file" | tr -d ' ')
    so_count=$(wc -l < "$so_file" | tr -d ' ')

    if [ "$kotlin_count" -eq 0 ]; then
        echo "  ERROR: No checksums extracted from Kotlin bindings"
        return 1
    fi
    if [ "$so_count" -eq 0 ]; then
        echo "  ERROR: No checksums extracted from $abi .so"
        return 1
    fi

    # UniFFI generates checksum functions in the .so for record field accessors
    # that the Kotlin side doesn't validate. Extra functions in the .so are
    # harmless. What matters for the runtime crash is:
    #   1. Functions Kotlin expects that are MISSING from the .so
    #   2. Functions present in BOTH with DIFFERENT values
    local kotlin_names so_names
    kotlin_names=$(awk '{print $1}' "$kotlin_file")
    so_names=$(awk '{print $1}' "$so_file")

    local missing_from_so
    missing_from_so=$(comm -23 <(echo "$kotlin_names") <(echo "$so_names"))
    local missing_count
    missing_count=$(echo "$missing_from_so" | grep -c . || true)

    local extra_in_so
    extra_in_so=$(comm -13 <(echo "$kotlin_names") <(echo "$so_names") | wc -l | tr -d ' ')

    # Check value mismatches on shared functions
    local common_names
    common_names=$(comm -12 <(echo "$kotlin_names") <(echo "$so_names"))
    local value_mismatches=0
    local value_diff=""
    if [ -n "$common_names" ]; then
        value_diff=$(
            diff \
                <(echo "$common_names" | while read -r name; do grep "^$name " "$kotlin_file"; done) \
                <(echo "$common_names" | while read -r name; do grep "^$name " "$so_file"; done) \
            || true
        )
        if [ -n "$value_diff" ]; then
            value_mismatches=$(echo "$value_diff" | grep -c '^[<>]' || true)
            value_mismatches=$((value_mismatches / 2))
        fi
    fi
    local common_count
    common_count=$(echo "$common_names" | grep -c . || true)
    local matched_count=$((common_count - value_mismatches))

    local failed=0
    if [ "$missing_count" -gt 0 ] || [ "$value_mismatches" -gt 0 ]; then
        failed=1
    fi

    if [ "$failed" -eq 0 ]; then
        echo "  $abi: OK ($matched_count/$kotlin_count checksums verified, $extra_in_so extra in .so)"
        return 0
    else
        echo "  $abi: FAILED"
        if [ "$missing_count" -gt 0 ]; then
            echo "    Functions in Kotlin but missing from .so: $missing_count"
            echo "$missing_from_so" | head -10 | sed 's/^/      /'
        fi
        if [ "$value_mismatches" -gt 0 ]; then
            echo "    Value mismatches on shared functions: $value_mismatches"
            echo "$value_diff" | head -20 | sed 's/^/      /'
        fi
        if [ "$extra_in_so" -gt 0 ]; then
            echo "    (Also $extra_in_so extra functions in .so — these are harmless)"
        fi
        return 1
    fi
}

# --- Main ---

MODE="s3"
VERSION=""

if [ "${1:-}" = "--local" ]; then
    MODE="local"
else
    VERSION="${1:?Usage: $0 <version> or $0 --local}"
fi

echo "=== UniFFI Checksum Validation ==="
echo ""

FAILED=0

if [ "$MODE" = "s3" ]; then
    echo "Downloading artifacts for version: $VERSION"

    KOTLIN_JAR="$WORK_DIR/kotlin-sources.jar"
    AAR_FILE="$WORK_DIR/android.aar"

    curl -sf "$S3_BASE/kotlin/$VERSION/kotlin-$VERSION-sources.jar" -o "$KOTLIN_JAR" \
        || { echo "ERROR: Failed to download Kotlin sources JAR"; exit 1; }
    curl -sf "$S3_BASE/android/$VERSION/android-$VERSION.aar" -o "$AAR_FILE" \
        || { echo "ERROR: Failed to download Android AAR"; exit 1; }

    echo "Extracting Kotlin checksums from sources JAR..."
    extract_kotlin_checksums "$KOTLIN_JAR" "$WORK_DIR/kotlin_checksums.txt"

    echo "Extracting .so files from AAR..."
    unzip -q "$AAR_FILE" 'jni/*' -d "$WORK_DIR/aar" 2>/dev/null

    echo ""
    echo "Comparing checksums per ABI:"
    for abi in "${ABIS[@]}"; do
        SO_FILE="$WORK_DIR/aar/jni/$abi/libwp_mobile.so"
        if [ ! -f "$SO_FILE" ]; then
            echo "  $abi: SKIPPED (not in AAR)"
            continue
        fi
        extract_so_checksums "$SO_FILE" "$WORK_DIR/so_checksums_${abi}.txt" "$abi"
        compare_checksums "$WORK_DIR/kotlin_checksums.txt" "$WORK_DIR/so_checksums_${abi}.txt" "$abi" || FAILED=1
    done

elif [ "$MODE" = "local" ]; then
    REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
    KOTLIN_DIR="$REPO_ROOT/native/kotlin/api/kotlin/build/generated/source/uniffi/java"
    JNI_DIR="$REPO_ROOT/native/kotlin/api/android/build/rustJniLibs/android"

    echo "Extracting Kotlin checksums from generated sources..."
    extract_kotlin_checksums "$KOTLIN_DIR" "$WORK_DIR/kotlin_checksums.txt"

    echo ""
    echo "Comparing checksums per ABI:"
    for abi in "${ABIS[@]}"; do
        SO_FILE="$JNI_DIR/$abi/libwp_mobile.so"
        if [ ! -f "$SO_FILE" ]; then
            echo "  $abi: SKIPPED (not built)"
            continue
        fi
        extract_so_checksums "$SO_FILE" "$WORK_DIR/so_checksums_${abi}.txt" "$abi"
        compare_checksums "$WORK_DIR/kotlin_checksums.txt" "$WORK_DIR/so_checksums_${abi}.txt" "$abi" || FAILED=1
    done
fi

echo ""
if [ "$FAILED" -ne 0 ]; then
    echo "FAILED: UniFFI checksum mismatches detected!"
    exit 1
else
    TOTAL=$(wc -l < "$WORK_DIR/kotlin_checksums.txt" | tr -d ' ')
    echo "PASSED: All $TOTAL checksums match across all ABIs."
    exit 0
fi
