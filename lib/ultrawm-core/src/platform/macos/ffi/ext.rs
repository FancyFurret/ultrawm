#![allow(dead_code)]

use crate::platform::macos::ffi::accessibility_attribute;
use application_services::accessibility_ui::AXUIElement;
use application_services::accessibility_value::AXValue;
use application_services::{
    kAXErrorFailure, kAXValueTypeCGPoint, kAXValueTypeCGSize, pid_t, AXError, AXUIElementRef,
    AXValueRef, AXValueType,
};
use core_foundation::array::CFArrayRef;
use core_foundation::base::{FromVoid, ItemRef, TCFTypeRef, ToVoid};
use core_foundation::boolean::CFBoolean;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation::{
    array::CFArray,
    base::TCFType,
    dictionary::{CFDictionary, CFDictionaryRef},
};
use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

pub trait TCFTypeOrExt {
    type CFType: TCFType<Ref = Self::CFTypeRef>;
    type CFTypeRef: TCFTypeRef;
    fn from(value: Self::CFType) -> Self;
}

impl<T: TCFType> TCFTypeOrExt for T {
    type CFType = T;
    type CFTypeRef = T::Ref;
    fn from(value: Self::CFType) -> Self {
        value
    }
}

impl TCFTypeOrExt for AXValueExt {
    type CFType = AXValue;
    type CFTypeRef = AXValueRef;
    fn from(value: Self::CFType) -> Self {
        Self::from(value)
    }
}

impl<T: TCFTypeOrExt> TCFTypeOrExt for CFArrayExt<T> {
    type CFType = CFArray;
    type CFTypeRef = CFArrayRef;
    fn from(value: Self::CFType) -> Self {
        Self::from(value)
    }
}

impl TCFTypeOrExt for AXUIElementExt {
    type CFType = AXUIElement;
    type CFTypeRef = AXUIElementRef;
    fn from(value: Self::CFType) -> Self {
        Self::from(value)
    }
}

impl TCFTypeOrExt for CFDictionaryExt {
    type CFType = CFDictionary;
    type CFTypeRef = CFDictionaryRef;
    fn from(value: Self::CFType) -> Self {
        Self::from(value)
    }
}

#[derive(Debug)]
pub struct CFArrayExt<T>
where
    T: TCFTypeOrExt,
{
    pub array: CFArray,
    _marker: PhantomData<T>,
}

impl<T> CFArrayExt<T>
where
    T: TCFTypeOrExt,
{
    pub fn from(array: CFArray) -> Self {
        Self {
            array,
            _marker: PhantomData,
        }
    }
}

impl<T> IntoIterator for CFArrayExt<T>
where
    T: TCFTypeOrExt,
{
    type Item = T;
    type IntoIter = CFArrayIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        CFArrayIterator::new(self.array)
    }
}

pub struct CFArrayIterator<T> {
    array: CFArray,
    index: isize,
    _marker: PhantomData<T>,
}

impl<T> CFArrayIterator<T> {
    pub fn new(array: CFArray) -> Self {
        Self {
            array,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<T> Iterator for CFArrayIterator<T>
where
    T: TCFTypeOrExt,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.array.len() {
            return None;
        }

        let item = *self.array.get(self.index)?;
        self.index += 1;

        let item_ref = unsafe { T::CFTypeRef::from_void_ptr(item) };
        Some(unsafe { T::from(T::CFType::wrap_under_get_rule(item_ref)) })
    }
}

#[derive(Debug)]
pub struct CFDictionaryExt {
    pub dictionary: CFDictionary,
}

impl CFDictionaryExt {
    pub fn from(dictionary: CFDictionary) -> Self {
        Self { dictionary }
    }

    pub fn get<T: FromVoid>(&self, key: CFString) -> ItemRef<'_, T> {
        let key = ToVoid::to_void(&key);
        let value = self.dictionary.get(key);
        unsafe { T::from_void(value.as_void_ptr()) }
    }

    pub fn get_int(&self, key: CFString) -> Option<i32> {
        let number = self.get::<CFNumber>(key);
        number.to_i32()
    }

    pub fn get_string(&self, key: CFString) -> String {
        let string = self.get::<CFString>(key);
        string.to_string()
    }

    pub fn get_bool(&self, key: CFString) -> bool {
        let boolean = self.get::<CFBoolean>(key);
        bool::from(boolean.to_owned())
    }

