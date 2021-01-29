use crate::{
    channel::Channel,
    ffi::{self, c_void},
    utils::check_rc,
    JsonPrintFlags, Result,
};
use core::{cmp, mem, slice};
use ejdb2_sys as sys;
pub trait AsJson<T> {
    /// to JSON string
    fn as_json(&self, flag: Option<JsonPrintFlags>) -> Result<T>;
}

pub trait JsonPrinter {
    fn print(&mut self, buf: &[u8], count: usize) -> Result<()>;
}
#[cfg(not(feature = "std"))]
impl JsonPrinter for crate::XString {
    #[inline]
    fn print(&mut self, buf: &[u8], count: usize) -> Result<()> {
        for _ in 0..count {
            let buf = buf;
            self.push_bytes(buf)?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<T: std::io::Write> JsonPrinter for T {
    #[inline]
    fn print(&mut self, buf: &[u8], count: usize) -> Result<()> {
        for _ in 0..count {
            self.write_all(buf)?;
        }
        self.flush()?;
        Ok(())
    }
}
#[inline]
pub(crate) fn doc_print_json<T: JsonPrinter>(
    doc: *mut sys::_EJDB_DOC,
    target: &mut T,
    flag: JsonPrintFlags,
) -> Result<()> {
    let flag = flag.bits;
    let doc = unsafe { &mut *doc };
    let mut chan = Channel(target, Ok(()));
    let op = &mut chan as *mut _ as *mut c_void;
    let f = print_json::<T>;
    let rc = unsafe {
        if !doc.node.is_null() {
            sys::jbn_as_json(doc.node, Some(f), op, flag)
        } else {
            sys::jbl_as_json(doc.raw, Some(f), op, flag)
        }
    };
    chan.get()?;
    check_rc(rc)
}
#[inline]
pub(crate) fn jbl_print_json<T: JsonPrinter>(
    jbl: sys::JBL,
    target: &mut T,
    flag: JsonPrintFlags,
) -> Result<()> {
    let flag = flag.bits;
    let mut chan = Channel(target, Ok(()));
    let op = &mut chan as *mut _ as *mut c_void;
    let f = print_json::<T>;
    let rc = unsafe { sys::jbl_as_json(jbl, Some(f), op, flag) };
    chan.get()?;
    check_rc(rc)
}

unsafe extern "C" fn print_json<T: JsonPrinter>(
    data: *const i8,
    size: i32,
    ch: i8,
    count: i32,
    op: *mut c_void,
) -> u64 {
    let target = &mut *(op as *mut Channel<&mut T, ()>);
    if data.is_null() {
        if count > 0 {
            let c = mem::transmute(ch);
            let buf = [c];
            target.unwrap_or_default(|p| p.print(&buf, count as usize));
        }
    } else {
        let count = cmp::max(1, count) as usize;
        let len = if size > 0 {
            size as usize
        } else {
            ffi::strlen(data)
        };

        let buf = slice::from_raw_parts(data as *const u8, len as usize);
        target.unwrap_or_default(|p| p.print(buf, count));
    }
    0
}
