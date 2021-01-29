use ejdb2_sys as sys;

use crate::{ffi::c_char, utils::check_rc, EjdbError, Result};
use core::{
    cmp,
    convert::From,
    ffi::c_void,
    fmt,
    ops::{Deref, DerefMut},
    slice,
};

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::string::String;
#[cfg(feature = "std")]
use std::ffi::{CStr, CString};

/// iwxstr
pub struct XString {
    ptr: *mut sys::IWXSTR,
}

impl XString {
    #[inline(always)]
    pub fn new() -> Self {
        let ptr = unsafe { sys::iwxstr_new() };
        Self::from_ptr(ptr)
    }
    /// allocate buffer with specified size to avoid reallocation
    #[inline(always)]
    pub fn new_with_size(size: usize) -> Self {
        let ptr = unsafe { sys::iwxstr_new2(size as sys::size_t) };
        Self::from_ptr(ptr)
    }

    #[inline(always)]
    pub(crate) fn from_ptr(ptr: *mut sys::IWXSTR) -> Self {
        Self { ptr }
    }

    /// copy bytes
    #[inline(always)]
    pub fn from_str_ptr(ptr: *const c_char) -> Self {
        let len = unsafe { crate::ffi::strlen(ptr) };
        let v = unsafe { slice::from_raw_parts(ptr as *const u8, len) };
        Self::from(v)
    }

    #[inline(always)]
    pub(crate) fn as_mut_ptr(&self) -> *mut sys::IWXSTR {
        self.ptr
    }

    /// str len
    #[inline(always)]
    pub fn size(&self) -> usize {
        unsafe { sys::iwxstr_size(self.as_mut_ptr()) as usize }
    }

    #[inline(always)]
    pub fn clear(&mut self) -> &mut Self {
        unsafe {
            sys::iwxstr_clear(self.as_mut_ptr());
        }
        self
    }
    #[inline(always)]
    pub fn pop(&mut self, pop_size: usize) -> &mut Self {
        unsafe {
            sys::iwxstr_pop(self.as_mut_ptr(), pop_size as u64);
        }
        self
    }
    #[inline(always)]
    pub fn shift(&mut self, shift_size: usize) -> &mut Self {
        unsafe {
            sys::iwxstr_shift(self.as_mut_ptr(), shift_size as u64);
        }
        self
    }

    #[inline(always)]
    pub fn push(&mut self, buf: impl AsRef<str>) -> &mut Self {
        self.push_bytes(buf.as_ref().as_bytes()).unwrap();
        self
    }

    #[inline(always)]
    pub fn unshift(&mut self, buf: impl AsRef<str>) -> &mut Self {
        self.unshift_bytes(buf.as_ref().as_bytes()).unwrap();
        self
    }

    #[inline]
    pub(crate) fn push_bytes(&mut self, buf: &[u8]) -> Result<()> {
        unsafe {
            let ptr = buf.as_ptr() as *const _;
            let rc = sys::iwxstr_cat(self.as_mut_ptr(), ptr, buf.len() as u64);
            if rc != 0 {
                return Err(EjdbError::AllocError);
            }
        }
        Ok(())
    }
    #[inline]
    pub(crate) fn unshift_bytes(&mut self, buf: &[u8]) -> Result<()> {
        let rc = unsafe {
            let ptr = buf.as_ptr() as *mut c_void;
            sys::iwxstr_unshift(self.as_mut_ptr(), ptr, buf.len() as u64)
        };
        check_rc(rc)
    }

    /// as C str ptr
    #[inline]
    pub fn as_ptr(&self) -> *const c_char {
        let ptr = unsafe { sys::iwxstr_ptr(self.as_mut_ptr()) };
        debug_assert!(!ptr.is_null());
        ptr
    }

    /// take inner ptr
    // #[inline]
    // pub fn into_inner(self) -> *mut sys::IWXSTR {
    //     self.ptr
    // }

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        let bytes: &[u8] = self.as_ref();
        Vec::from(bytes)
    }

    #[inline(always)]
    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            let ptr = sys::iwxstr_ptr(self.as_mut_ptr());
            let len = self.size();
            let slice = core::ptr::slice_from_raw_parts(ptr as *mut u8, len);
            &*slice
        }
    }

    #[inline(always)]
    pub fn to_bytes_mut(&self) -> &mut [u8] {
        unsafe {
            let ptr = sys::iwxstr_ptr(self.as_mut_ptr());
            let len = self.size();
            let slice = core::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len);
            &mut *slice
        }
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        let bytes = self.to_bytes();
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }

    #[inline(always)]
    pub fn as_str_mut(&mut self) -> &mut str {
        let bytes = self.to_bytes_mut();
        unsafe { core::str::from_utf8_unchecked_mut(bytes) }
    }
}

impl Default for XString {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for XString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for XString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XString{{\"{}\"}}", self.as_str())
    }
}

