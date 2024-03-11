android_project_root := ./native/android
android_generated_source_path := $(android_project_root)/lib/build/generated/source
jni_libs_root := $(android_project_root)/lib/src/main/jniLibs
udl_path := wp_api/src/wp_api.udl

# The directory where the git repo is mounted in the docker container
docker_container_repo_dir=/app

# Common docker options
rust_docker_container := public.ecr.aws/docker/library/rust:1.76
swiftlint_container := ghcr.io/realm/swiftlint:0.53.0

docker_opts_shared :=  --rm -v "$(PWD)":$(docker_container_repo_dir) -w $(docker_container_repo_dir)
rust_docker_run := docker run -v $(PWD):/$(docker_container_repo_dir) -w $(docker_container_repo_dir) -it -e CARGO_HOME=/app/.cargo $(rust_docker_container)
docker_build_and_run := docker build -t foo . && docker run $(docker_opts_shared) -it foo

clean:
	git clean -ffxd

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
	cargo run --release --bin uniffi_bindgen generate --library ./target/release/libwp_api.dylib --out-dir $(android_generated_source_path) --language kotlin
	cargo run --release --bin uniffi_bindgen generate wp_api/src/wp_api.udl --out-dir ./target/swift-bindings --language swift
	cp target/swift-bindings/wp_api.swift native/swift/Sources/wordpress-api-wrapper/wp_api.swift
	sed -i '' 's/wp_apiFFI/libwordpressFFI/g' native/swift/Sources/wordpress-api-wrapper/wp_api.swift

_test-android:
	./native/android/gradlew -p ./native/android cAT

_publish-android-local:
	./native/android/gradlew -p ./native/android publishToMavenLocal -exclude-task prepareToPublishToS3


# Builds the library for all the various architectures / systems required in an XCFramework
xcframework-libraries:
	# macOS
	$(MAKE) x86_64-apple-darwin-xcframework-library
	$(MAKE) aarch64-apple-darwin-xcframework-library

	# iOS
	$(MAKE) aarch64-apple-ios-xcframework-library
	$(MAKE) x86_64-apple-ios-xcframework-library
	$(MAKE) aarch64-apple-ios-sim-xcframework-library

	# tvOS
	$(MAKE) aarch64-apple-tvos-xcframework-library-with-nightly
	$(MAKE) aarch64-apple-tvos-sim-xcframework-library-with-nightly
	$(MAKE) x86_64-apple-tvos-xcframework-library-with-nightly

	# watchOS
	$(MAKE) arm64_32-apple-watchos-xcframework-library-with-nightly
	$(MAKE) aarch64-apple-watchos-sim-xcframework-library-with-nightly
	$(MAKE) x86_64-apple-watchos-sim-xcframework-library-with-nightly

%-xcframework-library:
	cargo build --target $* --package wp_api --release
	$(MAKE) $*-combine-libraries

%-xcframework-library-with-nightly:
	cargo +nightly build --target $* --package wp_api --release -Zbuild-std
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

	mv target/swift-bindings/*.h target/swift-bindings/headers
	cp .swiftpm/module.modulemap target/swift-bindings/headers/module.modulemap

# Generate the xcframework
#
# Requires the following runtimes:
#	rustup target add x86_64-apple-ios
#	rustup target add aarch64-apple-ios
#	rustup target add aarch64-apple-darwin
#	rustup target add x86_64-apple-darwin
#	rustup target add aarch64-apple-ios-sim
#
#	rustup toolchain install nightly
#	rustup component add rust-src --toolchain nightly-aarch64-apple-darwin
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

test-swift: xcframework
	swift test

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

build-in-docker:
	$(call bindings)
	$(docker_build_and_run)

dev-server:
	mkdir -p .wordpress
	docker-compose up
