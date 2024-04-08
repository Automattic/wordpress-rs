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


# Builds the library for all the various architectures / systems required in an XCFramework
xcframework-libraries:
	# macOS
	env MACOSX_DEPLOYMENT_TARGET=$(swift_package_platform_macos) $(MAKE) x86_64-apple-darwin-xcframework-library
	env MACOSX_DEPLOYMENT_TARGET=$(swift_package_platform_macos) $(MAKE) aarch64-apple-darwin-xcframework-library

	# iOS
	env IPHONEOS_DEPLOYMENT_TARGET=$(swift_package_platform_ios) $(MAKE) aarch64-apple-ios-xcframework-library
	env IPHONEOS_DEPLOYMENT_TARGET=$(swift_package_platform_ios) $(MAKE) x86_64-apple-ios-xcframework-library
	env IPHONEOS_DEPLOYMENT_TARGET=$(swift_package_platform_ios) $(MAKE) aarch64-apple-ios-sim-xcframework-library

	# tvOS
	env TVOS_DEPLOYMENT_TARGET=$(swift_package_platform_tvos) $(MAKE) aarch64-apple-tvos-xcframework-library-with-nightly
	env TVOS_DEPLOYMENT_TARGET=$(swift_package_platform_tvos) $(MAKE) aarch64-apple-tvos-sim-xcframework-library-with-nightly
	env TVOS_DEPLOYMENT_TARGET=$(swift_package_platform_tvos) $(MAKE) x86_64-apple-tvos-xcframework-library-with-nightly

	# watchOS
	env WATCHOS_DEPLOYMENT_TARGET=$(swift_package_platform_watchos) $(MAKE) arm64_32-apple-watchos-xcframework-library-with-nightly
	env WATCHOS_DEPLOYMENT_TARGET=$(swift_package_platform_watchos) $(MAKE) aarch64-apple-watchos-sim-xcframework-library-with-nightly
	env WATCHOS_DEPLOYMENT_TARGET=$(swift_package_platform_watchos) $(MAKE) x86_64-apple-watchos-sim-xcframework-library-with-nightly

%-xcframework-library:
	cargo build --target $* --package wp_api --release
	$(MAKE) $*-combine-libraries

%-xcframework-library-with-nightly:
	cargo +$(rust_nightly_toolchain) build --target $* --package wp_api --release -Zbuild-std
	$(MAKE) $*-combine-libraries

# Xcode doesn't properly support multiple XCFrameworks being used by the same target, so we need
# to combine the binaries
%-combine-libraries:
	xcrun libtool -static -o target/$*/release/libwordpress.a target/$*/release/libwp_api.a #target/$*/release/libwp_networking.a

# Some libraries need to be created in a multi-binary format, so we combine them here
xcframework-combined-libraries: xcframework-libraries

	rm -rf target/universal-*
	mkdir -p target/universal-macos/release target/universal-ios/release target/universal-tvos/release target/universal-watchos/release

	# Combine the macOS Binaries
	lipo -create target/aarch64-apple-darwin/release/libwordpress.a target/x86_64-apple-darwin/release/libwordpress.a \
		-output target/universal-macos/release/libwordpress.a
	lipo -info target/universal-macos/release/libwordpress.a

	# Combine iOS Simulator Binaries
	lipo -create target/aarch64-apple-ios-sim/release/libwordpress.a target/x86_64-apple-ios/release/libwordpress.a \
		-output target/universal-ios/release/libwordpress.a
	lipo -info target/universal-ios/release/libwordpress.a

	# Combine tvOS Simulator Binaries
	lipo -create target/aarch64-apple-tvos-sim/release/libwordpress.a target/x86_64-apple-tvos/release/libwordpress.a \
		-output target/universal-tvos/release/libwordpress.a
	lipo -info target/universal-tvos/release/libwordpress.a

	# Combine watchOS Simulator Binaries
	lipo -create target/aarch64-apple-watchos-sim/release/libwordpress.a target/x86_64-apple-watchos-sim/release/libwordpress.a \
		-output target/universal-watchos/release/libwordpress.a
	lipo -info target/universal-watchos/release/libwordpress.a

# An XCFramework relies on the .h file and the modulemap to interact with the precompiled binary
xcframework-headers: bindings
	rm -rvf target/swift-bindings/headers
	mkdir -p target/swift-bindings/headers

	cp target/swift-bindings/*.h target/swift-bindings/headers
	cp target/swift-bindings/libwordpressFFI.modulemap target/swift-bindings/headers/module.modulemap

ifeq ($(SKIP_PACKAGE_WP_API),true)
xcframework:
	@echo "Skip building libwordpressFFI.xcframework"
else

# Generate the xcframework
#
# Run `make setup-rust` to install required rust toolchain.
xcframework: bindings xcframework-combined-libraries xcframework-headers

	rm -rf target/libwordpressFFI.xcframework

	xcodebuild -create-xcframework \
		-library target/aarch64-apple-ios/release/libwordpress.a \
		-headers target/swift-bindings/headers \
		-library target/universal-macos/release/libwordpress.a \
		-headers target/swift-bindings/headers \
		-library target/universal-ios/release/libwordpress.a \
		-headers target/swift-bindings/headers \
		-library target/aarch64-apple-tvos/release/libwordpress.a \
		-headers target/swift-bindings/headers \
		-library target/universal-tvos/release/libwordpress.a \
		-headers target/swift-bindings/headers \
		-library target/universal-watchos/release/libwordpress.a \
		-headers target/swift-bindings/headers \
		-output target/libwordpressFFI.xcframework

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

test-rust-lib:
	$(rust_docker_run) cargo test --lib

test-rust-doc:
	$(rust_docker_run) cargo test --doc

test-rust-integration: test-server
	$(rust_docker_run) cargo test --test '*'

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
