use crate::platform::{PlatformMainThreadImpl, PlatformResult};
use icrate::block2::{ConcreteBlock, RcBlock};
use icrate::Foundation::{is_main_thread, NSBlockOperation, NSOperationQueue, NSThread};
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

pub struct MacOSMainThread;

impl PlatformMainThreadImpl for MacOSMainThread {
    fn is_main_thread() -> bool {
        NSThread::currentThread().isMainThread()
    }

    fn run_on_main_thread<F, R>(f: F) -> PlatformResult<R>
    where
        F: FnOnce() -> R + Send,
        R: Send + 'static,
    {
        if is_main_thread() {
            return Ok(f());
        }

        let func = Arc::new(Mutex::new(Some(f)));
        let result = Arc::new(Mutex::new(None));

        let block = {
            let result = result.clone();
            ConcreteBlock::new(move || {
                if let Some(func) = func.lock().unwrap().take() {
                    result.lock().unwrap().replace(Some(func()));
                }
            })
        };

        // This is how block.copy() works and produces an RcBlock
        // The issue is that block.copy() requires the block to be static, but
        // our block is not. We can safely create an RcBlock from this block
        // because we are waiting for the operation to finish before leaving this function.
        let mut ptr = ManuallyDrop::new(block);
        let ptr: *mut c_void = &mut *ptr as *mut _ as *mut c_void;
        let block: RcBlock<(), ()> = unsafe { RcBlock::copy(ptr.cast()) };

        unsafe {
            let op = NSBlockOperation::blockOperationWithBlock(&block);
            NSOperationQueue::mainQueue().addOperation(&op);
            op.waitUntilFinished();
        }

        let result = result.lock().unwrap().take().unwrap().unwrap();
        Ok(result)
    }
}
