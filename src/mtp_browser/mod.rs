mod actions;
mod sidebar;
mod status_bar;
mod table;
mod toolbar;

use std::collections::BTreeSet;

use gpui::*;
use gpui_component::table::{TableEvent, TableState};
use gpui_component::*;

use crate::model::Session;
use crate::mtp::{DeviceSummary, ObjectHandle};
use crate::update_check::UpdateInfo;
use actions::no_devices_found;
use table::FolderDelegate;

pub struct MtpBrowser {
    collapsed: bool,
    table: Entity<TableState<FolderDelegate>>,
    devices: Vec<DeviceSummary>,
    session: Option<Session>,
    status: Option<SharedString>,
    pub(super) selected_rows: BTreeSet<usize>,
    pub(super) anchor_row: Option<usize>,
    drag_anchor_row: Option<usize>,
    drag_end_row: Option<usize>,
    suppress_select_row: bool,
    update_info: Option<UpdateInfo>,
    _subscriptions: Vec<Subscription>,
}

impl MtpBrowser {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = FolderDelegate::new();
        let table = cx.new(|cx| TableState::new(delegate, window, cx));
        let subscriptions = vec![cx.subscribe_in(&table, window, Self::on_table_event)];

        let weak = cx.entity().downgrade();
        table.update(cx, |state, _| {
            state.delegate_mut().view = Some(weak);
        });

