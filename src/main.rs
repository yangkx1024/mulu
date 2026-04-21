mod format;
mod model;
mod mtp;
mod mtp_browser;
mod update_check;

use gpui::*;
use gpui_component::*;
use gpui_component_assets::Assets;
use mtp_browser::MtpBrowser;

rust_i18n::i18n!("locales", fallback = "en");

pub struct ThemeAutoFollow(pub bool);
impl Global for ThemeAutoFollow {}

pub fn set_app_locale(locale: &str) {
    gpui_component::set_locale(locale);
}

fn detect_system_locale() -> &'static str {
    let raw = sys_locale::get_locale().unwrap_or_default().to_lowercase();
    if raw.starts_with("zh") {
        let traditional = raw
            .split(['-', '_'])
            .any(|s| matches!(s, "hant" | "tw" | "hk" | "mo"));
        if traditional { "zh-HK" } else { "zh-CN" }
    } else if raw.starts_with("ja") {
        "ja"
    } else {
        "en"
    }
}

fn main() {
    set_app_locale(detect_system_locale());

    gpui_platform::application()
        .with_quit_mode(QuitMode::LastWindowClosed)
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
