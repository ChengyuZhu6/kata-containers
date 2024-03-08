# Copyright (c) 2022 Intel
#
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:20.04
ENV DEBIAN_FRONTEND=noninteractive
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ARG RUST_TOOLCHAIN
ENV RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static/
ENV RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
ENV https_proxy=http://proxy.cd.intel.com:912
ENV http_proxy=http://proxy.cd.intel.com:912
COPY ./rust.config /root/.cargo/config.toml

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        clang \
        curl \
        gcc \
        git \
        llvm \
        make \
        nasm \
        patch && \
    apt-get clean && rm -rf /var/lib/lists/ && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain ${RUST_TOOLCHAIN} && \
    source "$HOME/.cargo/env" && \
    rustup component add rust-src && \
    cargo install cargo-xbuild
