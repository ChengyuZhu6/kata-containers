#!/usr/bin/env bats
#
# Copyright (c) 2018 Intel Corporation
#
# SPDX-License-Identifier: Apache-2.0
#

load "${BATS_TEST_DIRNAME}/../../common.bash"
load "${BATS_TEST_DIRNAME}/tests_common.sh"

setup() {
	nginx_version="${docker_images_nginx_version}"
	nginx_image="nginx:$nginx_version"

	pod_name="handlers"

	get_pod_config_dir
	yaml_file="${pod_config_dir}/test-lifecycle-events.yaml"

	# Create yaml
	sed -e "s/\${nginx_version}/${nginx_image}/" \
		"${pod_config_dir}/lifecycle-events.yaml" > "${yaml_file}"

	# Add policy to yaml
	policy_settings_dir="$(create_tmp_policy_settings_dir "${pod_config_dir}")"
	
	display_message="cat /usr/share/message"
	exec_command=(sh -c "${display_message}")
	add_exec_to_policy_settings "${policy_settings_dir}" "${exec_command[@]}"
	
	add_requests_to_policy_settings "${policy_settings_dir}" "ReadStreamRequest"
	auto_generate_policy "${policy_settings_dir}" "${yaml_file}"
}

@test "Running with postStart and preStop handlers" {
	# Create the pod with postStart and preStop handlers
	kubectl create -f "${yaml_file}"

	# Check pod creation
	kubectl wait --for=condition=Ready --timeout=$timeout pod $pod_name

	# Check postStart message
	check_postStart=$(kubectl exec $pod_name -- "${exec_command[@]}")
	echo "check_postStart=$check_postStart"
	echo "$check_postStart" | grep "Hello from the postStart handler"
}

teardown(){
	# Debugging information
	kubectl describe "pod/$pod_name"

	rm -f "${yaml_file}"
	kubectl delete pod "$pod_name"

	delete_tmp_policy_settings_dir "${policy_settings_dir}"
}
