use crate::{AXValueCreate, AXValueGetTypeID, AXValueGetValue, AXValueRef, AXValueType};
use core_foundation::base::TCFType;
use core_foundation::{declare_TCFType, impl_TCFType};
use std::ffi::c_void;

declare_TCFType! {
    AXValue, AXValueRef
}
impl_TCFType!(AXValue, AXValueRef, AXValueGetTypeID);

impl AXValue {
    pub fn create(the_type: AXValueType, value: *const c_void) -> AXValue {
        unsafe {
            let value = AXValueCreate(the_type, value);
            TCFType::wrap_under_create_rule(value)
        }
    }

    pub fn get_value(&self, the_type: AXValueType, value: *mut c_void) -> Result<*mut c_void, ()> {
        unsafe {
            let success = AXValueGetValue(self.0, the_type, value);
            if success == 1 {
                Ok(value)
            } else {
                Err(())
            }
        }
    }
}
