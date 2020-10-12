/*! Testing functionality */

use ::data::Layout;
use ::logging;
use xkbcommon::xkb;


pub struct CountAndPrint(u32);

impl logging::Handler for CountAndPrint {
    fn handle(&mut self, level: logging::Level, warning: &str) {
        use logging::Level::*;
        match level {
            Panic | Bug | Error | Warning | Surprise => {
                self.0 += 1;
            },
            _ => {}
        }
        logging::Print{}.handle(level, warning)
    }
}

impl CountAndPrint {
    fn new() -> CountAndPrint {
        CountAndPrint(0)
    }
}

pub fn check_builtin_layout(name: &str, missing_return: bool) {
    check_layout(
        Layout::from_resource(name).expect("Invalid layout data"),
        missing_return,
    )
}

pub fn check_layout_file(path: &str) {
    check_layout(
        Layout::from_file(path.into()).expect("Invalid layout file"),
        false,
    )
}

fn check_sym_presence(
    state: &xkb::State,
    sym_name: &str,
    handler: &mut dyn logging::Handler,
) {
    let sym = xkb::keysym_from_name(sym_name, xkb::KEYSYM_NO_FLAGS);
    if sym == xkb::KEY_NoSymbol {
        panic!(format!("Entered invalid keysym: {}", sym_name));
    }
    let map = state.get_keymap();
    let range = map.min_keycode()..=map.max_keycode();
    let found = range.flat_map(|code| state.key_get_syms(code))
        .find(|s| **s == sym)
        .is_some();
    if !found {
        handler.handle(
            logging::Level::Surprise,
            &format!("There's no way to input the keysym {} on this layout", sym_name),
        )
    }
}

fn check_layout(layout: Layout, allow_missing_return: bool) {
    let handler = CountAndPrint::new();
    let (layout, mut handler) = layout.build(handler);

    if handler.0 > 0 {
        println!("{} problems while parsing layout", handler.0)
    }

    let layout = layout.expect("layout broken");

    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    
    let keymap_str = layout.keymap_str
        .clone()
        .into_string().expect("Failed to decode keymap string");
    
    let keymap = xkb::Keymap::new_from_string(
        &context,
        keymap_str.clone(),
        xkb::KEYMAP_FORMAT_TEXT_V1,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    ).expect("Failed to create keymap");

    let state = xkb::State::new(&keymap);

    check_sym_presence(&state, "BackSpace", &mut handler);
    let mut printer = logging::Print;
    check_sym_presence(
        &state,
        "Return",
        if allow_missing_return { &mut printer }
        else { &mut handler },
    );

    // "Press" each button with keysyms
    for (_pos, view) in layout.views.values() {
        for (_y, row) in view.get_rows() {
            for (_x, button) in row.get_buttons() {
                let keystate = button.state.borrow();
                for keycode in &keystate.keycodes {
                    match state.key_get_one_sym(*keycode) {
                        xkb::KEY_NoSymbol => {
                            eprintln!("{}", keymap_str);
                            panic!("Keysym {} on key {:?} can't be resolved", keycode, button.name);
                        },
                        _ => {},
                    }
                }
            }
        }
    }

    if handler.0 > 0 {
        panic!("Layout contains mistakes");
    }
}
