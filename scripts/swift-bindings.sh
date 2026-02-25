#!/bin/bash

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 /path/to/library"
  exit 1
fi

echo "Generating Swift bindings... (This may take a while when running for the first time.)"

module_name="libwordpressFFI"
library_path=$1
output_dir="$(dirname "$library_path")/swift-bindings"
rm -rf "$output_dir" && mkdir "$output_dir"

cargo run --release --quiet --bin wp_uniffi_bindgen generate \
    --library "$library_path" \
    --out-dir "$output_dir" \
    --no-format \
    --language swift

function generate_localized_error_extension {
    local package_name=$1
    local swift_binding=$2

    error_types=$(grep -r "impl WpSupportsLocalization for" "${package_name}/src" | grep -o "for [A-Za-z0-9]*" | cut -d' ' -f2)

    for error_type in $error_types; do
        cat <<EOF >> "$swift_binding"

extension $error_type: LocalizedError {
    public var errorDescription: String? {
        let preferred = wpLocaleResolve(langIds: Locale.preferredLanguages)
        return localize${error_type}(value: self, locale: preferred)
    }
}
EOF
    done
}

function patch_wp_api {
    local swift_binding=$1
    generate_localized_error_extension wp_api "$swift_binding"

    # Use sed to replace `import SQLite3` with the wrapped version
  sed -i.bak 's/^import SQLite3$/#if canImport(SQLite3)\
import SQLite3\
#endif/' "$swift_binding"

    cat >> "$swift_binding" <<'PATCH'

// AnyPostWith*Context types contain `AnyJson` (a reference type) which
// prevents automatic Hashable synthesis. Add the conformance manually.
extension AnyPostWithEditContext: Hashable {}
extension AnyPostWithEmbedContext: Hashable {}
extension AnyPostWithViewContext: Hashable {}
PATCH
}

function patch_wp_mobile {
    generate_localized_error_extension wp_mobile "$1"
}

for swift_binding in "$output_dir"/*.swift; do
    basename=$(basename "$swift_binding" .swift)
    if [ "$(type -t "patch_$basename")" = "function" ]; then
        "patch_$basename" "$swift_binding"
    fi
done

rm -f native/swift/Sources/wordpress-api-wrapper/*.swift
mv "$output_dir"/*.swift native/swift/Sources/wordpress-api-wrapper/

for swift_file in native/swift/Sources/wordpress-api-wrapper/*.swift; do
    basename=$(basename "$swift_file" .swift)
    output_file="native/swift/Sources/wordpress-api-wrapper/${basename}_cancellable.swift"

    swift run -c release --quiet \
        --package-path native/swift/Tools \
        generate-cancellable --log-level warning "$swift_file" "$output_file"
done

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
