use gpui::SharedString;
use mtp_rs::{ObjectHandle, StorageId};

pub struct FileEntry {
    pub handle: ObjectHandle,
    pub name: SharedString,
    pub modified: SharedString,
    pub size: SharedString,
    pub kind: SharedString,
    pub is_folder: bool,
}

pub struct StorageSummary {
    pub id: StorageId,
    pub description: SharedString,
    pub max_bytes: u64,
    pub free_bytes: u64,
}

pub struct DeviceSummary {
    pub location_id: u64,
    pub label: SharedString,
}
