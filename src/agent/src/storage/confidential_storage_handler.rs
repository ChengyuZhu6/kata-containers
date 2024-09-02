// Copyright (c) 2024 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use super::new_device;
use crate::cdh;
use crate::storage::{StorageContext, StorageHandler};
use anyhow::{anyhow, Result};
use kata_types::mount::StorageDevice;
use protocols::agent::Storage;
use std::sync::Arc;
use tracing::instrument;

const CONFIDENTIAL_EPHEMERAL_STORAGE: &str = "confidential_ephemeral";
const CONFIDENTIAL_PERSISTENT_STORAGE: &str = "confidential_persistent";

#[derive(Debug)]
pub struct ConfidentialStorageHandler {}

impl ConfidentialStorageHandler {
    async fn handle_confidential_ephemeral_volume(storage: &Storage) -> Result<String> {
        let options = std::collections::HashMap::from([
            ("deviceId".to_string(), storage.source().to_string()),
            ("encryptType".to_string(), "LUKS".to_string()),
            ("dataIntegrity".to_string(), "False".to_string()),
        ]);
        cdh::secure_mount("BlockDevice", &options, vec![], storage.mount_point()).await?;
        Ok(storage.mount_point().to_string())
    }

    async fn handle_confidential_persistent_volume(_storage: &Storage) -> Result<String> {
        Err(anyhow!(
            "missing the implementation for confidential persistent volume!"
        ))
    }
}

#[async_trait::async_trait]
impl StorageHandler for ConfidentialStorageHandler {
    #[instrument]
    fn driver_types(&self) -> &[&str] {
        &[
            CONFIDENTIAL_EPHEMERAL_STORAGE,
            CONFIDENTIAL_PERSISTENT_STORAGE,
        ]
    }

    #[instrument]
    async fn create_device(
        &self,
        storage: Storage,
        ctx: &mut StorageContext,
    ) -> Result<Arc<dyn StorageDevice>> {
        let storage_path = match storage.driver() {
            CONFIDENTIAL_EPHEMERAL_STORAGE => {
                Self::handle_confidential_ephemeral_volume(&storage).await?
            }
            CONFIDENTIAL_PERSISTENT_STORAGE => {
                Self::handle_confidential_persistent_volume(&storage).await?
            }
            _ => return Err(anyhow!("Unsupported storage driver: {}", storage.driver())),
        };
        new_device(storage_path)
    }
}
