use core::ops::{Deref, DerefMut};

use crate::Result;

/// capture Result for indirect communication
pub(crate) struct Channel<T, R>(pub T, pub Result<R>);

impl<T, R: Default> Channel<T, R> {
    /// returns result if OK, otherwise capture Error and returns default
    #[inline(always)]
    pub fn unwrap_or_default<F: FnOnce(&mut T) -> Result<R>>(&mut self, f: F) -> R {
        self.unwrap(Default::default(), f)
    }
}

impl<T, R> Channel<T, R> {
    /// returns result if OK, otherwise capture Error and returns default
    #[inline(always)]
    pub fn unwrap<F: FnOnce(&mut T) -> Result<R>>(&mut self, default: R, f: F) -> R {
        let res = (f)(&mut self.0);
        match res {
            Ok(v) => v,
            Err(e) => {
                self.1 = Err(e);
                default
            }
        }
    }

    /// returns result if OK, otherwise capture Error and returns default
    #[inline(always)]
    pub fn catch<F: FnOnce(&mut T) -> Result<R>>(&mut self, default: R, f: F) -> Result<R> {
        let res = (f)(&mut self.0);
        match res {
            Ok(v) => Ok(v),
            Err(e) => {
                self.1 = Err(e);
                Ok(default)
            }
        }
    }

    /// set result
    #[inline(always)]
    pub fn set(&mut self, v: Result<R>) {
        self.1 = v;
    }

    /// extract result
    #[inline(always)]
    pub fn get(self) -> Result<R> {
        self.1
    }
}

impl<T, R> Deref for Channel<T, R> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, R> DerefMut for Channel<T, R> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