        let mut this = Self {
            collapsed: false,
            table,
            devices: Vec::new(),
            session: None,
            status: Some(no_devices_found()),
            selected_rows: BTreeSet::new(),
            anchor_row: None,
            drag_anchor_row: None,
            drag_end_row: None,
            suppress_select_row: false,
            update_info: None,
            _subscriptions: subscriptions,
        };
        this.refresh_devices(cx);
        crate::mtp::watch_hotplug(cx);
        this.check_for_updates(cx);
        this
    }

    /// Programmatic single-row selection. Updates our state and the table's own
    /// `selected_row` (focus). Suppresses the `SelectRow` event the table emits
    /// in response, so we don't loop back through `on_table_event`.
    pub(super) fn replace_selection(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        self.selected_rows.clear();
        self.selected_rows.insert(row_ix);
        self.anchor_row = Some(row_ix);
        self.sync_table_focus_to(row_ix, cx);
        self.push_selection_to_delegate(cx);
        cx.notify();
    }

    pub(super) fn clear_selection(&mut self, cx: &mut Context<Self>) {
        let had_state = !self.selected_rows.is_empty() || self.anchor_row.is_some();
        self.selected_rows.clear();
        self.anchor_row = None;
        self.table.update(cx, |state, cx| state.clear_selection(cx));
        self.push_selection_to_delegate(cx);
        if had_state {
            cx.notify();
        }
    }

    fn sync_table_focus_to(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        if self.table.read(cx).selected_row() == Some(row_ix) {
            return;
        }
        self.suppress_select_row = true;
        self.table
            .update(cx, |state, cx| state.set_selected_row(row_ix, cx));
    }

    fn push_selection_to_delegate(&mut self, cx: &mut Context<Self>) {
        let selection = self.selected_rows.clone();
        self.table.update(cx, |state, cx| {
            let focus = state.selected_row();
            let delegate = state.delegate_mut();
            delegate.selected_rows = selection;
            delegate.focus_row = focus;
            cx.notify();
        });
    }

    /// Update selection state from a row mouse-down. Does NOT call
    /// `set_selected_row` on the table — the table's own `on_row_left_click`
    /// will follow this event and update its `selected_row` (focus). The
    /// `SelectRow` event it emits is suppressed via `suppress_select_row`.
    pub(super) fn handle_row_mouse_down(
        &mut self,
        row_ix: usize,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        let actual_count = self.table.read(cx).delegate().rows.len();
        if row_ix >= actual_count {
            return;
        }
        if modifiers.shift {
            let anchor = self.anchor_row.unwrap_or(row_ix);
            let (lo, hi) = sorted_pair(anchor, row_ix);
            if !modifiers.secondary() {
                self.selected_rows.clear();
            }
            for ix in lo..=hi {
                self.selected_rows.insert(ix);
            }
            self.anchor_row = Some(anchor);
        } else if modifiers.secondary() {
            if self.selected_rows.insert(row_ix) {
                self.anchor_row = Some(row_ix);
            } else {
                self.selected_rows.remove(&row_ix);
                if self.anchor_row == Some(row_ix) {
                    self.anchor_row = self.selected_rows.iter().next().copied();
                }
            }
        } else {
            self.selected_rows.clear();
            self.selected_rows.insert(row_ix);
            self.anchor_row = Some(row_ix);
            self.drag_anchor_row = Some(row_ix);
            self.drag_end_row = Some(row_ix);
            // on_click is suppressed during drags, so sync focus eagerly here
            // or the built-in highlight stays on the previously-clicked row.
            self.sync_table_focus_to(row_ix, cx);
        }
        // The click event fires next and will emit SelectRow; eat it.
        self.suppress_select_row = true;
        self.push_selection_to_delegate(cx);
        cx.notify();
    }

    pub(super) fn extend_drag_to_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        let Some(anchor) = self.drag_anchor_row else {
            return;
        };
        if self.drag_end_row == Some(row_ix) {
            return;
        }
        let actual_count = self.table.read(cx).delegate().rows.len();
        if row_ix >= actual_count {
            return;
        }
        self.drag_end_row = Some(row_ix);
        let (lo, hi) = sorted_pair(anchor, row_ix);
        self.selected_rows = (lo..=hi).collect();
        self.suppress_select_row = true;
        self.push_selection_to_delegate(cx);
        cx.notify();
    }

    pub(super) fn end_drag_select(&mut self) {
        // If the drag moved off the starting row, on_click won't fire to
        // consume suppress_select_row — clear it here so the next SelectRow
        // (e.g. keyboard navigation) isn't swallowed.
        if self.drag_anchor_row != self.drag_end_row {
            self.suppress_select_row = false;
        }
        self.drag_anchor_row = None;
        self.drag_end_row = None;
    }

    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        crate::mtp::spawn_mtp(
            cx,
            crate::update_check::check_for_update(),
            |this, info, cx| {
                if info.is_some() {
                    this.update_info = info;
                    cx.notify();
                }
            },
        );
    }

    pub(super) fn set_status(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.status = Some(msg.into());
        cx.notify();
    }

    fn navigate_to(&mut self, idx: usize, cx: &mut Context<Self>) {
        let select = self.session.as_mut().and_then(|s| s.truncate_to(idx));
        if select.is_some() {
            self.load_current_folder(select, cx);
        }
    }

    fn navigate_back(&mut self, cx: &mut Context<Self>) {
        let select = self.session.as_mut().and_then(|s| s.pop());
        if select.is_some() {
            self.load_current_folder(select, cx);
        }
    }

    pub(super) fn single_selected_folder(
        &self,
        cx: &App,
    ) -> Option<(ObjectHandle, SharedString)> {
        if self.selected_rows.len() != 1 {
            return None;
        }
        let row_ix = *self.selected_rows.iter().next()?;
        let (handle, name, is_folder) = self.row_entry(row_ix, cx)?;
        is_folder.then_some((handle, name))
    }

    pub(super) fn selected_entries(&self, cx: &App) -> Vec<(ObjectHandle, SharedString, bool)> {
        self.selected_rows
            .iter()
            .filter_map(|&ix| self.row_entry(ix, cx))
            .collect()
    }

    fn on_table_event(
        &mut self,
        _: &Entity<TableState<FolderDelegate>>,
        event: &TableEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            TableEvent::SelectRow(row_ix) => {
                if self.suppress_select_row {
                    self.suppress_select_row = false;
                    if !self.selected_rows.contains(row_ix) {
                        if let Some(next_focus) = self.selected_rows.iter().next().copied() {
                            self.sync_table_focus_to(next_focus, cx);
                            self.push_selection_to_delegate(cx);
                        } else {
                            self.clear_selection(cx);
                        }
                        return;
                    }
                    // The table just updated its `selected_row` (the focus
                    // overlay target). Mirror it into the delegate so render_tr
                    // skips our own overlay on the new focus row.
                    self.push_selection_to_delegate(cx);
                    return;
                }
                let actual_count = self.table.read(cx).delegate().rows.len();
                if *row_ix >= actual_count {
                    self.clear_selection(cx);
                } else {
                    // Keyboard / programmatic path: replace selection with this row.
                    self.replace_selection(*row_ix, cx);
                }
            }
            TableEvent::DoubleClickedRow(row_ix) => {
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
                        session.push_folder(name, handle);
                    }
                    self.clear_selection(cx);
                    self.load_current_folder(None, cx);
                }
            }
            TableEvent::RightClickedRow(Some(row_ix)) => {
                let row_ix = *row_ix;
                let actual_count = self.table.read(cx).delegate().rows.len();
                if row_ix < actual_count && !self.selected_rows.contains(&row_ix) {
                    self.replace_selection(row_ix, cx);
                }
            }
            TableEvent::ClearSelection
                if !self.selected_rows.is_empty() || self.anchor_row.is_some() =>
            {
                self.selected_rows.clear();
                self.anchor_row = None;
                self.push_selection_to_delegate(cx);
                cx.notify();
            }
            _ => {}
        }
    }
}

fn sorted_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}

impl Render for MtpBrowser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        v_flex()
            .size_full()
            .on_action(cx.listener(Self::on_context_import_here))
            .on_action(cx.listener(Self::on_context_import_current))
            .on_action(cx.listener(Self::on_context_export))
            .on_action(cx.listener(Self::on_context_delete))
            .on_action(cx.listener(Self::on_context_new_folder))
            .child(TitleBar::new())
            .child(
                h_flex()
                    .w_full()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_sidebar(window, cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .child(self.render_toolbar(cx))
                            .child(self.render_table(cx))
                            .child(self.render_status_bar(cx)),
                    ),
            )
            .children(dialog_layer)
            .children(notification_layer)
    }
}
