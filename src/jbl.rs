use core::{convert::TryFrom, ptr, slice, str::FromStr};

use crate::{
    ffi,
    printer::{self, AsJson, JsonPrinter},
    utils::check_rc,
    xstr::StringPtr,
    xstr::XString,
    EjdbError, JsonPrintFlags, Result,
};
use ejdb2_sys as sys;
pub use sys::jbl_type_t as JBLType;

///binary JSON object
pub struct JBL {
    handle: sys::JBL,
    writable: bool,
}

impl JBL {
    /// create empty array
    #[inline]
    pub fn new_array() -> Result<Self> {
        let mut h: sys::JBL = ptr::null_mut();
        let rc = unsafe { sys::jbl_create_empty_array(&mut h) };
        check_rc(rc)?;
        Ok(Self {
            handle: h,
            writable: true,
        })
    }
    /// create empty object
    #[inline]
    pub fn new_object() -> Result<Self> {
        let mut h: sys::JBL = ptr::null_mut();
        let rc = unsafe { sys::jbl_create_empty_object(&mut h) };
        check_rc(rc)?;
        Ok(Self {
            handle: h,
            writable: true,
        })
    }
    #[inline(always)]
    pub(crate) fn from_ptr(handle: *mut sys::_JBL) -> Self {
        Self {
            handle,
            writable: false,
        }
    }

