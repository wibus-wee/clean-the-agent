//! Shared visual primitives for Clean the Agent's Geist-inspired macOS interface.

use std::time::Duration;

use quickgui::{
    ClickListener, Color, ColorScheme, Element, Image, IntoElement, SystemColorRole,
    SystemPreferences, Transform2D, Transition, TransitionProperties, ViewContext, button,
    checkbox, div, img, text,
};

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

pub fn sidebar() -> Element {
    div()
        .id("app-sidebar")
        .w(SIDEBAR_WIDTH)
        .h_full()
        .min_h(0.0)
        .flex_none()
        .overflow_hidden()
        .flex_col()
}

pub fn toolbar() -> Element {
    div()
        .h(TOOLBAR_HEIGHT)
        .px(20.0)
        .flex()
        .items_center()
        .app_region_drag()
}

pub fn panel(theme: Theme) -> Element {
    div()
        .w_full()
        .rounded(PANEL_RADIUS)
        .border(1.0, theme.separator)
        .bg(theme.control)
}

pub fn sidebar_icon_row<V>(
    label: &str,
    icon: Element,
    selected: bool,
    listener: ClickListener<V>,
    theme: Theme,
) -> Element {
    button()
        .group()
        .w_full()
        .h(42.0)
        .px_3()
        .rounded(RADIUS)
        .flex()
        .items_center()
        .gap_2()
        .font_medium()
        .text_size(14.0)
        .cursor_pointer()
        .when(selected, |row| {
            row.bg(theme.selection).text_color(theme.selection_text)
        })
        .when(!selected, |row| {
            row.hover(|style| style.bg(theme.control_hover))
        })
        .on_click(listener)
        .child(icon)
        .child(label)
}

pub fn provider_sidebar_row<V>(
    label: &str,
    icon: Image,
    selected: bool,
    listener: ClickListener<V>,
    theme: Theme,
) -> Element {
    button()
        .w_full()
        .h(42.0)
        .px_3()
        .rounded(RADIUS)
        .flex()
        .items_center()
        .gap_2()
        .font_medium()
        .text_size(14.0)
        .cursor_pointer()
        .when(selected, |row| {
            row.bg(theme.selection).text_color(theme.selection_text)
        })
        .when(!selected, |row| {
            row.hover(|style| style.bg(theme.control_hover))
        })
        .on_click(listener)
        .child(provider_icon(icon, 22.0, 7.0))
        .child(label)
}

pub fn provider_icon(icon: Image, size: f32, radius: f32) -> Element {
    img(icon).w(size).h(size).rounded(radius)
}

pub fn setting_switch<V>(
    on: bool,
    label: &str,
    listener: ClickListener<V>,
    disabled: bool,
    theme: Theme,
) -> Element {
    checkbox(on)
        .relative()
        .flex_none()
        .w(44.0)
        .h(26.0)
        .overflow_hidden()
        .rounded_full()
        .border(1.0, if on { theme.accent } else { theme.separator })
        .bg(if on {
            theme.accent
        } else {
            theme.control_hover
        })
        .accessibility_label(label)
        .disabled(disabled)
        .when(disabled, |control| control.opacity(0.45))
        .when(!disabled, |control| control.cursor_pointer())
        .on_click(listener)
        .child(
            div()
                .absolute()
                .left(3.0)
                .top(3.0)
                .w(18.0)
                .h(18.0)
                .rounded_full()
                .bg(if on {
                    theme.accent_text
                } else {
                    theme.secondary_text
                })
                .transform(Transform2D::translate(if on { 18.0 } else { 0.0 }, 0.0))
                .transition(
                    Transition::new(Duration::from_millis(160))
                        .with_properties(TransitionProperties::TRANSFORM)
                        .with_easing(quickgui::ease_out_quint()),
                ),
        )
}

pub fn toolbar_button<V>(
    label: &str,
    listener: ClickListener<V>,
    disabled: bool,
    theme: Theme,
) -> Element {
    action_button(label, listener, disabled, theme, ActionStyle::Secondary)
}

pub fn primary_button<V>(
    label: &str,
    listener: ClickListener<V>,
    disabled: bool,
    theme: Theme,
) -> Element {
    action_button(label, listener, disabled, theme, ActionStyle::Primary)
}

pub fn destructive_button<V>(
    label: String,
    listener: ClickListener<V>,
    disabled: bool,
    theme: Theme,
) -> Element {
    action_button(label, listener, disabled, theme, ActionStyle::Destructive)
}

enum ActionStyle {
    Primary,
    Secondary,
    Destructive,
}

