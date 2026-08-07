# Local Development

Practical guidance for working on this repository day to day: which build to run
for the change you're making, and how to verify it.

For first-time setup of the Rust toolchains, run `make setup-rust`. For the full
dependency list, see the [README](../README.md#prerequisites).

## Choosing an xcframework build

`make xcframework` builds **11 targets across 4 platforms** (macOS, iOS, tvOS,
watchOS). It is what CI and releases need, but it is rarely what a local change
needs — a full build consumes on the order of **100 GB** in `target/` and takes
tens of minutes.

Pick the smallest build that verifies your change:

| Command                       | Targets | Approx. `target/` cost | Use when                                           |
| ----------------------------- | ------- | ---------------------- | -------------------------------------------------- |
| `cargo check`       | 0       | ~0                     | Verifying Rust compiles                            |
| `cargo test -p wp_api --lib`  | 0       | ~1 GB                  | Running unit tests                                 |
| `make xcframework-only-macos` | 2       | ~18 GB                 | Verifying `swift build` and generated bindings     |
| `make xcframework-only-ios`   | 3       | ~24 GB                 | Building an iOS consumer app against a local build |
| `make xcframework`            | 11      | ~100 GB                | Releases, or verifying every platform              |

Figures are for a cold build; incremental rebuilds are much cheaper.

> [!IMPORTANT]
> `xcframework-only-<platform>` **replaces** `target/libwordpressFFI.xcframework`
> rather than adding a slice to it. Each invocation assembles a framework
> containing only that platform.
>
> This matters because `swift build` on macOS links the macOS slice. Running
> `make xcframework-only-ios` and then `swift build` fails with
> `cannot find '...' in scope` for every symbol — not because the bindings are
> wrong, but because the macOS slice is no longer in the framework. Run
> `make xcframework-only-macos` to restore it.

The tvOS and watchOS slices build with `-Z build-std` on the pinned nightly
toolchain, which `make setup-rust` installs. If you skipped that step, those
targets fail with `can't find crate for 'core'`.

## Verifying a UniFFI change

When adding or changing an exported function, the fast path is:

1. **`cargo test --lib`** — the Rust side compiles and behaves.
2. **`make xcframework-only-macos`** — regenerates the bindings and builds a
   framework `swift build` can link.
3. **`swift build --target WordPressAPI`** — the generated Swift compiles and
   the FFI symbols resolve.
4. **`cd native/kotlin && ./gradlew :api:kotlin:compileKotlin :api:kotlin:detekt`**
   — the generated Kotlin compiles and passes lint.

The Kotlin step builds for the **host**, not Android, so it needs no NDK. Only
`api/android` and the Compose example app require `make setup-rust-android-targets`.

> [!WARNING]
> A partially failed `make xcframework` can still exit `0` and leave an
> assembled framework behind, because the assembly step reuses whatever slices
> already exist in `target/`. The result is a framework containing **stale**
> binaries alongside freshly generated Swift, which fails at link time with
> confusing "cannot find symbol" errors.
>
> When in doubt, check that your new symbols actually made it in:
>
> ```sh
> nm -gU target/libwordpressFFI.xcframework/macos-arm64_x86_64/libwp_mobile.a \
>   | grep my_new_exported_function
> ```
>
> No output means the framework predates your change — rebuild before trusting
> a green build.
