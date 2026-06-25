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

/// Alpha applied to the app's surface colors so the blurred window background
/// (`WindowBackgroundAppearance::Blurred`) shows through. Lower = more
/// see-through / stronger blur; raise toward 1.0 for a more solid look.
pub const WINDOW_TINT_ALPHA: f32 = 0.8;

/// Translucent fill for the `background`-colored chrome regions (toolbar,
/// status bar). `background` is shared with dialogs/popovers — which must stay
/// opaque — so it is tinted per element here rather than in [`tint_theme_for_blur`].
pub fn chrome_bg(cx: &App) -> Hsla {
    cx.theme().background.opacity(WINDOW_TINT_ALPHA)
}

/// Make the app's surface colors translucent so the blurred window background
/// is visible behind them. Must run after every theme (re)sync, because a sync
/// resets the theme colors back to their opaque base values — [`sync_and_tint`]
/// keeps that pairing in one place.
///
/// Only surfaces that aren't reused by dialogs/popovers are tinted, so modal
/// content stays fully opaque and readable. Note `gpui_component` paints its
/// own widgets from `theme.tokens` while this app's views read `theme.colors`
/// (the `Theme` `Deref` target), so each surface must be tinted on the field
/// its painter actually reads:
/// - `tokens.title_bar` — `gpui_component::TitleBar`
/// - `tokens.table` / `table_even` / `table_head` — `gpui_component::DataTable`
/// - `colors.sidebar` — this app's own sidebar panel (`mtp_browser::sidebar`)
///
/// Revisit on a `gpui_component` upgrade if a widget starts painting another
/// token (e.g. the table gaining a new row color).
pub fn tint_theme_for_blur(cx: &mut App) {
    let theme = cx.global_mut::<Theme>();
    // `ThemeToken` derefs to its `Hsla`, so `.opacity()` reads the color and
    // `.into()` rebuilds the token (including the `Background` that `.bg()`
    // actually paints) from the now-translucent color.
    theme.tokens.title_bar = theme.tokens.title_bar.opacity(WINDOW_TINT_ALPHA).into();
    theme.tokens.table = theme.tokens.table.opacity(WINDOW_TINT_ALPHA).into();
    theme.tokens.table_even = theme.tokens.table_even.opacity(WINDOW_TINT_ALPHA).into();
    theme.tokens.table_head = theme.tokens.table_head.opacity(WINDOW_TINT_ALPHA).into();
    theme.colors.sidebar = theme.colors.sidebar.opacity(WINDOW_TINT_ALPHA);
}

/// Re-detect the system light/dark appearance and immediately re-apply the blur
/// tint. The tint must follow every sync (a sync resets colors to opaque), so
/// the two are paired here to keep that invariant in one place.
fn sync_and_tint(window: &mut Window, cx: &mut App) {
    Theme::sync_system_appearance(Some(window), cx);
    tint_theme_for_blur(cx);
}

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
                        window_background: WindowBackgroundAppearance::Blurred,
                        ..Default::default()
                    },
                    |window, cx| {
                        sync_and_tint(window, cx);
                        cx.set_global(ThemeAutoFollow(true));
                        window
                            .observe_window_appearance(|window, cx| {
                                if !cx.global::<ThemeAutoFollow>().0 {
                                    return;
                                }
                                let new_mode: ThemeMode = window.appearance().into();
                                if cx.global::<Theme>().mode != new_mode {
                                    sync_and_tint(window, cx);
                                }
                            })
                            .detach();
                        let view = cx.new(|cx| MtpBrowser::new(window, cx));
                        // Fully transparent root: every surface above paints its
                        // own translucent tint, so the blurred window background
                        // shows through uniformly with no doubled-up layer.
                        cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().transparent))
                    },
                )
                .expect("Failed to open window");
            })
            .detach();
        });
}
