use crate::config::ConfigRef;
use crate::layouts::WindowLayout;
use crate::platform::Bounds;
use crate::window::Window;
use std::sync::atomic::{AtomicUsize, Ordering};

pub type WorkspaceId = usize;

#[derive(Debug)]
pub struct Workspace {
    id: WorkspaceId,
    name: String,
    layout: Box<dyn WindowLayout>,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

impl Workspace {
    pub fn new<TLayout: WindowLayout + 'static>(
        config: ConfigRef,
        bounds: Bounds,
        windows: Vec<Window>,
        name: String,
    ) -> Self {
        let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let layout = Box::new(TLayout::new(config, bounds, windows));
        Self { id, name, layout }
    }

    pub fn id(&self) -> WorkspaceId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn layout(&self) -> &dyn WindowLayout {
        &*self.layout
    }

    pub fn layout_mut(&mut self) -> &mut dyn WindowLayout {
        &mut *self.layout
    }
}
