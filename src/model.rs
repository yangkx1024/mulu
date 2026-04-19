use gpui::SharedString;

use crate::mtp::{MtpClient, ObjectHandle, StorageId, StorageSummary};

pub struct Crumb {
    pub name: SharedString,
    pub parent: Option<ObjectHandle>,
}

pub struct Session {
    pub client: MtpClient,
    pub device_location: u64,
    pub storages: Vec<StorageSummary>,
    pub path: Vec<Crumb>,
}

impl Session {
    pub fn current_parent(&self) -> Option<ObjectHandle> {
        self.path.last().and_then(|c| c.parent)
    }

    pub fn push_folder(&mut self, name: SharedString, handle: ObjectHandle) {
        self.path.push(Crumb {
            name,
            parent: Some(handle),
        });
    }

    pub fn pop(&mut self) -> bool {
        if self.path.len() > 1 {
            self.path.pop();
            true
        } else {
            false
        }
    }

    pub fn truncate_to(&mut self, idx: usize) {
        if idx < self.path.len() {
            self.path.truncate(idx + 1);
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.path.len() > 1
    }

    pub fn reset_to_storage(&mut self, id: StorageId, name: SharedString) {
        self.client.set_active(id);
        self.path = vec![Crumb { name, parent: None }];
    }
}
