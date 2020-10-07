
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
		&& cargo build --release --target-dir=$(BUILD_DIR) \
		&& cp $(BUILD_DIR)/$(TARGET)/release/firecracker $(PACK_DIR)/
	\
	chmod -R +x $(PACK_DIR)
	tar -C $(BUILD_DIR) -zcf $(PACK_NAME).tar.gz $(PACK_NAME)

install:
	$(CARGO) install -f --bins --path ./rexec --root=/usr/local/
	$(CARGO) install -f --bins --path ./client --root=/usr/local/
	$(CARGO) install -f --bins --path ./server --root=/usr/local/ # will fail on MacOS
	$(CARGO) install -f --bins --path ./proxy --root=/usr/local/  # will fail on MacOS

lint: githook
	$(CARGO) clippy
	cd core && $(CARGO) clippy --features="testmock"
	cd core && $(CARGO) clippy --no-default-features
	cd core && $(CARGO) clippy --no-default-features --features="zfs"
	cd core && $(CARGO) clippy --no-default-features --features="cow"
	cd core && $(CARGO) clippy --no-default-features --features="nft"
	cd core && $(CARGO) clippy --no-default-features --features="cow nft"
	cd server && $(CARGO) clippy --features="testmock"
	cd server && $(CARGO) clippy --no-default-features
	cd server && $(CARGO) clippy --no-default-features --features="zfs"
	cd server && $(CARGO) clippy --no-default-features --features="cow"
	cd server && $(CARGO) clippy --no-default-features --features="nft"
	cd server && $(CARGO) clippy --no-default-features --features="cow nft"
	cd proxy && $(CARGO) clippy --features="testmock"

test: stop
	$(CARGO) test -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --no-default-features --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --no-default-features --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd proxy && $(CARGO) test --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration

test_release: stop
	$(CARGO) test --release -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --release --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --release --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --release --no-default-features --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd server && $(CARGO) test --release --no-default-features --features="testmock, cow" -- --test-threads=1 --nocapture
	-@ pkill -9 integration
	cd proxy && $(CARGO) test --release --features="testmock" -- --test-threads=1 --nocapture
	-@ pkill -9 integration

fmt:
	@ ./tools/fmt.sh

doc:
	$(CARGO) doc --open -p tt
	$(CARGO) doc --open -p ttproxy
	$(CARGO) doc --open -p ttserver
	$(CARGO) doc --open -p ttcore
	$(CARGO) doc --open -p ttrexec

githook:
	@mkdir -p ./.git/hooks # play with online gitlab-ci
	@cp ./tools/pre-commit ./.git/hooks/

stop:
	-@ pkill -9 ttproxy
	-@ pkill -9 ttserver
	-@ pkill -9 ttrexec-daemon

clean:
	@ git clean -fdx
	@ $(CARGO) clean
	@ find . -type f -name "Cargo.lock" | xargs rm -f

cleanall: clean
	@ find . -type d -name "target" | xargs rm -rf
