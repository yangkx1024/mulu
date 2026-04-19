use std::io;
use std::path::Path;

use bytes::Bytes;
use mtp_rs::{Error as MtpError, MtpDevice, NewObjectInfo, ObjectHandle, StorageId};
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
            MtpOpError::Busy => {
                "Device is in use by another application (e.g. ptpcamerad on macOS). \
                 Disconnect it and try again."
                    .to_owned()
            }
            MtpOpError::NoStorages => "No storages on device".to_owned(),
            MtpOpError::Io(e) => format!("I/O error: {e}"),
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
        Ok((Self { device, active }, summaries))
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
    ) -> Result<(), MtpOpError> {
        let storage = self.device.storage(self.active).await?;
        storage.create_folder(parent, name).await?;
        Ok(())
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
        let file = tokio::fs::File::open(path).await?;
        let stream = futures::stream::unfold(file, |mut f| async move {
            let mut buf = vec![0u8; 64 * 1024];
            match f.read(&mut buf).await {
                Ok(0) => None,
                Ok(n) => {
                    buf.truncate(n);
                    Some((Ok::<_, io::Error>(Bytes::from(buf)), f))
                }
                Err(e) => Some((Err(e), f)),
            }
        });
        storage.upload(parent, info, Box::pin(stream)).await?;
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