    pub fn get_dict(&self, key: CFString) -> ItemRef<'_, CFDictionary> {
        let dict = self.get::<CFDictionary>(key);
        dict
    }

    pub fn get_rect(&self, key: CFString) -> Option<CGRect> {
        let dict = self.get_dict(key);
        CGRect::from_dict_representation(dict.deref())
    }

    pub fn contains_key(&self, key: CFString) -> bool {
        let key = ToVoid::to_void(&key);
        self.dictionary.contains_key(&key)
    }
}

#[derive(Debug, Clone)]
pub struct AXUIElementExt {
    pub element: AXUIElement,
}

unsafe impl Send for AXUIElementExt {}

pub type AXResult<T> = Result<T, AXError>;

impl AXUIElementExt {
    pub fn from(element: AXUIElement) -> Self {
        Self { element }
    }

    fn copy_attribute_value<T: TCFTypeOrExt>(&self, attribute: CFString) -> AXResult<T> {
        let value = self
            .element
            .copy_attribute_value(attribute.as_concrete_TypeRef())?;

        unsafe {
            let value_ref = T::CFTypeRef::from_void_ptr(value);
            Ok(T::from(T::CFType::wrap_under_create_rule(value_ref)))
        }
    }

    fn set_attribute_value(&self, attribute: CFString, value: AXValueExt) -> AXResult<()> {
        self.element
            .set_attribute_value(attribute.as_concrete_TypeRef(), value.value.as_CFTypeRef())
    }

    pub fn windows(&self) -> AXResult<CFArrayExt<AXUIElementExt>> {
        self.copy_attribute_value::<CFArrayExt<AXUIElementExt>>(accessibility_attribute::windows())
    }

    pub fn focused_window(&self) -> AXResult<AXUIElementExt> {
        self.copy_attribute_value::<AXUIElementExt>(accessibility_attribute::focused_window())
    }

    pub fn title(&self) -> AXResult<String> {
        self.copy_attribute_value::<CFString>(accessibility_attribute::title())
            .map(|s| s.to_string())
    }

    pub fn role(&self) -> AXResult<String> {
        self.copy_attribute_value::<CFString>(accessibility_attribute::role())
            .map(|s| s.to_string())
    }

    pub fn subrole(&self) -> AXResult<String> {
        self.copy_attribute_value::<CFString>(accessibility_attribute::subrole())
            .map(|s| s.to_string())
    }

    pub fn position(&self) -> AXResult<CGPoint> {
        self.copy_attribute_value::<AXValueExt>(accessibility_attribute::position())
            .map(|v| v.into_point().ok_or(kAXErrorFailure).unwrap())
    }

    pub fn size(&self) -> AXResult<CGSize> {
        self.copy_attribute_value::<AXValueExt>(accessibility_attribute::size())
            .map(|v| v.into_size().ok_or(kAXErrorFailure).unwrap())
    }

    pub fn minimized(&self) -> AXResult<bool> {
        self.copy_attribute_value::<CFBoolean>(accessibility_attribute::minimized())
            .map(|b| bool::from(b))
    }

    pub fn pid(&self) -> AXResult<pid_t> {
        self.element.get_pid()
    }

    pub fn set_position(&self, position: CGPoint) -> AXResult<()> {
        self.set_attribute_value(
            accessibility_attribute::position(),
            AXValueExt::from_point(position),
        )
    }

    pub fn set_size(&self, size: CGSize) -> AXResult<()> {
        self.set_attribute_value(accessibility_attribute::size(), AXValueExt::from_size(size))
    }
}

pub struct AXValueExt {
    pub value: AXValue,
}

impl AXValueExt {
    pub fn from(value: AXValue) -> Self {
        Self { value }
    }

    pub fn from_point(point: CGPoint) -> Self {
        Self {
            value: AXValue::create(kAXValueTypeCGPoint, &point as *const _ as *const c_void),
        }
    }

    pub fn from_size(size: CGSize) -> Self {
        Self {
            value: AXValue::create(kAXValueTypeCGSize, &size as *const _ as *const c_void),
        }
    }

    pub fn into_point(self) -> Option<CGPoint> {
        self.into(kAXValueTypeCGPoint)
    }

    pub fn into_size(self) -> Option<CGSize> {
        self.into(kAXValueTypeCGSize)
    }

    fn into<T: Sized>(self, the_type: AXValueType) -> Option<T> {
        unsafe {
            let mut value: T = mem::zeroed();
            let result = self
                .value
                .get_value(the_type, &mut value as *mut _ as *mut c_void);
            if result.is_err() {
                return None;
            }

            Some(value)
        }
    }
}
