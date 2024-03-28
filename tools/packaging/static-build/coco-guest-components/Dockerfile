# Copyright (c) 2024 Intel
# Copyright (c) 2024 IBM Corporation
#
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:20.04
ARG RUST_TOOLCHAIN

ENV DEBIAN_FRONTEND=noninteractive

# Note - the TDX lib is only available on x86, so there is an arch check in the package install
ENV RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static/
ENV RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
ENV https_proxy=http://proxy.cd.intel.com:912
ENV http_proxy=http://proxy.cd.intel.com:912
ENV no_proxy="10.239.0.0/16,172.168.0.0/16,127.0.0.0/8,localhost,10.0.0.0/8,192.168.0.0/16,192.168.14.0/24,.intel.com,100.64.0.0/10,172.16.0.0/12,10.112.0.0/16"
COPY ./rust.config /root/.cargo/config.toml
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN apt-get update && \
	apt-get --no-install-recommends install -y \
	ca-certificates \
	curl \
	gnupg && \
	if [ "$(uname -m)" == "x86_64" ]; then curl -sL https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key | gpg --dearmor --output /usr/share/keyrings/intel-sgx.gpg && \
	echo 'deb [arch=amd64 signed-by=/usr/share/keyrings/intel-sgx.gpg] https://download.01.org/intel-sgx/sgx_repo/ubuntu jammy main' | tee /etc/apt/sources.list.d/intel-sgx.list && \
	apt-get update && \
	apt-get --no-install-recommends -y install libtdx-attest-dev; fi && \
	apt-get --no-install-recommends -y install \
	binutils \
	clang \
	g++ \
	gcc \
	git \
	libssl-dev \
	libtss2-dev \
	make \
	musl-tools \
	openssl \
	perl \
	pkg-config \
	protobuf-compiler && \
	apt-get clean && rm -rf /var/lib/apt/lists/ && \
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_TOOLCHAIN}

# aarch64 requires this name -- link for all
RUN ln -s /usr/bin/musl-gcc "/usr/bin/$(uname -m)-linux-musl-gcc"
