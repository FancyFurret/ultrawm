use std::{cell::RefCell, sync::Arc, thread::ThreadId};

use crate::event_loop_main::run_on_main_thread_async;

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
    pub async fn new<F>(init: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        run_on_main_thread_async(move || {
            let inner = init();
            Self {
                inner: Arc::new(RefCell::new(Some(inner))),
                thread_id: std::thread::current().id(),
            }
        })
        .await
    }

    pub async fn access_async<F, R>(&self, accessor: F) -> R
    where
        F: FnOnce(&T) -> R + Send + 'static,
        R: Send + 'static,
    {
        if self.thread_id == std::thread::current().id() {
            accessor(&self.inner.borrow().as_ref().unwrap())
        } else {
            let lock = self.clone();
            run_on_main_thread_async(move || accessor(&lock.inner.borrow().as_ref().unwrap())).await
        }
    }

    pub fn access<F>(&self, accessor: F)
    where
        F: FnOnce(&T) + Send + 'static,
    {
        let lock = self.clone();
        tokio::spawn(async move {
            lock.access_async(accessor).await;
        });
    }

    pub async fn access_mut_async<F, R>(&self, accessor: F) -> R
    where
        F: FnOnce(&mut T) -> R + Send + 'static,
        R: Send + 'static,
    {
        if self.thread_id == std::thread::current().id() {
            accessor(&mut self.inner.borrow_mut().as_mut().unwrap())
        } else {
            let lock = self.clone();
            run_on_main_thread_async(move || {
                accessor(&mut lock.inner.borrow_mut().as_mut().unwrap())
            })
            .await
        }
    }

    pub fn access_mut<F>(&self, accessor: F)
    where
        F: FnOnce(&mut T) + Send + 'static,
    {
        let lock = self.clone();
        tokio::spawn(async move {
            lock.access_mut_async(accessor).await;
        });
    }

    /// Only use this if you only access the arc from the main thread!
    pub fn get_arc(&self) -> Arc<RefCell<Option<T>>> {
        // TODO
        // if self.thread_id != std::thread::current().id() {
        //     panic!("get_arc must be called from the main thread");
        // }

        self.inner.clone()
    }
}

/// # Safety
/// This ensures that T is always dropped on the main thread.
impl<T> Drop for MainThreadLock<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 && self.thread_id != std::thread::current().id() {
            let lock = self.clone();
            tokio::spawn(async move {
                run_on_main_thread_async(move || {
                    let data = lock.inner.borrow_mut().take().unwrap();
                    drop(data);
                })
                .await;
            });
        }
    }
}

impl<T> Clone for MainThreadLock<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            thread_id: self.thread_id,
        }
    }
}
