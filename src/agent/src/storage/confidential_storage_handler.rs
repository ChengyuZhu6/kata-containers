// Copyright (c) 2023 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use super::new_device;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::storage::{StorageContext, StorageHandler};
use anyhow::{anyhow, Result, Error};
use kata_types::mount::StorageDevice;
use kata_types::mount::KATA_VIRTUAL_VOLUME_IMAGE_GUEST_PULL;
use protocols::agent::Storage;
use std::sync::Arc;
use tracing::instrument;

const CONFIDENTIAL_EPHEMERAL_STORAGE: &str = "confidential_ephemeral";
const CONFIDENTIAL_PERSISTENT_STORAGE: &str = "confidential_persistent";

#[derive(PartialEq, Debug, Clone)]
enum ConfidentialStorageType {
    Ephemeral,
    Persistent,
}

impl ConfidentialStorageType {
    /// Check whether it's a pod container.
    pub fn is_ephemeral_storage(&self) -> bool {
        matches!(self, ConfidentialStorageType::Ephemeral)
    }

    /// Check whether it's a pod container.
    pub fn is_persistent_storage(&self) -> bool {
        matches!(self, ConfidentialStorageType::Persistent)
    }
}

impl Display for ConfidentialStorageType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ConfidentialStorageType::Ephemeral => write!(f, "{}", CONFIDENTIAL_EPHEMERAL_STORAGE),
            ConfidentialStorageType::Persistent => write!(f, "{}", CONFIDENTIAL_PERSISTENT_STORAGE),
        }
    }
}

impl FromStr for ConfidentialStorageType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            CONFIDENTIAL_EPHEMERAL_STORAGE => Ok(ConfidentialStorageType::Ephemeral),
            CONFIDENTIAL_PERSISTENT_STORAGE => Ok(ConfidentialStorageType::Persistent),
            _ => Err(anyhow!("missing Image information for ImagePull volume")),
        }
    }
}

#[derive(Debug)]
pub struct ConfidentialStorageHandler {}

impl ConfidentialStorageHandler {
    fn get_image_info(storage: &Storage) -> Result<ImagePullVolume> {
        for option in storage.driver_options.iter() {
            if let Some((key, value)) = option.split_once('=') {
                if key == KATA_VIRTUAL_VOLUME_IMAGE_GUEST_PULL {
                    let imagepull_volume: ImagePullVolume = serde_json::from_str(value)?;
                    return Ok(imagepull_volume);
                }
            }
        }
        Err(anyhow!("missing Image information for ImagePull volume"))
    }
}

#[async_trait::async_trait]
impl StorageHandler for ConfidentialStorageHandler {
    #[instrument]
    async fn create_device(
        &self,
        storage: Storage,
        ctx: &mut StorageContext,
    ) -> Result<Arc<dyn StorageDevice>> {
        //Currently the image metadata is not used to pulling image in the guest.
        let image_pull_volume = Self::get_image_info(&storage)?;
        debug!(ctx.logger, "image_pull_volume = {:?}", image_pull_volume);
        let image_name = storage.source();
        debug!(ctx.logger, "image_name = {:?}", image_name);

        let cid = ctx
            .cid
            .clone()
            .ok_or_else(|| anyhow!("failed to get container id"))?;
        let bundle_path = image::pull_image(image_name, &cid, &image_pull_volume.metadata).await?;

        new_device(bundle_path)
    }
}
