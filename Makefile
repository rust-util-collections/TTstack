
CARGO = ~/.cargo/bin/cargo
BUILD_DIR = /tmp/.__tt_build_dir__
PACK_NAME = tt
PACK_DIR = $(BUILD_DIR)/$(PACK_NAME)
TARGET = $(shell rustup toolchain list | grep default | sed 's/^[^-]\+-//' | sed 's/ \+(default).*//')

all: pack

build:
	$(CARGO) build --bins

release:
	$(CARGO) build --bins --release

pack: release
	-@ rm -rf $(PACK_DIR)
	mkdir -p $(PACK_DIR)
	cd target/release && cp tt ttserver ttproxy $(PACK_DIR)/
	cp tools/install.sh $(PACK_DIR)/
	\
	git submodule update --init --recursive
	cd tools/firecracker \
		&& cargo build --release --target=$(TARGET) --target-dir=$(BUILD_DIR) \
		&& cp $(BUILD_DIR)/$(TARGET)/release/firecracker $(PACK_DIR)/
	\
	chmod -R +x $(PACK_DIR)
	tar -C $(BUILD_DIR) -zcf $(PACK_NAME).tar.gz $(PACK_NAME)

install:
	$(CARGO) install -f --bins --path src/rexec --root=/usr/local/
	$(CARGO) install -f --bins --path src/client --root=/usr/local/
	$(CARGO) install -f --bins --path src/server --root=/usr/local/ # will fail on MacOS
	$(CARGO) install -f --bins --path src/proxy --root=/usr/local/  # will fail on MacOS

lint: githook
	$(CARGO) clippy
	cd src/core && $(CARGO) clippy --features="testmock"
	cd src/core && $(CARGO) clippy --no-default-features
	cd src/core && $(CARGO) clippy --no-default-features --features="zfs"
	cd src/core && $(CARGO) clippy --no-default-features --features="cow"
	cd src/core && $(CARGO) clippy --no-default-features --features="nft"
	cd src/core && $(CARGO) clippy --no-default-features --features="cow nft"
	cd src/server && $(CARGO) clippy --features="testmock"
	cd src/server && $(CARGO) clippy --no-default-features
	cd src/server && $(CARGO) clippy --no-default-features --features="zfs"
	cd src/server && $(CARGO) clippy --no-default-features --features="cow"
	cd src/server && $(CARGO) clippy --no-default-features --features="nft"
	cd src/server && $(CARGO) clippy --no-default-features --features="cow nft"
	cd src/proxy && $(CARGO) clippy --features="testmock"

test: test_debug

test_debug: stop
	$(CARGO) test -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --no-default-features --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --no-default-features --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/proxy && $(CARGO) test --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration

test_release: stop
	$(CARGO) test --release -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --release --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --release --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --release --no-default-features --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/server && $(CARGO) test --release --no-default-features --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd src/proxy && $(CARGO) test --release --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration

fmt:
	@ ./tools/fmt.sh

doc:
	$(CARGO) doc --open -p tt
	$(CARGO) doc --open -p ttrexec
	$(CARGO) doc --open -p ttproxy
	$(CARGO) doc --open -p ttserver # will fail on MacOS
	$(CARGO) doc --open -p ttcore   # will fail on MacOS

githook:
	@mkdir -p ./.git/hooks # play with online gitlab-ci
	@cp ./tools/githooks/pre-commit ./.git/hooks/

stop:
	-@ pkill -9 ttproxy
	-@ pkill -9 ttserver
	-@ pkill -9 ttrexec-daemon

clean:
	@ git clean -fdx
	@ $(CARGO) clean
	@ find . -type f -name "Cargo.lock" | xargs rm -f

cleanall: clean
	@ rm -rf client core core_def server server_def proxy rexec
	@ find . -type d -name "target" | xargs rm -rf
