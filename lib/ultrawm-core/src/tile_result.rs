use crate::window::WindowRef;

pub enum InsertResult {
    None,
    Swap(WindowRef)
}