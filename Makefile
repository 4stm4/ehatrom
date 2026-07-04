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

# Acceptance check against the reference eepmake/eepdump tools (raspberrypi/utils).
# Not part of CI — requires those binaries locally. See the script header.
acceptance:
	tests/acceptance/eepmake_compat.sh

.PHONY: ci ci_local acceptance
