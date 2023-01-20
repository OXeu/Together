FROM rust:alpine3.17
ENV RUSTUP_DIST_SERVER=https://rsproxy.cn
ENV RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup
ENV CARGO_UNSTABLE_SPARSE_REGISTRY=true
ENV CARGO_TARGET_DIR=/cargo
VOLUME ["/cargo"]
COPY ./config ~/.cargo/config
RUN source "$HOME/.cargo/env" && cargo install sccache
ENV RUSTC_WRAPPER=sccache