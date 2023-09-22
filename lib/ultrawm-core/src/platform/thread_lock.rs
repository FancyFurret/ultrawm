use std::{cell::RefCell, sync::Arc, thread::ThreadId};

use super::{Platform, PlatformImpl, PlatformResult};

pub struct MainThreadLock<T: 'static> {
    /// # Safety
    /// We are using RefCell instead of Mutex because we know that we will only ever access it
    /// from the main thread. We are wrapping T in Option<T> so that we can take it out and
    /// make sure it is dropped on the main thread.
    inner: Arc<RefCell<Option<T>>>,
    thread_id: ThreadId,
}

/// # Safety
/// This is safe because the inner value is only accessed on the main thread.
/// So we can mark this as `Send` and `Sync` even if the inner value is not.
unsafe impl<T> Send for MainThreadLock<T> {}
unsafe impl<T> Sync for MainThreadLock<T> {}

impl<T> MainThreadLock<T> {
    pub fn new<F>(init: F) -> PlatformResult<Self>
    where
        F: FnOnce() -> T + Send,
    {
        Platform::run_on_main_thread(|| {
            let inner = init();
            Self {
                inner: Arc::new(RefCell::new(Some(inner))),
                thread_id: std::thread::current().id(),
            }
        })
    }

    pub fn access<F, R>(&self, f: F) -> PlatformResult<R>
    where
        F: FnOnce(&T) -> R + Send,
        R: Send + 'static,
    {
        if self.thread_id == std::thread::current().id() {
            Ok(f(&self.inner.borrow().as_ref().unwrap()))
        } else {
            Platform::run_on_main_thread(|| f(&self.inner.borrow().as_ref().unwrap()))
        }
    }

    #[allow(dead_code)]
    pub fn access_mut<F, R>(&self, f: F) -> PlatformResult<R>
    where
        F: FnOnce(&mut T) -> R + Send,
        R: Send + 'static,
    {
        if self.thread_id == std::thread::current().id() {
            Ok(f(&mut self.inner.borrow_mut().as_mut().unwrap()))
        } else {
            Platform::run_on_main_thread(|| f(&mut self.inner.borrow_mut().as_mut().unwrap()))
        }
    }
}

/// # Safety
/// This ensures that T is always dropped on the main thread.
impl<T> Drop for MainThreadLock<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 && self.thread_id != std::thread::current().id() {
            Platform::run_on_main_thread(|| {
                let s = &self;
                let data = s.inner.borrow_mut().take().unwrap();
                drop(data);
            })
            .unwrap_or_else(|err| {
                eprintln!("Error dispatching Drop to main thread: {:?}", err);
            });
        }
    }
}
