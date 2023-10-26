android-build:
	cargo build --release
	cargo run --release --bin uniffi_bindgen generate wordpress_api/src/wordpress_api.udl --out-dir out --language kotlin
	./native/android/gradlew build -p ./native/android
