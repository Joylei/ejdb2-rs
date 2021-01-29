#![allow(dead_code)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

extern crate ejdb2_sys;
extern crate rand;
#[macro_use]
extern crate bitflags;

pub mod builder;
mod channel;
pub mod database;
pub mod error;
pub mod exec;
mod ffi;
mod jbl;
pub mod jql;
pub mod printer;
mod utils;
mod xstr;

pub use builder::EJDB2Builder;
pub use database::Database;
pub use error::EjdbError;
pub type Result<T> = core::result::Result<T, EjdbError>;

bitflags! {
    pub struct DatabaseOpenMode: u8 {
        /** Open storage file in read-only mode */
        const IWKV_RDONLY                  = 0x2;
        /** Truncate storage file on open */
        const IWKV_TRUNC                   = 0x4;
    }
}

bitflags! {
    pub struct JsonPrintFlags: u8 {
        const PRINT_PRETTY = 0x1;
        const PRINT_CODEPOINTS =0x2;
    }
}

pub use ffi::ejdb_version;
pub use xstr::{StringPtr, XString};

pub mod precludes {
    pub use crate::{
        builder::EJDB2Builder,
        database::Database,
        error::EjdbError,
        exec::{Query, VisitStep, Visitor},
        jbl::{JBLType, JBLValue},
        jql::{KeyParam, JQL},
        printer::{AsJson, JsonPrinter},
        DatabaseOpenMode, JsonPrintFlags, Result,
    };
}

#[cfg(test)]
mod test;
