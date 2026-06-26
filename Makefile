.DEFAULT_GOAL := help

# The directory where the git repo is mounted in the docker container
docker_container_repo_dir=/app

# Common docker options
rust_docker_container := public.ecr.aws/docker/library/rust:1.90.0

docker_opts_shared := --rm -v "$(PWD)":$(docker_container_repo_dir) -w $(docker_container_repo_dir)
rust_docker_run := docker run -v $(PWD):/$(docker_container_repo_dir) -w $(docker_container_repo_dir) -it -e TEST_ALL_PLUGINS -e CARGO_HOME=/app/.cargo $(rust_docker_container)
docker_build_and_run := docker build -t foo . && docker run $(docker_opts_shared) -it foo

swift_package_platform_version = $(shell swift package dump-package | jq -r '.platforms[] | select(.platformName=="$1") | .version')
swift_package_platform_macos = $(call swift_package_platform_version,macos)
swift_package_platform_ios = $(call swift_package_platform_version,ios)
swift_package_platform_watchos = $(call swift_package_platform_version,watchos)
swift_package_platform_tvos = $(call swift_package_platform_version,tvos)

certificate_name_release = Apple Distribution: Automattic, Inc. (PZYM8XX95Q)

# Required for supporting tvOS and watchOS. We can update the nightly toolchain version if needed.
rust_nightly_toolchain := nightly-2025-07-29

clean:
	@# Help: Remove untracked files from the project via Git.
	git clean -ffXd

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
	tar -czvf docs.tar.gz docs

swift-docs: xcframework swift-docs-only
	@# Help: Generate Xcode documentation.

swift-docs-only:
	mkdir -p docs/Swift
	swift package --allow-writing-to-directory docs generate-documentation --target WordPressAPI --output-path docs/Swift/WordPressAPI.doccarchive --disable-indexing
	swift package --allow-writing-to-directory docs generate-documentation --target WordPressAPIInternal --output-path docs/Swift/WordPressAPIInternal.doccarchive --disable-indexing
	tar -czf swift-docs.tar.gz docs

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
CARGO_PROFILE_DIRNAME := release
else
CARGO_PROFILE ?= dev
CARGO_PROFILE_DIRNAME := debug
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
_build-apple-%:
	cargo $(CARGO_OPTS) $(cargo_config_library) build --target $* --features export-uncancellable-endpoints --package wp_mobile --profile $(CARGO_PROFILE) --no-default-features
	./scripts/swift-bindings.sh target/$*/$(CARGO_PROFILE_DIRNAME)/libwp_mobile.a

# Build the library for one single platform, including real device and simulator.
build-apple-platform-macos := $(addprefix _build-apple-,$(apple-platform-targets-macos))
build-apple-platform-ios := $(addprefix _build-apple-,$(apple-platform-targets-ios))
build-apple-platform-tvos := $(addprefix _build-apple-,$(apple-platform-targets-tvos))
build-apple-platform-watchos := $(addprefix _build-apple-,$(apple-platform-targets-watchos))

# Build all targets for a specific platform (without creating xcframework).
build-apple-macOS: $(build-apple-platform-macos)
build-apple-iOS: $(build-apple-platform-ios)
build-apple-tvOS: $(build-apple-platform-tvos)
build-apple-watchOS: $(build-apple-platform-watchos)

# Creating xcframework for one single platform, including real device and simulator.
xcframework-only-macos: $(build-apple-platform-macos)
xcframework-only-ios: $(build-apple-platform-ios)
xcframework-only-tvos: $(build-apple-platform-tvos)
xcframework-only-watchos: $(build-apple-platform-watchos)
xcframework-only-%:
	cargo run --quiet --bin xcframework -- --profile $(CARGO_PROFILE) --targets $(apple-platform-targets-$*)

# Assemble pre-built targets into an xcframework (without building targets).
xcframework-assemble:
	cargo run --quiet --bin xcframework -- --profile $(CARGO_PROFILE) --targets $(apple-platform-targets)

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
	rm -rf target/libwordpressFFI.xcframework.zip
	ditto -c -k --sequesterRsrc --keepParent target/libwordpressFFI.xcframework/ target/libwordpressFFI.xcframework.zip

xcframework-package-sign: xcframework-all xcframework-sign xcframework-package

xcframework-package-checksum:
	swift package compute-checksum target/libwordpressFFI.xcframework.zip | tee target/libwordpressFFI.xcframework.zip.checksum.txt

xcframework-sign:
	codesign --timestamp -v --sign "${certificate_name_release}" target/libwordpressFFI.xcframework
	codesign -dvv target/libwordpressFFI.xcframework


docker-image-web:
	docker build -t wordpress-rs-web -f wp_rs_web/Dockerfile . --progress=plain

