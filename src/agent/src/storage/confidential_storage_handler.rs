// Copyright (c) 2023 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use super::new_device;
use crate::storage::{common_storage_handler, StorageContext, StorageHandler};
use anyhow::{anyhow, Error, Result};
use kata_types::mount::StorageDevice;
use protocols::agent::Storage;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;
use tracing::instrument;

const CONFIDENTIAL_EPHEMERAL_STORAGE: &str = "confidential_ephemeral";
const CONFIDENTIAL_PERSISTENT_STORAGE: &str = "confidential_persistent";

#[derive(PartialEq, Debug, Clone)]
enum ConfidentialStorageType {
    Ephemeral,
    Persistent,
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
    fn handle_confidential_ephemeral_volume(storage: &Storage) -> Result<String> {
        Ok("ephemeral_path".to_string())
    }

    fn handle_confidential_persistent_volume(storage: &Storage) -> Result<String> {
        Ok("persistent_path".to_string())
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
        let storage_type = ConfidentialStorageType::from_str(storage.driver())?;

        let path = common_storage_handler(ctx.logger, &storage)?;
        let storage_path = match storage_type {
            ConfidentialStorageType::Ephemeral => {
                Self::handle_confidential_ephemeral_volume(&storage)?
            }
            ConfidentialStorageType::Persistent => {
                Self::handle_confidential_persistent_volume(&storage)?
            }
        };
        new_device(storage_path)
    }
}
