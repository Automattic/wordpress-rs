# Debugging Rust Code in Xcode

This guide explains how to debug Rust code using Xcode's LLDB debugger, which is particularly useful when you need to debug how Rust code executes in an iOS application.

## Initial Setup

1. Create a `.lldbinit` file in your home directory with the following content:

    ```
    command script import <rust-toolchain-dir>/lib/rustlib/etc/lldb_lookup.py
    command source -s true <rust-toolchain-dir>/lib/rustlib/etc/lldb_commands
    ```

2. Replace `<rust-toolchain-dir>` with your Rust toolchain path, typically:
   `$HOME/.rustup/toolchains/<active-toolchain-name>`

   Example: `/Users/macuser/.rustup/toolchains/stable-aarch64-apple-darwin`

<details>
<summary>About the LLDB Configuration</summary>

The LLDB configuration is taken from [this file in the Rust source code repository](https://github.com/rust-lang/rust/blob/master/src/etc/rust-lldb). After installing Rust, you should have a `rust-lldb` command at `$HOME/.cargo/bin/rust-lldb`, which essentially calls the `lldb` command with the same configuration above. Ideally, we could configure Xcode to invoke `rust-lldb` instead of the default LLDB, so that we don't have to keep the `.lldbinit` content in sync with how `rust-lldb` runs the LLDB command. However, this isn't possible at the moment.
</details>

## Basic Debugging Steps

1. Set a breakpoint in your Rust code:

    ```
    breakpoint set --file <rust-file> --line <line-no>
    # Example:
    #   breakpoint set --file wp_api/src/api_client.rs --line 133
    ```
> [!Note]
> You _do not_ need to set the same breakpoints repeatedly because Xcode remembers breakpoints between sessions.

2. Run your application until it hits the breakpoint

3. Once paused, you can:
   - View variables in the current frame from the left side of the debugger console
   - Inspect struct fields as defined in the source code
   - Step through code using Xcode's debug controls

> [!Note]
> LLDB does not support Rust expressions, so you cannot run commands like `expression --language Rust -- api_root_url.as_str()` to execute functions and print the result.

## Debugging from an app

Your app should integrate this library via its releases, which is slightly different from developing the library within this repository. When debugging Rust code from your app, follow these additional steps:

1. Ensure your local `wordpress-rs` working copy matches the app's version:
   - Check the version used in your app
   - Go to your wordpress-rs repository and run `git checkout <version>` to switch to the correct version

2. After hitting your breakpoint, you'll see an error message:
   ```
   [...]. The file path does not exist on the file system: /opt/ci/builds/builder/automattic/wordpress-rs/wp_api/src/api_client.rs
   ```

3. This occurs because the app's binary is built in CI with different source paths. To fix this, run:
   ```
   settings set target.source-map /opt/ci/builds/builder/automattic/wordpress-rs /path/to/wordpress-rs
   ```

> [!Note]
> Add the source map command to your `.lldbinit` file to avoid running it in every debugging session.
