//Copyright (c) 2019-2022 Alibaba Cloud
//Copyright (c) 2019-2022 Ant Group
//Copyright (c) 2023 Nubificus Ltd
//
//SPDX-License-Identifier: Apache-2.0

use super::FcInner;
use crate::firecracker::{
    inner_hypervisor::{FC_AGENT_SOCKET_NAME, ROOT},
    sl,
};
use crate::VmmState;
use crate::{device::DeviceType, HybridVsockConfig, VsockConfig};
use anyhow::{anyhow, Context, Result};
use serde_json::json;

use super::block_source::{cleanup_vmdk, prepare_vmdk, VMDK_DESCRIPTOR, VMDK_PLACEHOLDER};
use super::fc_api::DISK_POOL_SIZE;
use crate::device::util::get_virt_drive_name;
use crate::virtio_blk_modern::{BlockConfigModern, BlockSourceFormat};
use std::path::{Path, PathBuf};

impl FcInner {
    pub(crate) async fn add_device(&mut self, device: DeviceType) -> Result<()> {
        if self.state == VmmState::NotReady {
            info!(sl(), "VMM not ready, queueing device {}", device);

            self.pending_devices.insert(0, device);

            return Ok(());
        }

        debug!(sl(), "Add Device {} ", &device);

        match device {
            DeviceType::BlockModern(block_mod) => {
                let block = block_mod.lock().await.clone();
                let config = &block.config;
                if let BlockSourceFormat::Vmdk(vmdk) = &config.source {
                    let guest_path = self.vmdk_guest_path(config)?;
                    let directory = self.vmdk_directory(config.index);
                    prepare_vmdk(vmdk, &directory)?;
                    let path = if self.jailed {
                        format!("/vmdk-{}/{VMDK_DESCRIPTOR}", config.index)
                    } else {
                        directory.join(VMDK_DESCRIPTOR).display().to_string()
                    };
                    if let Err(error) = self
                        .patch_drive(&format!("vmdk{}", config.index), &path)
                        .await
                    {
                        cleanup_vmdk(&directory)?;
                        return Err(error).context("patch VMDK drive");
                    }
                    block_mod.lock().await.config.virt_path = guest_path;
                    Ok(())
                } else {
                    self.hotplug_block_device(config.path_on_host.as_str(), config.index)
                        .await
                        .context("add block device")
                }
            }
            DeviceType::Network(network) => self
                .add_net_device(&network.config, network.device_id)
                .await
                .context("add net device"),
            DeviceType::HybridVsock(hvsock) => {
                self.add_hvsock(&hvsock.config).await.context("add vsock")
            }
            DeviceType::Vsock(vsock) => self.add_vsock(&vsock.config).await.context("add vsock"),
            _ => Err(anyhow!("unhandled device: {:?}", device)),
        }
    }

    // Since Firecracker doesn't support sharefs, we patch block devices on pre-start inserted
    // dummy drives
    pub(crate) async fn hotplug_block_device(&mut self, path: &str, id: u64) -> Result<()> {
        if id >= DISK_POOL_SIZE {
            return Err(anyhow!(
                "Firecracker block device index exceeds the drive pool"
            ));
        }
        if id > 0 {
            self.patch_container_rootfs(&id.to_string(), path).await?;
        }
        Ok(())
    }

    pub(crate) async fn remove_device(&mut self, device: DeviceType) -> Result<()> {
        info!(sl(), "Remove Device {} ", device);
        if let DeviceType::BlockModern(block) = device {
            let config = block.lock().await.config.clone();
            if matches!(config.source, BlockSourceFormat::Vmdk(_)) {
                let placeholder = Path::new(&self.vm_path)
                    .join(ROOT)
                    .join(VMDK_PLACEHOLDER)
                    .join(VMDK_DESCRIPTOR);
                let path = if self.jailed {
                    format!("/{VMDK_PLACEHOLDER}/{VMDK_DESCRIPTOR}")
                } else {
                    placeholder.display().to_string()
                };
                self.patch_drive(&format!("vmdk{}", config.index), &path)
                    .await?;
                cleanup_vmdk(&self.vmdk_directory(config.index))?;
            } else if config.index > 0 {
                let name = format!("drive{}", config.index);
                if self.jailed {
                    self.umount_jail_resource(&name)?;
                }
                let placeholder = if self.jailed {
                    format!("/{name}")
                } else {
                    Path::new(&self.vm_path)
                        .join(ROOT)
                        .join(&name)
                        .display()
                        .to_string()
                };
                self.patch_drive(&name, &placeholder).await?;
            }
        }
        Ok(())
    }

