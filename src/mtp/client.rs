use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bytes::Bytes;
use mtp_rs::{Error as MtpError, MtpDevice, NewObjectInfo, ObjectHandle, StorageId};
use rust_i18n::t;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use crate::format::{format_datetime, format_kind, format_size};
use crate::mtp::types::{DeviceSummary, FileEntry, StorageSummary};

// ── Error type ───────────────────────────────────────────────────────────────

pub enum MtpOpError {
    Busy,
    NoStorages,
    Io(io::Error),
    Mtp(MtpError),
}

impl MtpOpError {
    pub fn user_message(&self) -> String {
        match self {
            MtpOpError::Busy => t!("error.device_busy").to_string(),
            MtpOpError::NoStorages => t!("error.no_storages").to_string(),
            MtpOpError::Io(e) => t!("error.io", message = e.to_string()).to_string(),
            MtpOpError::Mtp(e) => format!("{e}"),
        }
    }
}

impl From<MtpError> for MtpOpError {
    fn from(e: MtpError) -> Self {
        if e.is_exclusive_access() {
            MtpOpError::Busy
        } else {
            MtpOpError::Mtp(e)
        }
    }
}

impl From<io::Error> for MtpOpError {
    fn from(e: io::Error) -> Self {
        MtpOpError::Io(e)
    }
}

// ── Client ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MtpClient {
    device: MtpDevice,
    active: StorageId,
    // Some Android devices reject parent=0x00000000 at root; discovered lazily on first failure.
    root_uses_all_handle: Arc<AtomicBool>,
}

impl MtpClient {
    pub async fn open(location_id: u64) -> Result<(Self, Vec<StorageSummary>), MtpOpError> {
        let device = MtpDevice::open_by_location(location_id).await?;
        let storages = device.storages().await?;
        if storages.is_empty() {
            return Err(MtpOpError::NoStorages);
        }
        let summaries: Vec<StorageSummary> = storages
            .iter()
            .map(|s| StorageSummary {
                id: s.id(),
                description: s.info().description.clone().into(),
                max_bytes: s.info().max_capacity,
                free_bytes: s.info().free_space_bytes,
            })
            .collect();
        let active = summaries[0].id;
        Ok((
            Self {
                device,
                active,
                root_uses_all_handle: Arc::new(AtomicBool::new(false)),
            },
            summaries,
        ))
    }

    fn root_parent(&self, parent: Option<ObjectHandle>) -> Option<ObjectHandle> {
        if parent.is_some() {
            return parent;
        }
        if self.root_uses_all_handle.load(Ordering::Relaxed) {
            Some(ObjectHandle::ALL)
        } else {
            None
        }
    }

    pub fn set_active(&mut self, id: StorageId) {
        self.active = id;
    }

    pub fn active(&self) -> StorageId {
        self.active
    }

    pub async fn list(&self, parent: Option<ObjectHandle>) -> Result<Vec<FileEntry>, MtpOpError> {
        let storage = self.device.storage(self.active).await?;
        let objects = storage.list_objects(parent).await?;
        Ok(objects
            .into_iter()
            .map(|obj| {
                let is_folder = obj.is_folder();
                FileEntry {
                    handle: obj.handle,
                    name: obj.filename.clone().into(),
                    modified: format_datetime(obj.modified),
                    size: if is_folder {
                        "—".into()
                    } else {
                        format_size(obj.size)
                    },
                    kind: format_kind(&obj.filename, is_folder),
                    is_folder,
                }
            })
            .collect())
    }

