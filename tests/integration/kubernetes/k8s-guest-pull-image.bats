#!/usr/bin/env bats
# Copyright (c) 2023 Intel Corporation
#
# SPDX-License-Identifier: Apache-2.0
#

load "${BATS_TEST_DIRNAME}/lib.sh"
load "${BATS_TEST_DIRNAME}/../../common.bash"
load "${BATS_TEST_DIRNAME}/tests_common.sh"

setup() {
    [[ "${PULL_TYPE}" =~  "guest-pull" ]] ||  skip "Test only working for pulling image inside the guest"
    setup_common
}

@test "Test can pull an unencrypted image inside the guest" {
    pod_config="$(new_pod_config quay.io/prometheus/busybox:latest "kata-${KATA_HYPERVISOR}")"

    kubectl create -f "${pod_config}"

    # Get pod specification
    kubectl wait --for=condition=Ready --timeout=$timeout pod "test-e2e"

    echo "Check the image was not pulled in the host"
    local pod_id=$(kubectl get pods -o jsonpath='{.items..metadata.name}')
    sandbox_id=$(ps -ef | grep containerd-shim-kata-v2 | egrep -o "\s\-id [a-z0-9]+" | awk '{print $2}')
    rootfs=($(find /run/kata-containers/shared/sandboxes/${sandbox_id}/shared \
    	-name rootfs))

    [ ${#rootfs[@]} -le 1 ]
}

teardown() {
    [[ "${PULL_TYPE}" =~  "guest-pull" ]] ||  skip "Test only working for pulling image inside the guest"

    kubectl describe -f "${pod_config}" || true
    kubectl delete -f "${pod_config}" || true
}