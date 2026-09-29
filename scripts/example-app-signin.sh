#!/bin/bash
#
# Sign an example app into WordPress.com with a bearer token, in one command.
#
# Launches the Swift example app on an iOS Simulator (`xcrun simctl launch -wpcom-token`) or the
# Android example app on an emulator/device (`adb shell am start --es wpcom-token`). The app stores
# the token as its WordPress.com account at launch — no OAuth app credentials or taps required.
# Build and install the app first (e.g. from Xcode or Android Studio).

set -euo pipefail

IOS_BUNDLE_ID="com.automattic.Example"
ANDROID_PACKAGE="rs.wordpress.example"
ANDROID_ACTIVITY="$ANDROID_PACKAGE/.ui.welcome.WelcomeActivity"

usage() {
    cat <<'EOF'
Sign an example app into WordPress.com with a bearer token, in one command.

Usage:
  scripts/example-app-signin.sh [options]

Options:
  -p, --platform <ios|android>  Example app to sign in (default: ios)
  -d, --device <id>             Target device — a simulator UDID/name, or an adb serial
                                (default: the running one; prompts if several)
  -r, --reset                   Clear the app's data first, for a clean slate
  -h, --help                    Show this help

Examples:
  scripts/example-app-signin.sh                     # Swift example app, booted simulator
  scripts/example-app-signin.sh --platform android  # Android example app, connected emulator
  scripts/example-app-signin.sh --reset             # clear app data first

The WordPress.com bearer token is read from (in order):
  1. WPCOM_TOKEN environment variable
  2. `bearer_token` in wp_com_test_credentials.json at the repository root
     (the same token the WordPress.com integration tests use)
  3. ~/.wpcom-token file

It is deliberately NOT accepted as a command-line flag: a token passed on the command line
would be saved in your shell history.
EOF
}

platform="ios"
device=""
reset=false

require_value() {
    # $1 = option name, $2 = remaining argument count ($#)
    if [[ "$2" -lt 2 ]]; then
        echo "error: $1 requires a value" >&2
        exit 1
    fi
}

choose_device() {
    # Set `device` from parallel `ids` / `names` arrays: the only entry if there's just one,
    # otherwise prompt to choose. $1 = a noun for messages, $2 = a hint for when there are none.
    local noun="$1" hint="$2"
    local n=${#ids[@]}
    if [[ "$n" -eq 0 ]]; then
        echo "error: no running $noun. $hint" >&2
        exit 1
    fi
    if [[ "$n" -eq 1 ]]; then
        device="${ids[0]}"
        echo "Using the only running $noun: ${names[0]} (${device})"
        return
    fi

    echo "Multiple ${noun}s are running — choose one:" >&2
    local i
    for (( i = 0; i < n; i++ )); do
        printf "  %2d) %s (%s)\n" "$(( i + 1 ))" "${names[i]}" "${ids[i]}" >&2
    done
    local sel
    while true; do
        printf "Select a %s [1-%d]: " "$noun" "$n" >&2
        if ! read -r sel; then
            echo >&2
            echo "error: no selection made; re-run with --device <id>." >&2
            exit 1
        fi
        if [[ "$sel" =~ ^[0-9]+$ ]] && [[ "$sel" -ge 1 ]] && [[ "$sel" -le "$n" ]]; then
            device="${ids[sel - 1]}"
            echo "Using ${names[sel - 1]} (${device})"
            return
        fi
        echo "  not a valid choice: '$sel'" >&2
    done
}

resolve_ios_device() {
    ids=()
    names=()
    local line udid
    while IFS= read -r line; do
        # `|| true` so a UUID-less line doesn't trip `set -e` before the `continue` can skip it.
        udid=$(printf '%s\n' "$line" | grep -oiE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1 || true)
        [[ -z "$udid" ]] && continue
        ids+=("$udid")
        names+=("$(printf '%s\n' "$line" | sed -E 's/^[[:space:]]*//; s/[[:space:]]*\([0-9A-Fa-f-]{36}\).*$//')")
    done < <(xcrun simctl list devices booted | grep -F "(Booted)" || true)

    choose_device "simulator" "Boot one (open Simulator, or 'xcrun simctl boot <udid>'), or pass --device <udid>."
}

resolve_android_device() {
    ids=()
    names=()
    local serial state rest model
    # `adb devices -l` lines look like: `emulator-5554  device product:... model:sdk_gphone64_arm64 ...`
    while read -r serial state rest; do
        [[ "$state" == "device" ]] || continue
        model=$(printf '%s\n' "$rest" | grep -oE 'model:[^ ]+' | cut -d: -f2 || true)
        ids+=("$serial")
        names+=("${model:-$serial}")
    done < <(adb devices -l | tail -n +2)

    choose_device "device" "Start an emulator (or connect a device), or pass --device <serial>."
}

reset_ios_app() {
    # A clean slate: uninstall the app (which removes its entire data container) and reinstall the
    # same bundle. `simctl install` is synchronous, so nothing races the sign-in launch.
    local app_bundle bundle_name
    app_bundle=$(xcrun simctl get_app_container "$device" "$IOS_BUNDLE_ID" app 2>/dev/null || true)
    if [[ -z "$app_bundle" || ! -d "$app_bundle" ]]; then
        echo "error: can't reset — $IOS_BUNDLE_ID isn't installed on '$device'. Build and run it from Xcode first." >&2
        exit 1
    fi
    bundle_name=$(basename "$app_bundle")

    # Uninstall deletes the installed bundle in place, so stage a copy to reinstall from.
    # `reset_staging` is intentionally global so the EXIT trap can clean it up at script exit.
    reset_staging=$(mktemp -d)
    trap 'rm -rf "$reset_staging"' EXIT
    cp -R "$app_bundle" "$reset_staging/$bundle_name"

    echo "Resetting $IOS_BUNDLE_ID (uninstall + reinstall for a clean slate)…"
    xcrun simctl uninstall "$device" "$IOS_BUNDLE_ID"
    xcrun simctl install "$device" "$reset_staging/$bundle_name"
}

reset_android_app() {
    echo "Resetting $ANDROID_PACKAGE (clearing app data for a clean slate)…"
    if ! adb -s "$device" shell pm clear "$ANDROID_PACKAGE" | grep -q Success; then
        echo "error: can't reset — $ANDROID_PACKAGE isn't installed on '$device'. Build and run it from Android Studio first." >&2
        exit 1
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -p|--platform) require_value "$1" "$#"; platform="$2"; shift 2 ;;
        -d|--device) require_value "$1" "$#"; device="$2"; shift 2 ;;
        -r|--reset) reset=true; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "error: unknown argument '$1'" >&2; usage >&2; exit 1 ;;
    esac
