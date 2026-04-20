use gpui::*;
use gpui_component::button::*;
use gpui_component::dialog::{DialogButtonProps, DialogFooter};
use gpui_component::input::{Input, InputState};
use gpui_component::*;
use rust_i18n::t;

use super::MtpBrowser;
use crate::model::{Crumb, Session};
use crate::mtp::{
    DeviceSummary, MtpClient, MtpOpError, ObjectHandle, StorageId, list_devices, spawn_mtp,
};

pub(super) fn no_devices_found() -> SharedString {
    t!("status.no_devices").to_string().into()
}

#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = mtp_browser, no_json)]
pub(super) struct ContextImportHere {
    pub row_ix: usize,
}

#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = mtp_browser, no_json)]
pub(super) struct ContextExport {
    pub row_ix: usize,
}

#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = mtp_browser, no_json)]
pub(super) struct ContextDelete {
    pub row_ix: usize,
}

gpui::actions!(mtp_browser, [ContextImportCurrent, ContextNewFolder]);

impl MtpBrowser {
    pub(super) fn refresh_devices(&mut self, cx: &mut Context<Self>) {
        self.apply_device_list(list_devices(), cx);
    }

    pub(crate) fn apply_device_list(
        &mut self,
        result: Result<Vec<DeviceSummary>, MtpOpError>,
        cx: &mut Context<Self>,
    ) {
        let new_devices = match result {
            Ok(d) => d,
            Err(e) => {
                self.status = Some(
                    t!("error.list_devices_failed", message = e.user_message())
                        .to_string()
                        .into(),
                );
                cx.notify();
                return;
            }
        };

        if new_devices == self.devices {
            return;
        }

        self.devices = new_devices;

        let active_gone = self.session.as_ref().is_some_and(|s| {
            !self
                .devices
                .iter()
                .any(|d| d.location_id == s.device_location)
        });
        if active_gone {
            self.close_session(cx);
        } else {
            self.status = if self.devices.is_empty() {
                Some(no_devices_found())
            } else {
                None
            };
        }

        cx.notify();
    }

    pub(super) fn on_locale_changed(&mut self, cx: &mut Context<Self>) {
        if self.session.is_some() {
            let count = self.table.read(cx).delegate().rows.len();
            self.status = Some(t!("status.items", count = count).to_string().into());
        } else if self.status.is_some() {
            self.status = if self.devices.is_empty() {
                Some(no_devices_found())
            } else {
                Some(t!("status.disconnected").to_string().into())
            };
        }
    }

    pub(super) fn close_session(&mut self, cx: &mut Context<Self>) {
        self.session = None;
        self.selected_row = None;
        self.table.update(cx, |state, cx| {
            state.delegate_mut().rows.clear();
            state.clear_selection(cx);
        });
        self.status = if self.devices.is_empty() {
            Some(no_devices_found())
        } else {
            Some(t!("status.disconnected").to_string().into())
        };
    }

    pub(super) fn open_device(&mut self, location_id: u64, cx: &mut Context<Self>) {
        self.status = Some(t!("status.connecting").to_string().into());
        cx.notify();

        spawn_mtp(
            cx,
            async move { MtpClient::open(location_id).await },
            move |this, result, cx| match result {
                Ok((client, storages)) => {
                    let root_name = storages[0].description.clone();
                    this.session = Some(Session {
                        client,
                        device_location: location_id,
                        storages,
                        path: vec![Crumb {
                            name: root_name,
                            handle: None,
                        }],
                    });
                    this.status = None;
                    this.load_current_folder(None, cx);
                }
                Err(e) => {
                    this.status = Some(e.user_message().into());
                    cx.notify();
                }
            },
        );
    }

    pub(super) fn select_storage(
        &mut self,
        storage_id: StorageId,
        storage_name: SharedString,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        session.reset_to_storage(storage_id, storage_name);
        self.load_current_folder(None, cx);
    }

