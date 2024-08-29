# These where required for building and running tests with sp1
# TODO: https://github.com/Ethos-Works/InfinityVM/issues/120
# RUSTFLAGS = '-Copt-level=3 -Cdebug-assertions -Coverflow-checks=y -Cdebuginfo=0 -C target-cpu=native'

.PHONY: clippy-ci
clippy-ci: contracts
	RISC0_SKIP_BUILD=true RUSTFLAGS="-D warnings" cargo clippy --workspace --lib --examples --tests --benches --all-features --locked

.PHONY: clippy
clippy:
	RISC0_SKIP_BUILD=true cargo clippy --fix --allow-dirty --workspace

.PHONY: fmt-ci
fmt-ci: 
	cargo +nightly fmt --all --check

.PHONY: fmt
fmt:
	cargo +nightly fmt --all

.PHONY: lint
lint: clippy fmt

.PHONY: doc-ci
doc-ci: contracts
	RUSTDOCFLAGS="--cfg docsrs -D warnings" \
	cargo doc --document-private-items --no-deps

.PHONY: doc
doc: contracts
	RUSTDOCFLAGS="--cfg docsrs -D warnings" \
	cargo doc --document-private-items --no-deps --open

# Use this for iterating on integration tests
.PHONY: test-all
test-all: contracts debug-coprocessor-node debug-clob-node
	@# Make sure to build the bins that the integration tests rely on
	@# and run the ignored tests
	cargo test --all -- --include-ignored --nocapture

.PHONY: build
build: contracts
	cargo build

.PHONY: contracts
contracts:
	cd contracts && forge build

.PHONY: debug-coprocessor-node
debug-coprocessor-node:
	cargo build -p coprocessor-node

.PHONY: debug-clob-node
debug-clob-node:
	cargo build -p clob-node
