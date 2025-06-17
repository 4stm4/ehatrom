FROM rust:bullseye
WORKDIR /ocultum
RUN apt-get update && apt-get install -y libi2c-dev
RUN rustup install nightly && \
    rustup component add --toolchain nightly rustfmt && \
    rustup component add --toolchain nightly clippy && \
    rustup component add --toolchain nightly rust-docs