    pub(super) fn load_current_folder(
        &mut self,
        select: Option<ObjectHandle>,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else { return };
        let client = session.client.clone();
        let parent = session.current_parent();
        let table = self.table.clone();

        self.selected_row = None;
        self.table.update(cx, |state, cx| state.clear_selection(cx));
        self.status = Some(t!("status.loading").to_string().into());
        cx.notify();

        spawn_mtp(
            cx,
            async move { client.list(parent).await },
            move |this, result, cx| {
                match result {
                    Ok(entries) => {
                        let count = entries.len();
                        table.update(cx, |state, cx| {
                            let select_idx = {
                                let delegate = state.delegate_mut();
                                delegate.rows = entries;
                                delegate.sort_default();
                                select
                                    .and_then(|h| delegate.rows.iter().position(|r| r.handle == h))
                            };
                            if let Some(idx) = select_idx {
                                state.set_selected_row(idx, cx);
                            } else {
                                cx.notify();
                            }
                        });
                        this.status = Some(t!("status.items", count = count).to_string().into());
                    }
                    Err(e) => {
                        this.status = Some(
                            t!("error.error_prefix", message = e.user_message())
                                .to_string()
                                .into(),
                        );
                    }
                }
                cx.notify();
            },
        );
    }

    pub fn on_trash(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some((handle, name, _)) = self.selected_row_info(cx) else {
            return;
        };
        self.delete_entry(handle, name, window, cx);
    }

    pub(super) fn row_entry(
        &self,
        row_ix: usize,
        cx: &App,
    ) -> Option<(ObjectHandle, SharedString, bool)> {
        let row = self.table.read(cx).delegate().rows.get(row_ix)?;
        Some((row.handle, row.name.clone(), row.is_folder))
    }

