
export CARGO_NET_GIT_FETCH_WITH_CLI = true

all: fmt ci

ci: lint release

install:
	cargo install --path src/tt

release:
	cargo build --release --bins

lint:
	cargo clippy --workspace --bins
	cargo check --workspace --tests
	cargo check --workspace --benches
	@# cargo check --workspace --examples

test:
	cargo test --workspace --tests --bins -- --test-threads=1 --nocapture
	cargo test --workspace --release --tests --bins -- --test-threads=1 --nocapture

bench:
	cargo bench --workspace

fmt:
	cargo +nightly fmt

fmtall:
	bash tools/fmt.sh

update:
	cargo update

clean:
	cargo clean
	git stash
	git clean -fdx

doc:
	cargo doc --open
