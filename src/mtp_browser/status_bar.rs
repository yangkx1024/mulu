use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::*;
use rust_i18n::t;

use super::MtpBrowser;

const VERSION_LABEL: &str = concat!("v", env!("CARGO_PKG_VERSION"));

impl MtpBrowser {
    pub(super) fn render_status_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let status_text = self
            .status
            .clone()
            .unwrap_or_else(|| t!("status.items", count = 0).to_string().into());
        h_flex()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(status_text)
            .child(
                h_flex()
                    .ml_auto()
                    .gap_3()
                    .child(VERSION_LABEL)
                    .when_some(self.update_info.as_ref(), |row, info| {
                        let notice =
                            t!("status.update_available", version = info.version.as_str());
                        let url: SharedString = info.url.clone().into();
                        row.child(
                            div()
                                .id("update-notice")
                                .cursor_pointer()
                                .text_color(cx.theme().primary)
                                .hover(|s| s.underline())
                                .child(SharedString::from(notice))
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.open_url(url.as_ref());
                                })),
                        )
                    }),
            )
            .into_any_element()
    }
}
