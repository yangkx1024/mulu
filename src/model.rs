use gpui::SharedString;

use crate::mtp::{MtpClient, ObjectHandle, StorageId, StorageSummary};

pub struct Crumb {
    pub name: SharedString,
    /// The folder's own object handle, used as the `parent` parameter when
    /// listing its children. `None` for storage roots.
    pub handle: Option<ObjectHandle>,
}

pub struct Session {
    pub client: MtpClient,
    pub device_location: u64,
    pub storages: Vec<StorageSummary>,
    pub path: Vec<Crumb>,
}

impl Session {
    pub fn current_parent(&self) -> Option<ObjectHandle> {
        self.path.last().and_then(|c| c.handle)
    }

    pub fn push_folder(&mut self, name: SharedString, handle: ObjectHandle) {
        self.path.push(Crumb {
            name,
            handle: Some(handle),
        });
    }

    pub fn pop(&mut self) -> Option<ObjectHandle> {
        if self.path.len() > 1 {
            self.path.pop().and_then(|c| c.handle)
        } else {
            None
        }
    }

    pub fn truncate_to(&mut self, idx: usize) -> Option<ObjectHandle> {
        if idx + 1 < self.path.len() {
            let select = self.path[idx + 1].handle;
            self.path.truncate(idx + 1);
            select
        } else {
            None
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.path.len() > 1
    }

    pub fn reset_to_storage(&mut self, id: StorageId, name: SharedString) {
        self.client.set_active(id);
        self.path = vec![Crumb { name, handle: None }];
    }
}
