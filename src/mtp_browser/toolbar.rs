use gpui::*;
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::button::*;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::*;
use rust_i18n::t;

use super::MtpBrowser;
use crate::set_app_locale;

const LOCALE_ITEMS: &[(&str, &str)] = &[
    ("en", "English"),
    ("zh-CN", "简体中文"),
    ("zh-HK", "繁體中文"),
    ("ja", "日本語"),
];

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
            t!("toolbar.switch_to_light").to_string()
        } else {
            t!("toolbar.switch_to_dark").to_string()
        };
        let language_tooltip = t!("toolbar.switch_language").to_string();

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
                            .tooltip(t!("toolbar.back").to_string())
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
                            .tooltip(t!("toolbar.import").to_string())
                            .on_click(cx.listener(Self::on_import)),
                    )
                    .child(
                        tool_btn("export", IconName::ArrowDown)
                            .disabled(!has_selection)
                            .tooltip(t!("toolbar.export").to_string())
                            .on_click(cx.listener(Self::on_export)),
                    )
                    .child(
                        tool_btn("new-folder", IconName::Plus)
                            .disabled(!has_session)
                            .tooltip(t!("toolbar.new_folder").to_string())
                            .on_click(cx.listener(Self::on_new_folder)),
                    )
                    .child(
                        tool_btn("trash", IconName::Delete)
                            .disabled(!has_selection)
                            .tooltip(t!("toolbar.delete").to_string())
                            .on_click(cx.listener(Self::on_trash)),
                    )
                    .child(div().w(px(8.)))
                    .child({
                        let view = cx.entity();
                        tool_btn("language-toggle", IconName::Globe)
                            .tooltip(language_tooltip)
                            .dropdown_menu(move |mut menu, window, _| {
                                for (code, label) in LOCALE_ITEMS {
                                    menu = menu.item(PopupMenuItem::new(*label).on_click(
                                        window.listener_for(&view, move |this, _, _, cx| {
                                            if &*rust_i18n::locale() == *code {
                                                return;
                                            }
                                            set_app_locale(code);
                                            this.relocalize_table(cx);
                                            this.on_locale_changed(cx);
                                            cx.refresh_windows();
                                        }),
                                    ));
                                }
                                menu
                            })
                    })
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
