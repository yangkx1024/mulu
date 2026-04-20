use gpui::*;
use gpui_component::menu::PopupMenu;
use gpui_component::table::{Column, ColumnSort, DataTable, TableDelegate, TableState};
use gpui_component::*;
use rust_i18n::t;

use super::MtpBrowser;
use super::actions::{
    ContextDelete, ContextExport, ContextImportCurrent, ContextImportHere, ContextNewFolder,
};
use crate::format::format_kind;
use crate::mtp::FileEntry;

const COL_NAME: &str = "name";
const COL_MODIFIED: &str = "modified";
const COL_SIZE: &str = "size";
const COL_KIND: &str = "kind";
const PADDING_ROWS: usize = 5;

pub struct FolderDelegate {
    pub rows: Vec<FileEntry>,
    columns: Vec<Column>,
}

impl FolderDelegate {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            columns: localized_columns(),
        }
    }

    pub fn relocalize(&mut self) {
        let current_sorts: Vec<_> = self
            .columns
            .iter()
            .map(|c| (c.key.clone(), c.sort))
            .collect();
        self.columns = localized_columns();
        for (key, sort) in current_sorts {
            if let Some(col) = self.columns.iter_mut().find(|c| c.key == key) {
                col.sort = sort;
            }
        }
        for row in &mut self.rows {
            row.kind = format_kind(&row.name, row.is_folder);
        }
    }

    pub fn sort_default(&mut self) {
        self.sort_rows(COL_NAME, false);
    }

    fn sort_rows(&mut self, key: &str, reverse: bool) {
        self.rows.sort_by(|a, b| {
            let by_kind = b.is_folder.cmp(&a.is_folder);
            if by_kind != std::cmp::Ordering::Equal {
                return by_kind;
            }
            let ord = match key {
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

fn localized_columns() -> Vec<Column> {
    vec![
        Column::new(COL_NAME, t!("table.col.name").to_string())
            .width(px(280.))
            .ascending(),
        Column::new(COL_MODIFIED, t!("table.col.modified").to_string())
            .width(px(200.))
            .sortable(),
        Column::new(COL_SIZE, t!("table.col.size").to_string())
            .width(px(80.))
            .sortable(),
        Column::new(COL_KIND, t!("table.col.kind").to_string())
            .width(px(120.))
            .sortable(),
    ]
}

impl TableDelegate for FolderDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.rows.len() + PADDING_ROWS
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        let (menu, has_real_row) = match self.rows.get(row_ix) {
            Some(row) => {
                let m = if row.is_folder {
                    let label = t!("table.menu.import_into", name = row.name.as_ref()).to_string();
                    menu.menu_with_icon(
                        label,
                        Icon::new(IconName::ArrowUp),
                        Box::new(ContextImportHere { row_ix }),
                    )
                } else {
                    menu.menu_with_icon(
                        t!("table.menu.export").to_string(),
                        Icon::new(IconName::ArrowDown),
                        Box::new(ContextExport { row_ix }),
                    )
                };
                (m, true)
            }
            None => (menu, false),
        };
        let menu = menu
            .menu_with_icon(
                t!("table.menu.import_current").to_string(),
                Icon::new(IconName::ArrowUp),
                Box::new(ContextImportCurrent),
            )
            .separator()
            .menu_with_icon(
                t!("table.menu.new_folder").to_string(),
                Icon::new(IconName::Plus),
                Box::new(ContextNewFolder),
            );
        if has_real_row {
            menu.separator().menu_with_icon(
                t!("table.menu.delete").to_string(),
                Icon::new(IconName::Delete),
                Box::new(ContextDelete { row_ix }),
            )
        } else {
            menu
        }
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        if row_ix >= self.rows.len() {
            return div().into_any_element();
        }
        let row = &self.rows[row_ix];
        let cell = |s: SharedString| -> AnyElement {
            h_flex().h_full().items_center().child(s).into_any_element()
        };
        match self.columns[col_ix].key.as_ref() {
            COL_NAME => {
                let icon = if row.is_folder {
                    Icon::new(IconName::Folder).text_color(cx.theme().blue)
                } else {
                    Icon::new(IconName::File).text_color(cx.theme().muted_foreground)
                };
                h_flex()
                    .h_full()
                    .gap_2()
                    .items_center()
                    .child(icon)
                    .child(row.name.clone())
                    .into_any_element()
            }
            COL_MODIFIED => cell(row.modified.clone()),
            COL_SIZE => cell(row.size.clone()),
            COL_KIND => cell(row.kind.clone()),
            _ => SharedString::default().into_any_element(),
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
        self.sort_rows(&key, reverse);
    }
}

impl MtpBrowser {
    pub(super) fn render_table(&self, _cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_1()
            .overflow_hidden()
            .child(DataTable::new(&self.table).stripe(true).bordered(false))
            .into_any_element()
    }

    pub(super) fn relocalize_table(&mut self, cx: &mut Context<Self>) {
        self.table.update(cx, |state, cx| {
            state.delegate_mut().relocalize();
            cx.notify();
        });
    }
}
