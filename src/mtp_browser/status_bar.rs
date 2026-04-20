use gpui::*;
use gpui_component::*;
use rust_i18n::t;

use super::MtpBrowser;

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
            .into_any_element()
    }
}
