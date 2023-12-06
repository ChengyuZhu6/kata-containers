// Copyright (c) 2021 Alibaba Cloud
// Copyright (c) 2021, 2023 IBM Corporation
// Copyright (c) 2022 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use image_rs::image::ImageClient;
use tokio::sync::Mutex;

use crate::rpc::CONTAINER_BASE;
use crate::AGENT_CONFIG;

const KATA_IMAGE_WORK_DIR: &str = "/run/image/";

#[rustfmt::skip]
lazy_static! {
    pub static ref IMAGE_SERVICE: Mutex<Option<ImageService>> = Mutex::new(None);
}

// Convenience function to obtain the scope logger.
fn sl() -> slog::Logger {
    slog_scope::logger().new(o!("subsystem" => "cgroups"))
}

#[derive(Clone)]
pub struct ImageService {
    image_client: Arc<Mutex<ImageClient>>,
    images: Arc<Mutex<HashMap<String, String>>>,
}

impl ImageService {
    pub fn new() -> Self {
        env::set_var("CC_IMAGE_WORK_DIR", KATA_IMAGE_WORK_DIR);

        Self {
            image_client: Arc::new(Mutex::new(ImageClient::default())),
            images: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the singleton instance of image service.
    pub async fn singleton() -> Result<ImageService> {
        IMAGE_SERVICE
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow!("image service is uninitialized"))
    }

    async fn add_image(&self, image: String, cid: String) {
        self.images.lock().await.insert(image, cid);
    }

    /// Set proxy environment from AGENT_CONFIG
    fn set_proxy_env_vars() {
        let https_proxy = &AGENT_CONFIG.https_proxy;
        if !https_proxy.is_empty() {
            env::set_var("HTTPS_PROXY", https_proxy);
        }
        let no_proxy = &AGENT_CONFIG.no_proxy;
        if !no_proxy.is_empty() {
            env::set_var("NO_PROXY", no_proxy);
        }
    }

    /// pull_image is used for call image-rs to pull image in the guest.
    /// # Parameters
    /// - `image`: Image name (exp: quay.io/prometheus/busybox:latest)
    /// - `cid`: Container id
    /// - `image_metadata`: Annotations about the image (exp: "containerd.io/snapshot/cri.layer-digest": "sha256:24fb2886d6f6c5d16481dd7608b47e78a8e92a13d6e64d87d57cb16d5f766d63")
    /// # Returns
    /// - The image rootfs bundle path. (exp. /run/kata-containers/cb0b47276ea66ee9f44cc53afa94d7980b57a52c3f306f68cb034e58d9fbd3c6/images/rootfs)
    pub async fn pull_image(
        &self,
        image: &str,
        cid: &str,
        image_metadata: &HashMap<String, String>,
    ) -> Result<String> {
        info!(sl(), "image metadata: {:?}", image_metadata);
        Self::set_proxy_env_vars();

        let bundle_path = Path::new(CONTAINER_BASE).join(cid).join("images");
        fs::create_dir_all(&bundle_path)?;
        info!(sl(), "pull image {:?}, bundle path {:?}", cid, bundle_path);

        let res = self
            .image_client
            .lock()
            .await
            .pull_image(image, &bundle_path, &None, &None)
            .await;
        match res {
            Ok(image) => {
                info!(
                    sl(),
                    "pull and unpack image {:?}, cid: {:?}, with image-rs succeed. ", image, cid
                );
            }
            Err(e) => {
                error!(
                    sl(),
                    "pull and unpack image {:?}, cid: {:?}, with image-rs failed with {:?}. ",
                    image,
                    cid,
                    e.to_string()
                );
                return Err(e);
            }
        };
        self.add_image(String::from(image), String::from(cid)).await;
        Ok(format! {"{}/rootfs",bundle_path.display()})
    }
}
