#!/bin/bash -eu

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y
source "$HOME/.cargo/env"

echo "--- :node: Installing Node.js"
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | PROFILE=/dev/null bash
export NVM_DIR="$HOME/.nvm"
# shellcheck source=/dev/null
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
nvm install --lts

echo "--- :docker: Starting wp-env (WordFence)"
make wp-env-wordfence-start

echo "--- 🧪 Running wp-env Tests"
cargo test -p wp_api_integration_tests --test test_login_wp_env --no-fail-fast

echo "--- :docker: Stopping wp-env"
make wp-env-wordfence-stop
