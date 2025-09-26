#!/bin/bash
# Copyright (c) 2018 Intel Corporation, 2021 IBM Corp.
#
# SPDX-License-Identifier: Apache-2.0

# This is a configuration file add extra variables to
# be used by build_rootfs() from rootfs_lib.sh the variables will be
# loaded just before call the function. For more information see the
# rootfs-builder/README.md file.

OS_VERSION=${OS_VERSION:-"4.4"}
OS_NAME=tencentos

PACKAGES="chrony iptables"
if [ "$AGENT_INIT" = no ]; then
    PACKAGES+=" systemd"
fi
if [ "$SECCOMP" = yes ]; then
    PACKAGES+=" libseccomp"
fi
if [ "$SELINUX" = yes ]; then
    PACKAGES+=" container-selinux"
fi
