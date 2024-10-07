.DEFAULT_GOAL := help

# The directory where the git repo is mounted in the docker container
docker_container_repo_dir=/app

# Common docker options
rust_docker_container := public.ecr.aws/docker/library/rust:1.80

docker_opts_shared := --rm -v "$(PWD)":$(docker_container_repo_dir) -w $(docker_container_repo_dir)
rust_docker_run := docker run -v $(PWD):/$(docker_container_repo_dir) -w $(docker_container_repo_dir) -it -e CARGO_HOME=/app/.cargo $(rust_docker_container)
docker_build_and_run := docker build -t foo . && docker run $(docker_opts_shared) -it foo

swift_package_platform_version = $(shell swift package dump-package | jq -r '.platforms[] | select(.platformName=="$1") | .version')
swift_package_platform_macos = $(call swift_package_platform_version,macos)
swift_package_platform_ios = $(call swift_package_platform_version,ios)
swift_package_platform_watchos = $(call swift_package_platform_version,watchos)
swift_package_platform_tvos = $(call swift_package_platform_version,tvos)

# Required for supporting tvOS and watchOS. We can update the nightly toolchain version if needed.
rust_nightly_toolchain := nightly-2024-04-30

uname := $(shell uname | tr A-Z a-z)
ifeq ($(uname), linux)
	dylib_ext := so
endif
ifeq ($(uname), darwin)
	dylib_ext := dylib
endif

define MODULEMAP_CONTENT
module libwordpressFFI {
  header "libwordpressFFI.h"
  export *
}
endef

clean:
	@# Help: Remove untracked files from the project via Git.
	git clean -ffXd

