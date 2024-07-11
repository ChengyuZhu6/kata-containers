#!/usr/bin/env bats
# Copyright (c) 2023 Intel Corporation
# Copyright (c) 2023 IBM Corporation
#
# SPDX-License-Identifier: Apache-2.0
#

load "${BATS_TEST_DIRNAME}/lib.sh"
load "${BATS_TEST_DIRNAME}/confidential_common.sh"

setup() {
    if ! is_confidential_runtime_class; then
        skip "Test not supported for ${KATA_HYPERVISOR}."
    fi

    [ "${SNAPSHOTTER:-}" = "nydus" ] || skip "None snapshotter was found but this test requires one"

    setup_common
    get_pod_config_dir
    unencrypted_image="quay.io/prometheus/busybox:latest"
    large_image="ghcr.io/confidential-containers/test-container:big-size"
}

@test "Test we can pull an unencrypted image outside the guest with runc and then inside the guest successfully" {
    # 1. Create one runc pod with the $unencrypted_image image
    # We want to have one runc pod, so we pass a fake runtimeclass "runc" and then delete the runtimeClassName,
    # because the runtimeclass is not optional in new_pod_config function.
    runc_pod_config="$(new_pod_config "$unencrypted_image" "runc")"
    sed -i '/runtimeClassName:/d' $runc_pod_config
    set_node "$runc_pod_config" "$node"
    set_container_command "$runc_pod_config" "0" "sleep" "30"

    # For debug sake
    echo "Pod $runc_pod_config file:"
    cat $runc_pod_config

    add_allow_all_policy_to_yaml "$runc_pod_config"
    k8s_create_pod "$runc_pod_config"

    echo "Runc pod test-e2e is running"
    kubectl delete -f "$runc_pod_config"

    # 2. Create one kata pod with the $unencrypted_image image and nydus annotation
    kata_pod_with_nydus_config="$(new_pod_config "$unencrypted_image" "kata-${KATA_HYPERVISOR}")"
    set_node "$kata_pod_with_nydus_config" "$node"
    set_container_command "$kata_pod_with_nydus_config" "0" "sleep" "30"

    # For debug sake
    echo "Pod $kata_pod_with_nydus_config file:"
    cat $kata_pod_with_nydus_config

    add_allow_all_policy_to_yaml "$kata_pod_with_nydus_config"
    k8s_create_pod "$kata_pod_with_nydus_config"
    echo "Kata pod test-e2e with nydus annotation is running"

    echo "Checking the image was pulled in the guest"
    sandbox_id=$(get_node_kata_sandbox_id $node)
    echo "sandbox_id is: $sandbox_id"
    # No rootfs can be found on host with guest pull
    assert_rootfs_count "$node" "$sandbox_id" "0"
}

@test "Test we can pull a large image inside the guest using trusted ephemeral storage" {
    
    # The image pulled in the guest will be downloaded and unpacked in the `/run/kata-containers/image` directory. 
    # However, by default, systemd allocates 10% of the available physical RAM to the `/run` directory using a `tmpfs` filesystem. 
    # It means that if we run a kata container with the default configuration (where the default memory assigned for a VM is 2048 MiB), 
    # `/run` would be allocated around 200 MiB. Consequently, we can only pull images up to 200 MiB in the guest. 
    # However, the unpacked size of image "ghcr.io/confidential-containers/test-container:big-size" is 965MB. 
    # It will fail to run the pod with pulling the image in the memory in the guest by default. 

    pod_config="$(new_pod_config "$large_image" "kata-${KATA_HYPERVISOR}")"
    set_node "$pod_config" "$node"
    set_container_command "$pod_config" "0" "sleep" "30"

    # For debug sake
    echo "Pod $pod_config file:"
    cat $pod_config

    # The pod should be failed because the default timeout of CreateContainerRequest is 60s
    assert_pod_fail "$pod_config"
    assert_logs_contain "$node" kata "$node_start_time" \
		'context deadline exceeded'

    kubectl delete -f $pod_config

    pod_config="${pod_config_dir}/pod-guest-pull-in-trusted-storage.yaml"
    storage_config="${pod_config_dir}/confidential/trusted-storage.yaml"
    local_device=$(create_loop_device "/tmp/trusted-storage.img")
    sed -i "s/runtimeClassName: .*/runtimeClassName: kata-${KATA_HYPERVISOR}/" $pod_config
    sed -i "s/NODE_NAME/$node/g" $pod_config
    sed -i "s/NODE_NAME/$node/g" $storage_config
    sed -i "s|LOCAL_DEVICE|$local_device|g" $storage_config

    # For debug sake
    echo "Trusted storage $storage_config file:"
    cat $storage_config
    echo "Pod $pod_config file:"
    cat $pod_config

    # Create persistent volume and persistent volume claim
    kubectl create -f $storage_config

    # Set CreateContainerRequest timeout in the annotation to pull large image in guest
    create_container_timeout=120
    set_metadata_annotation "$pod_config" \
        "io.katacontainers.config.runtime.create_container_timeout" \
        "${create_container_timeout}"

    # For debug sake
    echo "Pod $pod_config file:"
    cat $pod_config

    add_allow_all_policy_to_yaml "$pod_config"
    k8s_create_pod "$pod_config"
}

teardown() {
    if ! is_confidential_runtime_class; then
        skip "Test not supported for ${KATA_HYPERVISOR}."
    fi

    [ "${SNAPSHOTTER:-}" = "nydus" ] || skip "None snapshotter was found but this test requires one"

    kubectl describe pods
    k8s_delete_all_pods_if_any_exists || true
    kubectl delete -f "${pod_config_dir}/confidential/trusted-storage.yaml" || true
}
