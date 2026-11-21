# wp_rs_web

A little web tool for debugging connection issues.

## Getting Started

First, build the project from the root directory using `cargo build`.

Then, from the `wp_rs_web` directory, run:

```
npm install
npm run build:css
```

Then, in another browser tab, run `cargo run --bin wp_rs_web` (also from the `wp_rs_web` directory).

Open http://127.0.0.1:8000.

## Docker image

CI publishes `ghcr.io/automattic/wordpress-rs-web:latest` from trunk. To build the image and check that it starts, run from the root directory:

```
docker build -f wp_rs_web/Dockerfile -t wordpress-rs-web .
wp_rs_web/smoke-test.sh wordpress-rs-web
```

CI runs the same check on pull requests that change this directory, `Cargo.lock` or `rust-toolchain.toml`, and before each publish.