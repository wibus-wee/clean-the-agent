//! Retained vector icons and path-level motion for the desktop interface.

use std::sync::OnceLock;
use std::time::Duration;

use quickgui::{Color, Element, Image, Svg, Transition, TransitionProperties, div, img, svg};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconName {
    CircleAlert,
    CircleCheck,
    FileBraces,
    Info,
    LayoutDashboard,
    LoaderCircle,
    Power,
    RotateCcw,
    ShieldCheck,
}

pub fn icon(name: IconName, size: f32, color: Color) -> Element {
    svg(source(name))
        .w(size)
        .h(size)
        .flex_none()
        .text_color(color)
}

pub fn chatgpt_icon(size: f32, dark: bool) -> Element {
    static BLACK: OnceLock<Svg> = OnceLock::new();
    static WHITE: OnceLock<Svg> = OnceLock::new();
    let source = if dark {
        WHITE.get_or_init(|| {
            Svg::from_bytes(include_bytes!(
                "../resources/providers/openai-blossom-white.svg"
            ))
            .expect("valid bundled OpenAI Blossom SVG")
        })
    } else {
        BLACK.get_or_init(|| {
            Svg::from_bytes(include_bytes!(
                "../resources/providers/openai-blossom-black.svg"
            ))
            .expect("valid bundled OpenAI Blossom SVG")
        })
    };
    svg(source.clone()).w(size).h(size).flex_none()
}

/// Resolve a native SF Symbol and fall back to the vendored Lucide equivalent.
pub fn system_info_icon(size: f32, color: Color, dark: bool) -> Element {
    static SYSTEM_INFO: OnceLock<Option<Image>> = OnceLock::new();
    let system =
        SYSTEM_INFO.get_or_init(|| Image::named_system_sized("info.circle", size, 2.0).ok());
    system.as_ref().map_or_else(
        || icon(IconName::Info, size, color),
        |image| {
            img(image.clone())
                .w(size)
                .h(size)
                .flex_none()
                .grayscale(true)
                .when(dark, |glyph| glyph.invert(1.0))
        },
    )
}

/// The slash and key marks are separate SVG layers so feedback targets the glyph's parts.
pub fn animated_keyboard_icon(size: f32, blocked: bool, color: Color) -> Element {
    let motion = Transition::new(Duration::from_millis(180))
        .with_properties(TransitionProperties::TRANSFORM | TransitionProperties::OPACITY)
        .with_easing(quickgui::ease_out_quint());

    div()
        .relative()
        .w(size)
        .h(size)
        .flex_none()
        .text_color(color)
        .child(icon_layer(keyboard_frame(), motion.clone()))
        .child(
            icon_layer(keyboard_keys_left(), motion.clone())
                .group_hover(|style| style.translate(1.0, 0.0)),
        )
        .child(
            icon_layer(keyboard_keys_right(), motion.clone())
                .group_hover(|style| style.translate(-1.0, 0.0)),
        )
        .when(blocked, |glyph| {
            glyph.child(
                icon_layer(keyboard_slash(), motion)
                    .group_hover(|style| style.translate(1.0, -1.0)),
            )
        })
}

fn icon_layer(source: Svg, transition: Transition) -> Element {
    svg(source)
        .absolute()
        .inset_0()
        .w_full()
        .h_full()
        .transition(transition)
}

fn source(name: IconName) -> Svg {
    static CIRCLE_ALERT: OnceLock<Svg> = OnceLock::new();
    static CIRCLE_CHECK: OnceLock<Svg> = OnceLock::new();
    static FILE_BRACES: OnceLock<Svg> = OnceLock::new();
    static INFO: OnceLock<Svg> = OnceLock::new();
    static LAYOUT_DASHBOARD: OnceLock<Svg> = OnceLock::new();
    static LOADER_CIRCLE: OnceLock<Svg> = OnceLock::new();
    static POWER: OnceLock<Svg> = OnceLock::new();
    static ROTATE_CCW: OnceLock<Svg> = OnceLock::new();
    static SHIELD_CHECK: OnceLock<Svg> = OnceLock::new();

    let (slot, bytes): (&OnceLock<Svg>, &[u8]) = match name {
        IconName::CircleAlert => (
            &CIRCLE_ALERT,
            include_bytes!("../resources/icons/lucide/circle-alert.svg"),
        ),
        IconName::CircleCheck => (
            &CIRCLE_CHECK,
            include_bytes!("../resources/icons/lucide/circle-check.svg"),
        ),
        IconName::FileBraces => (
            &FILE_BRACES,
            include_bytes!("../resources/icons/lucide/file-braces.svg"),
        ),
        IconName::Info => (&INFO, include_bytes!("../resources/icons/lucide/info.svg")),
        IconName::LayoutDashboard => (
            &LAYOUT_DASHBOARD,
            include_bytes!("../resources/icons/lucide/layout-dashboard.svg"),
        ),
        IconName::LoaderCircle => (
            &LOADER_CIRCLE,
            include_bytes!("../resources/icons/lucide/loader-circle.svg"),
        ),
        IconName::Power => (
            &POWER,
            include_bytes!("../resources/icons/lucide/power.svg"),
        ),
        IconName::RotateCcw => (
            &ROTATE_CCW,
            include_bytes!("../resources/icons/lucide/rotate-ccw.svg"),
        ),
        IconName::ShieldCheck => (
            &SHIELD_CHECK,
            include_bytes!("../resources/icons/lucide/shield-check.svg"),
        ),
    };
    slot.get_or_init(|| Svg::from_bytes(bytes).expect("valid bundled Lucide SVG"))
        .clone()
}

fn layered(slot: &'static OnceLock<Svg>, markup: &'static str) -> Svg {
    slot.get_or_init(|| Svg::from_svg(markup).expect("valid layered icon SVG"))
        .clone()
}

fn keyboard_frame() -> Svg {
    static SVG: OnceLock<Svg> = OnceLock::new();
    layered(
        &SVG,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z"/><path d="M7 16h10"/></svg>"#,
    )
}

fn keyboard_keys_left() -> Svg {
    static SVG: OnceLock<Svg> = OnceLock::new();
    layered(
        &SVG,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 8h.01M8 12h.01"/></svg>"#,
    )
}

fn keyboard_keys_right() -> Svg {
    static SVG: OnceLock<Svg> = OnceLock::new();
    layered(
        &SVG,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M14 8h.01M18 8h.01M14 12h.01M18 12h.01"/></svg>"#,
    )
}

fn keyboard_slash() -> Svg {
    static SVG: OnceLock<Svg> = OnceLock::new();
    layered(
        &SVG,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="m2 2 20 20"/></svg>"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_icons_share_the_twenty_four_point_grid() {
        for name in [
            IconName::CircleAlert,
            IconName::CircleCheck,
            IconName::FileBraces,
            IconName::Info,
            IconName::LayoutDashboard,
            IconName::LoaderCircle,
            IconName::Power,
            IconName::RotateCcw,
            IconName::ShieldCheck,
        ] {
            let svg = source(name);
            assert_eq!(svg.width(), 24.0);
            assert_eq!(svg.height(), 24.0);
        }
    }
}
