.PHONY: clippy-ci
clippy-ci:
	RUSTFLAGS="-D warnings" cargo clippy --workspace --lib --examples --tests --benches --all-features --locked

.PHONY: clippy
clippy-fix:
	cargo clippy --fix --allow-dirty

.PHONY: fmt-ci
fmt-ci: 
	cargo +nightly fmt --workspace --lib --examples --tests --benches --all-features --locked

.PHONY: fmt
fmt:
	cargo +nightly fmt --all

.PHONY: test-ci
test-ci:
	cargo build
	cargo test --all -- --ignored

.PHONY: doc-ci
doc-ci:
	RUSTDOCFLAGS="--cfg docsrs --show-type-layout --generate-link-to-definition -D warnings" \
	cargo doc --document-private-items

.PHONY: doc
doc:
	RUSTDOCFLAGS="--cfg docsrs --show-type-layout --generate-link-to-definition -D warnings"
	cargo doc --document-private-items --open