    pub async fn create_folder(
        &self,
        parent: Option<ObjectHandle>,
        name: &str,
    ) -> Result<ObjectHandle, MtpOpError> {
        let storage = self.device.storage(self.active).await?;
        match storage.create_folder(self.root_parent(parent), name).await {
            Ok(handle) => Ok(handle),
            Err(e) => {
                if needs_all_handle_retry(parent, &e) {
                    self.root_uses_all_handle.store(true, Ordering::Relaxed);
                    let handle = storage.create_folder(Some(ObjectHandle::ALL), name).await?;
                    Ok(handle)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    pub async fn delete(&self, handle: ObjectHandle) -> Result<(), MtpOpError> {
        let storage = self.device.storage(self.active).await?;
        storage.delete(handle).await?;
        Ok(())
    }

    pub async fn upload_file(
        &self,
        parent: Option<ObjectHandle>,
        path: &Path,
    ) -> Result<(), MtpOpError> {
        let storage = self.device.storage(self.active).await?;
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unnamed".into());
        let size = tokio::fs::metadata(path).await?.len();
        let info = NewObjectInfo::file(file_name, size);

        let result = {
            let file = tokio::fs::File::open(path).await?;
            let stream = file_read_stream(file);
            storage
                .upload(self.root_parent(parent), info.clone(), Box::pin(stream))
                .await
        };

        // Android MTP quirk: some devices reject parent=0x00000000 at root and expect
        // ObjectHandle::ALL (0xFFFFFFFF) instead — retry once and cache the discovery.
        if let Err(e) = result {
            if needs_all_handle_retry(parent, &e) {
                self.root_uses_all_handle.store(true, Ordering::Relaxed);
                let file = tokio::fs::File::open(path).await?;
                let stream = file_read_stream(file);
                storage
                    .upload(Some(ObjectHandle::ALL), info, Box::pin(stream))
                    .await?;
            } else {
                return Err(e.into());
            }
        }
        Ok(())
    }

    pub async fn download_to(&self, handle: ObjectHandle, dest: &Path) -> Result<(), MtpOpError> {
        let storage = self.device.storage(self.active).await?;
        let mut dl = storage.download_stream(handle).await?;
        let mut file = tokio::fs::File::create(dest).await?;
        while let Some(chunk) = dl.next_chunk().await {
            let bytes = chunk?;
            file.write_all(&bytes).await?;
        }
        file.flush().await?;
        Ok(())
    }

    pub async fn upload_path(
        &self,
        parent: Option<ObjectHandle>,
        path: &Path,
    ) -> Result<(), MtpOpError> {
        if tokio::fs::metadata(path).await?.is_dir() {
            self.upload_folder(parent, path).await
        } else {
            self.upload_file(parent, path).await
        }
    }

    pub async fn upload_folder(
        &self,
        parent: Option<ObjectHandle>,
        local_path: &Path,
    ) -> Result<(), MtpOpError> {
        let name = local_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unnamed".into());
        let new_handle = self.create_folder(parent, &name).await?;

        let mut entries = tokio::fs::read_dir(local_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            Box::pin(self.upload_path(Some(new_handle), &entry.path())).await?;
        }
        Ok(())
    }

    pub async fn download_folder_to(
        &self,
        handle: ObjectHandle,
        local_dir: &Path,
    ) -> Result<(), MtpOpError> {
        tokio::fs::create_dir_all(local_dir).await?;
        let entries = self.list(Some(handle)).await?;
        for entry in entries {
            let child = local_dir.join(entry.name.as_ref());
            if entry.is_folder {
                Box::pin(self.download_folder_to(entry.handle, &child)).await?;
            } else {
                self.download_to(entry.handle, &child).await?;
            }
        }
        Ok(())
    }
}

fn needs_all_handle_retry(parent: Option<ObjectHandle>, err: &mtp_rs::Error) -> bool {
    parent.is_none()
        && matches!(
            err,
            mtp_rs::Error::Protocol {
                code: mtp_rs::ResponseCode::InvalidObjectHandle,
                ..
            }
        )
}

fn file_read_stream(
    file: tokio::fs::File,
) -> impl futures::Stream<Item = Result<Bytes, io::Error>> {
    futures::stream::unfold(file, |mut f| async move {
        let mut buf = vec![0u8; 64 * 1024];
        match f.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => {
                buf.truncate(n);
                Some((Ok(Bytes::from(buf)), f))
            }
            Err(e) => Some((Err(e), f)),
        }
    })
}

// ── Device listing ───────────────────────────────────────────────────────────

pub fn list_devices() -> Result<Vec<DeviceSummary>, MtpOpError> {
    let devices = MtpDevice::list_devices()?;
    Ok(devices
        .iter()
        .map(|info| DeviceSummary {
            location_id: info.location_id,
            label: format!(
                "{} {}",
                info.manufacturer.as_deref().unwrap_or("Unknown"),
                info.product.as_deref().unwrap_or("Unknown")
            )
            .into(),
        })
        .collect())
}