swift-linux-library:
	cargo build --release --features export-uncancellable-endpoints --package wp_mobile
	./scripts/swift-bindings.sh target/release/libwp_mobile.a
	mkdir -p target/release/libwordpressFFI-linux
	cp target/release/swift-bindings/Headers/* target/release/libwordpressFFI-linux/
	cp target/release/libwp_mobile.a target/release/libwordpressFFI-linux/

swift-example-app: swift-example-app-mac swift-example-app-ios

swift-example-app-mac:
	xcodebuild -project native/swift/Example/Example.xcodeproj -scheme Example -destination 'platform=macOS,arch=arm64' -skipPackagePluginValidation build

swift-example-app-ios:
	xcrun simctl create "iPhone 17 Pro Test Device" "com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro"
	bundle exec fastlane run run_tests project:native/swift/Example/Example.xcodeproj scheme:Example build_for_testing:true ensure_devices_found:true device:"iPhone 17 Pro Test Device (26.1)" xcargs:"-skipPackagePluginValidation"

test-swift:
	$(MAKE) test-swift-$(uname)

test-swift-linux:
	docker exec -w /app -it wordpress make test-swift-linux-in-docker

test-swift-linux-in-docker: swift-linux-library
	swift test -Xlinker -Ltarget/release/libwordpressFFI-linux -Xlinker -lwp_mobile --no-parallel

test-swift-darwin: xcframework
	swift test

test-swift-macOS: test-swift-darwin

test-swift-iOS: xcframework
	scripts/xcodebuild-test.sh iOS-26-1

test-swift-tvOS: xcframework
	scripts/xcodebuild-test.sh tvOS-26-1

test-swift-watchOS: xcframework
	scripts/xcodebuild-test.sh watchOS-26-1

test-rust-lib:
	$(rust_docker_run) cargo test --lib -- --nocapture

test-rust-doc:
	$(rust_docker_run) cargo test --doc -- --nocapture

test-rust-wp-derived-request-parser:
	$(rust_docker_run) cargo test --package wp_derive_request_builder

test-rust-integration:
	@# Help: Run Rust integration tests in test server.
	docker exec -i wordpress /bin/bash < ./scripts/run-rust-integration-tests.sh

test-rust-integration-wordpress-org-api:
	$(rust_docker_run) cargo test --package wp_api_integration_tests --test test_plugin_directory -- --nocapture

test-kotlin-integration:
	@# Help: Run Kotlin integration tests in test server.
	docker exec -i wordpress /bin/bash < ./scripts/run-kotlin-integration-tests.sh

runComposeDesktopApp:
	@# Help: Run the Compose Multiplatform desktop application.
	cd native/kotlin && ./gradlew :example:composeApp:run

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
	xcrun swift format lint --strict --recursive --parallel --ignore-unparsable-files \
		native/swift/Sources/wordpress-api \
		native/swift/Sources/wordpress-api-cache \
		native/swift/Tests \
		native/swift/Example \
		native/swift/Tools

lintfix-swift: fmt-swift

fmt-rust:
	$(rust_docker_run) /bin/bash -c "rustup component add rustfmt && cargo fmt"

fmt-check-rust:
	$(rust_docker_run) /bin/bash -c "rustup component add rustfmt && cargo fmt --all -- --check"

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

# Platform-specific Rust setup (only installs what each platform needs in CI)
setup-rust-macOS:
	rustup toolchain install stable
	rustup target add --toolchain stable x86_64-apple-darwin aarch64-apple-darwin

setup-rust-iOS:
	rustup toolchain install stable
	rustup target add --toolchain stable aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim

setup-rust-tvOS setup-rust-watchOS:
	rustup toolchain install stable
	rustup toolchain install $(rust_nightly_toolchain)
	rustup component add rust-src --toolchain $(rust_nightly_toolchain)

setup-rust-android-targets:
	rustup target add \
		x86_64-linux-android \
		i686-linux-android \
		armv7-linux-androideabi \
		aarch64-linux-android

run-wp-cli-command:
	@docker exec wordpress /bin/bash -c "wp --allow-root $(ARGS)"

validate-localizations:
	@# Help: Validate localization files using `wp_localization_validation` crate
	$(rust_docker_run) /bin/bash -c "cargo run --bin wp_localization_validation -- --localization-folder ./wp_localization/localization/"

fmt-swift:
	@# Help: Format Swift code.
	xcrun swift format --in-place --recursive --parallel --ignore-unparsable-files native/swift

help:
	@printf "%-40s %s\n" "Target" "Description"
	@printf "%-40s %s\n" "------" "-----------"
	@make -pqR : 2>/dev/null \
		| awk -v RS= -F: '/^# File/,/^# Finished Make data base/ {if ($$1 !~ "^[#.]") {print $$1}}' \
		| sort \
		| egrep -v -e '^[^[:alnum:]]' -e '^$@$$' \
		| xargs -I _ sh -c 'printf "%-40s " _; make _ -nB | (grep -i "^# Help:" || echo "") | tail -1 | sed "s/^# Help: //g"'
