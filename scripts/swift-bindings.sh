#!/bin/bash

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 /path/to/library"
  exit 1
fi

module_name="libwordpressFFI"
library_path=$1
output_dir="$(dirname "$library_path")/swift-bindings"
rm -rf "$output_dir" && mkdir "$output_dir"

cargo run --release --quiet --bin wp_uniffi_bindgen generate \
    --library "$library_path" \
    --out-dir "$output_dir" \
    --language swift

function patch_wp_api {
    error_types=$(grep -r "impl WpSupportsLocalization for" wp_api/src | grep -o "for [A-Za-z0-9]*" | cut -d' ' -f2)

    for error_type in $error_types; do
        cat <<EOF >> "$1"

extension $error_type: LocalizedError {
    public var errorDescription: String? {
        let preferred = wpLocaleResolve(langIds: Locale.preferredLanguages)
        return localize${error_type}(value: self, locale: preferred)
    }
}
EOF
    done
}

# The search-and-replace below can be removed after updating to a uniffi-rs
# version that includes this PR https://github.com/mozilla/uniffi-rs/pull/2341
for swift_binding in "$output_dir"/*.swift; do
    options=("-i")
    if [[ $(uname) == "Darwin" ]]; then
        options+=("")
    fi
    sed "${options[@]}" 's/^protocol UniffiForeignFutureTask /fileprivate protocol UniffiForeignFutureTask /' "$swift_binding"

    basename=$(basename "$swift_binding" .swift)
    if [ "$(type -t "patch_$basename")" = "function" ]; then
        "patch_$basename" "$swift_binding"
    fi
done

mv "$output_dir"/*.swift native/swift/Sources/wordpress-api-wrapper/

header_dir="$output_dir/Headers"
mkdir -p "$header_dir"

{
    for header_file in "$output_dir"/*.h; do
        echo "#include \"$(basename "$header_file")\""
    done
} > "$header_dir/$module_name.h"
mv "$output_dir"/*.h "$header_dir/"

cat <<EOF > "$header_dir/module.modulemap"
module $module_name {
    header "$module_name.h"
    export *
}
EOF
