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
        let is_dark = cx.theme().mode.is_dark();
        let theme_icon = if is_dark {
            IconName::Sun
        } else {
            IconName::Moon
        };
        let theme_tooltip = if is_dark {
            "Switch to light theme"
        } else {
            "Switch to dark theme"
        };

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
            .bg(cx.theme().background)
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
                            .tooltip("Back")
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
                            .tooltip("Import from computer")
                            .on_click(cx.listener(Self::on_import)),
                    )
                    .child(
                        tool_btn("export", IconName::ArrowDown)
                            .disabled(!has_selection)
                            .tooltip("Export to computer")
                            .on_click(cx.listener(Self::on_export)),
                    )
                    .child(
                        tool_btn("new-folder", IconName::Plus)
                            .disabled(!has_session)
                            .tooltip("New folder")
                            .on_click(cx.listener(Self::on_new_folder)),
                    )
                    .child(
                        tool_btn("trash", IconName::Delete)
                            .disabled(!has_selection)
                            .tooltip("Delete")
                            .on_click(cx.listener(Self::on_trash)),
                    )
                    .child(div().w(px(8.)))
                    .child(
                        tool_btn("theme-toggle", theme_icon)
                            .tooltip(theme_tooltip)
                            .on_click(cx.listener(move |_, _, window, cx| {
                                let next = if is_dark {
                                    ThemeMode::Light
                                } else {
                                    ThemeMode::Dark
                                };
                                if cx.global::<crate::ThemeAutoFollow>().0 {
                                    cx.set_global(crate::ThemeAutoFollow(false));
                                }
                                Theme::change(next, Some(window), cx);
                            })),
                    ),
            )
            .into_any_element()
    }
}
