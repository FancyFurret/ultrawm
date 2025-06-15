pub trait PlatformImpl {
    fn list_visible_windows() -> PlatformResult<Vec<PlatformWindow>>;
    fn list_all_displays() -> PlatformResult<Vec<Display>>;
    fn get_mouse_position() -> PlatformResult<Position>;
    fn set_cursor(cursor_type: CursorType) -> PlatformResult<()>;
    fn reset_cursor() -> PlatformResult<()>;
}