    fn vmdk_directory(&self, index: u64) -> PathBuf {
        Path::new(&self.vm_path)
            .join(ROOT)
            .join(format!("vmdk-{index}"))
    }

    fn vmdk_guest_path(&self, config: &BlockConfigModern) -> Result<String> {
        if !self.config.blockdev_info.enable_vmdk {
            return Err(anyhow!("Firecracker VMDK requires enable_vmdk = true"));
        }
        if !config.is_readonly {
            return Err(anyhow!(
                "Firecracker VMDK requires a read-only block device"
            ));
        }
        if config.index == 0 || config.index >= DISK_POOL_SIZE {
            return Err(anyhow!("Firecracker VMDK index exceeds the drive pool"));
        }
        let name = get_virt_drive_name((DISK_POOL_SIZE - 1 + config.index) as i32)?;
        Ok(format!("/dev/{name}"))
    }

    pub(crate) async fn update_device(&mut self, device: DeviceType) -> Result<()> {
        info!(sl(), "update device {:?}", &device);
        Ok(())
    }

    pub(crate) async fn add_hvsock(&mut self, config: &HybridVsockConfig) -> Result<()> {
        let rel_uds_path = match self.jailed {
            false => [self.vm_path.as_str(), FC_AGENT_SOCKET_NAME].join("/"),
            true => FC_AGENT_SOCKET_NAME.to_string(),
        };
        let body_vsock: String = json!({
            "vsock_id": String::from(ROOT),
            "guest_cid": config.guest_cid,
            "uds_path": rel_uds_path,
        })
        .to_string();

        info!(sl(), "HybridVsock configure: {:?}", &body_vsock);

        self.request_with_retry(hyper::Method::PUT, "/vsock", body_vsock)
            .await?;
        Ok(())
    }

    pub(crate) async fn add_vsock(&mut self, config: &VsockConfig) -> Result<()> {
        let rel_uds_path = match self.jailed {
            false => [self.vm_path.as_str(), FC_AGENT_SOCKET_NAME].join("/"),
            true => FC_AGENT_SOCKET_NAME.to_string(),
        };
        let body_vsock: String = json!({
            "vsock_id": String::from(ROOT),
            "guest_cid": config.guest_cid,
            "uds_path": rel_uds_path,
        })
        .to_string();

        info!(sl(), "HybridVsock configure: {:?}", &body_vsock);

        self.request_with_retry(hyper::Method::PUT, "/vsock", body_vsock)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn vmdk_slots_require_opt_in_readonly_and_valid_index() {
        let (sender, _) = mpsc::channel(1);
        let mut fc = FcInner::new(sender);
        let mut config = BlockConfigModern {
            index: 1,
            is_readonly: true,
            ..Default::default()
        };
        assert!(fc.vmdk_guest_path(&config).is_err());
        fc.config.blockdev_info.enable_vmdk = true;
        assert_eq!(fc.vmdk_guest_path(&config).unwrap(), "/dev/vdg");
        config.index = DISK_POOL_SIZE - 1;
        assert_eq!(fc.vmdk_guest_path(&config).unwrap(), "/dev/vdk");
        for index in [0, DISK_POOL_SIZE, u64::MAX] {
            config.index = index;
            assert!(fc.vmdk_guest_path(&config).is_err());
        }
        config.index = 1;
        config.is_readonly = false;
        assert!(fc.vmdk_guest_path(&config).is_err());
    }
}
