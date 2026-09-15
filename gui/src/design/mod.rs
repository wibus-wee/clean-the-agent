//! Design tokens, semantic colors, and reusable interface components.

mod components;

pub use components::*;
use quickgui::{Color, ColorScheme, SystemColorRole, SystemPreferences, ViewContext};

pub const SIDEBAR_WIDTH: f32 = 232.0;
pub const TOOLBAR_HEIGHT: f32 = 64.0;
pub const CONTROL_HEIGHT: f32 = 36.0;
pub const RADIUS: f32 = 10.0;
pub const PANEL_RADIUS: f32 = 16.0;

#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub window: Color,
    pub sidebar: Color,
    pub control: Color,
    pub control_hover: Color,
    pub text: Color,
    pub secondary_text: Color,
    pub separator: Color,
    pub accent: Color,
    pub accent_text: Color,
    pub selection: Color,
    pub selection_text: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
}

impl Theme {
    pub fn from_context<V: 'static>(cx: &mut ViewContext<'_, V>) -> Self {
        Self::from_preferences(cx.system_preferences())
    }

    fn from_preferences(preferences: SystemPreferences) -> Self {
        let semantic = |role, fallback| {
            preferences.system_color(role).map_or(fallback, |color| {
                Color::rgba8(color.red, color.green, color.blue, color.alpha)
            })
        };
        match preferences.color_scheme() {
            ColorScheme::Dark => Self {
                dark: true,
                window: semantic(SystemColorRole::WindowBackground, Color::rgb8(30, 30, 30)),
                sidebar: semantic(SystemColorRole::ControlBackground, Color::rgb8(36, 36, 38)),
                control: Color::rgb8(39, 39, 39),
                control_hover: Color::rgb8(46, 46, 46),
                text: semantic(SystemColorRole::WindowText, Color::rgb8(245, 245, 247)),
                secondary_text: Color::rgb8(154, 154, 154),
                separator: Color::rgb8(52, 52, 52),
                accent: semantic(SystemColorRole::WindowText, Color::rgb8(245, 245, 247)),
                accent_text: semantic(SystemColorRole::WindowBackground, Color::rgb8(30, 30, 30)),
                selection: Color::rgb8(70, 70, 70),
                selection_text: semantic(SystemColorRole::WindowText, Color::rgb8(245, 245, 247)),
                danger: Color::rgb8(255, 97, 102),
                warning: Color::rgb8(245, 166, 35),
                success: Color::rgb8(70, 167, 88),
            },
            ColorScheme::Light | ColorScheme::Unknown => Self {
                dark: false,
                window: Color::rgb8(255, 255, 255),
                sidebar: Color::rgb8(250, 250, 250),
                control: Color::rgb8(255, 255, 255),
                control_hover: Color::rgb8(245, 245, 245),
                text: Color::rgb8(23, 23, 23),
                secondary_text: Color::rgb8(102, 102, 102),
                separator: Color::rgb8(234, 234, 234),
                accent: Color::rgb8(23, 23, 23),
                accent_text: Color::rgb8(255, 255, 255),
                selection: Color::rgb8(235, 235, 235),
                selection_text: Color::rgb8(23, 23, 23),
                danger: Color::rgb8(238, 0, 0),
                warning: Color::rgb8(173, 92, 0),
                success: Color::rgb8(0, 112, 51),
            },
        }
    }
}
