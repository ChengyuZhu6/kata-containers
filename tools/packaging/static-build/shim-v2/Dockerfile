# Copyright (c) 2021 Intel Corporation
#
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:20.04
ENV DEBIAN_FRONTEND=noninteractive
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
        gcc \
        git \
        make \
        musl-tools \
        protobuf-compiler \
        sudo && \
    apt-get clean && rm -rf /var/lib/apt/lists/

COPY install_go_rust.sh /usr/bin/install_go_rust.sh
ARG GO_VERSION
ARG RUST_VERSION
RUN install_go_rust.sh "${GO_VERSION}" "${RUST_VERSION}"
ENV PATH=/usr/local/go/bin:${PATH}
ENV PATH=/root/.cargo/bin/:${PATH}
