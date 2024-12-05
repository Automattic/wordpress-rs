#!/bin/bash

set -euo pipefail

output_dir="target/swift-bindings"
rm -rf "$output_dir" && mkdir "$output_dir"

cargo build --lib --release
cargo build --bin wp_uniffi_bindgen --release

echo '// Auto-generated' > "$output_dir/libwordpressFFI.h"

for lib_name in wp_api wordpress_org_api; do
  cargo run --release --bin wp_uniffi_bindgen generate \
    --library "./target/release/lib${lib_name}.dylib" \
    --out-dir "$output_dir" \
    --language swift

  header_file="${lib_name}_uniffi.h"
  [ -f "$output_dir/$header_file" ] || {
    echo "Error: $output_dir/$header_file not found"
    exit 1
  }

  echo "#include \"${lib_name}_uniffi.h\"" >> "$output_dir/libwordpressFFI.h"

  # The search-and-replace below can be removed once this PR is merged
  # https://github.com/mozilla/uniffi-rs/pull/2341
  for swift_binding in "$output_dir"/*.swift; do
    sed -i '' 's/^protocol UniffiForeignFutureTask /fileprivate protocol UniffiForeignFutureTask /' "$swift_binding"
  done
done

cat <<EOT >> "$output_dir/libwordpressFFI.modulemap"
module libwordpressFFI {
  header "libwordpressFFI.h"
  export *
}
EOT
