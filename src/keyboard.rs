/*! State of the emulated keyboard and keys.
 * Regards the keyboard as if it was composed of switches. */

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::rc::Rc;
use std::string::FromUtf8Error;

use ::action::Action;
use ::logging;

// Traits
use std::io::Write;
use std::iter::{ FromIterator, IntoIterator };

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PressType {
    Released = 0,
    Pressed = 1,
}

pub type KeyCode = u32;

bitflags!{
    /// Map to `virtual_keyboard.modifiers` modifiers values
    /// From https://www.x.org/releases/current/doc/kbproto/xkbproto.html#Keyboard_State
    pub struct Modifiers: u8 {
        const SHIFT = 0x1;
        const LOCK = 0x2;
        const CONTROL = 0x4;
        /// Alt
        const MOD1 = 0x8;
        const MOD2 = 0x10;
        const MOD3 = 0x20;
        /// Meta
        const MOD4 = 0x40;
        /// AltGr
        const MOD5 = 0x80;
    }
}

/// When the submitted actions of keys need to be tracked,
/// they need a stable, comparable ID
#[derive(Clone, PartialEq)]
pub struct KeyStateId(*const KeyState);

#[derive(Debug, Clone)]
pub struct KeyState {
    pub pressed: PressType,
    /// A cache of raw keycodes derived from Action::Submit given a keymap
    pub keycodes: Vec<KeyCode>,
    /// Static description of what the key does when pressed or released
    pub action: Action,
}

impl KeyState {
    #[must_use]
    pub fn into_released(self) -> KeyState {
        KeyState {
            pressed: PressType::Released,
            ..self
        }
    }

    #[must_use]
    pub fn into_pressed(self) -> KeyState {
        KeyState {
            pressed: PressType::Pressed,
            ..self
        }
    }

    /// KeyStates instances are the unique identifiers of pressed keys,
    /// and the actions submitted with them.
    pub fn get_id(keystate: &Rc<RefCell<KeyState>>) -> KeyStateId {
        KeyStateId(keystate.as_ptr() as *const KeyState)
    }
}

/// Sorts an iterator by converting it to a Vector and back
fn sorted<'a, I: Iterator<Item=&'a str>>(
    iter: I
) -> impl Iterator<Item=&'a str> {
    let mut v: Vec<&'a str> = iter.collect();
    v.sort();
    v.into_iter()
}

/// Generates a mapping where each key gets a keycode, starting from ~~8~~
/// HACK: starting from 9, because 8 results in keycode 0,
/// which the compositor likes to discard
pub fn generate_keycodes<'a, C: IntoIterator<Item=&'a str>>(
    key_names: C
) -> HashMap<String, u32> {
    let special_keysyms = ["BackSpace", "Return"].iter().map(|&s| s);
    HashMap::from_iter(
        // sort to remove a source of indeterminism in keycode assignment
        sorted(key_names.into_iter().chain(special_keysyms))
            .map(|name| String::from(name))
            .zip(9..)
    )
}

#[derive(Debug)]
pub enum FormattingError {
    Utf(FromUtf8Error),
    Format(io::Error),
}

impl fmt::Display for FormattingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormattingError::Utf(e) => write!(f, "UTF: {}", e),
            FormattingError::Format(e) => write!(f, "Format: {}", e),
        }
    }
}

impl From<io::Error> for FormattingError {
    fn from(e: io::Error) -> Self {
        FormattingError::Format(e)
    }
}

/// Generates a de-facto single level keymap.
// TODO: don't rely on keys and their order,
// but rather on what keysyms and keycodes are in use.
// Iterating actions makes it hard to deduplicate keysyms.
pub fn generate_keymap(
    keystates: &HashMap::<String, KeyState>
) -> Result<String, FormattingError> {
    let mut buf: Vec<u8> = Vec::new();
    writeln!(
        buf,
        "xkb_keymap {{

    xkb_keycodes \"squeekboard\" {{
        minimum = 8;
        maximum = 255;"
    )?;
    
    for (name, state) in keystates.iter() {
        match &state.action {
            Action::Submit { text: _, keys } => {
                if let 0 = keys.len() {
                    log_print!(
                        logging::Level::Warning,
                        "Key {} has no keysyms", name,
                    );
                };
                for (named_keysym, keycode) in keys.iter().zip(&state.keycodes) {
                    write!(
                        buf,
                        "
        <{}> = {};",
                        named_keysym.0,
                        keycode,
                    )?;
                }
            },
            Action::Erase => {
                let mut keycodes = state.keycodes.iter();
                write!(
                    buf,
                    "
        <BackSpace> = {};",
                    keycodes.next().expect("Erase key has no keycode"),
                )?;
                if let Some(_) = keycodes.next() {
                    log_print!(
                        logging::Level::Bug,
                        "Erase key has multiple keycodes",
                    );
                }
            },
            _ => {},
        }
    }
    
    writeln!(
        buf,
        "
    }};
    
    xkb_symbols \"squeekboard\" {{

        name[Group1] = \"Letters\";
        name[Group2] = \"Numbers/Symbols\";
        
        key <BackSpace> {{ [ BackSpace ] }};"
    )?;
    
    for (_name, state) in keystates.iter() {
        if let Action::Submit { text: _, keys } = &state.action {
            for keysym in keys.iter() {
                write!(
                    buf,
                    "
        key <{}> {{ [ {0} ] }};",
                    keysym.0,
                )?;
            }
        }
    }
    writeln!(
        buf,
        "
    }};

    xkb_types \"squeekboard\" {{

	type \"TWO_LEVEL\" {{
            modifiers = Shift;
            map[Shift] = Level2;
            level_name[Level1] = \"Base\";
            level_name[Level2] = \"Shift\";
	}};
    }};

    xkb_compatibility \"squeekboard\" {{
    }};
}};"
    )?;
    
    //println!("{}", String::from_utf8(buf.clone()).unwrap());
    String::from_utf8(buf).map_err(FormattingError::Utf)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use xkbcommon::xkb;

    use ::action::KeySym;

    #[test]
    fn test_keymap_multi() {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

        let keymap_str = generate_keymap(&hashmap!{
            "ac".into() => KeyState {
                action: Action::Submit {
                    text: None,
                    keys: vec!(KeySym("a".into()), KeySym("c".into())),
                },
                keycodes: vec!(9, 10),
                pressed: PressType::Released,
            },
        }).unwrap();

        let keymap = xkb::Keymap::new_from_string(
            &context,
            keymap_str.clone(),
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        ).expect("Failed to create keymap");

        let state = xkb::State::new(&keymap);

        assert_eq!(state.key_get_one_sym(9), xkb::KEY_a);
        assert_eq!(state.key_get_one_sym(10), xkb::KEY_c);
    }
}
