use crate::layouts::WindowLayout;
use crate::platform::PlatformResult;
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
    pub fn new(layout: Box<dyn WindowLayout>, name: String) -> Self {
        let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
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

    pub fn flush(&self) -> PlatformResult<()> {
        let dirty_windows = self.layout.iter().filter(|w| w.dirty());
        for window in dirty_windows {
            window.flush()?;
        }

        Ok(())
    }
}
