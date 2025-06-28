use crate::platform::macos::event_listener_ax::EventListenerAX;
use crate::platform::{PlatformResult, ProcessId};
use block2::{Block, StackBlock};
use log::{error, warn};
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::NSRunningApplication;
use objc2_app_kit::NSWorkspace;
use objc2_app_kit::NSWorkspaceApplicationKey;
use objc2_app_kit::{
    NSWorkspaceDidLaunchApplicationNotification, NSWorkspaceDidTerminateApplicationNotification,
};
use objc2_foundation::NSOperationQueue;
use objc2_foundation::{NSNotification, NSNotificationName};
use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;

type EventHandlerBlock = Block<dyn Fn(NonNull<NSNotification>) -> ()>;

pub struct EventListenerNS {
    listener_ax: Rc<RefCell<EventListenerAX>>,
    observers: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

impl EventListenerNS {
    pub fn run(listener_ax: Rc<RefCell<EventListenerAX>>) -> PlatformResult<Rc<RefCell<Self>>> {
        let listener = Rc::new(RefCell::new(Self {
            listener_ax,
            observers: Vec::new(),
        }));

        {
            let state = listener.clone();
            let block = StackBlock::new(move |notification: NonNull<NSNotification>| {
                if let Err(e) = state.borrow().handle_event(notification) {
                    error!("Error handling NS event: {:?}", e);
                }
            });

            // Documentation says we need to copy the block before passing it to the observer
            let block = block.copy();

            let mut state = listener.borrow_mut();
            state.setup_observers(&block)?;
        }

        Ok(listener)
    }

    fn handle_event(&self, notification: NonNull<NSNotification>) -> PlatformResult<()> {
        unsafe {
            let user_info = notification
                .as_ref()
                .userInfo()
                .ok_or("Could not get user info")?;

            let app = Retained::cast_unchecked::<NSRunningApplication>(
                user_info
                    .objectForKey(NSWorkspaceApplicationKey)
                    .ok_or("Could not get application")?,
            );

            let pid = app.processIdentifier();
            let name = notification.as_ref().name();

            if name.isEqualToString(NSWorkspaceDidLaunchApplicationNotification) {
                self.listener_ax
                    .borrow_mut()
                    .app_launched(pid as ProcessId)?;
            } else if name.isEqualToString(NSWorkspaceDidTerminateApplicationNotification) {
                self.listener_ax
                    .borrow_mut()
                    .app_terminated(pid as ProcessId)?;
            } else {
                warn!("Unknown notification: {:?}", notification);
            }
        }

        Ok(())
    }

    fn setup_observers(&mut self, block: &EventHandlerBlock) -> PlatformResult<()> {
        unsafe {
            self.add_observer(NSWorkspaceDidLaunchApplicationNotification, block)?;
            self.add_observer(NSWorkspaceDidTerminateApplicationNotification, block)?;
        }

        Ok(())
    }

    fn add_observer(
        &mut self,
        name: &NSNotificationName,
        block: &EventHandlerBlock,
    ) -> PlatformResult<()> {
        unsafe {
            let notification_center = NSWorkspace::sharedWorkspace().notificationCenter();
            let observer = notification_center.addObserverForName_object_queue_usingBlock(
                Some(name),
                None,
                Some(&NSOperationQueue::mainQueue()),
                &block,
            );

            self.observers.push(observer);

            Ok(())
        }
    }
}
