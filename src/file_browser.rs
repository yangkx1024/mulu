use std::sync::OnceLock;
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::animation::{Transition, ease_in_out_cubic};
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::button::*;
use gpui_component::progress::Progress;
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableState};
use gpui_component::*;
use mtp_rs::mtp::MtpDeviceInfo;
use mtp_rs::ptp::ObjectInfo;
use mtp_rs::{DateTime, Error as MtpError, MtpDevice, ObjectHandle, StorageId};
use tokio::runtime::Runtime;

// ── Tokio runtime bridge ─────────────────────────────────────────────────────

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn tokio_rt() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

fn spawn_mtp<T, Fut, Done>(cx: &mut Context<FileBrowser>, fut: Fut, done: Done)
where
    Fut: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
    Done: FnOnce(&mut FileBrowser, T, &mut Context<FileBrowser>) + 'static,
{
    let handle = tokio_rt().spawn(fut);
    cx.spawn(async move |this, cx| {
        if let Ok(v) = handle.await {
            this.update(cx, |this, cx| done(this, v, cx)).ok();
        }
    })
    .detach();
}

// ── Column keys ──────────────────────────────────────────────────────────────

const COL_NAME: &str = "name";
const COL_MODIFIED: &str = "modified";
const COL_SIZE: &str = "size";
const COL_KIND: &str = "kind";

// ── Table row data ───────────────────────────────────────────────────────────

struct FolderRow {
    name: SharedString,
    modified: SharedString,
    size: SharedString,
    kind: SharedString,
    is_folder: bool,
    handle: ObjectHandle,
}

struct FolderDelegate {
    rows: Vec<FolderRow>,
    columns: Vec<Column>,
}

impl TableDelegate for FolderDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let row = &self.rows[row_ix];
        match self.columns[col_ix].key.as_ref() {
            COL_NAME => {
                let icon = if row.is_folder {
                    Icon::new(IconName::Folder).text_color(cx.theme().blue)
                } else {
                    Icon::new(IconName::File).text_color(cx.theme().muted_foreground)
                };
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(icon)
                    .child(row.name.clone())
                    .into_any_element()
            }
            COL_MODIFIED => row.modified.clone().into_any_element(),
            COL_SIZE => row.size.clone().into_any_element(),
            COL_KIND => row.kind.clone().into_any_element(),
            _ => "".to_string().into_any_element(),
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        let key = self.columns[col_ix].key.clone();
        let reverse = matches!(sort, ColumnSort::Descending);
        self.rows.sort_by(|a, b| {
            // Always keep folders before files.
            let by_kind = b.is_folder.cmp(&a.is_folder);
            if by_kind != std::cmp::Ordering::Equal {
                return by_kind;
            }
            let ord = match key.as_ref() {
                COL_NAME => a.name.cmp(&b.name),
                COL_MODIFIED => a.modified.cmp(&b.modified),
                COL_SIZE => a.size.cmp(&b.size),
                COL_KIND => a.kind.cmp(&b.kind),
                _ => std::cmp::Ordering::Equal,
            };
            if reverse { ord.reverse() } else { ord }
        });
    }
}

// ── Session state ────────────────────────────────────────────────────────────

struct StorageSnapshot {
    id: StorageId,
    description: SharedString,
    max_bytes: u64,
    free_bytes: u64,
}

struct Crumb {
    name: SharedString,
    parent: Option<ObjectHandle>,
}

struct Session {
    device: MtpDevice,
    device_location: u64,
    storages: Vec<StorageSnapshot>,
    active_storage: StorageId,
    path: Vec<Crumb>,
}

// ── FileBrowser ──────────────────────────────────────────────────────────────

pub struct FileBrowser {
    collapsed: bool,
    table: Entity<TableState<FolderDelegate>>,
    devices: Vec<MtpDeviceInfo>,
    session: Option<Session>,
    status: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl FileBrowser {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let columns = vec![
            Column::new(COL_NAME, "Name").width(px(280.)).ascending(),
            Column::new(COL_MODIFIED, "Date Modified")
                .width(px(200.))
                .sortable(),
            Column::new(COL_SIZE, "Size").width(px(80.)).sortable(),
            Column::new(COL_KIND, "Kind").width(px(120.)).sortable(),
        ];
        let delegate = FolderDelegate {
            rows: Vec::new(),
            columns,
        };
        let table = cx.new(|cx| TableState::new(delegate, window, cx));
        let subscriptions = vec![cx.subscribe_in(&table, window, Self::on_table_event)];

        let mut this = Self {
            collapsed: false,
            table,
            devices: Vec::new(),
            session: None,
            status: None,
            _subscriptions: subscriptions,
        };
        this.refresh_devices(cx);
        this
    }

