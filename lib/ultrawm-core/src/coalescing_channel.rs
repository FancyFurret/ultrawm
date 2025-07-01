use std::collections::VecDeque;
use tokio::sync::mpsc;

pub struct CoalescingAsyncChannel<T> {
    sender: mpsc::UnboundedSender<T>,
    receiver: mpsc::UnboundedReceiver<T>,
    pending_messages: VecDeque<T>,
}

impl<T> CoalescingAsyncChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            pending_messages: VecDeque::new(),
        }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<T> {
        self.sender.clone()
    }

    pub fn try_recv(&mut self) -> Result<T, mpsc::error::TryRecvError> {
        if let Some(message) = self.pending_messages.pop_front() {
            return Ok(message);
        }

        self.receiver.try_recv()
    }

    pub async fn recv(&mut self) -> Option<T> {
        if let Some(message) = self.pending_messages.pop_front() {
            return Some(message);
        }

        self.receiver.recv().await
    }

    pub async fn coalesce<F>(&mut self, mut predicate: F) -> Option<T>
    where
        F: FnMut(&T) -> bool,
    {
        if let Some(message) = self.pending_messages.pop_front() {
            if predicate(&message) {
                return Some(self.coalesce_messages(message, &mut predicate));
            } else {
                return Some(message);
            }
        }

        if let Some(message) = self.receiver.recv().await {
            if predicate(&message) {
                Some(self.coalesce_messages(message, &mut predicate))
            } else {
                Some(message)
            }
        } else {
            None
        }
    }

    pub fn try_coalesce<F>(&mut self, mut predicate: F) -> Option<T>
    where
        F: FnMut(&T) -> bool,
    {
        if let Some(message) = self.pending_messages.pop_front() {
            if predicate(&message) {
                return Some(self.coalesce_messages(message, &mut predicate));
            } else {
                return Some(message);
            }
        }

        match self.receiver.try_recv() {
            Ok(message) => {
                if predicate(&message) {
                    Some(self.coalesce_messages(message, &mut predicate))
                } else {
                    Some(message)
                }
            }
            Err(_) => None,
        }
    }

    fn coalesce_messages<F>(&mut self, mut coalesced_message: T, predicate: &mut F) -> T
    where
        F: FnMut(&T) -> bool,
    {
        loop {
            match self.receiver.try_recv() {
                Ok(msg) if predicate(&msg) => {
                    coalesced_message = msg;
                }
                Ok(other_msg) => {
                    self.pending_messages.push_back(other_msg);
                }
                Err(_) => {
                    break;
                }
            }
        }
        coalesced_message
    }
}
