RELEASE_TARGET = target/arm-unknown-linux-gnueabihf/release/lightshow
INSTALL = /home/neil/Projects/home-automation/roles/lightshow

clean:
	cargo clean

run:
	cargo run

test:
	cargo test

release:
	cargo build --target=arm-unknown-linux-gnueabihf --release
	arm-linux-gnueabihf-strip $(RELEASE_TARGET)

install: release
	cp -f $(RELEASE_TARGET) $(INSTALL)/files/build
	cp -f Settings.yml $(INSTALL)/files/lib

install-to-device: release
	scp $(RELEASE_TARGET) root@gardenpi:/usr/local/bin/lightshow
	scp Settings.yml root@gardenpi:/var/lib/lightshow/Settings.yml

