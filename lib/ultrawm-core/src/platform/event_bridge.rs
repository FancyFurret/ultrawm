use crate::coalescing_channel::CoalescingAsyncChannel;
use crate::platform::WMEvent;
use tokio::sync::mpsc;

pub struct EventBridge {
    channel: CoalescingAsyncChannel<WMEvent>,
}

impl EventBridge {
    pub fn new() -> Self {
        Self {
            channel: CoalescingAsyncChannel::new(),
        }
    }

    pub fn dispatcher(&self) -> EventDispatcher {
        EventDispatcher::new(self.channel.sender())
    }

    pub async fn next_event(&mut self) -> Option<WMEvent> {
        self.channel
            .coalesce(|event| matches!(event, WMEvent::MouseMoved(_)))
            .await
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
