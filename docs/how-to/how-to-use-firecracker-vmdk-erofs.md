# Use Firecracker with VMDK and the EROFS snapshotter

The Rust runtime can present containerd EROFS image layers to Firecracker as
one read-only VMDK disk. The writable container layer remains a separate ext4
disk. The guest agent mounts the EROFS layers and combines them with the
writable layer using overlayfs. No shared filesystem or devmapper thin pool is
required on the host.

## Requirements and limits

- Linux with KVM and a supported Firecracker CPU architecture.
- A Rust shim **and guest agent built from this change**. The Go runtime is
  not supported by this feature.
- A Firecracker build containing [native VMDK support, PR #5741](https://github.com/firecracker-microvm/firecracker/pull/5741).
  The first merged revision is `f660f6ba6fbed73ebc5b1f6e3bcd2c403cf04769`.
  The default Firecracker version in `versions.yaml` may predate this feature;
  enabling the option does not upgrade the VMM automatically.
- Containerd 2.4 with its native EROFS snapshotter and differ, erofs-utils
  >= 1.8.2, `mkfs.ext4`, `ctr`, `crictl`, `curl`, and Python 3.
- Guest kernel support for virtio-mmio, ext4, EROFS, overlayfs and GPT
  (`CONFIG_EFI_PARTITION=y`). The packaged Kata kernel can be reused when
  these features are present.

!!! note "Containerd 2.4 does not generate fsmerge metadata"
    Containerd [removed automatic fsmerge generation](https://github.com/containerd/containerd/commit/b7f8c3510d9135ad56277140f9e5aa86eb0e9a66).
    Do not configure `fsmerge` or `max_unmerged_layers` with 2.4. Its native
    multi-layer mounts use Kata's GPT+VMDK path: each independent EROFS layer
    becomes a partition of one disk. Existing fsmerge mounts with `device=`
    options instead use a contiguous VMDK layout without GPT.

Firecracker accepts VMDK only with `image_format = "Vmdk"`,
`is_read_only = true`, and `io_engine = "Sync"`. Its PATCH API changes the
backing path but cannot change these properties. Setting `enable_vmdk = true`
therefore adds five read-only VMDK slots after the existing five writable Raw
slots. The option defaults to false to preserve older VMM compatibility.

The existing limit of five logical container block sources per VM still
applies. Each explicit ext4 upper and each EROFS disk consumes one source;
the sandbox image also consumes sources. A sandbox with a single-layer pause
image and one multi-layer workload uses four sources. Image layer count does
not consume additional slots when the layers are combined into VMDK.

The VMDK slots use guest disk names `/dev/vdg` through `/dev/vdk`; the runtime
translates each logical index to the correct disk. Each slot also consumes a
virtio-mmio interrupt. Additional NICs and devices must fit Firecracker's
architecture-specific interrupt capacity.

VMDK backing files must be regular files. Kata validates their lengths and
bind-mounts read-only aliases beside the descriptor because Firecracker
rejects implicit extents outside that directory. Container teardown returns
the slot to its placeholder and removes these mounts. The image layer files
remain owned by containerd. This guide tests ordinary EROFS layers without
dm-verity; dm-verity requires a guest agent built with that feature.

## Build and install

Use the repository's [local build guide](how-to-build-and-deploy-local-artifacts.md)
for the Rust shim, agent and rootfs image. Both the shim and image must include
this change. Direct builds used during development are:

```bash
make -C src/runtime-rs static-checks-build PREFIX=/opt/kata
make -C src/agent static-checks-build
cargo build --locked --release --target x86_64-unknown-linux-musl \
  -p runtime-rs --bin containerd-shim-kata-v2
cargo build --locked --release --target x86_64-unknown-linux-musl -p kata-agent
```

Use the target appropriate for your architecture. Install the agent into a
new guest image using the image builder; replacing only the host shim is
insufficient. A static musl agent without the `devicemapper` feature is enough
for the ordinary EROFS mode here.

Build Firecracker and its matching jailer from a checkout containing the
revision above, following the [upstream build instructions](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md).
Install them under a separate directory so existing workloads keep their VMM.

Start with the generated `src/runtime-rs/config/configuration-rs-fc.toml`.
Set the following fields to the actual installed paths:

```toml title="/work/kata-fc-vmdk/kata-fc.toml (relevant fields)"
[hypervisor.firecracker]
path = "/work/kata-fc-vmdk/bin/firecracker"
jailer_path = "/work/kata-fc-vmdk/bin/jailer"
valid_hypervisor_paths = ["/work/kata-fc-vmdk/bin/firecracker"]
valid_jailer_paths = ["/work/kata-fc-vmdk/bin/jailer"]
kernel = "/opt/kata/share/kata-containers/vmlinux.container"
image = "/work/kata-fc-vmdk/kata.img"
rootfs_type = "ext4"
block_device_driver = "virtio-blk-mmio"
enable_vmdk = true

[agent.kata]
dial_timeout_ms = 10
reconnect_timeout_ms = 45000
```

These are overrides within the generated configuration, not a replacement
for its other sections. Use containerd runtime `ConfigPath` below to select
this file. An arbitrary path in `KATA_CONF_FILE` is rejected by the Rust shim's
configuration trust checks.

## Configure an isolated containerd 2.4 instance

Install the official containerd release archive and verify its accompanying
checksum. Keep the binaries, root, state and socket separate from the host's
running daemon. The following configuration uses version 3, which containerd
2.4 migrates on load. Version 4 moves gRPC configuration into plugin sections;
copying the old `[grpc]` section into a version 4 file does not select its
socket.

```toml title="/work/kata-fc-vmdk/containerd.toml"
version = 3
root = "/work/kata-fc-vmdk/root"
state = "/work/kata-fc-vmdk/state"
imports = []
required_plugins = ["io.containerd.snapshotter.v1.erofs", "io.containerd.differ.v1.erofs"]

[grpc]
address = "/work/kata-fc-vmdk/containerd.sock"

[plugins."io.containerd.cri.v1.images"]
snapshotter = "erofs"
use_local_image_pull = true

[plugins."io.containerd.cri.v1.runtime".containerd.runtimes.kata-fc-vmdk]
runtime_type = "io.containerd.kata.v2"
runtime_path = "/work/kata-fc-vmdk/bin/containerd-shim-kata-v2"
snapshotter = "erofs"
privileged_without_host_devices = true

[plugins."io.containerd.cri.v1.runtime".containerd.runtimes.kata-fc-vmdk.options]
ConfigPath = "/work/kata-fc-vmdk/kata-fc.toml"

[plugins."io.containerd.differ.v1.erofs"]
mkfs_options = ["-T0", "--mkfs-time", "--sort=none"]
enable_tar_index = false

[plugins."io.containerd.service.v1.diff-service"]
default = ["erofs", "walking"]

[plugins."io.containerd.snapshotter.v1.erofs"]
default_size = "256MiB"
```

The small writable disk size is for the test workload; increase it for your
application. Set containerd's sandbox image to a reachable pause image, or
import that image into namespace `k8s.io` before testing. Use host networking
for the smoke test, or configure CNI separately for networked pods.

```bash
sudo modprobe erofs
sudo env PATH="/work/kata-fc-vmdk/bin:/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin" \
  /work/kata-fc-vmdk/bin/containerd \
  --config /work/kata-fc-vmdk/containerd.toml \
  --address /work/kata-fc-vmdk/containerd.sock \
  --root /work/kata-fc-vmdk/root --state /work/kata-fc-vmdk/state
```

Run this in a separate terminal or supervised service. Check that both EROFS
plugins report `ok`:

```bash
sudo /work/kata-fc-vmdk/bin/ctr \
  --address /work/kata-fc-vmdk/containerd.sock plugins ls
```

## Run the end-to-end test

Save a local Docker image with `/bin/sh`, `sleep`, `cat`, `sha256sum`, and
`uname`. BusyBox is sufficient. Exporting a cached image avoids registry
access during the test:

```bash
docker image save busybox:latest -o /work/kata-fc-vmdk/base.tar
sudo env CONTAINERD_ADDRESS=/work/kata-fc-vmdk/containerd.sock \
  CTR=/work/kata-fc-vmdk/bin/ctr \
  BASE_IMAGE_ARCHIVE=/work/kata-fc-vmdk/base.tar \
  bash tests/integration/cri-containerd/fc-vmdk.sh
```

The test appends two layers with an overwrite, whiteout and a 2 MiB payload,
imports them with the EROFS snapshotter, starts a CRI sandbox, and creates and
deletes a workload three times in that sandbox. It checks layer order,
whiteouts, payload integrity, writable overlay operations, clean writable
state on recreation, native VMDK API configuration, and extent cleanup after
each container removal. It cleans its containers, sandbox, generated image
and temporary archive on both success and failure.

The test requires local root access to Firecracker's API socket and generated
descriptors. It supports the default jailed layout and the non-jailed layout.
It uses the `kata-fc-vmdk` runtime handler unless `RUNTIME_HANDLER` overrides it.

## Diagnose and clean up

### Validated configuration

On 2026-11-09, the CRI test passed three consecutive workload cycles with
jailer enabled and another three with jailer disabled on an x86_64 KVM host.
The test used containerd `v2.4.0`, Firecracker revision
`f660f6ba6fbed73ebc5b1f6e3bcd2c403cf04769`, erofs-utils
`1.9.4-gd1369d77`, host Linux `6.19.4-061904-generic`, and guest Linux
`6.12.47`. A single-layer EROFS container also passed using Firecracker
`v1.12.1` with `enable_vmdk = false`.

### Logs and teardown

Enable `enable_debug = true` in the generated hypervisor, agent and runtime
sections to collect logs. Inspect the dedicated containerd log and the shim's
journal records for the sandbox ID. The Firecracker socket is normally
`/run/kata/firecracker/<sandbox-id>/root/run/fc.sock`; `GET /vm/config` should
show an active read-only `Vmdk` drive using the `Sync` engine. Other VMDK slots
still point to `vmdk-placeholder/disk.vmdk`.

- Failure while reserving VMDK drives usually means the selected Firecracker
  binary lacks the native `image_format` API.
- A missing GPT partition requires a guest kernel with GPT support and the
  updated agent, which rereads the partition table after a backing-file swap.
- A missing `mkfs.ext4` error requires adding the host's `sbin` directories to
  the containerd daemon's `PATH`.
- A drive-pool error means the sandbox exhausted the five logical sources.

Stop and remove only workloads belonging to the test instance before stopping
that daemon. Verify that its tasks and CRI sandboxes are gone, and no mounts
remain below its root/state directories or the sandbox's `/run/kata` directory.
Then remove the dedicated containerd root/state, test archives, copied guest
image and build target directories to reclaim storage. Retain configuration,
logs and binary checksums if needed for reproducing the result. Never remove
an active daemon's data directory or use a host-wide prune for this test.
