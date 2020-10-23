/*! Managing the events belonging to virtual-keyboard interface. */

use ::keyboard::{ Modifiers, PressType };
use ::submission::Timestamp;

/// Standard xkb keycode
type KeyCode = u32;

/// Gathers stuff defined in C or called by C
pub mod c {
    use std::ffi::CStr;
    use std::os::raw::{ c_char, c_void };

    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct ZwpVirtualKeyboardV1(*const c_void);

    #[repr(C)]
    pub struct KeyMap {
        fd: u32,
        fd_len: usize,
    }
    
    impl KeyMap {
        pub fn from_cstr(s: &CStr) -> KeyMap {
            unsafe {
                squeek_key_map_from_str(s.as_ptr())
            }
        }
    }

    impl Drop for KeyMap {
        fn drop(&mut self) {
            unsafe {
                close(self.fd as u32);
            }
        }
    }

    #[no_mangle]
    extern "C" {
        // From libc, to let KeyMap get deallocated.
        fn close(fd: u32);

        pub fn eek_virtual_keyboard_v1_key(
            virtual_keyboard: ZwpVirtualKeyboardV1,
            timestamp: u32,
            keycode: u32,
            press: u32,
        );

        pub fn eek_virtual_keyboard_update_keymap(
            virtual_keyboard: ZwpVirtualKeyboardV1,
            keymap: *const KeyMap,
        );
        
        pub fn eek_virtual_keyboard_set_modifiers(
            virtual_keyboard: ZwpVirtualKeyboardV1,
            modifiers: u32,
        );
        
        pub fn squeek_key_map_from_str(keymap_str: *const c_char) -> KeyMap;
    }
}

/// Layout-independent backend. TODO: Have one instance per program or seat
pub struct VirtualKeyboard(pub c::ZwpVirtualKeyboardV1);

impl VirtualKeyboard {
    // TODO: error out if keymap not set
    pub fn switch(
        &self,
        keycode: KeyCode,
        action: PressType,
        timestamp: Timestamp,
    ) {
        let keycode = keycode - 8;
        unsafe {
            c::eek_virtual_keyboard_v1_key(
                self.0, timestamp.0, keycode, action.clone() as u32
            );
        }
    }
    
    pub fn set_modifiers_state(&self, modifiers: Modifiers) {
        let modifiers = modifiers.bits() as u32;
        unsafe {
            c::eek_virtual_keyboard_set_modifiers(self.0, modifiers);
        }
    }
    
    pub fn update_keymap(&self, keymap: &c::KeyMap) {
        unsafe {
            c::eek_virtual_keyboard_update_keymap(
                self.0,
                keymap as *const c::KeyMap,
            );
        }
    }
}
