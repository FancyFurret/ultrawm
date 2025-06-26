use crate::platform::WMEvent;
use std::collections::VecDeque;
use tokio::sync::mpsc;

pub struct EventBridge {
    sender: mpsc::UnboundedSender<WMEvent>,
    receiver: mpsc::UnboundedReceiver<WMEvent>,
    pending_events: VecDeque<WMEvent>,
}

impl EventBridge {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            pending_events: VecDeque::new(),
        }
    }

    pub fn dispatcher(&self) -> EventDispatcher {
        EventDispatcher::new(self.sender.clone())
    }

    pub async fn next_event(&mut self) -> Option<WMEvent> {
        // If we have buffered events from a previous call, return the first one.
        if let Some(event) = self.pending_events.pop_front() {
            return Some(event);
        }

        // Await the next event. If the channel is closed, return None.
        let mut event = self.receiver.recv().await?;

        // Coalesce all MouseMoved events so that only the most recent one is processed.
        if matches!(event, WMEvent::MouseMoved(_)) {
            // Drain all available events from the receiver
            loop {
                match self.receiver.try_recv() {
                    Ok(WMEvent::MouseMoved(pos)) => {
                        // Keep the newest mouse position
                        event = WMEvent::MouseMoved(pos);
                    }
                    Ok(other_event) => {
                        // Save non-mouse events to the queue
                        self.pending_events.push_back(other_event);
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

#[derive(Clone, Debug)]
pub struct EventDispatcher {
    sender: mpsc::UnboundedSender<WMEvent>,
}

impl EventDispatcher {
    pub fn new(sender: mpsc::UnboundedSender<WMEvent>) -> Self {
        Self { sender }
    }

    pub fn send(&self, event: WMEvent) {
        // If send fails, then the WM is shutting down.
        let _ = self.sender.send(event);
    }
}
