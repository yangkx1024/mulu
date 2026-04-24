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
        self.clear_selection(cx);
        self.table.update(cx, |state, _| {
            state.delegate_mut().rows.clear();
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

        self.clear_selection(cx);
        self.status = Some(t!("status.loading").to_string().into());
        cx.notify();

        spawn_mtp(
            cx,
            async move { client.list(parent).await },
            move |this, result, cx| {
                match result {
                    Ok(entries) => {
                        let count = entries.len();
                        let select_idx = table.update(cx, |state, cx| {
                            let select_idx = {
                                let delegate = state.delegate_mut();
                                delegate.rows = entries;
                                delegate.sort_default();
                                select
                                    .and_then(|h| delegate.rows.iter().position(|r| r.handle == h))
                            };
                            if select_idx.is_none() {
                                cx.notify();
                            }
                            select_idx
                        });
                        if let Some(idx) = select_idx {
                            this.replace_selection(idx, cx);
                        }
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
        let entries = self.selected_entries(cx);
        if entries.is_empty() {
            return;
        }
        self.delete_entries(entries, window, cx);
    }

    pub(super) fn row_entry(
        &self,
        row_ix: usize,
        cx: &App,
    ) -> Option<(ObjectHandle, SharedString, bool)> {
        let row = self.table.read(cx).delegate().rows.get(row_ix)?;
        Some((row.handle, row.name.clone(), row.is_folder))
    }

    pub(super) fn import_into(
        &mut self,
        parent: Option<ObjectHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else { return };
        let client = session.client.clone();
        let window_handle = window.window_handle();

        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: cx.can_select_mixed_files_and_dirs(),
            multiple: true,
            prompt: Some(t!("prompt.import_files_or_folders").to_string().into()),
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
                        let mut errors: Vec<(SharedString, String)> = Vec::new();
                        for path in &paths {
                            if let Err(e) = client.upload_path(parent, path).await {
                                let name: SharedString = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or_default()
                                    .to_string()
                                    .into();
                                errors.push((name, e.user_message().to_string()));
                            }
                        }
                        errors
                    },
                    move |this, errors, cx| {
                        this.load_current_folder(None, cx);
                        if !errors.is_empty() {
                            open_error_list_dialog(
                                window_handle,
                                cx,
                                t!("dialog.import_error.title").to_string(),
                                errors,
                            );
                        }
                    },
                );
            });
        })
        .detach();
    }

    fn export_entries(
        &mut self,
        entries: Vec<(ObjectHandle, SharedString, bool)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entries.is_empty() {
            return;
        }
        let Some(session) = &self.session else { return };
        let client = session.client.clone();
        let window_handle = window.window_handle();

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
            let _ = this.update(cx, |this, cx| {
                let total = entries.len();
                let single = (total == 1).then(|| entries[0].2);
                this.status = Some(match single {
                    Some(is_folder) => {
                        let key = if is_folder {
                            "status.exporting_folder"
                        } else {
                            "status.exporting"
                        };
                        t!(key, name = entries[0].1.as_ref()).to_string().into()
                    }
                    None => t!("status.exporting_n", count = total).to_string().into(),
                });
                cx.notify();
                spawn_mtp(
                    cx,
                    async move {
                        let mut errors: Vec<(SharedString, String)> = Vec::new();
                        for (handle, name, is_folder) in entries {
                            let dest = dir.join(name.as_ref());
                            let result = if is_folder {
                                client.download_folder_to(handle, &dest).await
                            } else {
                                client.download_to(handle, &dest).await
                            };
                            if let Err(e) = result {
                                errors.push((name, e.user_message().to_string()));
                            }
                        }
                        errors
                    },
                    move |this, errors, cx| {
                        let failed = errors.len();
                        let msg = match (single, errors.first()) {
                            (Some(is_folder), None) => {
                                let key = if is_folder {
                                    "status.exported_folder"
                                } else {
                                    "status.exported"
                                };
                                t!(key).to_string()
                            }
                            (None, None) => {
                                t!("status.exported_n", count = total).to_string()
                            }
                            (Some(is_folder), Some((_, err))) => {
                                let key = if is_folder {
                                    "error.export_folder_failed"
                                } else {
                                    "error.export_failed"
                                };
                                t!(key, message = err.as_str()).to_string()
                            }
                            (None, Some((name, err))) => {
                                let first = format!("{name}: {err}");
                                t!(
                                    "error.export_n_failed",
                                    failed = failed,
                                    total = total,
                                    first = first.as_str()
                                )
                                .to_string()
                            }
                        };
                        this.set_status(msg, cx);
                        if !errors.is_empty() {
                            open_error_list_dialog(
                                window_handle,
                                cx,
                                t!("dialog.export_error.title").to_string(),
                                errors,
                            );
                        }
                    },
                );
            });
        })
        .detach();
    }

    fn delete_entries(
        &mut self,
        entries: Vec<(ObjectHandle, SharedString, bool)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entries.is_empty() {
            return;
        }
        let Some(session) = &self.session else { return };
        let client = session.client.clone();
        let view = cx.entity().downgrade();
        let description = delete_description(&entries);
        let window_handle = window.window_handle();

        window.open_alert_dialog(cx, move |alert, _, _| {
            let view = view.clone();
            let client = client.clone();
            let entries = entries.clone();
            let description = description.clone();
            alert
                .title(t!("dialog.delete.title").to_string())
                .description(description)
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("dialog.delete.ok").to_string())
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text(t!("dialog.cancel").to_string())
                        .show_cancel(true),
                )
                .on_ok(move |_, _, cx| {
                    let client = client.clone();
                    let entries = entries.clone();
                    view.update(cx, |this, cx| {
                        this.status = Some(t!("status.deleting").to_string().into());
                        cx.notify();
                        spawn_mtp(
                            cx,
                            async move {
                                let mut errors: Vec<(SharedString, String)> = Vec::new();
                                for (handle, name, _) in entries {
                                    if let Err(e) = client.delete(handle).await {
                                        errors.push((name, e.user_message().to_string()));
                                    }
                                }
                                errors
                            },
                            move |this, errors, cx| {
                                this.clear_selection(cx);
                                this.load_current_folder(None, cx);
                                if !errors.is_empty() {
                                    open_error_list_dialog(
                                        window_handle,
                                        cx,
                                        t!("dialog.delete_error.title").to_string(),
                                        errors,
                                    );
                                }
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((handle, _, true)) = self.row_entry(action.row_ix, cx) else {
            return;
        };
        self.import_into(Some(handle), window, cx);
    }

    pub(super) fn on_context_import_current(
        &mut self,
        _: &ContextImportCurrent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else { return };
        let parent = session.current_parent();
        self.import_into(parent, window, cx);
    }

    pub(super) fn on_context_export(
        &mut self,
        action: &ContextExport,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = if self.selected_rows.contains(&action.row_ix) {
            self.selected_entries(cx)
        } else {
            self.row_entry(action.row_ix, cx).into_iter().collect()
        };
        self.export_entries(entries, window, cx);
    }

    pub(super) fn on_context_delete(
        &mut self,
        action: &ContextDelete,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = if self.selected_rows.contains(&action.row_ix) {
            self.selected_entries(cx)
        } else {
            self.row_entry(action.row_ix, cx).into_iter().collect()
        };
        self.delete_entries(entries, window, cx);
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
                                        let window_handle = window.window_handle();
                                        view.update(cx, |_this, cx| {
                                            let folder_name: SharedString = name.clone().into();
                                            spawn_mtp(
                                                cx,
                                                async move {
                                                    client.create_folder(parent, &name).await
                                                },
                                                move |this, result, cx| match result {
                                                    Ok(_) => this.load_current_folder(None, cx),
                                                    Err(e) => open_error_list_dialog(
                                                        window_handle,
                                                        cx,
                                                        t!("dialog.create_folder_error.title")
                                                            .to_string(),
                                                        vec![(
                                                            folder_name,
                                                            e.user_message().to_string(),
                                                        )],
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

    pub fn on_import(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = &self.session else { return };
        let parent = session.current_parent();
        self.import_into(parent, window, cx);
    }

    pub fn on_export(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let entries = self.selected_entries(cx);
        if entries.is_empty() {
            return;
        }
        self.export_entries(entries, window, cx);
    }
}

fn open_error_list_dialog(
    window_handle: AnyWindowHandle,
    cx: &mut App,
    title: String,
    errors: Vec<(SharedString, String)>,
) {
    window_handle
        .update(cx, |_, window, cx| {
            window.open_alert_dialog(cx, move |alert, _, _| {
                let errors = errors.clone();
                let title = title.clone();
                alert
                    .title(title)
                    .description(v_flex().gap_1().children(
                        errors
                            .into_iter()
                            .map(|(name, err)| div().child(format!("{name}: {err}"))),
                    ))
                    .button_props(
                        DialogButtonProps::default().ok_text(t!("dialog.ok").to_string()),
                    )
            });
        })
        .ok();
}

fn delete_description(entries: &[(ObjectHandle, SharedString, bool)]) -> String {
    if entries.len() == 1 {
        t!(
            "dialog.delete.description",
            name = entries[0].1.as_ref()
        )
        .to_string()
    } else {
        const SHOW: usize = 5;
        let total = entries.len();
        let mut names: Vec<String> = entries
            .iter()
            .take(SHOW)
            .map(|(_, name, _)| name.to_string())
            .collect();
        if total > SHOW {
            names.push(
                t!("dialog.delete.and_more", count = total - SHOW).to_string(),
            );
        }
        let names_joined = names.join("\n");
        t!(
            "dialog.delete.description_n",
            count = total,
            names = names_joined.as_str()
        )
        .to_string()
    }
}
