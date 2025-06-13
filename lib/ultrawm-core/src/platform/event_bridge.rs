use crate::platform::PlatformEvent;
use tokio::sync::mpsc;

pub struct EventBridge {
    sender: mpsc::UnboundedSender<PlatformEvent>,
    receiver: mpsc::UnboundedReceiver<PlatformEvent>,
}

impl EventBridge {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }

    pub fn dispatcher(&self) -> EventDispatcher {
        EventDispatcher::new(self.sender.clone())
    }

    pub async fn next_event(&mut self) -> Option<PlatformEvent> {
        self.receiver.recv().await
    }
}

#[derive(Clone)]
pub struct EventDispatcher {
    sender: mpsc::UnboundedSender<PlatformEvent>,
}

impl EventDispatcher {
    pub fn new(sender: mpsc::UnboundedSender<PlatformEvent>) -> Self {
        Self { sender }
    }

    pub fn send(&self, event: PlatformEvent) {
        // If send fails, then the WM is shutting down.
        let _ = self.sender.send(event);
    }
}
