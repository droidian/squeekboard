/*! Managing the events belonging to virtual-keyboard interface. */

use ::keyboard::{ KeyCode, Modifiers, PressType };
use ::layout::c::LevelKeyboard;
use ::submission::Timestamp;

/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;
    use std::os::raw::c_void;

    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct ZwpVirtualKeyboardV1(*const c_void);

    #[no_mangle]
    extern "C" {
        pub fn eek_virtual_keyboard_v1_key(
            virtual_keyboard: ZwpVirtualKeyboardV1,
            timestamp: u32,
            keycode: u32,
            press: u32,
        );

        pub fn eek_virtual_keyboard_update_keymap(
            virtual_keyboard: ZwpVirtualKeyboardV1,
            keyboard: LevelKeyboard,
        );
        
        pub fn eek_virtual_keyboard_set_modifiers(
            virtual_keyboard: ZwpVirtualKeyboardV1,
            modifiers: u32,
        );
    }
}

/// Layout-independent backend. TODO: Have one instance per program or seat
pub struct VirtualKeyboard(pub c::ZwpVirtualKeyboardV1);

impl VirtualKeyboard {
    // TODO: error out if keymap not set
    pub fn switch(
        &self,
        keycodes: &[KeyCode],
        action: PressType,
        timestamp: Timestamp,
    ) {
        let keycodes_count = keycodes.len();
        for keycode in keycodes.iter() {
            let keycode = keycode - 8;
            match (action, keycodes_count) {
                // Pressing a key made out of a single keycode is simple:
                // press on press, release on release.
                (_, 1) => unsafe {
                    c::eek_virtual_keyboard_v1_key(
                        self.0, timestamp.0, keycode, action.clone() as u32
                    );
                },
                // A key made of multiple keycodes
                // has to submit them one after the other
                (PressType::Pressed, _) => unsafe {
                    c::eek_virtual_keyboard_v1_key(
                        self.0, timestamp.0, keycode, PressType::Pressed as u32
                    );
                    c::eek_virtual_keyboard_v1_key(
                        self.0, timestamp.0, keycode, PressType::Released as u32
                    );
                },
                // Design choice here: submit multiple all at press time
                // and do nothing at release time
                (PressType::Released, _) => {},
            }
        }
    }
    
    pub fn set_modifiers_state(&self, modifiers: Modifiers) {
        let modifiers = modifiers.bits() as u32;
        unsafe {
            c::eek_virtual_keyboard_set_modifiers(self.0, modifiers);
        }
    }
    
    pub fn update_keymap(&self, keyboard: LevelKeyboard) {
        unsafe {
            c::eek_virtual_keyboard_update_keymap(self.0, keyboard);
        }
    }
}
