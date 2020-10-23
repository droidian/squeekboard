/*! State of the emulated keyboard and keys.
 * Regards the keyboard as if it was composed of switches. */

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::mem;
use std::ptr;
use std::rc::Rc;
use std::string::FromUtf8Error;

use ::action::Action;
use ::util;

// Traits
use std::io::Write;
use std::iter::{ FromIterator, IntoIterator };

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PressType {
    Released = 0,
    Pressed = 1,
}

/// The extended, unambiguous layout-keycode
#[derive(Debug, Clone)]
pub struct KeyCode {
    pub code: u32,
    pub keymap_idx: usize,
}

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
fn sorted<'a, I: Iterator<Item=String>>(
    iter: I
) -> impl Iterator<Item=String> {
    let mut v: Vec<String> = iter.collect();
    v.sort();
    v.into_iter()
}

/// Generates a mapping where each key gets a keycode, starting from ~~8~~
/// HACK: starting from 9, because 8 results in keycode 0,
/// which the compositor likes to discard
pub fn generate_keycodes<'a, C: IntoIterator<Item=String>>(
    key_names: C,
) -> HashMap<String, KeyCode> {
    HashMap::from_iter(
        // Sort to remove a source of indeterminism in keycode assignment.
        sorted(key_names.into_iter())
            .zip(util::cycle_count(9..255))
            .map(|(name, (code, keymap_idx))| (
                String::from(name),
                KeyCode { code, keymap_idx },
            ))
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

/// Index is the key code, String is the occupant.
/// Starts all empty.
/// https://gitlab.freedesktop.org/xorg/xserver/-/issues/260
type SingleKeyMap = [Option<String>; 256];

fn single_key_map_new() -> SingleKeyMap {
    // Why can't we just initialize arrays without tricks -_- ?
    unsafe {
        // Inspired by
        // https://www.reddit.com/r/rust/comments/5n7bh1/how_to_create_an_array_of_a_type_with_clone_but/
        #[cfg(feature = "rustc_less_1_36")]
        let mut array: SingleKeyMap = mem::uninitialized();
        #[cfg(not(feature = "rustc_less_1_36"))]
        let mut array: SingleKeyMap = mem::MaybeUninit::uninit().assume_init();

        for element in array.iter_mut() {
            ptr::write(element, None);
        }
        array
    }
}

pub fn generate_keymaps(symbolmap: HashMap::<String, KeyCode>)
    -> Result<Vec<String>, FormattingError>
{
    let mut bins: Vec<SingleKeyMap> = Vec::new();
    
    for (name, KeyCode { code, keymap_idx }) in symbolmap.into_iter() {
        if keymap_idx >= bins.len() {
            bins.resize_with(
                keymap_idx + 1,
                || single_key_map_new(),
            );
        }
        bins[keymap_idx][code as usize] = Some(name);
    }

    let mut out = Vec::new();
    for bin in bins {
        out.push(generate_keymap(&bin)?);
    }
    Ok(out)
}

/// Generates a de-facto single level keymap.
/// Key codes must not repeat and must remain between 9 and 255.
fn generate_keymap(
    symbolmap: &SingleKeyMap,
) -> Result<String, FormattingError> {
    let mut buf: Vec<u8> = Vec::new();
    writeln!(
        buf,
        "xkb_keymap {{

    xkb_keycodes \"squeekboard\" {{
        minimum = 8;
        maximum = 255;"
    )?;

    let pairs: Vec<(&String, usize)> = symbolmap.iter()
        // Attach a key code to each cell.
        .enumerate()
        // Get rid of empty keycodes.
        .filter_map(|(code, name)| name.as_ref().map(|n| (n, code)))
        .collect();
    
    // Xorg can only consume up to 255 keys, so this may not work in Xwayland.
    // Two possible solutions:
    // - use levels to cram multiple characters into one key
    // - swap layouts on key presses
    for (_name, keycode) in &pairs {
        write!(
            buf,
            "
        <I{}> = {0};",
            keycode,
        )?;
    }

    writeln!(
        buf,
        "
        indicator 1 = \"Caps Lock\"; // Xwayland won't accept without it.
    }};
    
    xkb_symbols \"squeekboard\" {{
"
    )?;
    
    for (name, keycode) in pairs {
        write!(
            buf,
            "
key <I{}> {{ [ {} ] }};",
            keycode,
            name,
        )?;
    }

    writeln!(
        buf,
        "
    }};

    xkb_types \"squeekboard\" {{
        virtual_modifiers Squeekboard; // No modifiers! Needed for Xorg for some reason.
    
        // Those names are needed for Xwayland.
        type \"ONE_LEVEL\" {{
            modifiers= none;
            level_name[Level1]= \"Any\";
        }};
        type \"TWO_LEVEL\" {{
            level_name[Level1]= \"Base\";
        }};
        type \"ALPHABETIC\" {{
            level_name[Level1]= \"Base\";
        }};
        type \"KEYPAD\" {{
            level_name[Level1]= \"Base\";
        }};
        type \"SHIFT+ALT\" {{
            level_name[Level1]= \"Base\";
        }};

    }};

    xkb_compatibility \"squeekboard\" {{
        // Needed for Xwayland again.
        interpret Any+AnyOf(all) {{
            action= SetMods(modifiers=modMapMods,clearLocks);
        }};
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

    #[test]
    fn test_keymap_single_resolve() {
        let mut key_map = single_key_map_new();
        key_map[9] = Some("a".into());
        key_map[10] = Some("c".into());

        let keymap_str = generate_keymap(&key_map).unwrap();

        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

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

    #[test]
    fn test_keymap_second_resolve() {
        let keymaps = generate_keymaps(hashmap!(
            "a".into() => KeyCode { keymap_idx: 1, code: 9 },
        )).unwrap();

        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

        let keymap = xkb::Keymap::new_from_string(
            &context,
            keymaps[1].clone(), // this index is part of the test
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        ).expect("Failed to create keymap");

        let state = xkb::State::new(&keymap);

        assert_eq!(state.key_get_one_sym(9), xkb::KEY_a);
    }

    #[test]
    fn test_symbolmap_overflow() {
        // The 257th key (U1101) is interesting.
        // Use Unicode encoding for being able to use in xkb keymaps.
        let keynames = (0..258).map(|num| format!("U{:04X}", 0x1000 + num));
        let keycodes = generate_keycodes(keynames);
        
        // test now
        let code = keycodes.get("U1101").expect("Did not find the tested keysym");
        assert_eq!(code.keymap_idx, 1);
    }
}
