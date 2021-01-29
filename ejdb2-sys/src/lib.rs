#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// Due to this issue we currently have to supress this kind of warning
// https://github.com/rust-lang/rust-bindgen/issues/1549
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
