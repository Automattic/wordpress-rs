android_project_root := ./native/android
android_generated_source_path := $(android_project_root)/lib/build/generated/source
jni_libs_root := $(android_project_root)/lib/src/main/jniLibs
udl_path := wp_api/src/wp_api.udl

# The directory where the git repo is mounted in the docker container
docker_container_repo_dir=/app

# Common docker options
docker_opts_shared :=  --rm -v "$(PWD)":$(docker_container_repo_dir) -w $(docker_container_repo_dir)
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

	#wp_networking
	cargo run --release --bin uniffi_bindgen generate --library ./target/release/libwp_networking.dylib --out-dir $(android_generated_source_path) --language kotlin
	cargo run --release --bin uniffi_bindgen generate wp_networking/src/wp_networking.udl --out-dir ./target/swift-bindings --language swift
	cp target/swift-bindings/wp_networking.swift native/swift/Sources/wordpress-api-wrapper/wp_networking.swift

	#wp_parsing
	cargo run --release --bin uniffi_bindgen generate --library ./target/release/libwp_parsing.dylib --out-dir $(android_generated_source_path) --language kotlin
	cargo run --release --bin uniffi_bindgen generate wp_parsing/src/wp_parsing.udl --out-dir ./target/swift-bindings --language swift
	cp target/swift-bindings/wp_parsing.swift native/swift/Sources/wordpress-api-wrapper/wp_parsing.swift

_test-android:
	./native/android/gradlew -p ./native/android cAT

_publish-android-local:
	./native/android/gradlew -p ./native/android publishToMavenLocal -exclude-task prepareToPublishToS3


# Builds the library for all the various architectures / systems required in an XCFramework
xcframework-libraries:
	# macOS
	cargo build --target x86_64-apple-darwin --release
	cargo build --target aarch64-apple-darwin --release

	# iOS
	cargo build --target aarch64-apple-ios --release
	cargo build --target x86_64-apple-ios --release
	cargo build --target aarch64-apple-ios-sim --release

	# tvOS
	cargo +nightly build --target aarch64-apple-tvos --release -Zbuild-std 
	cargo +nightly build --target aarch64-apple-tvos-sim --release -Zbuild-std
	cargo +nightly build --target x86_64-apple-tvos --release -Zbuild-std 

	# watchOS
	cargo +nightly build --target arm64_32-apple-watchos --release -Zbuild-std 
	cargo +nightly build --target aarch64-apple-watchos-sim --release -Zbuild-std
	cargo +nightly build --target x86_64-apple-watchos-sim --release -Zbuild-std 

# Some libraries need to be created in a multi-binary format, so we combine them here
%-xcframework-combined-libraries: xcframework-libraries

	rm -rf target/universal-*
	mkdir -p target/universal-macos/release target/universal-ios/release target/universal-tvos/release target/universal-watchos/release

	# Combine the macOS Binaries
	lipo -create target/aarch64-apple-darwin/release/lib$*.a target/x86_64-apple-darwin/release/lib$*.a \
		-output target/universal-macos/release/lib$*.a
	lipo -info target/universal-macos/release/lib$*.a

	# Combine iOS Simulator Binaries
	lipo -create target/aarch64-apple-ios-sim/release/lib$*.a target/x86_64-apple-ios/release/lib$*.a \
		-output target/universal-ios/release/lib$*.a
	lipo -info target/universal-ios/release/lib$*.a

	# Combine tvOS Simulator Binaries
	lipo -create target/aarch64-apple-tvos-sim/release/lib$*.a target/x86_64-apple-tvos/release/lib$*.a \
		-output target/universal-tvos/release/lib$*.a
	lipo -info target/universal-tvos/release/lib$*.a

	# Combine watchOS Simulator Binaries
	lipo -create target/aarch64-apple-watchos-sim/release/lib$*.a target/x86_64-apple-watchos-sim/release/lib$*.a \
		-output target/universal-watchos/release/lib$*.a
	lipo -info target/universal-watchos/release/lib$*.a

# An XCFramework relies on the .h file and the modulemap to interact with the precompiled binary
%-xcframework-headers: xcframework-libraries
	mkdir -p target/swift-bindings/$*-headers

	cp target/swift-bindings/$*FFI.h target/swift-bindings/$*-headers/$*FFI.h
	cp target/swift-bindings/$*FFI.modulemap target/swift-bindings/$*-headers/module.modulemap

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
%-xcframework: bindings %-xcframework-headers %-xcframework-combined-libraries
	rm -rf target/$*.xcframework
	xcodebuild -create-xcframework \
		-library target/aarch64-apple-ios/release/lib$*.a \
		-headers target/swift-bindings/$*-headers \
		-library target/universal-macos/release/lib$*.a \
		-headers target/swift-bindings/$*-headers \
		-library target/universal-ios/release/lib$*.a \
		-headers target/swift-bindings/$*-headers \
		-library target/aarch64-apple-tvos/release/lib$*.a \
		-headers target/swift-bindings/$*-headers \
		-library target/universal-tvos/release/lib$*.a \
		-headers target/swift-bindings/$*-headers \
		-library target/universal-watchos/release/lib$*.a \
		-headers target/swift-bindings/$*-headers \
		-output target/$*.xcframework

xcframeworks:
	$(MAKE) wp_api-xcframework
	$(MAKE) wp_networking-xcframework
	$(MAKE) wp_parsing-xcframework

test-swift: xcframeworks
	swift test

test-android: bindings _test-android

publish-android-local: bindings _publish-android-local

build-in-docker:
	$(call bindings)
	$(docker_build_and_run)
