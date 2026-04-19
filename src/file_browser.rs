use std::time::Duration;

use gpui::*;
use gpui_component::animation::{Transition, ease_in_out_cubic};
use gpui_component::button::*;
use gpui_component::progress::Progress;
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::table::{Column, ColumnSort, DataTable, TableDelegate, TableState};
use gpui_component::*;

const COL_NAME: &str = "name";
const COL_MODIFIED: &str = "modified";
const COL_SIZE: &str = "size";
const COL_KIND: &str = "kind";

struct FolderRow {
    name: SharedString,
    modified: SharedString,
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
            COL_NAME => h_flex()
                .gap_2()
                .items_center()
                .child(Icon::new(IconName::Folder).text_color(cx.theme().blue))
                .child(row.name.clone())
                .into_any_element(),
            COL_MODIFIED => row.modified.clone().into_any_element(),
            COL_SIZE => "—".to_string().into_any_element(),
            COL_KIND => "Folder".to_string().into_any_element(),
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
        match sort {
            ColumnSort::Ascending => match self.columns[col_ix].key.as_ref() {
                COL_NAME => self.rows.sort_by(|a, b| a.name.cmp(&b.name)),
                COL_MODIFIED => self.rows.sort_by(|a, b| a.modified.cmp(&b.modified)),
                _ => {}
            },
            ColumnSort::Descending => match self.columns[col_ix].key.as_ref() {
                COL_NAME => self.rows.sort_by(|a, b| b.name.cmp(&a.name)),
                COL_MODIFIED => self.rows.sort_by(|a, b| b.modified.cmp(&a.modified)),
                _ => {}
            },
            ColumnSort::Default => self.rows.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }
}

pub struct FileBrowser {
    table: Entity<TableState<FolderDelegate>>,
    collapsed: bool,
}

impl FileBrowser {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let names = [
            ("Alarms", "May 2, 2023 at 05:44"),
            ("Android", "May 2, 2023 at 05:44"),
            ("Audiobooks", "May 2, 2023 at 05:44"),
            ("DCIM", "May 2, 2023 at 05:44"),
            ("Documents", "May 2, 2023 at 05:44"),
            ("Download", "Mar 28, 2026 at 00:44"),
            ("Movies", "May 2, 2023 at 05:44"),
            ("Music", "May 2, 2023 at 05:44"),
            ("Notifications", "May 2, 2023 at 05:44"),
            ("Pictures", "May 2, 2023 at 05:49"),
            ("Podcasts", "May 2, 2023 at 05:44"),
            ("Recordings", "May 2, 2023 at 05:44"),
            ("Ringtones", "May 2, 2023 at 05:44"),
        ];
        let rows = names
            .iter()
            .map(|(name, modified)| FolderRow {
                name: SharedString::from(*name),
                modified: SharedString::from(*modified),
            })
            .collect();

        let columns = vec![
            Column::new(COL_NAME, "Name").width(px(280.)).ascending(),
            Column::new(COL_MODIFIED, "Date Modified")
                .width(px(200.))
                .sortable(),
            Column::new(COL_SIZE, "Size").width(px(80.)).sortable(),
            Column::new(COL_KIND, "Kind").width(px(120.)).sortable(),
        ];

        let delegate = FolderDelegate { rows, columns };
        let table = cx.new(|cx| TableState::new(delegate, window, cx));
        Self {
            table,
            collapsed: false,
        }
    }
}

fn tool_btn(id: &'static str, icon: IconName) -> Button {
    Button::new(id)
        .ghost()
        .small()
        .rounded(ButtonRounded::Large)
        .icon(Icon::new(icon).size_4())
}

impl Render for FileBrowser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.collapsed;
        let expanded_w = px(240.);

        // use_keyed_state persists (from, to) across re-renders triggered by the animation itself.
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

        let sidebar_content = v_flex()
            .w(expanded_w)
            .h_full()
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().sidebar)
            .py_2()
            .child(
                div()
                    .px_4()
                    .py_1()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Mi MIX 2S"),
            )
            .child(
                div()
                    .mx_2()
                    .p_2()
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().sidebar_accent)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().sidebar_accent_foreground)
                            .child(Icon::new(IconName::HardDrive).size_4())
                            .child("Internal shared storage"),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .pt_2()
                            .child(
                                Progress::new("storage-progress")
                                    .value(9.56)
                                    .color(cx.theme().blue),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("52.0 GB free of 57.5 GB"),
                            ),
                    ),
            );

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

        h_flex()
            .size_full()
            .child(animated_sidebar)
            .child(
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
                                    .child(
                                        SidebarToggleButton::new()
                                            .collapsed(collapsed)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.collapsed = !this.collapsed;
                                                cx.notify();
                                            })),
                                    )
                                    .child(tool_btn("back", IconName::ChevronLeft))
                                    .child(div().font_bold().child("Device")),
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
                        div().flex_1().overflow_hidden().child(
                            DataTable::new(&self.table).stripe(true).bordered(false),
                        ),
                    )
                    .child(
                        h_flex()
                            .px_4()
                            .py_2()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("13 items"),
                    ),
            )
    }
}
