#!/bin/bash
#
# Run a script of UI steps against the Swift example app on an iOS Simulator, and save screenshots of it.
#
# The steps are tapped by a UI test (`native/swift/Example/ExampleUITests/ScriptedUITests.swift`, which lists
# them), so a check of the app's screens can be repeated without tapping by hand. Build the Rust library for the
# simulator first: see `Documentation/local-development.md`.

set -euo pipefail

usage() {
    cat <<'USAGE'
Run a script of UI steps against the Swift example app on an iOS Simulator.

Usage:
  scripts/example-app-ui-script.sh [options] <script> <output folder>

Options:
  -d, --device <udid>       The simulator to use (default: the only booted one)
  -b, --build               Build the app and the UI test first. Needed once, and after a change.
  -D, --derived-data <dir>  Where Xcode builds (default: target/example-ios-derived-data)
  -h, --help                Show this help

The output folder gets a PNG and a text list of the screen for each `shot` step, and `transcript.txt`.
Keep it outside the checkout when the screens show real content.

Example:
  scripts/example-app-ui-script.sh --build \
      native/swift/Example/ExampleUITests/Scripts/reader-signed-out.txt ~/Desktop/reader-screens
USAGE
}

root="$(cd "$(dirname "$0")/.." && pwd)"
device=""
build=false
derived_data="$root/target/example-ios-derived-data"
positional=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        -d|--device) device="$2"; shift 2 ;;
        -b|--build) build=true; shift ;;
        -D|--derived-data) derived_data="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        -*) echo "error: unknown option '$1'" >&2; usage >&2; exit 1 ;;
        *) positional+=("$1"); shift ;;
    esac
done

if [[ ${#positional[@]} -ne 2 ]]; then
    usage >&2
    exit 1
fi

script="$(cd "$(dirname "${positional[0]}")" && pwd)/$(basename "${positional[0]}")"
mkdir -p "${positional[1]}"
output="$(cd "${positional[1]}" && pwd)"

if [[ ! -f "$script" ]]; then
    echo "error: no script at '$script'" >&2
    exit 1
fi

if [[ -z "$device" ]]; then
    booted=()
    while IFS= read -r udid; do
        booted+=("$udid")
    done < <(xcrun simctl list devices booted | grep -oE '[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}' || true)
    if [[ ${#booted[@]} -ne 1 ]]; then
        echo "error: ${#booted[@]} simulators are booted. Pass --device <udid>." >&2
        exit 1
    fi
    device="${booted[0]}"
fi

xcodebuild_arguments=(
    -project "$root/native/swift/Example/Example.xcodeproj"
    -scheme ExampleUITests
    -destination "platform=iOS Simulator,id=$device"
    -derivedDataPath "$derived_data"
    -skipPackagePluginValidation
    -skipMacroValidation
)

if [[ "$build" == true ]]; then
    xcodebuild "${xcodebuild_arguments[@]}" build-for-testing > "$output/build.log" 2>&1 || {
        echo "error: the build failed. See $output/build.log" >&2
        exit 1
    }
fi

# `TEST_RUNNER_` variables reach the test, without the prefix.
status=0
TEST_RUNNER_UI_SCRIPT="$script" TEST_RUNNER_UI_OUTPUT="$output" \
    xcodebuild "${xcodebuild_arguments[@]}" test-without-building \
    -only-testing:ExampleUITests/ScriptedUITests/testRunScript > "$output/test.log" 2>&1 || status=$?

if [[ -f "$output/transcript.txt" ]]; then
    cat "$output/transcript.txt"
fi
if [[ $status -ne 0 ]]; then
    echo "error: the script didn't finish. See $output/test.log" >&2
fi
exit $status