    /// from JSON string
    #[inline]
    pub fn from_json<'a>(json: impl Into<StringPtr<'a>>) -> Result<Self> {
        let json = json.into();
        unsafe { Self::from_c_str(json.as_ptr()) }
    }
    /// from JSON string
    #[inline]
    pub unsafe fn from_c_str(str_ptr: *const i8) -> Result<Self> {
        // if str_ptr.is_null() {
        //     panic!("null ptr")
        // }
        let mut handle = ptr::null_mut();
        let rc = sys::jbl_from_json(&mut handle, str_ptr);
        check_rc(rc)?;
        Ok(Self::from_ptr(handle))
    }

    /// writable only if created by create_array or create_object
    #[inline(always)]
    pub fn writable(&self) -> bool {
        self.writable
    }

    #[inline(always)]
    pub(crate) fn raw_ptr(&self) -> sys::JBL {
        self.handle
    }

    /// underline buffer size
    #[inline(always)]
    pub(crate) fn size(&self) -> usize {
        unsafe { sys::jbl_size(self.raw_ptr()) as usize }
    }

    /// child element count
    #[inline(always)]
    pub fn count(&self) -> usize {
        unsafe { sys::jbl_count(self.raw_ptr()) as usize }
    }

    /// append value if JBL is a JSON array; Note: only work if writable
    #[inline]
    pub fn append<'a, 'b>(&mut self, val: impl IntoJBLValue<'b>) -> Result<()> {
        let val = val.into_value();
        let key: Option<&str> = None;
        match val {
            JBLValue::Null => self.set_null(key),
            JBLValue::EmptyArray => self.set_empty_array(key),
            JBLValue::EmptyObject => self.set_empty_object(key),
            JBLValue::Boolean(v) => self.set_bool(key, v),
            JBLValue::Float(v) => self.set_f64(key, v),
            JBLValue::Integer(v) => self.set_i64(key, v),
            JBLValue::Nested(v) => self.set_nested(key, v),
            JBLValue::String(v) => self.set_str(key, v),
        }
    }

    /// set property if JBL is a JSON object; Note: only work if writable
    #[inline]
    pub fn set_prop<'a, 'b>(
        &mut self,
        key: impl Into<StringPtr<'a>>,
        val: impl IntoJBLValue<'b>,
    ) -> Result<()> {
        let val = val.into_value();
        let key = Some(key);
        match val {
            JBLValue::Null => self.set_null(key),
            JBLValue::EmptyArray => self.set_empty_array(key),
            JBLValue::EmptyObject => self.set_empty_object(key),
            JBLValue::Boolean(v) => self.set_bool(key, v),
            JBLValue::Float(v) => self.set_f64(key, v),
            JBLValue::Integer(v) => self.set_i64(key, v),
            JBLValue::Nested(v) => self.set_nested(key, v),
            JBLValue::String(v) => self.set_str(key, v),
        }
    }

    /// set object property
    #[inline]
    fn set_i64<'a, K: Into<StringPtr<'a>>>(&mut self, key: Option<K>, val: i64) -> Result<()> {
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_int64(self.raw_ptr(), key.as_ptr(), val)
            },
            None => unsafe { sys::jbl_set_int64(self.raw_ptr(), ptr::null(), val) },
        };
        check_rc(rc)
    }
    /// set object property
    #[inline]
    fn set_f64<'a, K: Into<StringPtr<'a>>>(&mut self, key: Option<K>, val: f64) -> Result<()> {
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_f64(self.raw_ptr(), key.as_ptr(), val)
            },
            None => unsafe { sys::jbl_set_f64(self.raw_ptr(), ptr::null(), val) },
        };
        check_rc(rc)
    }

    /// set object property
    #[inline]
    fn set_bool<'a, K: Into<StringPtr<'a>>>(&mut self, key: Option<K>, val: bool) -> Result<()> {
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_bool(self.raw_ptr(), key.as_ptr(), val)
            },
            None => unsafe { sys::jbl_set_bool(self.raw_ptr(), ptr::null(), val) },
        };
        check_rc(rc)
    }

    /// set object property
    #[inline]
    fn set_str<'a, 'b, K: Into<StringPtr<'a>>>(
        &mut self,
        key: Option<K>,
        val: impl Into<StringPtr<'b>>,
    ) -> Result<()> {
        let val = val.into();
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_string(self.raw_ptr(), key.as_ptr(), val.as_ptr())
            },
            None => unsafe { sys::jbl_set_string(self.raw_ptr(), ptr::null(), val.as_ptr()) },
        };
        check_rc(rc)
    }

    #[inline]
    fn set_nested<'a, K: Into<StringPtr<'a>>>(&mut self, key: Option<K>, val: JBL) -> Result<()> {
        let ptr = val.raw_ptr();
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_nested(self.raw_ptr(), key.as_ptr(), ptr)
            },
            None => unsafe { sys::jbl_set_nested(self.raw_ptr(), ptr::null(), ptr) },
        };
        check_rc(rc)?;
        Ok(())
    }
    #[inline]
    fn set_null<'a, K: Into<StringPtr<'a>>>(&mut self, key: Option<K>) -> Result<()> {
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_null(self.raw_ptr(), key.as_ptr())
            },
            None => unsafe { sys::jbl_set_null(self.raw_ptr(), ptr::null()) },
        };
        check_rc(rc)
    }
    #[inline]
    fn set_empty_array<'a, K: Into<StringPtr<'a>>>(&mut self, key: Option<K>) -> Result<()> {
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_empty_array(self.raw_ptr(), key.as_ptr())
            },
            None => unsafe { sys::jbl_set_empty_array(self.raw_ptr(), ptr::null()) },
        };
        check_rc(rc)
    }
    #[inline]
    fn set_empty_object<'a, K: Into<StringPtr<'a>>>(&mut self, key: Option<K>) -> Result<()> {
        let rc = match key {
            Some(key) => unsafe {
                let key = key.into();
                sys::jbl_set_empty_object(self.raw_ptr(), key.as_ptr())
            },
            None => unsafe { sys::jbl_set_empty_object(self.raw_ptr(), ptr::null()) },
        };
        check_rc(rc)
    }
    ///Note: only work if writable
    #[inline]
    pub fn patch<'a>(&mut self, json: impl Into<StringPtr<'a>>) -> Result<()> {
        let json = json.into();
        let rc = unsafe { sys::jbl_patch_from_json(self.raw_ptr(), json.as_ptr()) };
        check_rc(rc)
    }
    ///Note: only work if writable
    #[inline]
    pub fn merge<'a>(&mut self, json: impl Into<StringPtr<'a>>) -> Result<()> {
        let json = json.into();
        let rc = unsafe { sys::jbl_merge_patch(self.raw_ptr(), json.as_ptr()) };
        check_rc(rc)
    }

    /// get property if JBL is a JSON object;
    #[inline]
    pub fn get_bool<'a>(&self, key: impl Into<StringPtr<'a>>) -> Result<bool> {
        let key = key.into();
        let mut val = false;
        let rc = unsafe {
            let ptr = &mut val as *mut _;
            sys::jbl_object_get_bool(self.raw_ptr(), key.as_ptr(), ptr)
        };
        check_rc(rc).and(Ok(val))
    }
    /// get property if JBL is a JSON object;
    #[inline]
    pub fn get_i64<'a>(&self, key: impl Into<StringPtr<'a>>) -> Result<i64> {
        let key = key.into();
        let mut val = 0_i64;
        let rc = unsafe {
            let ptr = &mut val as *mut _;
            sys::jbl_object_get_i64(self.raw_ptr(), key.as_ptr(), ptr)
        };
        check_rc(rc).and(Ok(val))
    }
    /// get property if JBL is a JSON object;
    #[inline]
    pub fn get_f64<'a>(&self, key: impl Into<StringPtr<'a>>) -> Result<f64> {
        let key = key.into();
        let mut val = 0_f64;
        let rc = unsafe {
            let ptr = &mut val as *mut _;
            sys::jbl_object_get_f64(self.raw_ptr(), key.as_ptr(), ptr)
        };
        check_rc(rc).and(Ok(val))
    }
    /// get property if JBL is a JSON object;
    #[inline]
    pub fn get_str<'a>(&self, key: impl Into<StringPtr<'a>>) -> Result<XString> {
        let key = key.into();
        let mut out_ptr = ptr::null();
        let rc = unsafe { sys::jbl_object_get_str(self.raw_ptr(), key.as_ptr(), &mut out_ptr) };
        check_rc(rc)?;
        Ok(XString::from_str_ptr(out_ptr))
    }
    /// get property if JBL is a JSON object;
    #[inline]
    pub fn get_type<'a>(&self, key: impl Into<StringPtr<'a>>) -> Result<JBLType> {
        let key = key.into();
        let res = unsafe { sys::jbl_object_get_type(self.raw_ptr(), key.as_ptr()) };
        Ok(res)
    }

    /// find value by rfc6901 path
    #[inline]
    pub fn find<'a>(&self, path: impl Into<StringPtr<'a>>) -> Result<JBL> {
        let path = path.into();
        let mut h = ptr::null_mut();
        let rc = unsafe { sys::jbl_at(self.raw_ptr(), path.as_ptr(), &mut h) };
        check_rc(rc)?;
        Ok(Self::from_ptr(h))
    }

    /// convert to f64, returns 0 if value cannot be converted
    #[inline(always)]
    pub fn as_f64(&self) -> f64 {
        unsafe { sys::jbl_get_f64(self.raw_ptr()) }
    }

    /// convert to i64, returns 0 if value cannot be converted
    #[inline(always)]
    pub fn as_i64(&self) -> i64 {
        unsafe { sys::jbl_get_i64(self.raw_ptr()) }
    }

    /// convert to i32, returns 0 if value cannot be converted
    #[inline(always)]
    pub fn as_i32(&self) -> i32 {
        unsafe { sys::jbl_get_i32(self.raw_ptr()) }
    }

    /// convert to str
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe {
            let data = sys::jbl_get_str(self.raw_ptr());
            let len = ffi::strlen(data);
            let buf = slice::from_raw_parts(data as *const u8, len);
            core::str::from_utf8_unchecked(buf)
        }
    }

    /// print json to writer
    #[inline]
    pub fn print<T: JsonPrinter>(
        &self,
        target: &mut T,
        flag: Option<JsonPrintFlags>,
    ) -> Result<()> {
        let flag = flag.unwrap_or(JsonPrintFlags::PRINT_CODEPOINTS);
        printer::jbl_print_json(self.raw_ptr(), target, flag)
    }
}

