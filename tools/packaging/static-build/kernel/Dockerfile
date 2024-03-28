# Copyright (c) 2021 Intel Corporation
#
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
ENV https_proxy=http://proxy.cd.intel.com:912
ENV http_proxy=http://proxy.cd.intel.com:912
ENV no_proxy="10.239.0.0/16,172.168.0.0/16,127.0.0.0/8,localhost,10.0.0.0/8,192.168.0.0/16,192.168.14.0/24,.intel.com,100.64.0.0/10,172.16.0.0/12,10.112.0.0/16"

ARG ARCH

# kernel deps
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
	    bc \
	    bison \
	    build-essential \
	    ca-certificates \
	    curl \
	    debhelper \
	    flex \
	    git \
	    iptables \
	    kmod \
	    libelf-dev \
	    libssl-dev \
	    gettext \
	    rsync \
	    cpio \
	    patch && \
    if [ "${ARCH}" != "$(uname -m)" ]; then apt-get install --no-install-recommends -y gcc-"${ARCH}"-linux-gnu binutils-"${ARCH}"-linux-gnu; fi && \
    apt-get clean && apt-get autoclean && rm -rf /var/lib/apt/lists/*