impl fmt::Write for XString {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push(s);
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::io::Write for XString {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.push_bytes(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }
    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[cfg(feature = "std")]
impl std::io::Read for XString {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = cmp::min(buf.len(), self.size());
        let src: &[u8] = self.as_ref();
        for (d, s) in buf[..len].iter_mut().zip(src[..len].iter()) {
            *d = *s;
        }
        self.shift(len);
        Ok(len)
    }
}
#[cfg(feature = "std")]
impl AsRef<std::ffi::CStr> for XString {
    #[inline(always)]
    fn as_ref(&self) -> &std::ffi::CStr {
        unsafe {
            let ptr = sys::iwxstr_ptr(self.as_mut_ptr());
            std::ffi::CStr::from_ptr(ptr)
        }
    }
}

impl AsRef<str> for XString {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for XString {
    type Target = str;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for XString {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_str_mut()
    }
}

impl AsRef<[c_char]> for XString {
    #[inline(always)]
    fn as_ref(&self) -> &[c_char] {
        unsafe {
            let ptr = sys::iwxstr_ptr(self.as_mut_ptr());
            let len = self.size();
            let slice = core::ptr::slice_from_raw_parts(ptr, len);
            &*slice
        }
    }
}

impl AsRef<[u8]> for XString {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.to_bytes()
    }
}

impl core::ops::Add for XString {
    type Output = XString;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let mut v = self;
        v.push(rhs.as_str());
        v
    }
}

impl Drop for XString {
    #[inline(always)]
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                sys::iwxstr_destroy(self.ptr);
            }
        }
    }
}

impl From<&[u8]> for XString {
    #[inline]
    fn from(data: &[u8]) -> Self {
        let mut this = XString::new_with_size(data.len());
        this.push_bytes(data).unwrap();
        this
    }
}

#[cfg(feature = "std")]
impl From<&CStr> for XString {
    #[inline]
    fn from(data: &CStr) -> Self {
        let bytes = data.to_bytes();
        bytes.into()
    }
}

impl From<&str> for XString {
    #[inline]
    fn from(data: &str) -> Self {
        let buf = data.as_bytes();
        let mut this = XString::new_with_size(buf.len());
        this.push(data);
        this
    }
}
#[cfg(feature = "std")]
impl From<String> for XString {
    #[inline(always)]
    fn from(s: String) -> Self {
        let s: &str = &s;
        Self::from(s)
    }
}

impl Clone for XString {
    #[inline]
    fn clone(&self) -> Self {
        let s: &[u8] = self.as_ref();
        let mut dst = XString::new_with_size(self.size());
        dst.push_bytes(s).unwrap();
        dst
    }
}

impl PartialEq<[u8]> for XString {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        let bytes = self.to_bytes();
        bytes.eq(other)
    }
}

impl<T: AsRef<str>> PartialEq<T> for XString {
    #[inline(always)]
    fn eq(&self, other: &T) -> bool {
        let other = other.as_ref();
        self.size() == other.len() && self.to_bytes().eq(other.as_bytes())
    }
}

impl Eq for XString {}

/// repr c string, either value or reference
#[derive(Debug)]
pub enum StringPtr<'a> {
    XString(XString),
    XStringRef(&'a XString),
    #[cfg(feature = "std")]
    CString(CString),
    #[cfg(feature = "std")]
    CStr(&'a CStr),
}

impl StringPtr<'_> {
    #[inline]
    pub(crate) fn as_ptr(&self) -> *const c_char {
        match self {
            StringPtr::XString(v) => v.as_ptr(),
            StringPtr::XStringRef(v) => v.as_ptr(),
            #[cfg(feature = "std")]
            StringPtr::CString(v) => v.as_ptr(),
            #[cfg(feature = "std")]
            StringPtr::CStr(v) => v.as_ptr(),
        }
    }

    #[inline]
    pub(crate) fn to_owned(self) -> XString {
        match self {
            StringPtr::XString(v) => v,
            StringPtr::XStringRef(v) => v.clone(),
            #[cfg(feature = "std")]
            StringPtr::CString(v) => v.as_c_str().into(),
            #[cfg(feature = "std")]
            StringPtr::CStr(v) => v.into(),
        }
    }
}

impl From<XString> for StringPtr<'_> {
    #[inline(always)]
    fn from(s: XString) -> Self {
        StringPtr::XString(s)
    }
}

impl<'a> From<&'a XString> for StringPtr<'a> {
    #[inline(always)]
    fn from(s: &'a XString) -> Self {
        StringPtr::XStringRef(s)
    }
}
#[cfg(feature = "std")]
impl From<String> for StringPtr<'_> {
    #[inline]
    fn from(s: String) -> Self {
        StringPtr::XString(s.into())
    }
}
#[cfg(feature = "std")]
impl From<&String> for StringPtr<'_> {
    #[inline]
    fn from(s: &String) -> Self {
        let s: &str = &s;
        StringPtr::XString(s.into())
    }
}

impl<'a> From<&'a str> for StringPtr<'a> {
    #[inline]
    fn from(s: &'a str) -> Self {
        StringPtr::XString(s.into())
    }
}
#[cfg(feature = "std")]
impl From<CString> for StringPtr<'_> {
    #[inline(always)]
    fn from(s: CString) -> Self {
        StringPtr::CString(s)
    }
}
#[cfg(feature = "std")]
impl<'a> From<&'a CStr> for StringPtr<'a> {
    #[inline(always)]
    fn from(s: &'a CStr) -> Self {
        StringPtr::CStr(s)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_slice() {
        let buf = b"hello";
        let xstr: XString = buf[..].into();
        assert_eq!(xstr.size(), buf.len());
    }

    #[test]
    fn test_xstr() {
        let mut xstr: XString = XString::new();
        assert_eq!(xstr.size(), 0);
        let buf = "hello";
        xstr.push(buf);
        assert_eq!(xstr.size(), 5);
        let buf = "world";
        xstr.push(buf);
        assert_eq!(xstr.size(), 10);
        xstr.pop(3);
        assert_eq!(xstr.size(), 7);
        let buf = "abcd";
        xstr.unshift(buf);
        assert_eq!(xstr.size(), 11);
        xstr.shift(5);
        assert_eq!(xstr.size(), 6);
    }
}