impl FromStr for JBL {
    type Err = EjdbError;
    #[inline]
    fn from_str(json: &str) -> Result<Self> {
        Self::from_json(json)
    }
}

impl TryFrom<XString> for JBL {
    type Error = EjdbError;
    #[inline]
    fn try_from(value: XString) -> Result<Self> {
        let ptr = value.as_ptr();
        unsafe { Self::from_c_str(ptr) }
    }
}

impl AsJson<XString> for JBL {
    /// more efficient than use print() for XString
    #[inline]
    fn as_json(&self, flag: Option<JsonPrintFlags>) -> Result<XString> {
        let flag = flag.unwrap_or(JsonPrintFlags::PRINT_CODEPOINTS);
        let size = self.size() * 2;
        let data = XString::new_with_size(size);
        let rc = unsafe {
            sys::jbl_as_json(
                self.raw_ptr(),
                Some(sys::jbl_xstr_json_printer),
                data.as_ptr() as *mut _,
                flag.bits,
            )
        };
        check_rc(rc).and(Ok(data))
    }
}
#[cfg(any(feature = "std", feature = "alloc"))]
impl AsJson<Vec<u8>> for JBL {
    #[inline]
    fn as_json(&self, flag: Option<JsonPrintFlags>) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        self.print(&mut buf, flag)?;
        Ok(buf)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl AsJson<String> for JBL {
    #[inline]
    fn as_json(&self, flag: Option<JsonPrintFlags>) -> Result<String> {
        self.as_json(flag)
            .map(|x| unsafe { String::from_utf8_unchecked(x) })
    }
}

impl Drop for JBL {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            sys::jbl_destroy(&mut self.handle);
        }
    }
}

