mod actions;
mod sidebar;
mod status_bar;
mod table;
mod toolbar;

use gpui::*;
use gpui_component::table::{TableEvent, TableState};
use gpui_component::*;

use crate::model::Session;
use crate::mtp::{DeviceSummary, ObjectHandle};
use table::FolderDelegate;

pub struct MtpBrowser {
    collapsed: bool,
    table: Entity<TableState<FolderDelegate>>,
    devices: Vec<DeviceSummary>,
    session: Option<Session>,
    status: Option<SharedString>,
    selected_row: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl MtpBrowser {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = FolderDelegate::new();
        let table = cx.new(|cx| TableState::new(delegate, window, cx));
        let subscriptions = vec![cx.subscribe_in(&table, window, Self::on_table_event)];

        let mut this = Self {
            collapsed: false,
            table,
            devices: Vec::new(),
            session: None,
            status: None,
            selected_row: None,
            _subscriptions: subscriptions,
        };
        this.refresh_devices(cx);
        this
    }

    pub(super) fn set_status(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.status = Some(msg.into());
        cx.notify();
    }

    fn navigate_to(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(session) = self.session.as_mut() {
            session.truncate_to(idx);
        }
        self.load_current_folder(cx);
    }

    fn navigate_back(&mut self, cx: &mut Context<Self>) {
        if self.session.as_mut().map_or(false, |s| s.pop()) {
            self.load_current_folder(cx);
        }
    }

    fn selected_row_info(&self, cx: &App) -> Option<(ObjectHandle, SharedString, bool)> {
        self.row_entry(self.selected_row?, cx)
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
                let actual_count = self.table.read(cx).delegate().rows.len();
                if *row_ix >= actual_count {
                    // Always clear the table's internal selection — it sets selected_row
                    // before emitting this event, so the padding row would otherwise render
                    // as highlighted even when our tracked selection was already None.
                    self.table.update(cx, |state, cx| state.clear_selection(cx));
                    if self.selected_row.is_some() {
                        self.selected_row = None;
                        cx.notify();
                    }
                } else if self.selected_row != Some(*row_ix) {
                    self.selected_row = Some(*row_ix);
                    cx.notify();
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
                    self.load_current_folder(cx);
                }
            }
            _ => {}
        }
    }
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