done

case "$platform" in
    ios|android) ;;
    *) echo "error: unknown platform '$platform' (expected 'ios' or 'android')" >&2; exit 1 ;;
esac

# The token is intentionally not a command-line argument, to keep it out of shell history.
token="${WPCOM_TOKEN:-}"
token_source="WPCOM_TOKEN"

# The `bearer_token` the WordPress.com integration tests use. `plutil` reads JSON natively on macOS.
credentials_file="$(cd "$(dirname "$0")/.." && pwd)/wp_com_test_credentials.json"
if [[ -z "$token" && -f "$credentials_file" ]]; then
    token="$(plutil -extract bearer_token raw -o - "$credentials_file" 2>/dev/null || true)"
    token_source="wp_com_test_credentials.json"
    # Ignore the unedited placeholder from `wp_com_test_credentials.json-example`.
    if [[ "$token" == "replace_with_your_oauth2_token" ]]; then
        token=""
    fi
fi

if [[ -z "$token" && -f "$HOME/.wpcom-token" ]]; then
    token="$(tr -d '[:space:]' < "$HOME/.wpcom-token")"
    token_source="~/.wpcom-token"
fi

if [[ -z "$token" ]]; then
    echo "error: no WordPress.com token found. Set \`bearer_token\` in $credentials_file" >&2
    echo "       (see wp_com_test_credentials.json-example), set WPCOM_TOKEN, or write ~/.wpcom-token." >&2
    exit 1
fi
echo "Using the WordPress.com token from $token_source"

if [[ -z "$device" ]]; then
    if [[ "$platform" == ios ]]; then resolve_ios_device; else resolve_android_device; fi
fi

echo "Signing the $platform example app into WordPress.com on '$device'…"

if [[ "$platform" == ios ]]; then
    if [[ "$reset" == true ]]; then
        reset_ios_app
    fi
    xcrun simctl launch --terminate-running-process "$device" "$IOS_BUNDLE_ID" -wpcom-token "$token"
else
    if [[ "$reset" == true ]]; then
        reset_android_app
    fi
    # `adb shell` joins its arguments into a command line for the device's shell, so single-quote the
    # token — WordPress.com tokens contain shell metacharacters like `#`, `(`, and `$`.
    quoted_token="'$(printf '%s' "$token" | sed "s/'/'\\\\''/g")'"
    # `-S` force-stops a running instance first, so the extra is delivered to a fresh `onCreate`.
    adb -s "$device" shell am start -S -W -n "$ANDROID_ACTIVITY" --es wpcom-token "$quoted_token" > /dev/null
fi

echo "Done. The app is signed into WordPress.com — open its WordPress.com section."
