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

use anyhow::{anyhow, Context, Result};
use image_rs::image::ImageClient;
use tokio::sync::Mutex;

use crate::rpc::CONTAINER_BASE;
use crate::AGENT_CONFIG;

// A marker to merge container spec for images pulled inside guest.
const ANNO_K8S_IMAGE_NAME: &str = "io.kubernetes.cri.image-name";
const KATA_CC_IMAGE_WORK_DIR: &str = "/run/image/";
const CONFIG_JSON: &str = "config.json";

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
        env::set_var("CC_IMAGE_WORK_DIR", KATA_CC_IMAGE_WORK_DIR);

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

    // When being passed an image name through a container annotation, merge its
    // corresponding bundle OCI specification into the passed container creation one.
    pub async fn merge_bundle_oci(&self, container_oci: &mut oci::Spec) -> Result<()> {
        if let Some(image_name) = container_oci
            .annotations
            .get(&ANNO_K8S_IMAGE_NAME.to_string())
        {
            let images = self.images.lock().await;
            if let Some(container_id) = images.get(image_name) {
                let image_oci_config_path = Path::new(CONTAINER_BASE)
                    .join(container_id)
                    .join(CONFIG_JSON);
                debug!(
                    sl(),
                    "Image bundle config path: {:?}", image_oci_config_path
                );

                let image_oci =
                    oci::Spec::load(image_oci_config_path.to_str().ok_or_else(|| {
                        anyhow!(
                            "Invalid container image OCI config path {:?}",
                            image_oci_config_path
                        )
                    })?)
                    .context("load image bundle")?;

                if let Some(container_root) = container_oci.root.as_mut() {
                    if let Some(image_root) = image_oci.root.as_ref() {
                        let root_path = Path::new(CONTAINER_BASE)
                            .join(container_id)
                            .join(image_root.path.clone());
                        container_root.path =
                            String::from(root_path.to_str().ok_or_else(|| {
                                anyhow!("Invalid container image root path {:?}", root_path)
                            })?);
                    }
                }

                if let Some(container_process) = container_oci.process.as_mut() {
                    if let Some(image_process) = image_oci.process.as_ref() {
                        self.merge_oci_process(container_process, image_process);
                    }
                }
            }
        }

        Ok(())
    }

    // Partially merge an OCI process specification into another one.
    fn merge_oci_process(&self, target: &mut oci::Process, source: &oci::Process) {
        if target.args.is_empty() && !source.args.is_empty() {
            target.args.append(&mut source.args.clone());
        }

        if target.cwd == "/" && source.cwd != "/" {
            target.cwd = String::from(&source.cwd);
        }

        for source_env in &source.env {
            let variable_name: Vec<&str> = source_env.split('=').collect();
            if !target.env.iter().any(|i| i.contains(variable_name[0])) {
                target.env.push(source_env.to_string());
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::ImageService;
    #[tokio::test]
    async fn test_merge_cwd() {
        #[derive(Debug)]
        struct TestData<'a> {
            container_process_cwd: &'a str,
            image_process_cwd: &'a str,
            expected: &'a str,
        }

        let tests = &[
            // Image cwd should override blank container cwd
            // TODO - how can we tell the user didn't specifically set it to `/` vs not setting at all? Is that scenario valid?
            TestData {
                container_process_cwd: "/",
                image_process_cwd: "/imageDir",
                expected: "/imageDir",
            },
            // Container cwd should override image cwd
            TestData {
                container_process_cwd: "/containerDir",
                image_process_cwd: "/imageDir",
                expected: "/containerDir",
            },
            // Container cwd should override blank image cwd
            TestData {
                container_process_cwd: "/containerDir",
                image_process_cwd: "/",
                expected: "/containerDir",
            },
        ];

        let image_service = ImageService::new();

        for (i, d) in tests.iter().enumerate() {
            let msg = format!("test[{}]: {:?}", i, d);

            let mut container_process = oci::Process {
                cwd: d.container_process_cwd.to_string(),
                ..Default::default()
            };

            let image_process = oci::Process {
                cwd: d.image_process_cwd.to_string(),
                ..Default::default()
            };

            image_service.merge_oci_process(&mut container_process, &image_process);

            assert_eq!(d.expected, container_process.cwd, "{}", msg);
        }
    }

    #[tokio::test]
    async fn test_merge_env() {
        #[derive(Debug)]
        struct TestData {
            container_process_env: Vec<String>,
            image_process_env: Vec<String>,
            expected: Vec<String>,
        }

        let tests = &[
            // Test that the pods environment overrides the images
            TestData {
                container_process_env: vec!["ISPRODUCTION=true".to_string()],
                image_process_env: vec!["ISPRODUCTION=false".to_string()],
                expected: vec!["ISPRODUCTION=true".to_string()],
            },
            // Test that multiple environment variables can be overrided
            TestData {
                container_process_env: vec![
                    "ISPRODUCTION=true".to_string(),
                    "ISDEVELOPMENT=false".to_string(),
                ],
                image_process_env: vec![
                    "ISPRODUCTION=false".to_string(),
                    "ISDEVELOPMENT=true".to_string(),
                ],
                expected: vec![
                    "ISPRODUCTION=true".to_string(),
                    "ISDEVELOPMENT=false".to_string(),
                ],
            },
            // Test that when none of the variables match do not override them
            TestData {
                container_process_env: vec!["ANOTHERENV=TEST".to_string()],
                image_process_env: vec![
                    "ISPRODUCTION=false".to_string(),
                    "ISDEVELOPMENT=true".to_string(),
                ],
                expected: vec![
                    "ANOTHERENV=TEST".to_string(),
                    "ISPRODUCTION=false".to_string(),
                    "ISDEVELOPMENT=true".to_string(),
                ],
            },
            // Test a mix of both overriding and not
            TestData {
                container_process_env: vec![
                    "ANOTHERENV=TEST".to_string(),
                    "ISPRODUCTION=true".to_string(),
                ],
                image_process_env: vec![
                    "ISPRODUCTION=false".to_string(),
                    "ISDEVELOPMENT=true".to_string(),
                ],
                expected: vec![
                    "ANOTHERENV=TEST".to_string(),
                    "ISPRODUCTION=true".to_string(),
                    "ISDEVELOPMENT=true".to_string(),
                ],
            },
        ];

        let image_service = ImageService::new();

        for (i, d) in tests.iter().enumerate() {
            let msg = format!("test[{}]: {:?}", i, d);

            let mut container_process = oci::Process {
                env: d.container_process_env.clone(),
                ..Default::default()
            };

            let image_process = oci::Process {
                env: d.image_process_env.clone(),
                ..Default::default()
            };

            image_service.merge_oci_process(&mut container_process, &image_process);

            assert_eq!(d.expected, container_process.env, "{}", msg);
        }
    }
}
