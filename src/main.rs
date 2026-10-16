#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod config;
mod pages;
mod schema;
mod server;
mod shell;
mod theme;
mod ui;

use gpui::{px, size, App, AppContext as _, Application, Bounds, KeyBinding, WindowBounds, WindowOptions};
use gpui_component::{Root, TitleBar};

fn main() {
    let application = Application::new().with_assets(assets::Assets);

    application.run(|cx: &mut App| {
        gpui_component::init(cx);
        theme::apply(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.bind_keys([KeyBinding::new("ctrl-s", app::SaveConfig, None)]);

        let bounds = Bounds::centered(None, size(px(1320.), px(860.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(size(px(1000.), px(640.))),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| app::Manager::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("could not open the main window");
        cx.activate(true);
    });
}
