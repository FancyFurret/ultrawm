use crate::{
    pid_t, AXError, AXUIElementCopyAttributeValue, AXUIElementCreateApplication,
    AXUIElementGetTypeID, AXUIElementRef, AXUIElementSetAttributeValue,
};
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::{declare_TCFType, impl_TCFType};
use std::ffi::c_void;
use std::{fmt, ptr};

pub fn create_application_element(pid: pid_t) -> AXUIElement {
    unsafe {
        let app_ref = AXUIElementCreateApplication(pid);
        TCFType::wrap_under_create_rule(app_ref)
    }
}

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
    pub fn copy_attribute_value(&self, attribute: CFStringRef) -> Result<CFTypeRef, AXError> {
        let mut value: CFTypeRef = ptr::null();
        unsafe {
            let error = AXUIElementCopyAttributeValue(self.0, attribute, &mut value);
            if error == 0 {
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
            if error == 0 {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}
