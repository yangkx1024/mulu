mod file_browser;

use file_browser::FileBrowser;
use gpui::*;
use gpui_component::*;
use gpui_component_assets::Assets;

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
                        ..Default::default()
                    },
                    |window, cx| {
                        let view = cx.new(|cx| FileBrowser::new(window, cx));
                        cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
                    },
                )
                .expect("Failed to open window");
            })
            .detach();
        });
}
