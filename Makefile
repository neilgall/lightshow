RELEASE_TARGET = target/arm-unknown-linux-gnueabihf/release/lightshow

clean:
	cargo clean

run:
	cargo run

test:
	cargo test

release:
	cargo build --target=arm-unknown-linux-gnueabihf --release
	arm-linux-gnueabihf-strip $(RELEASE_TARGET)