    fn import_into(&mut self, parent: Option<ObjectHandle>, cx: &mut Context<Self>) {
        let Some(session) = &self.session else { return };
        let client = session.client.clone();

        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(t!("prompt.import").to_string().into()),
        });

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let count = paths.len();
            let _ = this.update(cx, |this, cx| {
                this.status = Some(t!("status.uploading", count = count).to_string().into());
                cx.notify();
                spawn_mtp(
                    cx,
                    async move {
                        for path in &paths {
                            client.upload_file(parent, path).await?;
                        }
                        Ok::<(), MtpOpError>(())
                    },
                    |this, result, cx| match result {
                        Ok(()) => this.load_current_folder(None, cx),
                        Err(e) => this.set_status(
                            t!("error.upload_failed", message = e.user_message()).to_string(),
                            cx,
                        ),
                    },
                );
            });
        })
        .detach();
    }

    fn export_entry(&mut self, handle: ObjectHandle, name: SharedString, cx: &mut Context<Self>) {
        let Some(session) = &self.session else { return };
        let client = session.client.clone();

        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t!("prompt.export_to").to_string().into()),
        });

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dirs))) = rx.await else {
                return;
            };
            let Some(dir) = dirs.into_iter().next() else {
                return;
            };
            let dest = dir.join(name.as_ref());
            let _ = this.update(cx, |this, cx| {
                this.status = Some(
                    t!("status.exporting", name = name.as_ref())
                        .to_string()
                        .into(),
                );
                cx.notify();
                spawn_mtp(
                    cx,
                    async move { client.download_to(handle, &dest).await },
                    |this, result, cx| match result {
                        Ok(()) => this.set_status(t!("status.exported").to_string(), cx),
                        Err(e) => this.set_status(
                            t!("error.export_failed", message = e.user_message()).to_string(),
                            cx,
                        ),
                    },
                );
            });
        })
        .detach();
    }

    fn delete_entry(
        &mut self,
        handle: ObjectHandle,
        name: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else { return };
        let client = session.client.clone();
        let view = cx.entity().downgrade();
        let desc_name = name.clone();

        window.open_alert_dialog(cx, move |alert, _, _| {
            let view = view.clone();
            let name = desc_name.clone();
            let client = client.clone();
            alert
                .title(t!("dialog.delete.title").to_string())
                .description(t!("dialog.delete.description", name = name.as_ref()).to_string())
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("dialog.delete.ok").to_string())
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text(t!("dialog.cancel").to_string())
                        .show_cancel(true),
                )
                .on_ok(move |_, _, cx| {
                    let client = client.clone();
                    view.update(cx, |this, cx| {
                        this.status = Some(t!("status.deleting").to_string().into());
                        cx.notify();
                        spawn_mtp(
                            cx,
                            async move { client.delete(handle).await },
                            |this, result, cx| match result {
                                Ok(()) => {
                                    this.selected_row = None;
                                    this.load_current_folder(None, cx);
                                }
                                Err(e) => this.set_status(
                                    t!("error.delete_failed", message = e.user_message())
                                        .to_string(),
                                    cx,
                                ),
                            },
                        );
                    })
                    .ok();
                    true
                })
        });
    }

    pub(super) fn on_context_import_here(
        &mut self,
        action: &ContextImportHere,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((handle, _, true)) = self.row_entry(action.row_ix, cx) else {
            return;
        };
        self.import_into(Some(handle), cx);
    }

    pub(super) fn on_context_import_current(
        &mut self,
        _: &ContextImportCurrent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else { return };
        let parent = session.current_parent();
        self.import_into(parent, cx);
    }

    pub(super) fn on_context_export(
        &mut self,
        action: &ContextExport,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((handle, name, false)) = self.row_entry(action.row_ix, cx) else {
            return;
        };
        self.export_entry(handle, name, cx);
    }

    pub(super) fn on_context_delete(
        &mut self,
        action: &ContextDelete,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((handle, name, _)) = self.row_entry(action.row_ix, cx) else {
            return;
        };
        self.delete_entry(handle, name, window, cx);
    }

    pub fn on_new_folder(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.new_folder(window, cx);
    }

    fn new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = &self.session else { return };
        let client = session.client.clone();
        let parent = session.current_parent();
        let view = cx.entity().downgrade();

        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("dialog.new_folder.placeholder").to_string())
        });

        window.open_dialog(cx, {
            let input = input.clone();
            move |dialog, _, _| {
                let input_for_create = input.clone();
                let view = view.clone();
                let client = client.clone();
                dialog
                    .title(t!("dialog.new_folder.title").to_string())
                    .child(v_flex().py_3().child(Input::new(&input)))
                    .footer(
                        DialogFooter::new()
                            .child(
                                Button::new("cancel")
                                    .label(t!("dialog.cancel").to_string())
                                    .outline()
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            )
                            .child({
                                let view = view.clone();
                                let client = client.clone();
                                Button::new("create")
                                    .label(t!("dialog.new_folder.create").to_string())
                                    .primary()
                                    .on_click(move |_, window, cx| {
                                        let name = input_for_create.read(cx).value().to_string();
                                        if name.trim().is_empty() {
                                            return;
                                        }
                                        let client = client.clone();
                                        view.update(cx, |_this, cx| {
                                            spawn_mtp(
                                                cx,
                                                async move {
                                                    client.create_folder(parent, &name).await
                                                },
                                                |this, result, cx| match result {
                                                    Ok(()) => this.load_current_folder(None, cx),
                                                    Err(e) => this.set_status(
                                                        t!(
                                                            "error.create_failed",
                                                            message = e.user_message()
                                                        )
                                                        .to_string(),
                                                        cx,
                                                    ),
                                                },
                                            );
                                        })
                                        .ok();
                                        window.close_dialog(cx);
                                    })
                            }),
                    )
            }
        });

        input.update(cx, |state, cx| state.focus(window, cx));
    }

    pub(super) fn on_context_new_folder(
        &mut self,
        _: &ContextNewFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_folder(window, cx);
    }

    pub fn on_import(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = &self.session else { return };
        let parent = session.current_parent();
        self.import_into(parent, cx);
    }

    pub fn on_export(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some((handle, name, is_folder)) = self.selected_row_info(cx) else {
            return;
        };
        if is_folder {
            return;
        }
        self.export_entry(handle, name, cx);
    }
}
