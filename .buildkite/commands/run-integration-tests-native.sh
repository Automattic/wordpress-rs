#!/bin/bash -eu

# Runs an integration test suite on a macOS agent, against a test server running directly in the VM
# rather than in Docker – the VMs can't run Docker.

SUITE=$1

echo "--- :rust: Installing Rust"
# No default toolchain – `rust-toolchain.toml` picks the one to install.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y --default-toolchain none

source "$HOME/.cargo/env"

if [ "$SUITE" = "kotlin" ]; then
	echo "--- :java: Installing JDK 21"
	# The JDK in the VM image is newer than the one the Kotlin project's toolchain asks for.
	brew install openjdk@21
	JAVA_HOME="$(brew --prefix openjdk@21)/libexec/openjdk.jdk/Contents/Home"
	export JAVA_HOME
fi

echo "--- :homebrew: Installing Test Server Dependencies"
make test-server-native-deps

make test-server-native

echo "--- 🧪 Running Integration Tests"
make "test-$SUITE-integration-native"