fn action_button<V>(
    label: impl IntoElement,
    listener: ClickListener<V>,
    disabled: bool,
    theme: Theme,
    style: ActionStyle,
) -> Element {
    let (background, foreground, border) = match style {
        ActionStyle::Primary => (theme.accent, theme.accent_text, theme.accent),
        ActionStyle::Secondary => (theme.control, theme.text, theme.separator),
        ActionStyle::Destructive => (theme.danger, Color::WHITE, theme.danger),
    };

    button()
        .h(CONTROL_HEIGHT)
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded(RADIUS)
        .border(1.0, border)
        .bg(background)
        .text_color(foreground)
        .font_medium()
        .text_size(14.0)
        .line_height(20.0)
        .app_region_no_drag()
        .disabled(disabled)
        .when(disabled, |item| {
            item.bg(theme.control)
                .border(1.0, theme.separator)
                .text_color(theme.secondary_text)
                .opacity(0.62)
        })
        .when(!disabled, |item| {
            item.cursor_pointer().hover(|style| style.opacity(0.78))
        })
        .on_click(listener)
        .child(label)
}

pub fn review_checkbox<V>(
    checked: bool,
    listener: ClickListener<V>,
    disabled: bool,
    theme: Theme,
) -> Element {
    checkbox(checked)
        .flex()
        .items_center()
        .gap_2()
        .text_size(14.0)
        .disabled(disabled)
        .when(disabled, |item| item.opacity(0.45))
        .when(!disabled, |item| item.cursor_pointer())
        .on_click(listener)
        .child(
            div()
                .w(16.0)
                .h(16.0)
                .rounded(4.0)
                .border(
                    1.0,
                    if checked {
                        theme.accent
                    } else {
                        theme.separator
                    },
                )
                .when(checked, |indicator| {
                    indicator
                        .bg(theme.accent)
                        .text_color(theme.accent_text)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .child("✓")
                }),
        )
        .child("Include Items Requiring Review")
}

pub fn status_badge(label: &str, color: Color, theme: Theme) -> Element {
    div()
        .px_2()
        .py_1()
        .rounded_full()
        .border(1.0, mix(theme.control, color, 0.24))
        .bg(mix(theme.control, color, 0.08))
        .text_color(color)
        .text_size(12.0)
        .font_medium()
        .child(label)
}

pub fn category_tile<V>(
    title: &str,
    detail: &str,
    meta: String,
    selected: bool,
    listener: ClickListener<V>,
    theme: Theme,
) -> Element {
    button()
        .min_w(180.0)
        .min_h(148.0)
        .flex_grow(1.0)
        .flex_basis(180.0)
        .p_4()
        .rounded(14.0)
        .border(1.0, theme.separator)
        .bg(if selected {
            theme.sidebar
        } else {
            theme.control
        })
        .flex_col()
        .items_start()
        .justify_between()
        .gap_3()
        .text_left()
        .cursor_pointer()
        .hover(|style| style.bg(theme.control_hover))
        .on_click(listener)
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(text(title).text_size(14.0).font_semibold())
                .child(
                    div()
                        .w(18.0)
                        .h(18.0)
                        .rounded(6.0)
                        .border(
                            1.0,
                            if selected {
                                theme.accent
                            } else {
                                theme.separator
                            },
                        )
                        .when(selected, |indicator| {
                            indicator
                                .bg(theme.accent)
                                .text_color(theme.accent_text)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(11.0)
                                .child("✓")
                        }),
                ),
        )
        .child(
            text(detail)
                .text_size(13.0)
                .line_height(19.0)
                .text_color(theme.secondary_text),
        )
        .child(
            text(meta)
                .text_size(12.0)
                .font_medium()
                .text_color(theme.secondary_text),
        )
}

pub fn detail_group(label: &str, value: &str, theme: Theme) -> Element {
    div()
        .flex_col()
        .gap_1()
        .child(
            text(label)
                .text_size(12.0)
                .font_medium()
                .text_color(theme.secondary_text),
        )
        .child(text(value).text_size(14.0).line_height(20.0))
}

pub fn empty_state(title: &str, detail: &str, theme: Theme) -> Element {
    div()
        .w_full()
        .min_h(320.0)
        .p_6()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .text_center()
        .child(
            div()
                .w(48.0)
                .h(48.0)
                .rounded(15.0)
                .border(1.0, theme.separator)
                .bg(theme.sidebar)
                .flex()
                .items_center()
                .justify_center()
                .text_size(20.0)
                .font_medium()
                .child("✓"),
        )
        .child(text(title).text_size(18.0).font_semibold())
        .child(
            text(detail)
                .text_size(14.0)
                .max_w(420.0)
                .line_height(21.0)
                .text_color(theme.secondary_text),
        )
}

pub fn message_panel(title: &str, detail: &str, color: Color, theme: Theme) -> Element {
    panel(theme)
        .p_3()
        .border_left(3.0, color)
        .flex_col()
        .gap_1()
        .child(text(title).text_size(14.0).font_medium())
        .child(
            text(detail)
                .text_size(14.0)
                .text_color(theme.secondary_text),
        )
}

fn mix(first: Color, second: Color, amount: f32) -> Color {
    let inverse = 1.0 - amount;
    Color::linear(
        first.r * inverse + second.r * amount,
        first.g * inverse + second.g * amount,
        first.b * inverse + second.b * amount,
        1.0,
    )
}
