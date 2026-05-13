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

    # Build comma-separated symbol list for --disassemble-symbols
    local sym_list
    sym_list=$(echo "$symbols" | tr '\n' ',' | sed 's/,$//')

    # Determine the right triple for correct disassembly
    local triple_args=()
    if [ -n "$abi" ]; then
        local triple
        triple=$(objdump_triple_for_abi "$abi")
        if [ -n "$triple" ]; then
            triple_args=(--triple="$triple")
        fi
    fi

    # Disassemble only the checksum functions and extract return values.
    # Each checksum function is a single immediate load + return:
    #   aarch64: mov w0, #0x1234 ; ret
    #   thumb2:  movw/mov.w/movs r0, #0x1234 ; bx lr
    #   x86:     movl $0x1234, %eax ; retl/retq
    "$OBJDUMP" -d "${triple_args[@]}" --disassemble-symbols="$sym_list" "$so_file" 2>/dev/null \
    | python3 -c "
import re, sys

current_fn = None
for line in sys.stdin:
    # Match function labels: <uniffi_wp_api_checksum_func_foo>:
    fn_match = re.search(r'<_?(uniffi_\w*checksum_\w+)>:', line)
    if fn_match:
        current_fn = fn_match.group(1)
        continue
    if current_fn:
        # ARM/AArch64: #0xNNNN
        m = re.search(r'#(0x[0-9a-fA-F]+)', line)
        if not m:
            # x86 Intel syntax: mov eax, 0xNNNN or movabs rax, 0xNNNN
            m = re.search(r'(?:mov\w*)\s+\w+,\s*(0x[0-9a-fA-F]+)', line)
        if not m:
            # x86 AT&T syntax: movl \$0xNNNN, %eax
            m = re.search(r'\\\$(0x[0-9a-fA-F]+)', line)
        if m:
            val = int(m.group(1), 16)
            print(f'{current_fn} {val}')
            current_fn = None
" | sort > "$out_file"
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

    local diff_output
    diff_output=$(diff "$kotlin_file" "$so_file" || true)

    if [ -z "$diff_output" ]; then
        echo "  $abi: OK ($so_count/$kotlin_count checksums match)"
        return 0
    else
        local only_in_kotlin only_in_so value_mismatches
        only_in_kotlin=$(echo "$diff_output" | grep -c '^< ' || true)
        only_in_so=$(echo "$diff_output" | grep -c '^> ' || true)

        # Separate missing functions from value mismatches
        local kotlin_names so_names
        kotlin_names=$(awk '{print $1}' "$kotlin_file")
        so_names=$(awk '{print $1}' "$so_file")
        local missing_from_so missing_from_kotlin
        missing_from_so=$(comm -23 <(echo "$kotlin_names") <(echo "$so_names") | wc -l | tr -d ' ')
        missing_from_kotlin=$(comm -13 <(echo "$kotlin_names") <(echo "$so_names") | wc -l | tr -d ' ')
        local common_names
        common_names=$(comm -12 <(echo "$kotlin_names") <(echo "$so_names"))
        value_mismatches=0
        if [ -n "$common_names" ]; then
            value_mismatches=$(
                diff \
                    <(echo "$common_names" | while read -r name; do grep "^$name " "$kotlin_file"; done) \
                    <(echo "$common_names" | while read -r name; do grep "^$name " "$so_file"; done) \
                | grep -c '^[<>]' || true
            )
            value_mismatches=$((value_mismatches / 2))
        fi

        echo "  $abi: MISMATCH (kotlin=$kotlin_count, so=$so_count)"
        if [ "$missing_from_so" -gt 0 ]; then
            echo "    Functions in Kotlin but not in .so: $missing_from_so"
        fi
        if [ "$missing_from_kotlin" -gt 0 ]; then
            echo "    Functions in .so but not in Kotlin: $missing_from_kotlin"
        fi
        if [ "$value_mismatches" -gt 0 ]; then
            echo "    Value mismatches on shared functions: $value_mismatches"
        fi
        echo "    First differences:"
        echo "$diff_output" | head -20 | sed 's/^/      /'
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
