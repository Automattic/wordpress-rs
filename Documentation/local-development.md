# Local Development

Practical guidance for working on this repository day to day: what to install,
which build to run for the change you're making, and how to reclaim the disk
space these builds consume.

For first-time setup of the Rust toolchains, run `make setup-rust`. For the full
dependency list, see the [README](../README.md#prerequisites).

## Java toolchain (Kotlin builds)

The Kotlin bindings pin a **Java 21** toolchain in
`native/kotlin/api/kotlin/build.gradle.kts`. Gradle requires an exact match and
will not substitute a newer JDK, so a Java 21 installation must be discoverable
on your machine:

```
Cannot find a Java installation on your machine (Mac OS X ...) matching:
{languageVersion=21, vendor=any vendor, ...}
```

**Android Studio's bundled JDK is not a substitute.** Recent versions ship
Java 25, and pointing `JAVA_HOME` at the app bundle has two further problems:

- Gradle still cannot find a 21 *toolchain* to compile with, even though it
  happily *runs* on 25.
- Every Android Studio update can move or re-version that JDK, silently breaking
  terminal builds that worked the day before.

Install a standalone JDK 21 instead. With Homebrew:

```sh
brew install openjdk@21
```

`openjdk@21` is keg-only, so it is not linked into a standard location. Either
point `JAVA_HOME` at it directly:

```sh
export JAVA_HOME=/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home
```

…or register it system-wide, which also makes it discoverable to
`/usr/libexec/java_home` and to Android Studio's Gradle JDK picker:

```sh
sudo ln -sfn /opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk \
  /Library/Java/JavaVirtualMachines/openjdk-21.jdk
export JAVA_HOME=$(/usr/libexec/java_home -v 21)
```

Prefer `java_home -v 21` over the bare `java_home`, which returns the *newest*
registered JDK and will silently switch away from 21 if you install a later one.

Related: if Android Studio cannot find `cargo`, see the
[README troubleshooting note](../README.md#android-studio-configuration) — the
Kotlin build shells out to `cargo`, and the IDE's bundled JDK does not inherit
your shell `PATH`.

## Choosing an xcframework build

`make xcframework` builds **11 targets across 4 platforms** (macOS, iOS, tvOS,
watchOS). It is what CI and releases need, but it is rarely what a local change
needs — a full build consumes on the order of **100 GB** in `target/` and takes
tens of minutes.

Pick the smallest build that verifies your change:

| Command                      | Targets | Approx. `target/` cost | Use when                                        |
| ---------------------------- | ------- | ---------------------- | ----------------------------------------------- |
| `cargo check -p wp_api`      | 0       | ~0                     | Verifying Rust compiles                         |
| `cargo test -p wp_api --lib` | 0       | ~1 GB                  | Running unit tests                              |
| `make xcframework-only-macos`| 2       | ~18 GB                 | Verifying `swift build` and generated bindings   |
| `make xcframework-only-ios`  | 3       | ~24 GB                 | Building an iOS consumer app against a local build |
| `make xcframework`           | 11      | ~100 GB                | Releases, or verifying every platform            |

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

1. **`cargo test -p wp_api --lib`** — the Rust side compiles and behaves.
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

## Reclaiming disk space

`target/` grows quickly, since each Apple target keeps its own full build tree.

To see where it went:

```sh
du -sh target/* | sort -rh
```

`cargo clean` removes everything, at the cost of a slow cold rebuild. To trim
selectively instead, delete the platform directories you are not currently
building for — they are regenerated on demand:

```sh
rm -rf target/aarch64-apple-tvos target/aarch64-apple-tvos-sim
rm -rf target/arm64_32-apple-watchos target/x86_64-apple-watchos-sim target/aarch64-apple-watchos-sim
```

> [!NOTE]
> Keep **both** `aarch64-apple-darwin` and `x86_64-apple-darwin` if you want
> `swift build` to keep working. The macOS slice is universal
> (`macos-arm64_x86_64`), so removing either half forces a rebuild of the
> framework `swift build` links against.

Xcode accumulates far more than this repository does, and it is worth checking
before blaming `target/`:

```sh
du -sh ~/Library/Developer/Xcode/DerivedData \
       ~/Library/Developer/Xcode/iOS\ DeviceSupport \
       ~/Library/Developer/CoreSimulator
```

- **DerivedData** — build output, safe to delete; projects rebuild from scratch.
- **iOS DeviceSupport** — debug symbols cached per device and OS version, several
  GB each. Regenerated the next time you attach a device on that version.
- **CoreSimulator** — simulator devices. Remove ones whose runtimes are gone with
  `xcrun simctl delete unavailable`.
