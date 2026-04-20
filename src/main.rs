mod format;
mod model;
mod mtp;
mod mtp_browser;

use gpui::*;
use gpui_component::*;
use gpui_component_assets::Assets;
use mtp_browser::MtpBrowser;

pub struct ThemeAutoFollow(pub bool);
impl Global for ThemeAutoFollow {}

fn main() {
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            gpui_component::init(cx);

            cx.spawn(async move |cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds {
                            origin: point(px(100.), px(100.)),
                            size: size(px(940.), px(620.)),
                        })),
                        titlebar: Some(TitleBar::title_bar_options()),
                        ..Default::default()
                    },
                    |window, cx| {
                        Theme::sync_system_appearance(Some(window), cx);
                        cx.set_global(ThemeAutoFollow(true));
                        window
                            .observe_window_appearance(|window, cx| {
                                if !cx.global::<ThemeAutoFollow>().0 {
                                    return;
                                }
                                let new_mode: ThemeMode = window.appearance().into();
                                if cx.global::<Theme>().mode != new_mode {
                                    Theme::sync_system_appearance(Some(window), cx);
                                }
                            })
                            .detach();
                        let view = cx.new(|cx| MtpBrowser::new(window, cx));
                        cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
                    },
                )
                .expect("Failed to open window");
            })
            .detach();
        });
}
