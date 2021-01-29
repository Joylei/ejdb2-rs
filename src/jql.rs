use core::ptr;

use crate::{ffi::c_char, jbl::JBL, utils::check_rc, xstr::StringPtr, EjdbError, Result, XString};
use ejdb2_sys as sys;

const JQL_KEEP_QUERY_ON_PARSE_ERROR: u8 = 0x1;
const JQL_SILENT_ON_PARSE_ERROR: u8 = 0x2;

#[inline(always)]
fn jql_error(handle: sys::JQL) -> XString {
    let msg = unsafe { sys::jql_error(handle) };
    XString::from_str_ptr(msg)
}

#[derive(Debug)]
pub struct JQL {
    handle: sys::JQL,
}

impl JQL {
    #[inline]
    pub fn create<'a>(query: impl Into<StringPtr<'a>>) -> Result<Self> {
        let query = query.into();
        Self::create_jql(query, None)
    }

    #[inline]
    pub fn create_with_collection<'a, 'b>(
        query: impl Into<StringPtr<'a>>,
        collection: impl Into<StringPtr<'b>>,
    ) -> Result<Self> {
        let query = query.into();
        let coll = collection.into();
        Self::create_jql(query, Some(coll))
    }
    #[inline]
    fn create_jql<'a, 'b>(query: StringPtr<'a>, coll: Option<StringPtr<'b>>) -> Result<Self> {
        let mut handle = ptr::null_mut();
        let mode = JQL_KEEP_QUERY_ON_PARSE_ERROR | JQL_SILENT_ON_PARSE_ERROR;
        let coll_ptr = match coll {
            Some(v) => v.as_ptr(),
            None => ptr::null(),
        };
        let rc = unsafe { sys::jql_create2(&mut handle, coll_ptr, query.as_ptr(), mode) };
        if rc != 0 {
            let e = EjdbError::JQLParseError {
                rc,
                error: jql_error(handle),
            };
            unsafe {
                sys::jql_destroy(&mut handle);
            }
            return Err(e);
        }
        Ok(Self { handle })
    }

    #[inline(always)]
    pub(crate) fn raw_ptr(&self) -> sys::JQL {
        self.handle
    }

    /// collection name from query
    #[inline]
    pub fn collection(&self) -> Result<XString> {
        let c_str = unsafe { sys::jql_collection(self.raw_ptr()) };
        let res = XString::from_str_ptr(c_str);
        Ok(res)
    }

    #[inline]
    pub fn skip(&self) -> Result<i64> {
        let mut num = 0;
        let rc = unsafe {
            let num = &mut num as *mut i64;
            sys::jql_get_skip(self.raw_ptr(), num)
        };
        check_rc(rc).and(Ok(num))
    }

    #[inline]
    pub fn limit(&self) -> Result<i64> {
        let mut num = 0;
        let rc = unsafe {
            let num = &mut num as *mut i64;
            sys::jql_get_limit(self.raw_ptr(), num)
        };
        check_rc(rc).and(Ok(num))
    }

    #[inline]
    pub fn set_i64<'a>(&self, key: impl Into<KeyParam<'a>>, val: i64) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let rc = unsafe { sys::jql_set_i64(self.raw_ptr(), key.as_ptr(), key.as_index(), val) };
        check_rc(rc)
    }

    #[inline]
    pub fn set_bool<'a>(&self, key: impl Into<KeyParam<'a>>, val: bool) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let rc = unsafe { sys::jql_set_bool(self.raw_ptr(), key.as_ptr(), key.as_index(), val) };
        check_rc(rc)
    }

    #[inline]
    pub fn set_f64<'a>(&self, key: impl Into<KeyParam<'a>>, val: f64) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let rc = unsafe { sys::jql_set_f64(self.raw_ptr(), key.as_ptr(), key.as_index(), val) };
        check_rc(rc)
    }

    #[inline]
    pub(crate) fn set_json<'a, 'b>(
        &self,
        key: impl Into<KeyParam<'a>>,
        val: impl Into<StringPtr<'b>>,
    ) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let jbl = JBL::from_json(val)?;
        let rc = unsafe {
            sys::jql_set_json_jbl(self.raw_ptr(), key.as_ptr(), key.as_index(), jbl.raw_ptr())
        };
        check_rc(rc)
    }

    #[inline]
    pub(crate) fn set_json_jbn<'a>(
        &self,
        key: impl Into<KeyParam<'a>>,
        val: sys::JBL_NODE,
    ) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let rc = unsafe { sys::jql_set_json(self.raw_ptr(), key.as_ptr(), key.as_index(), val) };
        check_rc(rc)
    }

    #[inline]
    pub(crate) fn set_json_jbl<'a, 'j>(
        &'j self,
        key: impl Into<KeyParam<'a>>,
        val: &'j JBL,
    ) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let rc = unsafe {
            sys::jql_set_json_jbl(self.raw_ptr(), key.as_ptr(), key.as_index(), val.raw_ptr())
        };
        check_rc(rc)
    }

    #[inline]
    pub fn set_null<'a>(&self, key: impl Into<KeyParam<'a>>) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let rc = unsafe { sys::jql_set_null(self.raw_ptr(), key.as_ptr(), key.as_index()) };
        check_rc(rc)
    }

    #[inline]
    pub fn set_regex<'a, 'b>(
        &self,
        key: impl Into<KeyParam<'a>>,
        expr: impl Into<StringPtr<'b>>,
    ) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let expr = expr.into();
        let rc = unsafe {
            sys::jql_set_regexp(self.raw_ptr(), key.as_ptr(), key.as_index(), expr.as_ptr())
        };
        check_rc(rc)
    }

    #[inline]
    pub fn set_str<'a, 'b>(
        &self,
        key: impl Into<KeyParam<'a>>,
        val: impl Into<StringPtr<'b>>,
    ) -> Result<()> {
        let key: KeyParam<'_> = key.into();
        let val = val.into();
        let rc =
            unsafe { sys::jql_set_str(self.raw_ptr(), key.as_ptr(), key.as_index(), val.as_ptr()) };

        check_rc(rc)
    }

    #[inline]
    pub fn reset(&self, reset_match_cache: bool, reset_placeholders: bool) -> &Self {
        unsafe { sys::jql_reset(self.raw_ptr(), reset_match_cache, reset_placeholders) };
        self
    }
}

