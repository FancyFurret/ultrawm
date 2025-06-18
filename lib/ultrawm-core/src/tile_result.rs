use crate::window::WindowRef;

#[derive(Debug)]
pub enum InsertResult {
    None,
    Swap(WindowRef),
}
