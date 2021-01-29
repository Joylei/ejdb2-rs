use core::slice;
use ejdb2_sys as sys;

pub use core::ffi::c_void;
pub use libc::c_char;
pub use libc::strlen;

pub fn ejdb_version() -> (u32, u32, u32) {
    unsafe {
        (
            sys::ejdb_version_major(),
            sys::ejdb_version_minor(),
            sys::ejdb_version_patch(),
        )
    }
}

#[inline]
pub fn iwlog_ecode_explained<'a>(rc: u64) -> &'a str {
    let ptr = unsafe { sys::iwlog_ecode_explained(rc) };
    if ptr.is_null() {
        return Default::default();
    }
    let len = unsafe { strlen(ptr) };
    let v = unsafe { slice::from_raw_parts(ptr as *const u8, len) };
    unsafe { core::str::from_utf8_unchecked(v) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ejdb_version() {
        assert!(ejdb_version() == (2, 0, 59));
    }
}
