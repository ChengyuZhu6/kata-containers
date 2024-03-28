# Copyright (c) 2022 Intel
#
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:20.04
ENV DEBIAN_FRONTEND=noninteractive
ARG RUST_TOOLCHAIN
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ENV RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static/
ENV RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
ENV https_proxy=http://proxy.cd.intel.com:912
ENV http_proxy=http://proxy.cd.intel.com:912
ENV no_proxy="10.239.0.0/16,172.168.0.0/16,127.0.0.0/8,localhost,10.0.0.0/8,192.168.0.0/16,192.168.14.0/24,.intel.com,100.64.0.0/10,172.16.0.0/12,10.112.0.0/16"
COPY ./rust.config /root/.cargo/config.toml
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        libcap-ng-dev \
        libseccomp-dev \
        unzip && \
    apt-get clean && rm -rf /var/lib/lists/ && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_TOOLCHAIN}
