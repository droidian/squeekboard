/*! Locale-specific functions. 
 * 
 * This file is intended as a library:
 * it must pass errors upwards
 * and panicking is allowed only when
 * this code encounters an internal inconsistency.
 */

use std::cmp;
use std::ffi::{ CStr, CString };
use std::fmt;
use std::os::raw::c_char;
use std::ptr;
use std::str::Utf8Error;

mod c {
    use super::*;
    use std::os::raw::c_void;

    #[allow(non_camel_case_types)]
    pub type c_int = i32;

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct GnomeXkbInfo(*const c_void);

    extern "C" {
        // from libc
        pub fn strcoll(cs: *const c_char, ct: *const c_char) -> c_int;
        // from gnome-desktop3
        pub fn gnome_xkb_info_new() -> GnomeXkbInfo;
        pub fn gnome_xkb_info_get_layout_info (
            info: GnomeXkbInfo,
            id: *const c_char,
            display_name: *mut *const c_char,
            short_name: *const *const c_char,
            xkb_layout: *const *const c_char,
            xkb_variant: *const *const c_char
        ) -> c_int;
        pub fn g_object_unref(o: GnomeXkbInfo);
    }
}

#[derive(Debug)]
pub enum Error {
    StringConversion(Utf8Error),
    NoInfo,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

pub struct XkbInfo(c::GnomeXkbInfo);

impl XkbInfo {
    pub fn new() -> XkbInfo {
        XkbInfo(unsafe { c::gnome_xkb_info_new() })
    }
    pub fn get_display_name(&self, id: &str) -> Result<String, Error> {
        let id = cstring_safe(id);
        let id_ref = id.as_ptr();
        let mut display_name: *const c_char = ptr::null();
        let found = unsafe {
            c::gnome_xkb_info_get_layout_info(
                self.0,
                id_ref,
                &mut display_name as *mut *const c_char,
                ptr::null(), ptr::null(), ptr::null(),
            )
        };
        if found != 0 && !display_name.is_null() {
            let display_name = unsafe { CStr::from_ptr(display_name) };
            display_name.to_str()
                .map(str::to_string)
                .map_err(Error::StringConversion)
        } else {
            Err(Error::NoInfo)
        }
    }
}

impl Drop for XkbInfo {
    fn drop(&mut self) {
        unsafe { c::g_object_unref(self.0) }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedTranslation(pub String);

fn cstring_safe(s: &str) -> CString {
    CString::new(s)
        .unwrap_or(CString::new("").unwrap())
}

pub fn compare_current_locale(a: &str, b: &str) -> cmp::Ordering {
    let a = cstring_safe(a);
    let b = cstring_safe(b);
    let a = a.as_ptr();
    let b = b.as_ptr();
    let result = unsafe { c::strcoll(a, b) };
    if result == 0 {
        cmp::Ordering::Equal
    } else if result > 0 {
        cmp::Ordering::Greater
    } else if result < 0 {
        cmp::Ordering::Less
    } else {
        unreachable!()
    }
}
