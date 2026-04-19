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
        let ix = self.selected_row?;
        let row = self.table.read(cx).delegate().rows.get(ix)?;
        Some((row.handle, row.name.clone(), row.is_folder))
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
                self.selected_row = Some(*row_ix);
                cx.notify();
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

        div()
            .size_full()
            .child(
                h_flex()
                    .size_full()
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