    fn refresh_devices(&mut self, cx: &mut Context<Self>) {
        match MtpDevice::list_devices() {
            Ok(devices) => {
                if devices.is_empty() {
                    self.status = Some("No MTP devices found".into());
                } else {
                    self.status = None;
                }
                self.devices = devices;
            }
            Err(e) => {
                self.status = Some(format!("Failed to list devices: {e}").into());
            }
        }
        cx.notify();
    }

    fn open_device(&mut self, info: MtpDeviceInfo, cx: &mut Context<Self>) {
        let location_id = info.location_id;
        self.status = Some("Connecting…".into());
        cx.notify();

        spawn_mtp(
            cx,
            async move {
                let device = MtpDevice::open_by_location(location_id).await?;
                let storages = device.storages().await?;
                Ok::<_, MtpError>((device, storages))
            },
            move |this, result, cx| match result {
                Ok((device, storages)) => {
                    if storages.is_empty() {
                        this.status = Some("No storages on device".into());
                        cx.notify();
                        return;
                    }
                    let snapshots: Vec<StorageSnapshot> = storages
                        .iter()
                        .map(|s| StorageSnapshot {
                            id: s.id(),
                            description: s.info().description.clone().into(),
                            max_bytes: s.info().max_capacity,
                            free_bytes: s.info().free_space_bytes,
                        })
                        .collect();
                    let first_id = snapshots[0].id;
                    let root_name = snapshots[0].description.clone();
                    this.session = Some(Session {
                        device,
                        device_location: location_id,
                        storages: snapshots,
                        active_storage: first_id,
                        path: vec![Crumb {
                            name: root_name,
                            parent: None,
                        }],
                    });
                    this.status = None;
                    this.load_current_folder(cx);
                }
                Err(e) => {
                    let msg = if e.is_exclusive_access() {
                        "Device is in use by another application (e.g. ptpcamerad on macOS). \
                         Disconnect it and try again."
                            .into()
                    } else {
                        format!("Failed to connect: {e}").into()
                    };
                    this.status = Some(msg);
                    cx.notify();
                }
            },
        );
    }

    fn select_storage(
        &mut self,
        storage_id: StorageId,
        storage_name: SharedString,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        session.active_storage = storage_id;
        session.path = vec![Crumb {
            name: storage_name,
            parent: None,
        }];
        self.load_current_folder(cx);
    }

    fn load_current_folder(&mut self, cx: &mut Context<Self>) {
        let Some(session) = &self.session else {
            return;
        };
        let device = session.device.clone();
        let storage_id = session.active_storage;
        let parent = session.path.last().and_then(|c| c.parent);
        let table = self.table.clone();

        self.status = Some("Loading…".into());
        cx.notify();

        spawn_mtp(
            cx,
            async move {
                let storage = device.storage(storage_id).await?;
                storage.list_objects(parent).await
            },
            move |this, result, cx| {
                match result {
                    Ok(objects) => {
                        let count = objects.len();
                        let rows: Vec<FolderRow> = objects
                            .into_iter()
                            .map(|obj| {
                                let is_folder = obj.is_folder();
                                FolderRow {
                                    handle: obj.handle,
                                    modified: format_datetime(obj.modified),
                                    size: if is_folder {
                                        "—".into()
                                    } else {
                                        format_size(obj.size)
                                    },
                                    kind: format_kind(&obj),
                                    name: obj.filename.into(),
                                    is_folder,
                                }
                            })
                            .collect();
                        table.update(cx, |state, cx| {
                            state.delegate_mut().rows = rows;
                            cx.notify();
                        });
                        this.status = Some(format!("{count} items").into());
                    }
                    Err(e) => {
                        this.status = Some(format!("Error: {e}").into());
                    }
                }
                cx.notify();
            },
        );
    }

