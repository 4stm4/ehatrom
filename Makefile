ci:
	docker run --rm -it \
		-v "$(PWD)":/ehatrom \
		-w /ehatrom \
		ehatrom-ci \
		bash -c "cargo +nightly fmt --all -- --check && \
				cargo +nightly clippy --workspace --all-targets -- -D warnings && \
		         cargo build --workspace --all-targets --verbose && \
		         cargo test --workspace --all-targets --verbose"

ci_local:
	cd ehatrom && \
	cargo clippy --workspace --all-targets -- -D warnings && \
	cargo build --workspace --all-targets --verbose && \
	cargo test --workspace --all-targets --verbose
