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

## Checking notification work against the private sample

The private sample is a folder of real WordPress.com notifications, one note per line in `.jsonl` files, that is
kept on one machine and never enters the repository. Neither CI nor anyone without the folder can run these
tests, so run them yourself on the machine that holds it.

The corpus test reads every note in the sample with the library's model, and reports counts, note types and key
paths:

```bash
NOTIFICATIONS_CORPUS=/path/to/sample cargo test -p wp_api --lib \
    wp_com::notifications::corpus -- --ignored --nocapture
```

The fixture check fails when any notification fixture or hand-built test input shares a value with the sample: a
number of six or more digits, a hash, a URL's host, or a word of seven or more letters. It reads every file Git
would commit whose path contains `notification` or `styled_text`, and takes about a minute:

```bash
NOTIFICATIONS_CORPUS=/path/to/sample cargo test -p wp_api --lib \
    wp_com::notifications::fixture_check -- --ignored --nocapture
```

**The fixture check must pass before notification work is pushed.** When it fails, it names each shared token and
the file it's in: replace each with an invented value. Write fixtures by hand rather than copying a note from the
sample. Both tests take several folders, separated by `:`.

The fixture check fails when `NOTIFICATIONS_CORPUS` isn't set or names a missing folder, and says that nothing
was checked. To see that it catches a copied note, put a note in a folder outside the checkout and name that
folder in `NOTIFICATIONS_FIXTURE_CHECK_EXTRA`; the check reads its files as if they were in the repository.

## Signing an example app into WordPress.com

The Swift and Android example apps read a WordPress.com bearer token at launch and store it as
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

The script hands the token over in a file, never on a command line, where the process list would
show it: the Swift example app's `Library/wpcom-token` in its Simulator data container, or the
Android example app's private `files/wpcom-token`. The app reads the file at launch, deletes it, and
uses the token only when no WordPress.com account is signed in, so signing out stays signed out.

## Checking the Swift example app's screens from a script

The Swift example app's screens can be checked on an iOS Simulator without tapping by hand. A UI test
(`native/swift/Example/ExampleUITests/ScriptedUITests.swift`) reads a script of steps, such as `tap Reader`,
`type #tag-slug => knitting` and `shot 01-home`, and saves a screenshot and a text list of what's on screen for
each `shot`. The steps it knows are listed at the top of that file.

Build the Rust library for the simulator only, which is much smaller than `make xcframework-only-ios`, boot a
simulator, and run a script:

```bash
make _build-apple-aarch64-apple-ios-sim
cargo run --quiet --bin xcframework -- --profile dev --targets aarch64-apple-ios-sim

scripts/example-app-ui-script.sh --build --device <udid> \
    native/swift/Example/ExampleUITests/Scripts/reader-signed-out.txt ~/Desktop/reader-screens
```

`--build` builds the app and the UI test: leave it out to run another script against the same build. The script
in the example goes through every Reader screen as a reader who isn't signed in, so it sends nothing that changes
an account. To run a script as an account, sign the app in first with `make example-app-login`: the app stays
signed in from one script to the next.

Keep the output folder outside the checkout when the screens show real content.

One launch argument helps a script: `launch -reader-offline` starts the app with every Reader request failing as it
does with no connection, which shows the Reader's screens from the cache alone.

`reader-offline-signed-out.txt`, next to that script, saves two posts, downloads their text and images, and opens
one from the device. It needs a fresh install, so that neither post is saved yet.

## Checking the Kotlin example app's screens from a script

The Kotlin example app can be checked the same way on an Android emulator or device, over `adb`.
`scripts/example-app-adb-script.py` reads a script of the same kind of steps, finds what's on screen with
UI Automator, and saves a screenshot and a text list of the screen for each `shot`. The steps it knows are listed
at the top of the file.

```bash
cd native/kotlin && ./gradlew :example:androidApp:installDebug -PcargoTarget=arm64 && cd ../..

scripts/example-app-adb-script.py <adb serial> \
    native/kotlin/example/ui-scripts/reader-signed-out.txt ~/Desktop/reader-screens
```

`-PcargoTarget=arm64` builds the Rust library for one ABI, which is what an emulator on an Apple Silicon Mac needs.
`reader-signed-out.txt` goes through the Reader as a reader who isn't signed in, and
`reader-offline-signed-out.txt` saves two posts, downloads their text and images, and opens one from the device.
To run a script as an account, sign the app in first with `make example-app-login PLATFORM=android`. To check a
screen with no connection, turn airplane mode on first: `adb shell cmd connectivity airplane-mode enable`.
