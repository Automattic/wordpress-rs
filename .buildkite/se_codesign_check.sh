#!/bin/bash
set -euo pipefail

echo "--- :information_source: System info"
system_profiler SPHardwareDataType
sw_vers

echo "--- :mag: Building standalone binary"
swiftc -o /tmp/se_standalone .buildkite/se_test_single.swift
echo "Standalone binary built at /tmp/se_standalone"

echo "--- :mag: Building swift test binary"
cd .buildkite/se-investigation
swift build --build-tests 2>&1
echo ""

echo "--- :lock: Code signing: standalone binary"
codesign -dvvv /tmp/se_standalone 2>&1 || echo "(no signature)"
echo ""

echo "--- :lock: Code signing: swift test binary"
# Find the test binary
TEST_BINARY=$(find .build -name "*.xctest" -type d 2>/dev/null | head -1)
if [ -n "$TEST_BINARY" ]; then
    echo "Test bundle: $TEST_BINARY"
    MACHO=$(find "$TEST_BINARY" -type f -perm +111 | head -1)
    if [ -n "$MACHO" ]; then
        echo "Test executable: $MACHO"
        codesign -dvvv "$MACHO" 2>&1 || echo "(no signature)"
        echo ""
        echo "--- :key: Entitlements: test binary"
        codesign -d --entitlements - "$MACHO" 2>&1 || echo "(no entitlements)"
    fi
else
    echo "No .xctest bundle found, looking for test executable directly..."
    MACHO=$(find .build -name "SEInvestigationPackageTests" -type f 2>/dev/null | head -1)
    if [ -n "$MACHO" ]; then
        echo "Test executable: $MACHO"
        codesign -dvvv "$MACHO" 2>&1 || echo "(no signature)"
        echo ""
        echo "--- :key: Entitlements: test binary"
        codesign -d --entitlements - "$MACHO" 2>&1 || echo "(no entitlements)"
    else
        echo "Could not find test binary. Listing .build contents:"
        find .build -type f -name "*Test*" 2>/dev/null || echo "(none found)"
    fi
fi

echo ""
echo "--- :key: Entitlements: standalone binary"
codesign -d --entitlements - /tmp/se_standalone 2>&1 || echo "(no entitlements)"

echo ""
echo "--- :lock: Code signing: swift interpreter"
SWIFT_PATH=$(which swift)
codesign -dvvv "$SWIFT_PATH" 2>&1 || echo "(no signature)"

echo ""
echo "--- :white_check_mark: Code signing comparison complete"
