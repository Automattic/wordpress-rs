# Local Development

Practical guidance for working on this repository day to day: which build to run
for the change you're making, and how to verify it.

For first-time setup of the Rust toolchains, run `make setup-rust`. For the full
dependency list, see the [README](../README.md#prerequisites).

## Choosing an xcframework build

`make xcframework` builds **12 targets across 5 platforms** (macOS, iOS, Mac
Catalyst, tvOS, watchOS). It is what CI and releases need, but it is rarely what a local change
needs — a full build consumes on the order of **100 GB** in `target/` and takes
tens of minutes.

Pick the smallest build that verifies your change:

| Command                       | Targets | Approx. `target/` cost | Use when                                           |
| ----------------------------- | ------- | ---------------------- | -------------------------------------------------- |
| `cargo check`       | 0       | ~0                     | Verifying Rust compiles                            |
| `cargo test -p wp_api --lib`  | 0       | ~1 GB                  | Running unit tests                                 |
| `make xcframework-only-macos` | 2       | ~18 GB                 | Verifying `swift build` and generated bindings     |
| `make xcframework-only-ios`   | 3       | ~24 GB                 | Building an iOS consumer app against a local build |
| `make xcframework`            | 12      | ~100 GB                | Releases, or verifying every platform              |

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

## Signing an example app into WordPress.com

The Swift and Android example apps accept a WordPress.com bearer token at launch and store it as
their WordPress.com account, so you can sign a Simulator or emulator in without configuring OAuth
app credentials in `wp_com_test_credentials.json`. Build and install the app first, then:

```bash
make example-app-login                    # Swift example app, the booted iOS Simulator
make example-app-login PLATFORM=android   # Android example app, the connected emulator/device
make example-app-login RESET=1            # clear the app's data first (clean slate)
make example-app-login DEVICE=<id>        # a specific simulator UDID or adb serial
```

`make example-app-login` forwards to `scripts/example-app-signin.sh`, which takes the same options
as flags — `--platform`, `--device`, `--reset`. When more than one simulator or device is running,
it prompts you to choose.

The token is resolved from, in order:

1. the `WPCOM_TOKEN` environment variable;
2. `bearer_token` in `wp_com_test_credentials.json` at the repository root — the same token the
   WordPress.com integration tests use (see `wp_com_test_credentials.json-example`);
3. a `~/.wpcom-token` file, which also works in fresh worktrees that don't have a copy of
   `wp_com_test_credentials.json`, and matches WordPress-iOS's `make sim-login`.

There's deliberately no command-line token flag — a token passed as an argument would be saved in
your shell history.

The app only uses the token when no WordPress.com account is signed in, and only once per launch,
so signing out stays signed out. To launch with a token by hand:

```bash
# iOS Simulator
xcrun simctl launch --terminate-running-process booted com.automattic.Example -wpcom-token <token>

# Android (single-quote the token — the device shell interprets it)
adb shell am start -S -n rs.wordpress.example/.ui.welcome.WelcomeActivity --es wpcom-token "'<token>'"
```

Either way, the token briefly appears in the process list while the launch command runs; that's
inherent to passing it as a launch argument.

## Running the TLS tests against a local badssl.com

A few login tests check how TLS failures are reported, against sites from
[badssl.com](https://badssl.com). CI runs a copy of badssl.com on each machine with
`scripts/badssl-server.sh`, so they don't depend on the live site, which sometimes drops
connections. Without `BADSSL_DOMAIN`, the tests use the live site.

To use a copy on a Mac, with nginx from Homebrew:

```bash
scripts/badssl-server.sh prepare
scripts/badssl-server.sh start
BADSSL_DOMAIN=badssl.localhost swift test --filter LoginTests
scripts/badssl-server.sh stop
```

`make test-server` also starts a copy, as the `badssl` Docker service, for the Kotlin integration
tests. The copy's certificates are signed by a root it generates, which nothing trusts unless it's
added: the Kotlin tests add it inside the `wordpress` container, and the Swift tests leave it out.
