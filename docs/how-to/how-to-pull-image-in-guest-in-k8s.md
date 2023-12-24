# How to Pull Image in the Guest in kubernetes
This document provides an overview on how to Pull Image in the guest with Confidential Containers. 

## Introduction
Confidential Containers (CoCo) protects data in Trusted Execution Environment (TEE) by pulling images in the guest with forked containerd.
To optimize resource usage and avoid the need for forking containerd while enabling image pulls within the guest, we employ the [Nydus Snapshotter](https://github.com/containerd/nydus-snapshotter) as a proxy plugin. 
This external plugin for containerd offers the modes for pulling images in the guest. 
See [Image management proposal](https://github.com/confidential-containers/confidential-containers/issues/133) for detailed design.

## Prerequisites
- Confidential Containers
- Nydus Snapshotter

## Install Confidential Containers

Follow [How to build, run and test Kata CCv0
](https://github.com/kata-containers/kata-containers/blob/CCv0/docs/how-to/how-to-build-and-test-ccv0.md) or [Operator Installation](https://github.com/confidential-containers/operator/blob/main/docs/INSTALL.md) to install and configure Confidential Containers.

## Install and Configure Nydus Snapshotter
### Install Nydus Snapshotter
Because upstream currently only releases the nydus-snapshotter for x86_64 platform. Therefore, for non-x86_64 platforms, the binary can only be obtained through source code compilation. 
- install from tarball (for x86_64):
```bash
export nydus_snapshotter_version="v0.13.4"
export nydus_snapshotter_repo="https://github.com/containerd/nydus-snapshotter"
export nydus_snapshotter_tarball_url="${nydus_snapshotter_repo}/releases/download/${nydus_version}/nydus-snapshotter-${nydus_snapshotter_version}-x86_64.tgz
tmp_dir=$(mktemp -d -t install-nydus-snapshotter-tmp.XXXXXXXXXX)
sudo curl -Ls "${nydus_snapshotter_tarball_url}" | sudo tar xfz - -C ${tmp_dir} --strip-components=1
sudo install -D -m 755 "${tmp_dir}/containerd-nydus-grpc" "/usr/local/bin/"
sudo install -D -m 755 "${tmp_dir}/nydus-overlayfs" "/usr/local/bin/"
"
```

- install from source codes (for all platforms):
```bash
export ARCH="$(uname -m)"
export GOARCH=$(case "$ARCH" in
		aarch64) echo "arm64";;
		ppc64le) echo "ppc64le";;
		x86_64) echo "amd64";;
		s390x) echo "s390x";;
	esac)
export nydus_snapshotter_version="v0.13.4"
export nydus_snapshotter_repo="https://github.com/containerd/nydus-snapshotter"
export nydus_snapshotter_repo_dir="${GOPATH}/src/${nydus_snapshotter_repo}"
sudo mkdir -p "${nydus_snapshotter_repo_dir}"
sudo git clone ${nydus_snapshotter_repo_git} "${nydus_snapshotter_repo_dir}" || true
pushd "${nydus_snapshotter_repo_dir}"
sudo git checkout "${nydus_snapshotter_version}"
sudo -E PATH=$PATH:$GOPATH/bin make
sudo install -D -m 755 "bin/containerd-nydus-grpc" "/usr/local/bin/containerd-nydus-grpc"
sudo install -D -m 755 "bin/nydus-overlayfs" "/usr/local/bin/nydus-overlayfs"
popd
```

### Install Nydus Snapshotter config files
```bash
sudo curl -L https://raw.githubusercontent.com/containerd/nydus-snapshotter/main/misc/snapshotter/config-coco-guest-pulling.toml -o "/usr/local/share/config-coco-guest-pulling.toml"
```

#### Containerd as Container Runtime

CoCo v0.8.0 uses vanilla containerd, which requires specific modifications depending on different containerd version:

- Containerd v1.7.0 and above:
    To use the snapshotter specified under `kata-qemu-tdx`, we need to add the following annotation in metadata 
    to each pod yaml: `io.containerd.cri.runtime-handler: kata-qemu-tdx`. This is because CoCo has enabled 
    the [`Runtime Specific Snapshotter`](https://github.com/containerd/containerd/blob/1da783894b99e1459624b0b2a60466eb0f35837c/RELEASES.md?plain=1#L480C1-L481C1) feature, which is still experimental for containerd v1.7.0. By adding the annotation, we can ensure that the feature works as expected.

    exp:
    ```yaml
    apiVersion: v1
    kind: Pod
    metadata:
        name: busybox-cc
        annotations:
        io.containerd.cri.runtime-handler: kata-qemu-tdx
    spec:
        runtimeClassName: kata-qemu-tdx
        containers:
        - name: busybox
        image: quay.io/prometheus/busybox:latest
        imagePullPolicy: Always
    ```
- Containerd v1.7.0 below:
     CoCo has enabled the [`Runtime Specific Snapshotter`](https://github.com/containerd/containerd/blob/1da783894b99e1459624b0b2a60466eb0f35837c/RELEASES.md?plain=1#L480C1-L481C1) feature, but it only works for containerd v1.7.0 and above. So for Containerd v1.7.0 below, we need to set the global snapshotter to `nydus` in containerd config(default path: /etc/containerd/config.toml). 

    exp: 

    ```toml
     [plugins."io.containerd.grpc.v1.cri".containerd]
      default_runtime_name = "runc"
      disable_snapshot_annotations = false
      discard_unpacked_layers = false
      ignore_blockio_not_enabled_errors = false
      ignore_rdt_not_enabled_errors = false
      no_pivot = false
      snapshotter = "nydus"
    ```

## Run
### Run Nydus Snapshotter
- For image sharing on the host:
```bash
/usr/local/bin/containerd-nydus-grpc --config /usr/local/share/config-coco-host-sharing.toml >/dev/stdout 2>&1 &
```
- For image pulling in the guest:
```
/usr/local/bin/containerd-nydus-grpc --config /usr/local/share/config-coco-guest-pulling.toml >/dev/stdout 2>&1 &
```
### Run pod
- Create an pod configuration
```bash
$ cat > pod.yaml <<EOF
apiVersion: v1
kind: Pod
metadata:
  name: busybox
  namespace: default
spec:
  runtimeClassName: kata
  containers:
  - name: busybox
    image: quay.io/library/busybox:latest
```
- Create the pod
  ```bash
  $ sudo -E kubectl apply -f pod.yaml
  ```
- Check pod is running
  ```bash
  $ sudo -E kubectl get pods
  ```
