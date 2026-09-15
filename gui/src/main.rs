mod app;
mod design;
mod icons;

use app::CleanerApp;
use quickgui::{App, AppInfo, Application, TitleBarStyle, WindowOptions};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .app_info(
            AppInfo::new(
                "Clean the Agent",
                env!("CARGO_PKG_VERSION"),
                "dev.cleantheagent.app",
            )
            .expect("valid application identity"),
        )
        .on_reopen(|has_visible_windows, cx| {
            if !has_visible_windows {
                open_window(cx);
            }
        })
        .run(open_window)
}

fn open_window(cx: &mut App) {
    cx.open_window(
        WindowOptions::new("Clean the Agent")
            .size(1120.0, 740.0)
            .minimum_size(920.0, 620.0)
            .title_bar_style(TitleBarStyle::HiddenInset)
            .traffic_light_position(18.0, 22.0),
        CleanerApp::new(),
    );
}
