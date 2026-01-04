use crate::platform::Bounds;
use skia_safe::Color;

#[derive(Debug, Clone)]
pub struct OverlayWindowConfig {
    pub fade_animation_ms: u32,
    pub move_animation_ms: u32,
    pub border_radius: f32,
    pub blur: bool,
    pub background: Option<OverlayWindowBackgroundStyle>,
    pub border: Option<OverlayWindowBorderStyle>,
}

#[derive(Debug, Clone)]
pub struct OverlayWindowBackgroundStyle {
    pub opacity: f32,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct OverlayWindowBorderStyle {
    pub color: Color,
    pub width: u32,
}

#[derive(Debug, Clone)]
pub enum OverlayWindowCommand {
    Show,
    Hide,
    MoveTo(Bounds),
    Exit,
}