bindings:
	rm -rf target/swift-bindings
	cargo build --release

	cargo run --release --bin wp_uniffi_bindgen generate --library ./target/release/libwp_api.$(dylib_ext) --out-dir ./target/swift-bindings --language swift
	cargo run --release --bin wp_uniffi_bindgen generate --library ./target/release/libjetpack_api.$(dylib_ext) --out-dir ./target/swift-bindings --language swift
	cp target/swift-bindings/*.swift native/swift/Sources/wordpress-api-wrapper/

.PHONY: docs # Rebuild docs each time we run this command
docs:
	@# Help: Generate project documentation.
	rm -rf docs
	mkdir -p docs
	$(rust_docker_run) /bin/bash -c 'cargo doc'
	cp -r target/doc/static.files docs/static.files
	cp -r target/doc/wp_api docs/wp_api
	cp -r target/doc/wp_contextual docs/wp_contextual

docs-archive: docs
	@# Help: Archive the generated project documentation.
	tar -czvf  docs.tar.gz docs

release-on-ci:
	@[ -n "$(BUILDKITE_API_TOKEN)" ] || (echo "BUILDKITE_API_TOKEN is not set" && exit 1)
	@[ -n "$(WORDPRESS_RS_NEW_VERSION)" ] || (echo "WORDPRESS_RS_NEW_VERSION is not set" && exit 1)

	@echo "Triggering a release job on Buildkite. New version: $(WORDPRESS_RS_NEW_VERSION)"

	@mkdir -p .build
	@echo '{ \
			"commit": "HEAD", \
			"branch": "trunk", \
			"message": "Publishing a new release", \
			"env": {"NEW_VERSION":"${WORDPRESS_RS_NEW_VERSION}"} \
		}' | jq > .build/buildkite_release_job_request.json

	@curl -s "https://api.buildkite.com/v2/organizations/automattic/pipelines/wordpress-rs/builds" \
		-H "Authorization: Bearer $(BUILDKITE_API_TOKEN)" \
		--json @.build/buildkite_release_job_request.json \
		--output .build/buildkite_release_job_response.json

	@echo "Buildkite job triggerd. See .build/buildkite_release_job_response.json for the buildkite job details."
	@echo ""
	@echo "Swift package will be released by https://buildkite.com/automattic/wordpress-rs/builds/$$(jq -r '.number' .build/buildkite_release_job_response.json)"
	@echo "Once that job finishes, Android libraries will be release by https://buildkite.com/automattic/wordpress-rs/builds?branch=$(WORDPRESS_RS_NEW_VERSION)"

# An XCFramework relies on the .h file and the modulemap to interact with the precompiled binary
export MODULEMAP_CONTENT
xcframework-headers: bindings
	rm -rvf target/swift-bindings/headers
	mkdir -p target/swift-bindings/headers

	cp target/swift-bindings/*.h target/swift-bindings/headers
	find target/swift-bindings/headers -name '*.h' -exec basename {} \; | xargs -I {} echo '#include "{}"' > target/swift-bindings/headers/libwordpressFFI.h
	echo "$$MODULEMAP_CONTENT" > target/swift-bindings/headers/module.modulemap

apple-platform-targets-macos := x86_64-apple-darwin aarch64-apple-darwin
apple-platform-targets-ios := aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
apple-platform-targets-tvos := aarch64-apple-tvos aarch64-apple-tvos-sim
apple-platform-targets-watchos := arm64_32-apple-watchos x86_64-apple-watchos-sim aarch64-apple-watchos-sim
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

cargo_config_library = --config profile.$(CARGO_PROFILE).debug=true --config 'profile.$(CARGO_PROFILE).panic="abort"'

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
	cargo $(CARGO_OPTS) $(cargo_config_library) build --target $* --package wp_api --profile $(CARGO_PROFILE)
	cargo $(CARGO_OPTS) $(cargo_config_library) build --target $* --package jetpack_api --profile $(CARGO_PROFILE)

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
	cargo run --quiet --bin xcframework -- --profile $(CARGO_PROFILE) --targets $(apple-platform-targets-$*)

# Creating xcframework for all platforms.
xcframework-all: $(build-apple-platform-macos) $(build-apple-platform-ios) $(build-apple-platform-tvos) $(build-apple-platform-watchos)
	cargo run --quiet --bin xcframework -- --profile $(CARGO_PROFILE) --targets $(apple-platform-targets)

ifeq ($(SKIP_PACKAGE_WP_API),true)
xcframework:
	@echo "Skip building libwordpressFFI.xcframework"
else
xcframework: xcframework-all
endif

xcframework-package: xcframework-all
	rm -rf libwordpressFFI.xcframework.zip
	ditto -c -k --sequesterRsrc --keepParent target/libwordpressFFI.xcframework/ libwordpressFFI.xcframework.zip

xcframework-package-checksum:
	swift package compute-checksum libwordpressFFI.xcframework.zip | tee libwordpressFFI.xcframework.zip.checksum.txt


docker-image-swift:
	docker build -t wordpress-rs-swift -f Dockerfile.swift .

swift-linux-library: bindings
	mkdir -p target/swift-bindings/libwordpressFFI-linux
	cp target/swift-bindings/*.h target/swift-bindings/libwordpressFFI-linux/
	cp target/swift-bindings/libwordpressFFI.modulemap target/swift-bindings/libwordpressFFI-linux/module.modulemap
	cp target/release/libwp_api.a target/swift-bindings/libwordpressFFI-linux/

swift-example-app: swift-example-app-mac swift-example-app-ios

swift-example-app-mac:
	xcodebuild -project native/swift/Example/Example.xcodeproj -scheme Example -destination 'platform=macOS,arch=arm64' -skipPackagePluginValidation build

swift-example-app-ios:
	xcodebuild -project native/swift/Example/Example.xcodeproj -scheme Example -destination 'platform=iOS,name=iPhone 15' -skipPackagePluginValidation build

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

test-rust-lib:
	$(rust_docker_run) cargo test --lib -- --nocapture

test-rust-doc:
	$(rust_docker_run) cargo test --doc -- --nocapture

test-rust-wp-derived-request-parser:
	$(rust_docker_run) cargo test --package wp_derive_request_builder

test-rust-integration:
	@# Help: Run Rust integration tests in test server.
	docker exec -i wordpress /bin/bash < ./scripts/run-rust-integration-tests.sh

test-kotlin-integration:
	@# Help: Run Kotlin integration tests in test server.
	docker exec -i wordpress /bin/bash < ./scripts/run-kotlin-integration-tests.sh

restore-test-server:
	@# Help: Restore the test server from backup.
	curl "http://localhost:4000/restore?db=true&plugins=true"

start-test-server: stop-server
	@# Help: Start the test server.
	docker-compose up -d --build
	docker exec -i wordpress /bin/bash < ./scripts/setup-test-site.sh

integration-test-backend:
	@# Help: Start the integration test helper server.
	docker exec -i wordpress /bin/bash -c " if pgrep wp_api_integ; then pkill wp_api_integ; fi" # Kill the previous server
	docker exec -i wordpress /bin/bash < ./scripts/start-wp-api-integration-tests-backend.sh

test-server: start-test-server integration-test-backend

print-log-integration-test-server:
	@# Help: Print the logs of integration test helper server.
	docker exec -i wordpress /bin/bash -c "cat /app/target/release/wp_api_integration_tests_backend.log"

stop-server:
	@# Help: Stop the running server.
	docker-compose down

lint: lint-rust lint-swift
	@# Help: Run the linter for all languages.

lint-rust:
	@# Help: Run the linter for Rust.
	$(rust_docker_run) /bin/bash -c "rustup component add clippy && cargo clippy --all -- -D warnings && cargo clippy --tests --all -- -D warnings"

lint-swift:
	@# Help: Run the linter for Swift.
	swift package plugin swiftlint

lintfix-swift:
	@# Help: Run the linter for Swift and correct fixable issues.
	swift package plugin swiftlint --autocorrect

fmt-rust:
	$(rust_docker_run) /bin/bash -c "rustup component add rustfmt && cargo fmt"

fmt-check-rust:
	$(rust_docker_run) /bin/bash -c "rustup component add rustfmt && cargo fmt --all -- --check"

build-in-docker:
	$(call bindings)
	$(docker_build_and_run)

setup-rust:
	@# Help: Install the necessary Rust toolchains on your development computer (for macOS).
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

setup-rust-android-targets:
	rustup target add \
		x86_64-linux-android \
		i686-linux-android \
		armv7-linux-androideabi \
		aarch64-linux-android

run-wp-cli-command:
	@docker exec wordpress /bin/bash -c "wp --allow-root $(ARGS)"

help:
	@printf "%-40s %s\n" "Target" "Description"
	@printf "%-40s %s\n" "------" "-----------"
	@make -pqR : 2>/dev/null \
		| awk -v RS= -F: '/^# File/,/^# Finished Make data base/ {if ($$1 !~ "^[#.]") {print $$1}}' \
		| sort \
		| egrep -v -e '^[^[:alnum:]]' -e '^$@$$' \
		| xargs -I _ sh -c 'printf "%-40s " _; make _ -nB | (grep -i "^# Help:" || echo "") | tail -1 | sed "s/^# Help: //g"'
