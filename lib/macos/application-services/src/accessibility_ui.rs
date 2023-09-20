use crate::{
    kAXErrorSuccess, pid_t, AXError, AXObserverAddNotification, AXObserverCreate,
    AXObserverGetRunLoopSource, AXObserverGetTypeID, AXObserverRef, AXObserverRemoveNotification,
    AXUIElementCopyAttributeValue, AXUIElementCreateApplication, AXUIElementCreateSystemWide,
    AXUIElementGetPid, AXUIElementGetTypeID, AXUIElementRef, AXUIElementSetAttributeValue,
};
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::runloop::CFRunLoopSource;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::{declare_TCFType, impl_TCFType};
use std::ffi::c_void;
use std::ops::Deref;
use std::{fmt, ptr};

declare_TCFType! {
   AXUIElement, AXUIElementRef
}
impl_TCFType!(AXUIElement, AXUIElementRef, AXUIElementGetTypeID);

impl fmt::Debug for AXUIElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AXUIElement({:?})", self.as_concrete_TypeRef())
    }
}

impl AXUIElement {
    pub fn create_application(pid: pid_t) -> Result<Self, ()> {
        unsafe {
            let app_ref = AXUIElementCreateApplication(pid);
            if app_ref.is_null() {
                Err(())
            } else {
                Ok(TCFType::wrap_under_create_rule(app_ref))
            }
        }
    }

    pub fn create_system_wide() -> Result<Self, ()> {
        unsafe {
            let app_ref = AXUIElementCreateSystemWide();
            if app_ref.is_null() {
                Err(())
            } else {
                Ok(TCFType::wrap_under_create_rule(app_ref))
            }
        }
    }

    pub fn copy_attribute_value(&self, attribute: CFStringRef) -> Result<CFTypeRef, AXError> {
        let mut value: CFTypeRef = ptr::null();
        unsafe {
            let error = AXUIElementCopyAttributeValue(self.0, attribute, &mut value);
            if error == kAXErrorSuccess {
                Ok(value)
            } else {
                Err(error)
            }
        }
    }

    pub fn set_attribute_value(
        &self,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> Result<(), AXError> {
        unsafe {
            let error = AXUIElementSetAttributeValue(self.0, attribute, value);
            if error == kAXErrorSuccess {
                Ok(())
            } else {
                Err(error)
            }
        }
    }

    pub fn get_pid(&self) -> Result<pid_t, AXError> {
        unsafe {
            let mut pid: pid_t = 0;
            let error = AXUIElementGetPid(self.0, &mut pid);
            if error == kAXErrorSuccess {
                Ok(pid)
            } else {
                Err(error)
            }
        }
    }
}

declare_TCFType! {
   AXObserver, AXObserverRef
}
impl_TCFType!(AXObserver, AXObserverRef, AXObserverGetTypeID);

pub type AXObserverCallbackFn<'a, T = ()> =
    Box<dyn Fn(AXObserver, AXUIElement, CFString, &T) -> () + 'a>;

type AXObserverCallbackFnInternal<'a> = Box<dyn Fn(AXObserver, AXUIElement, CFString) -> () + 'a>;

#[no_mangle]
unsafe extern "C" fn ax_observer_callback_internal(
    observer: AXObserverRef,
    element: AXUIElementRef,
    notification: CFStringRef,
    refcon: *mut c_void,
) {
    let observer = AXObserver::wrap_under_get_rule(observer);
    let element = AXUIElement::wrap_under_get_rule(element);
    let notification = CFString::wrap_under_get_rule(notification);
    let callback = refcon as *mut AXObserverCallbackFnInternal;
    (*callback)(observer, element, notification);
}

impl AXObserver {
    pub fn new(pid: pid_t) -> Result<AXObserver, ()> {
        unsafe {
            let mut observer_ref: AXObserverRef = ptr::null_mut();
            let error =
                AXObserverCreate(pid, Some(ax_observer_callback_internal), &mut observer_ref);
            if error == kAXErrorSuccess {
                let observer = AXObserver::wrap_under_create_rule(observer_ref);
                Ok(observer)
            } else {
                Err(())
            }
        }
    }

    pub fn add_notification<'a, F, T>(
        &self,
        element: &AXUIElement,
        notification: CFString,
        callback: F,
        data: T,
    ) -> Result<AXNotification<'a>, AXError>
    where
        F: Fn(AXObserver, AXUIElement, CFString, &T) -> () + 'a,
        T: 'a,
    {
        let callback: Box<AXObserverCallbackFnInternal> =
            Box::new(Box::new(move |observer, element, notification| {
                (callback)(observer, element, notification, &data);
            }));

        let raw = Box::into_raw(callback);

        unsafe {
            let result = AXObserverAddNotification(
                self.as_concrete_TypeRef(),
                element.as_concrete_TypeRef(),
                notification.as_concrete_TypeRef(),
                raw as *mut c_void,
            );

            if result == kAXErrorSuccess {
                Ok(AXNotification {
                    observer: self.clone(),
                    element: element.clone(),
                    name: notification,
                    _callback: Box::from_raw(raw),
                })
            } else {
                let _ = Box::from_raw(raw);
                Err(result)
            }
        }
    }

    pub fn remove_notification<'a>(
        &self,
        notification: &AXNotification<'a>,
    ) -> Result<(), AXError> {
        unsafe {
            let result = AXObserverRemoveNotification(
                self.as_concrete_TypeRef(),
                notification.element.as_concrete_TypeRef(),
                notification.name.as_concrete_TypeRef(),
            );

            if result == kAXErrorSuccess {
                Ok(())
            } else {
                Err(result)
            }
        }
    }

    pub fn get_run_loop_source(&self) -> CFRunLoopSource {
        unsafe {
            let source = AXObserverGetRunLoopSource(self.as_concrete_TypeRef());
            TCFType::wrap_under_get_rule(source)
        }
    }
}

pub struct AXNotification<'a> {
    pub observer: AXObserver,
    pub element: AXUIElement,
    pub name: CFString,
    _callback: Box<AXObserverCallbackFnInternal<'a>>,
}

impl Drop for AXNotification<'_> {
    fn drop(&mut self) {
        // If the notification is dropped, then the callback will be dropped. We need to make sure to
        // remove the notification from the observer so that it doesn't try to call the callback.
        let _ = self.observer.remove_notification(self);
    }
}