    fn navigate_to(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if idx < session.path.len() {
            session.path.truncate(idx + 1);
        }
        self.load_current_folder(cx);
    }

    fn navigate_back(&mut self, cx: &mut Context<Self>) {
        if self.session.as_ref().map_or(false, |s| s.path.len() > 1) {
            self.session.as_mut().unwrap().path.pop();
            self.load_current_folder(cx);
        }
    }

    fn on_table_event(
        &mut self,
        _: &Entity<TableState<FolderDelegate>>,
        event: &TableEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let TableEvent::DoubleClickedRow(row_ix) = event {
            let row_ix = *row_ix;
            let (is_folder, handle, name) = self
                .table
                .read(cx)
                .delegate()
                .rows
                .get(row_ix)
                .map_or((false, ObjectHandle::ROOT, SharedString::default()), |r| {
                    (r.is_folder, r.handle, r.name.clone())
                });
            if is_folder {
                if let Some(session) = self.session.as_mut() {
                    session.path.push(Crumb {
                        name,
                        parent: Some(handle),
                    });
                }
                self.load_current_folder(cx);
            }
        }
    }
}

// ── Toolbar button helper ────────────────────────────────────────────────────

fn tool_btn(id: &'static str, icon: IconName) -> Button {
    Button::new(id)
        .ghost()
        .small()
        .rounded(ButtonRounded::Large)
        .icon(Icon::new(icon).size_4())
}

// ── Formatting helpers ───────────────────────────────────────────────────────

fn format_size(bytes: u64) -> SharedString {
    if bytes == 0 {
        return "0 B".into();
    }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B").into()
    } else {
        format!("{value:.1} {}", UNITS[unit]).into()
    }
}

fn format_datetime(dt: Option<DateTime>) -> SharedString {
    match dt {
        None => "—".into(),
        Some(d) => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            d.year, d.month, d.day, d.hour, d.minute
        )
        .into(),
    }
}

fn format_kind(obj: &ObjectInfo) -> SharedString {
    if obj.is_folder() {
        return "Folder".into();
    }
    match obj.filename.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => format!("{} File", ext.to_uppercase()).into(),
        _ => "File".into(),
    }
}

// ── Render ───────────────────────────────────────────────────────────────────

