//! Reusable visual primitives for the application interface.

use std::time::Duration;

use super::{CONTROL_HEIGHT, PANEL_RADIUS, RADIUS, SIDEBAR_WIDTH, TOOLBAR_HEIGHT, Theme};
use quickgui::{
    ClickListener, Color, Element, Image, IntoElement, Transform2D, Transition,
    TransitionProperties, button, checkbox, div, img, text,
};

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
        .flex_none()
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
        .min_w(300.0)
        .min_h(96.0)
        .flex_grow(1.0)
        .flex_basis(340.0)
        .p_3()
        .rounded(12.0)
        .border(1.0, theme.separator)
        .bg(if selected {
            theme.sidebar
        } else {
            theme.control
        })
        .flex_col()
        .items_start()
        .justify_between()
        .gap_2()
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
