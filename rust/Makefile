.PHONY: clippy-ci
clippy-ci:
	RISC0_SKIP_BUILD=true RUSTFLAGS="-D warnings" cargo clippy --workspace --lib --examples --tests --benches --all-features --locked

.PHONY: clippy
clippy:
	RISC0_SKIP_BUILD=true cargo clippy --fix --allow-dirty

.PHONY: fmt-ci
fmt-ci: 
	cargo +nightly fmt --all --check

.PHONY: fmt
fmt:
	cargo +nightly fmt --all

.PHONY: lint
lint: clippy fmt

.PHONY: test-ci
test-ci:
	cargo build
	cargo test --all -- --ignored

.PHONY: doc-ci
doc-ci:
	RUSTDOCFLAGS="--cfg docsrs -D warnings" \
	cargo doc --document-private-items --no-deps

.PHONY: doc
doc:
	RUSTDOCFLAGS="--cfg docsrs -D warnings" \
	cargo doc --document-private-items --no-deps --open

# Use this for iterating on integration tests
.PHONY: test-all
test-all:
	@# Make sure to build the bins that the integration tests rely on
	@# and run the ignored tests
	cargo build && \
	cargo test -- --ignored

.PHONY: build
build:
	cargo build