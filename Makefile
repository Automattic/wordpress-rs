android_project_root := ./native/android
android_generated_source_path := $(android_project_root)/lib/build/generated/source
jni_libs_root := $(android_project_root)/lib/src/main/jniLibs

# The directory where the git repo is mounted in the docker container
docker_container_repo_dir=/app

# Common docker options
rust_docker_container := public.ecr.aws/docker/library/rust:1.76
swiftlint_container := ghcr.io/realm/swiftlint:0.53.0

docker_opts_shared :=  --rm -v "$(PWD)":$(docker_container_repo_dir) -w $(docker_container_repo_dir)
rust_docker_run := docker run -v $(PWD):/$(docker_container_repo_dir) -w $(docker_container_repo_dir) -it -e CARGO_HOME=/app/.cargo $(rust_docker_container)
docker_build_and_run := docker build -t foo . && docker run $(docker_opts_shared) -it foo

swift_package_platform_version = $(shell swift package dump-package | jq -r '.platforms[] | select(.platformName=="$1") | .version')
swift_package_platform_macos := $(call swift_package_platform_version,macos)
swift_package_platform_ios := $(call swift_package_platform_version,ios)
swift_package_platform_watchos := $(call swift_package_platform_version,watchos)
swift_package_platform_tvos :=	$(call swift_package_platform_version,tvos)

# Required for supporting tvOS and watchOS. We can update the nightly toolchain version if needed.
# The project doesn't compile with the nightly toolchain built on 2024-03-28 and onward.
rust_nightly_toolchain := nightly-2024-03-27

uname := $(shell uname | tr A-Z a-z)
ifeq ($(uname), linux)
	dylib_ext := so
endif
ifeq ($(uname), darwin)
	dylib_ext := dylib
endif

clean:
	git clean -ffXd

_generate-jni-libs:
	rm -rf $(jni_libs_root)
	cargo build --release --lib --target x86_64-linux-android --target i686-linux-android --target armv7-linux-androideabi --target aarch64-linux-android
	mkdir -p $(jni_libs_root)/arm64-v8a
	mkdir -p $(jni_libs_root)/armeabi-v7a
	mkdir -p $(jni_libs_root)/x86
	mkdir -p $(jni_libs_root)/x86_64
	cp ./target/aarch64-linux-android/release/libwp_api.so $(jni_libs_root)/arm64-v8a/libuniffi_wp_api.so
	cp ./target/armv7-linux-androideabi/release/libwp_api.so $(jni_libs_root)/armeabi-v7a/libuniffi_wp_api.so
	cp ./target/i686-linux-android/release/libwp_api.so $(jni_libs_root)/x86/libuniffi_wp_api.so
	cp ./target/x86_64-linux-android/release/libwp_api.so $(jni_libs_root)/x86_64/libuniffi_wp_api.so

bindings:
	rm -rf $(android_generated_source_path) target/swift-bindings
	cargo build --release

	#wp_api
	cargo run --release --bin wp_uniffi_bindgen generate --library ./target/release/libwp_api.$(dylib_ext) --out-dir $(android_generated_source_path) --language kotlin
	cargo run --release --bin wp_uniffi_bindgen generate --library ./target/release/libwp_api.$(dylib_ext) --out-dir ./target/swift-bindings --language swift
	cp target/swift-bindings/wp_api.swift native/swift/Sources/wordpress-api-wrapper/wp_api.swift

.PHONY: docs # Rebuild docs each time we run this command
docs:
	rm -rf docs
	mkdir -p docs
	$(rust_docker_run) /bin/bash -c 'cargo doc'
	cp -r target/doc/static.files docs/static.files
	cp -r target/doc/wp_api docs/wp_api
	cp -r target/doc/wp_derive docs/wp_derive
	cp -r target/doc/wp_networking docs/wp_networking

docs-archive: docs
	tar -czvf  docs.tar.gz docs

_test-android:
	./native/android/gradlew -p ./native/android cAT

_publish-android-local:
	./native/android/gradlew -p ./native/android publishToMavenLocal -exclude-task prepareToPublishToS3