impl Drop for JQL {
    #[inline]
    fn drop(&mut self) {
        unsafe { sys::jql_destroy(&mut self.handle) };
    }
}

/// repr either index or name
#[derive(Debug)]
pub struct KeyParam<'a> {
    index: i32,
    name: Option<StringPtr<'a>>,
}

impl KeyParam<'_> {
    /// number if key is index, otherwise 0
    #[inline]
    pub(crate) fn as_index(&self) -> i32 {
        self.index
    }

    /// name if key is str, otherwise nullptr
    #[inline]
    pub(crate) fn as_ptr(&self) -> *const c_char {
        match self.name {
            Some(ref v) => v.as_ptr(),
            None => ptr::null(),
        }
    }
}

impl From<u32> for KeyParam<'_> {
    #[inline(always)]
    fn from(v: u32) -> Self {
        Self {
            index: v as i32,
            name: None,
        }
    }
}

impl<'a> From<XString> for KeyParam<'a> {
    #[inline]
    fn from(v: XString) -> Self {
        Self {
            index: 0,
            name: Some(v.into()),
        }
    }
}

impl<'a> From<&'a XString> for KeyParam<'a> {
    #[inline]
    fn from(v: &'a XString) -> Self {
        Self {
            index: 0,
            name: Some(v.into()),
        }
    }
}
impl<'a> From<&'a str> for KeyParam<'a> {
    #[inline]
    fn from(v: &'a str) -> Self {
        Self {
            index: 0,
            name: Some(v.into()),
        }
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a> From<&'a String> for KeyParam<'a> {
    #[inline]
    fn from(v: &'a String) -> Self {
        Self {
            index: 0,
            name: Some(v.into()),
        }
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a> From<String> for KeyParam<'a> {
    #[inline]
    fn from(v: String) -> Self {
        Self {
            index: 0,
            name: Some(v.into()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_jql_invalid() {
        let res = JQL::create("*****");
        assert!(res.is_err());
    }

    #[test]
    fn test_jql_collection_name() {
        let query = JQL::create("@abc/*").unwrap();
        let name = query.collection().unwrap();
        assert_eq!(name, "abc");
    }

    #[test]
    fn test_jql_limit_not_set() {
        let query = JQL::create("@abc/*").unwrap();
        let limit = query.limit().unwrap();
        let skip = query.skip().unwrap();
        assert_eq!(limit, 0);
        assert_eq!(skip, 0);
    }

    #[test]
    fn test_jql_limit() {
        let query = JQL::create("@c1/* |limit 2 skip 3").unwrap();
        let limit = query.limit().unwrap();
        let skip = query.skip().unwrap();
        assert_eq!(limit, 2);
        assert_eq!(skip, 3);
    }

    #[test]
    fn test_jql_named_params() {
        let query = JQL::create("@c1/[name=:name and age=:age]").unwrap();
        query.set_str("name", "lily").unwrap();
        query.set_i64("age", 18).unwrap();
    }

    #[test]
    fn test_jql_indexed_params() {
        let query = JQL::create("@c1/[name=:? and age=:?]").unwrap();
        query.set_str(0, "john").unwrap();
        query.set_i64(1, 20).unwrap();
    }
}
