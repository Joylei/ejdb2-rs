use crate::{ffi::iwlog_ecode_explained as decode, xstr::XString};
use core::{any::Any, fmt, str::Utf8Error};
#[cfg(feature = "std")]
use std::{error::Error as StdError, ffi::NulError, io};

pub enum EjdbError {
    /// EJDB2 library init error
    InitError(u64),
    /// Database open error
    OpenError {
        rc: u64,
        file: XString,
    },
    /// allocation failure
    AllocError,
    /// invalid json data
    InvalidJson(u64),
    /// invalid json data
    Utf8Error(Utf8Error),
    /// generic EJDB2 error
    Generic(u64),

    JQLParseError {
        rc: u64,
        error: XString,
    },

    /// IO related error
    #[cfg(feature = "std")]
    IoError(io::Error),

    /// Panic from catch_unwind
    #[cfg(feature = "std")]
    Panic(Box<dyn Any + Send>),

    /// Other errors
    #[cfg(feature = "std")]
    Other(Box<dyn StdError + 'static>),
}

impl fmt::Debug for EjdbError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl fmt::Display for EjdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitError(rc) => write!(f, "Failed to init EJDB2 library: {}", decode(*rc)),
            Self::OpenError { rc, file } => {
                write!(
                    f,
                    "Failed to open EJDB2 database file ({}): {}",
                    file,
                    decode(*rc)
                )
            }
            Self::Generic(rc) => write!(f, "EJDB2 error: {}", decode(*rc)),
            Self::JQLParseError { rc, error } => {
                write!(f, "{}: {}", decode(*rc), error)
            }
            Self::AllocError => write!(f, "Failed to allocate memory"),
            Self::InvalidJson(rc) => write!(f, "Invalid json data: {}", decode(*rc)),
            Self::Utf8Error(e) => write!(f, "IO error: {}", e),
            #[cfg(feature = "std")]
            Self::IoError(e) => write!(f, "IO error: {}", e),
            #[cfg(feature = "std")]
            Self::Panic(_e) => write!(f, "Unwind panic captured"),
            #[cfg(feature = "std")]
            Self::Other(e) => write!(f, "Error occurs: {}", e),
        }
    }
}
#[cfg(feature = "std")]
impl From<NulError> for EjdbError {
    #[inline]
    fn from(e: NulError) -> Self {
        Self::Other(Box::new(e))
    }
}

impl From<Utf8Error> for EjdbError {
    #[inline]
    fn from(e: Utf8Error) -> Self {
        Self::Utf8Error(e)
    }
}
#[cfg(feature = "std")]
impl From<io::Error> for EjdbError {
    #[inline]
    fn from(e: io::Error) -> Self {
        Self::IoError(e)
    }
}
#[cfg(feature = "std")]
impl StdError for EjdbError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

unsafe impl Send for EjdbError {}
unsafe impl Sync for EjdbError {}