pub enum JBLValue<'a> {
    Null,
    EmptyArray,
    EmptyObject,
    Float(f64),
    Integer(i64),
    String(StringPtr<'a>),
    Boolean(bool),
    Nested(JBL),
}
pub trait IntoJBLValue<'a> {
    fn into_value(self) -> JBLValue<'a>;
}
macro_rules! impl_for_f64 {
    ($type:ident) => {
        impl<'a> IntoJBLValue<'a> for $type {
            fn into_value(self) -> JBLValue<'a> {
                JBLValue::Float(self as f64)
            }
        }

        impl<'a> IntoJBLValue<'a> for &$type {
            fn into_value(self) -> JBLValue<'a> {
                JBLValue::Float(*self as f64)
            }
        }
    };
}

macro_rules! impl_for_i64 {
    ($type:ident) => {
        impl<'a> IntoJBLValue<'a> for $type {
            fn into_value(self) -> JBLValue<'a> {
                JBLValue::Integer(self as i64)
            }
        }

        impl<'a> IntoJBLValue<'a> for &$type {
            fn into_value(self) -> JBLValue<'a> {
                JBLValue::Integer(*self as i64)
            }
        }
    };
}
impl_for_f64!(f64);
impl_for_f64!(f32);
impl_for_i64!(i64);
impl_for_i64!(i32);
impl_for_i64!(i16);
impl_for_i64!(i8);
impl_for_i64!(u64);
impl_for_i64!(u32);
impl_for_i64!(u16);
impl_for_i64!(u8);
impl_for_i64!(usize);

impl<'a> IntoJBLValue<'a> for bool {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::Boolean(self)
    }
}

impl<'a> IntoJBLValue<'a> for &bool {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::Boolean(*self)
    }
}

impl<'a> IntoJBLValue<'a> for JBL {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::Nested(self)
    }
}

impl<'a> IntoJBLValue<'a> for JBLValue<'a> {
    fn into_value(self) -> JBLValue<'a> {
        self
    }
}

impl<'a> IntoJBLValue<'a> for &'a str {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::String(self.into())
    }
}
#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a> IntoJBLValue<'a> for String {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::String(self.into())
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a> IntoJBLValue<'a> for &'a String {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::String(self.into())
    }
}

impl<'a> IntoJBLValue<'a> for XString {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::String(self.into())
    }
}

impl<'a> IntoJBLValue<'a> for &'a XString {
    fn into_value(self) -> JBLValue<'a> {
        JBLValue::String(self.into())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_from_json_object() {
        unsafe {
            let rc = sys::jbl_init();
            check_rc(rc).unwrap();
        }

        let json = "{\"a\":1,\"b\":\"OK\", \"c\":null}";
        let obj: JBL = json.parse().unwrap();
        let a = obj.get_i64("a").unwrap();
        assert_eq!(a, 1);

        let b = obj.get_str("b").unwrap();
        assert_eq!(b, "OK");

        let t = obj.get_type("c").unwrap();
        assert_eq!(t, JBLType::JBV_NULL);
    }

    #[test]
    fn test_empty_object() {
        let mut jbl = JBL::new_object().unwrap();
        jbl.set_prop("a", true).unwrap();
        jbl.set_prop("b", 12345 as f64).unwrap();
        jbl.set_prop("c", JBLValue::Null).unwrap();
        jbl.set_prop("d", JBLValue::EmptyArray).unwrap();
        jbl.set_prop("e", "{\"a\":1,\"b\":2}".parse::<JBL>().unwrap())
            .unwrap();
        let json: String = jbl.as_json(None).unwrap();
        assert_eq!(
            json,
            "{\"a\":true,\"b\":12345,\"c\":null,\"d\":[],\"e\":{\"a\":1,\"b\":2}}"
        );
    }

    #[test]
    fn test_empty_array() {
        let json = "[true,12345,null,[],{\"a\":1,\"b\":2}]";
        let mut jbl = JBL::new_array().unwrap();
        jbl.append(true).unwrap();
        jbl.append(12345_f64).unwrap();
        jbl.append(JBLValue::Null).unwrap();
        jbl.append(JBLValue::EmptyArray).unwrap();
        jbl.append("{\"a\":1,\"b\":2}".parse::<JBL>().unwrap())
            .unwrap();
        let res: String = jbl.as_json(None).unwrap();
        assert_eq!(res, json);
    }
}
