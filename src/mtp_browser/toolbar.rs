use gpui::*;
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::button::*;
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::*;

use super::MtpBrowser;

fn tool_btn(id: &'static str, icon: IconName) -> Button {
    Button::new(id)
        .ghost()
        .small()
        .rounded(ButtonRounded::Large)
        .icon(Icon::new(icon).size_4())
}

impl MtpBrowser {
    pub(super) fn render_toolbar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let collapsed = self.collapsed;
        let can_go_back = self.session.as_ref().map_or(false, |s| s.can_go_back());
        let has_session = self.session.is_some();
        let has_selection = self.selected_row.is_some();

        let mut crumb_items: Vec<BreadcrumbItem> = Vec::new();
        if let Some(session) = &self.session {
            let last = session.path.len().saturating_sub(1);
            for (i, crumb) in session.path.iter().enumerate() {
                let crumb_name = crumb.name.clone();
                let item = if i < last {
                    BreadcrumbItem::new(crumb_name)
                        .on_click(cx.listener(move |this, _, _, cx| this.navigate_to(i, cx)))
                } else {
                    BreadcrumbItem::new(crumb_name)
                };
                crumb_items.push(item);
            }
        }

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
                    .child(
                        tool_btn("back", IconName::ChevronLeft)
                            .disabled(!can_go_back)
                            .on_click(cx.listener(|this, _, _, cx| this.navigate_back(cx))),
                    )
                    .child(Breadcrumb::new().children(crumb_items)),
            )
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        tool_btn("import", IconName::ArrowUp)
                            .disabled(!has_session)
                            .on_click(cx.listener(Self::on_import)),
                    )
                    .child(
                        tool_btn("export", IconName::ArrowDown)
                            .disabled(!has_selection)
                            .on_click(cx.listener(Self::on_export)),
                    )
                    .child(
                        tool_btn("new-folder", IconName::Plus)
                            .disabled(!has_session)
                            .on_click(cx.listener(Self::on_new_folder)),
                    )
                    .child(
                        tool_btn("trash", IconName::Delete)
                            .disabled(!has_selection)
                            .on_click(cx.listener(Self::on_trash)),
                    )
                    .child(div().w(px(8.)))
                    .child(tool_btn("info", IconName::Info)),
            )
            .into_any_element()
    }
}
