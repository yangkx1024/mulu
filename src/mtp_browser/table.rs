use gpui::*;
use gpui_component::table::{Column, ColumnSort, DataTable, TableDelegate, TableState};
use gpui_component::*;

use super::MtpBrowser;
use crate::mtp::FileEntry;

const COL_NAME: &str = "name";
const COL_MODIFIED: &str = "modified";
const COL_SIZE: &str = "size";
const COL_KIND: &str = "kind";

pub struct FolderDelegate {
    pub rows: Vec<FileEntry>,
    columns: Vec<Column>,
}

impl FolderDelegate {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            columns: vec![
                Column::new(COL_NAME, "Name").width(px(280.)).ascending(),
                Column::new(COL_MODIFIED, "Date Modified")
                    .width(px(200.))
                    .sortable(),
                Column::new(COL_SIZE, "Size").width(px(80.)).sortable(),
                Column::new(COL_KIND, "Kind").width(px(120.)).sortable(),
            ],
        }
    }
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
        self.rows.sort_by(|a, b| {
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

impl MtpBrowser {
    pub(super) fn render_table(&self, _cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_1()
            .overflow_hidden()
            .child(DataTable::new(&self.table).stripe(true).bordered(false))
            .into_any_element()
    }
}