impl Render for FileBrowser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.collapsed;
        let expanded_w = px(240.);

        let prev_col = window.use_keyed_state("sidebar-prev-col", cx, |_, _| collapsed);
        let anim_widths = window.use_keyed_state("sidebar-anim-w", cx, |_, _| {
            let w = if collapsed { px(0.) } else { expanded_w };
            (w, w)
        });
        if *prev_col.read(cx) != collapsed {
            let (from, to) = if collapsed {
                (expanded_w, px(0.))
            } else {
                (px(0.), expanded_w)
            };
            anim_widths.update(cx, |v, _| *v = (from, to));
            prev_col.update(cx, |v, _| *v = collapsed);
        }
        let (from_w, to_w) = *anim_widths.read(cx);

        let session_location = self.session.as_ref().map(|s| s.device_location);
        let mut device_rows: Vec<AnyElement> = Vec::new();

        for info in &self.devices {
            let location_id = info.location_id;
            let label: SharedString = format!(
                "{} {}",
                info.manufacturer.as_deref().unwrap_or("Unknown"),
                info.product.as_deref().unwrap_or("Unknown")
            )
            .into();
            let is_active = session_location == Some(location_id);
            let info_clone = info.clone();

            let row = div()
                .id(ElementId::Integer(location_id))
                .w_full()
                .px_2()
                .py_1p5()
                .cursor_pointer()
                .hover(|s| s.bg(cx.theme().sidebar_accent.opacity(0.4)))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .text_sm()
                        .font_medium()
                        .text_color(cx.theme().foreground)
                        .child(label),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_device(info_clone.clone(), cx);
                }))
                .into_any_element();

            device_rows.push(row);

            if is_active {
                if let Some(session) = &self.session {
                    for storage in &session.storages {
                        let storage_id = storage.id;
                        let storage_name = storage.description.clone();
                        let is_storage_active = storage_id == session.active_storage;
                        let free = storage.free_bytes;
                        let max = storage.max_bytes;

                        let progress_val = if max > 0 {
                            ((max - free) as f64 / max as f64 * 100.0) as f32
                        } else {
                            0.0
                        };

                        let free_str: SharedString =
                            format!("{} free of {}", format_size(free), format_size(max)).into();

                        let storage_row = v_flex()
                            .id(ElementId::Integer(storage_id.0 as u64))
                            .ml_2()
                            .mr_2()
                            .pl_2()
                            .pr_2()
                            .py_1p5()
                            .cursor_pointer()
                            .rounded(cx.theme().radius)
                            .when(is_storage_active, |s| s.bg(cx.theme().sidebar_accent))
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .items_center()
                                    .text_sm()
                                    .when(is_storage_active, |s| {
                                        s.font_medium()
                                            .text_color(cx.theme().sidebar_accent_foreground)
                                    })
                                    .when(!is_storage_active, |s| {
                                        s.text_color(cx.theme().foreground)
                                    })
                                    .child(Icon::new(IconName::HardDrive).size_3p5().text_color(
                                        if is_storage_active {
                                            cx.theme().blue
                                        } else {
                                            cx.theme().muted_foreground
                                        },
                                    ))
                                    .child(storage_name.clone()),
                            )
                            .when(is_storage_active, |this| {
                                this.child(
                                    v_flex()
                                        .gap_1()
                                        .pt_2()
                                        .pb_1()
                                        .child(
                                            Progress::new("storage-progress")
                                                .value(progress_val)
                                                .color(cx.theme().blue),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(free_str),
                                        ),
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_storage(storage_id, storage_name.clone(), cx);
                            }))
                            .into_any_element();

                        device_rows.push(storage_row);
                    }
                }
            }
        }

        let sidebar_content = v_flex()
            .w(expanded_w)
            .h_full()
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().sidebar)
            .pt_3()
            .pb_2()
            .child(
                div()
                    .px_2()
                    .pb_1()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child("Devices"),
            )
            .children(device_rows);

        let sidebar_wrapper = div()
            .id("sidebar-wrapper")
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .child(sidebar_content);

        let animated_sidebar = Transition::new(Duration::from_millis(200))
            .ease(ease_in_out_cubic)
            .width(from_w, to_w)
            .apply(
                sidebar_wrapper,
                ElementId::NamedInteger("sidebar-w".into(), collapsed as u64),
            );

        let can_go_back = self.session.as_ref().map_or(false, |s| s.path.len() > 1);

        let mut crumb_items: Vec<BreadcrumbItem> = Vec::new();
        if let Some(session) = &self.session {
            let last = session.path.len().saturating_sub(1);
            for (i, crumb) in session.path.iter().enumerate() {
                let crumb_name = crumb.name.clone();
                let item = if i < last {
                    BreadcrumbItem::new(crumb_name).on_click(cx.listener(move |this, _, _, cx| {
                        this.navigate_to(i, cx);
                    }))
                } else {
                    BreadcrumbItem::new(crumb_name)
                };
                crumb_items.push(item);
            }
        }

        let status_text = self.status.clone().unwrap_or_else(|| "0 items".into());

        h_flex().size_full().child(animated_sidebar).child(
            v_flex()
                .flex_1()
                .h_full()
                .overflow_hidden()
                .child(
                    h_flex()
                        .px_2()
                        .py_3()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .justify_between()
                        .items_center()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(SidebarToggleButton::new().collapsed(collapsed).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.collapsed = !this.collapsed;
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    tool_btn("back", IconName::ChevronLeft)
                                        .disabled(!can_go_back)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.navigate_back(cx);
                                        })),
                                )
                                .child(Breadcrumb::new().children(crumb_items)),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(tool_btn("import", IconName::ArrowUp))
                                .child(tool_btn("export", IconName::ArrowDown))
                                .child(tool_btn("new-folder", IconName::Plus))
                                .child(tool_btn("trash", IconName::Delete))
                                .child(div().w(px(8.)))
                                .child(tool_btn("info", IconName::Info)),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(DataTable::new(&self.table).stripe(true).bordered(false)),
                )
                .child(
                    h_flex()
                        .px_4()
                        .py_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(status_text),
                ),
        )
    }
}
