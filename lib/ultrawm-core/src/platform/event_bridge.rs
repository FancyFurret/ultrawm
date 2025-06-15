use crate::platform::PlatformEvent;
use tokio::sync::mpsc;

pub struct EventBridge {
    sender: mpsc::UnboundedSender<PlatformEvent>,
    receiver: mpsc::UnboundedReceiver<PlatformEvent>,
    pending_event: Option<PlatformEvent>,
}

impl EventBridge {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            pending_event: None,
        }
    }

    pub fn dispatcher(&self) -> EventDispatcher {
        EventDispatcher::new(self.sender.clone())
    }

    pub async fn next_event(&mut self) -> Option<PlatformEvent> {
        // If we have a buffered event from a previous call, return it immediately.
        if let Some(event) = self.pending_event.take() {
            return Some(event);
        }

        // Await the next event. If the channel is closed, return None.
        let mut event = self.receiver.recv().await?;

        // Coalesce consecutive MouseMoved events so that only the most recent one is processed.
        if matches!(event, PlatformEvent::MouseMoved(_)) {
            loop {
                match self.receiver.try_recv() {
                    Ok(PlatformEvent::MouseMoved(pos)) => {
                        // Keep the newest mouse position and continue draining.
                        event = PlatformEvent::MouseMoved(pos);
                    }
                    Ok(other_event) => {
                        // Buffer the first non-mouse event so it will be returned on the next call.
                        self.pending_event = Some(other_event);
                        break;
                    }
                    Err(_) => {
                        // No more events available right now.
                        break;
                    }
                }
            }
        }

        Some(event)
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