# An XCFramework relies on the .h file and the modulemap to interact with the precompiled binary
xcframework-headers: bindings
	rm -rvf target/swift-bindings/headers
	mkdir -p target/swift-bindings/headers

	cp target/swift-bindings/*.h target/swift-bindings/headers
	cp target/swift-bindings/libwordpressFFI.modulemap target/swift-bindings/headers/module.modulemap


# TODO: Add arm64_32-apple-watchos to the list
apple-platform-targets-macos := x86_64-apple-darwin aarch64-apple-darwin
apple-platform-targets-ios := aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
apple-platform-targets-tvos := aarch64-apple-tvos aarch64-apple-tvos-sim
apple-platform-targets-watchos := x86_64-apple-watchos-sim aarch64-apple-watchos-sim
apple-platform-targets := \
	$(apple-platform-targets-macos) \
	$(apple-platform-targets-ios) \
	$(apple-platform-targets-tvos) \
	$(apple-platform-targets-watchos)

ifeq ($(BUILDKITE), true)
CARGO_PROFILE ?= release
else
CARGO_PROFILE ?= dev
endif

# Set deployment targets for each platform
_build-apple-%-darwin: export MACOSX_DEPLOYMENT_TARGET=$(swift_package_platform_macos)
_build-apple-%-ios _build-apple-%-ios-sim: export IPHONEOS_DEPLOYMENT_TARGET=$(swift_package_platform_ios)
_build-apple-%-tvos _build-apple-%-tvos-sim: export TVOS_DEPLOYMENT_TARGET=$(swift_package_platform_tvos)
_build-apple-%-watchos _build-apple-%-watchos-sim: export WATCHOS_DEPLOYMENT_TARGET=$(swift_package_platform_watchos)

# Use nightly toolchain for tvOS and watchOS
_build-apple-%-tvos _build-apple-%-tvos-sim _build-apple-%-watchos _build-apple-%-watchos-sim: \
	CARGO_OPTS = +$(rust_nightly_toolchain) -Z build-std=panic_abort,std

# Build the library for a specific target
_build-apple-%: xcframework-headers
	cargo $(CARGO_OPTS) build --target $* --package wp_api --profile $(CARGO_PROFILE)

# Build the library for one single platform, including real device and simulator.
build-apple-platform-macos := $(addprefix _build-apple-,$(apple-platform-targets-macos))
build-apple-platform-ios := $(addprefix _build-apple-,$(apple-platform-targets-ios))
build-apple-platform-tvos := $(addprefix _build-apple-,$(apple-platform-targets-tvos))
build-apple-platform-watchos := $(addprefix _build-apple-,$(apple-platform-targets-watchos))

# Creating xcframework for one single platform, including real device and simulator.
xcframework-only-macos: $(build-apple-platform-macos)
xcframework-only-ios: $(build-apple-platform-ios)
xcframework-only-tvos: $(build-apple-platform-tvos)
xcframework-only-watchos: $(build-apple-platform-watchos)
xcframework-only-%:
	cargo run --quiet --bin tools -- create-xcframework --profile $(CARGO_PROFILE) --targets $(apple-platform-targets-$*)

# Creating xcframework for all platforms.
xcframework-all: $(build-apple-platform-macos) $(build-apple-platform-ios) $(build-apple-platform-tvos) $(build-apple-platform-watchos)
	cargo run --quiet --bin tools -- create-xcframework --profile $(CARGO_PROFILE) --targets $(apple-platform-targets)

ifeq ($(SKIP_PACKAGE_WP_API),true)
xcframework:
	@echo "Skip building libwordpressFFI.xcframework"
else
xcframework: xcframework-all
endif

docker-image-swift:
	docker build -t wordpress-rs-swift -f Dockerfile.swift .

swift-linux-library: bindings
	mkdir -p target/swift-bindings/libwordpressFFI-linux
	cp target/swift-bindings/*.h target/swift-bindings/libwordpressFFI-linux/
	cp target/swift-bindings/libwordpressFFI.modulemap target/swift-bindings/libwordpressFFI-linux/module.modulemap
	cp target/release/libwp_api.a target/swift-bindings/libwordpressFFI-linux/

test-swift:
	$(MAKE) test-swift-$(uname)

test-swift-linux: docker-image-swift
	docker run $(docker_opts_shared) -it wordpress-rs-swift make test-swift-linux-in-docker

test-swift-linux-in-docker: swift-linux-library
	swift test -Xlinker -Ltarget/swift-bindings/libwordpressFFI-linux -Xlinker -lwp_api

test-swift-darwin: xcframework
	swift test

test-swift-macOS: test-swift-darwin

test-swift-iOS: xcframework
	scripts/xcodebuild-test.sh iOS-17-4

test-swift-tvOS: xcframework
	scripts/xcodebuild-test.sh tvOS-17-4

test-swift-watchOS: xcframework
	scripts/xcodebuild-test.sh watchOS-10-4

test-android: bindings _test-android

publish-android-local: bindings _publish-android-local

test-rust:
	$(rust_docker_run) cargo test

test-server:
	rm -rf test_credentials && touch test_credentials && chmod 777 test_credentials
	docker-compose up -d
	docker-compose run wpcli

stop-server:
	docker-compose down

lint: lint-rust lint-swift

lint-rust:
	$(rust_docker_run) /bin/bash -c "rustup component add clippy && cargo clippy --all -- -D warnings"

lint-swift:
	docker run -v $(PWD):$(docker_container_repo_dir) -w $(docker_container_repo_dir) -it $(swiftlint_container) swiftlint

lintfix-swift:
	docker run -v $(PWD):$(docker_container_repo_dir) -w $(docker_container_repo_dir) -it $(swiftlint_container) swiftlint --autocorrect

fmt-rust:
	$(rust_docker_run) /bin/bash -c "rustup component add rustfmt && cargo fmt"

fmt-check-rust:
	$(rust_docker_run) /bin/bash -c "rustup component add rustfmt && cargo fmt --all -- --check"

build-in-docker:
	$(call bindings)
	$(docker_build_and_run)

dev-server:
	mkdir -p .wordpress
	docker-compose up

setup-rust:
	RUST_TOOLCHAIN=stable $(MAKE) setup-rust-toolchain
	RUST_TOOLCHAIN=$(rust_nightly_toolchain) $(MAKE) setup-rust-toolchain

setup-rust-toolchain:
	rustup toolchain install $(RUST_TOOLCHAIN)
	rustup component add rust-src --toolchain $(RUST_TOOLCHAIN)
	rustup target add --toolchain $(RUST_TOOLCHAIN) \
		x86_64-apple-ios \
		aarch64-apple-ios \
		aarch64-apple-darwin \
		x86_64-apple-darwin \
		aarch64-apple-ios-sim
