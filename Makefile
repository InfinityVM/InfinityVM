.PHONY: clippy-ci
clippy-ci: contracts
	SP1_SKIP_BUILD=true RUSTFLAGS="-D warnings" cargo clippy --workspace --lib --examples --tests --benches --all-features --locked

.PHONY: clippy
clippy:
	SP1_SKIP_BUILD=true cargo clippy --fix --allow-dirty --workspace --tests --all-features

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
	SP1_SKIP_BUILD=true RUSTDOCFLAGS="--cfg docsrs -D warnings" \
	cargo doc --document-private-items --no-deps --open

# Use this for iterating on integration tests
.PHONY: test-all
test-all: contracts
	@# Make sure to run the ignored tests
	cargo test --all -- --include-ignored --nocapture

.PHONY: test-all-ci
test-all-ci: contracts
	@# Make sure to run the ignored tests
	@# We use release because the total build + test execution time in CI is faster,
	@# however it tends to be slower locally.
	cargo test --all --release -- --include-ignored --nocapture

.PHONY: build
build: contracts
	cargo build

# IMPORTANT: if you update the contracts that ivm-contract crates rely on
# you will need to update this script
.PHONY: contracts
contracts:
	cd contracts && forge build
	cp contracts/out/JobManager.sol/JobManager.json crates/contracts/json
	cp contracts/out/IJobManager.sol/IJobManager.json crates/contracts/json
	cp contracts/out/MockConsumer.sol/MockConsumer.json crates/contracts/json
	cp contracts/out/TransparentUpgradeableProxy.sol/TransparentUpgradeableProxy.json crates/contracts/json
	cp contracts/out/ProxyAdmin.sol/ProxyAdmin.json crates/contracts/json
	cp contracts/out/Consumer.sol/Consumer.json crates/contracts/json

.PHONY: run
run:
	cargo run --bin local
