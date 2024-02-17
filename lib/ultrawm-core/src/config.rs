use std::rc::Rc;

pub type ConfigRef = Rc<Config>;

#[derive(Debug)]
pub struct Config {
    /// The number of pixels between windows
    pub window_gap: u32,
    /// The number of pixels between the edge of the partition and the windows
    pub partition_gap: u32,
    /// Whether to float windows by default when they are created
    pub float_new_windows: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_gap: 20,
            partition_gap: 40,
            float_new_windows: true,
        }
    }
}
