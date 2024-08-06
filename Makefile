# These where required for building and running tests with sp1
# TODO: https://github.com/Ethos-Works/InfinityVM/issues/120
# RUSTFLAGS = '-Copt-level=3 -Cdebug-assertions -Coverflow-checks=y -Cdebuginfo=0 -C target-cpu=native'

.PHONY: clippy-ci
clippy-ci:
	cd contracts && forge build
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
doc-ci:
	cd contracts && forge build
	RUSTDOCFLAGS="--cfg docsrs -D warnings" \
	cargo doc --document-private-items --no-deps

.PHONY: doc
doc:
	RUSTDOCFLAGS="--cfg docsrs -D warnings" \
	cargo doc --document-private-items --no-deps --open

.PHONY: build-release
build-release:
	cargo build --release

# Use this for iterating on integration tests
.PHONY: test-all
test-all:
	@# Make sure to build the bins that the integration tests rely on
	@# and run the ignored tests
	cd contracts && forge build
	cargo build -p coprocessor-node && \
	cargo test --all -- --include-ignored --nocapture

.PHONY: build
build:
	cargo build