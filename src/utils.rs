use crate::{EjdbError, Result};

#[inline(always)]
pub fn check_rc(rc: u64) -> Result<()> {
    if rc != 0 {
        Err(EjdbError::Generic(rc))
    } else {
        Ok(())
    }
}

#[cfg(feature = "std")]
pub use std::panic::catch_unwind;

#[cfg(not(feature = "std"))]
#[inline]
pub fn catch_unwind<F: FnOnce() -> R, R>(f: F) -> crate::Result<R> {
    let v = (f)();
    Ok(v)
}
