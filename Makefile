android_project_root := ./native/android
android_generated_source_path := $(android_project_root)/lib/build/generated/source
jni_libs_root := $(android_project_root)/lib/src/main/jniLibs
udl_path := wordpress_api/src/wordpress_api.udl

android-build:
	cargo build --release
	cargo run --release --bin uniffi_bindgen generate wordpress_api/src/wordpress_api.udl --out-dir out --language kotlin
	./native/android/gradlew build -p ./native/android

generate-jni-libs:
	rm -rf $(jni_libs_root)
	cargo build --release --lib --target x86_64-linux-android --target i686-linux-android --target armv7-linux-androideabi --target aarch64-linux-android
	mkdir -p $(jni_libs_root)/arm64-v8a
	mkdir -p $(jni_libs_root)/armeabi-v7a
	mkdir -p $(jni_libs_root)/x86
	mkdir -p $(jni_libs_root)/x86_64
	cp ./target/aarch64-linux-android/release/libwordpress_api.so $(jni_libs_root)/arm64-v8a/libuniffi_wordpress_api.so
	cp ./target/armv7-linux-androideabi/release/libwordpress_api.so $(jni_libs_root)/armeabi-v7a/libuniffi_wordpress_api.so
	cp ./target/i686-linux-android/release/libwordpress_api.so $(jni_libs_root)/x86/libuniffi_wordpress_api.so
	cp ./target/x86_64-linux-android/release/libwordpress_api.so $(jni_libs_root)/x86_64/libuniffi_wordpress_api.so

generate-bindings:
	rm -rf $(android_generated_source_path)
	cargo run --release --bin uniffi_bindgen generate $(udl_path) --out-dir $(android_generated_source_path) --language kotlin
