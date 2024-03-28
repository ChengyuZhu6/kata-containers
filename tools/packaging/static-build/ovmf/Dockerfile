# Copyright (c) 2022 IBM
#
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:20.04
ENV DEBIAN_FRONTEND=noninteractive
ENV https_proxy=http://proxy.cd.intel.com:912
ENV http_proxy=http://proxy.cd.intel.com:912
ENV no_proxy="10.239.0.0/16,172.168.0.0/16,127.0.0.0/8,localhost,10.0.0.0/8,192.168.0.0/16,192.168.14.0/24,.intel.com,100.64.0.0/10,172.16.0.0/12,10.112.0.0/16"

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        g++ \
        gcc \
        git \
        iasl  \
        make \
        python \
        python3 \
        python3-distutils \
        uuid-dev && \
    apt-get clean && rm -rf /var/lib/lists/ && \
    cd /tmp && curl -fsLO https://www.nasm.us/pub/nasm/releasebuilds/2.15.05/nasm-2.15.05.tar.gz && \
    tar xf nasm-2.15.05.tar.gz && \
    cd nasm-2.15.05 && \
    ./configure && \
    make -j"$(nproc)" && \
    make install && \
    cd /tmp && rm -r nasm-2.15.05*
