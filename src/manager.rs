/*! Procedures relating to the management of the switching of layouts */
use ::util;

pub mod c {
    use std::os::raw::{c_char, c_void};

    /// EekboardContextService*
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct Manager(*const c_void);
    
    extern "C" {
        pub fn eekboard_context_service_set_overlay(
            manager: Manager,
            name: *const c_char,
        );

        pub fn eekboard_context_service_get_overlay(
            manager: Manager,
        ) -> *const c_char;
    }
}

/// Returns the overlay name.
/// The result lifetime is "as long as the C copy lives"
pub fn get_overlay(manager: c::Manager) -> Option<String> {
    let raw_str = unsafe {
        c::eekboard_context_service_get_overlay(manager)
    };
    // this string is generated from Rust, should never be invalid
    util::c::as_str(&raw_str).unwrap()
        .map(String::from)
}
