/*! Managing the state of text input in the application.
 * 
 * This is a library module.
 * 
 * It needs to combine text-input and virtual-keyboard protocols
 * to achieve a consistent view of the text-input state,
 * and to submit exactly what the user wanted.
 * 
 * It must also not get tripped up by sudden disappearances of interfaces.
 * 
 * The virtual-keyboard interface is always present.
 * 
 * The text-input interface may not be presented,
 * and, for simplicity, no further attempt to claim it is made.
 * 
 * The text-input interface may be enabled and disabled at arbitrary times,
 * and those events SHOULD NOT cause any lost events.
 * */

use std::collections::HashSet;
use std::ffi::CString;
use ::action::Modifier;
use ::imservice;
use ::imservice::IMService;
use ::keyboard::{ KeyCode, KeyStateId, Modifiers, PressType };
use ::layout;
use ::ui_manager::VisibilityManager;
use ::util::vec_remove;
use ::vkeyboard;
use ::vkeyboard::VirtualKeyboard;

// traits
use std::iter::FromIterator;

/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;
    
    use std::os::raw::c_void;

    use ::imservice::c::InputMethod;
    use ::util::c::Wrapped;
    use ::vkeyboard::c::ZwpVirtualKeyboardV1;

    // The following defined in C

    /// EekboardContextService*
    #[repr(transparent)]
    pub struct StateManager(*const c_void);

    #[no_mangle]
    pub extern "C"
    fn submission_new(
        im: *mut InputMethod,
        vk: ZwpVirtualKeyboardV1,
        state_manager: *const StateManager,
        visibility_manager: Wrapped<VisibilityManager>,
    ) -> *mut Submission {
        let imservice = if im.is_null() {
            None
        } else {
            let visibility_manager = visibility_manager.clone_ref();
            Some(IMService::new(
                im,
                state_manager,
                Box::new(move |active| visibility_manager.borrow_mut().set_im_active(active)),
            ))
        };
        // TODO: add vkeyboard too
        Box::<Submission>::into_raw(Box::new(
            Submission {
                imservice,
                modifiers_active: Vec::new(),
                virtual_keyboard: VirtualKeyboard(vk),
                pressed: Vec::new(),
                keymap_fds: Vec::new(),
                keymap_idx: None,
            }
        ))
    }

    #[no_mangle]
    pub extern "C"
    fn submission_use_layout(
        submission: *mut Submission,
        layout: *const layout::Layout,
        time: u32,
    ) {
        if submission.is_null() {
            panic!("Null submission pointer");
        }
        let submission: &mut Submission = unsafe { &mut *submission };
        let layout = unsafe { &*layout };
        submission.use_layout(layout, Timestamp(time));
    }
}

#[derive(Clone, Copy)]
pub struct Timestamp(pub u32);

#[derive(Clone)]
enum SubmittedAction {
    /// A collection of keycodes that were pressed
    VirtualKeyboard(Vec<KeyCode>),
    IMService,
}

pub struct Submission {
    imservice: Option<Box<IMService>>,
    virtual_keyboard: VirtualKeyboard,
    modifiers_active: Vec<(KeyStateId, Modifier)>,
    pressed: Vec<(KeyStateId, SubmittedAction)>,
    keymap_fds: Vec<vkeyboard::c::KeyMap>,
    keymap_idx: Option<usize>,
}

pub enum SubmitData<'a> {
    Text(&'a CString),
    Erase,
    Keycodes,
}

impl Submission {
    /// Sends a submit text event if possible;
    /// otherwise sends key press and makes a note of it
    pub fn handle_press(
        &mut self,
        key_id: KeyStateId,
        data: SubmitData,
        keycodes: &Vec<KeyCode>,
        time: Timestamp,
    ) {
        let mods_are_on = !self.modifiers_active.is_empty();

        let was_committed_as_text = match (&mut self.imservice, mods_are_on) {
            (Some(imservice), false) => {
                enum Outcome {
                    Submitted(Result<(), imservice::SubmitError>),
                    NotSubmitted,
                };

                let submit_outcome = match data {
                    SubmitData::Text(text) => {
                        Outcome::Submitted(imservice.commit_string(text))
                    },
                    SubmitData::Erase => {
                        /* Delete_surrounding_text takes byte offsets,
                         * so cannot work without get_surrounding_text.
                         * This is a bug in the protocol.
                         */
                        // imservice.delete_surrounding_text(1, 0),
                        Outcome::NotSubmitted
                    },
                    SubmitData::Keycodes => Outcome::NotSubmitted,
                };

                match submit_outcome {
                    Outcome::Submitted(result) => {
                        match result.and_then(|()| imservice.commit()) {
                            Ok(()) => true,
                            Err(imservice::SubmitError::NotActive) => false,
                        }
                    },
                    Outcome::NotSubmitted => false,
                }
            },
            (_, _) => false,
        };

        let submit_action = match was_committed_as_text {
            true => SubmittedAction::IMService,
            false => {
                let keycodes_count = keycodes.len();
                for keycode in keycodes.iter() {
                    self.select_keymap(keycode.keymap_idx, time);
                    let keycode = keycode.code;
                    match keycodes_count {
                        // Pressing a key made out of a single keycode is simple:
                        // press on press, release on release.
                        1 => self.virtual_keyboard.switch(
                            keycode,
                            PressType::Pressed,
                            time,
                        ),
                        // A key made of multiple keycodes
                        // has to submit them one after the other.
                        _ => {
                            self.virtual_keyboard.switch(
                                keycode.clone(),
                                PressType::Pressed,
                                time,
                            );
                            self.virtual_keyboard.switch(
                                keycode.clone(),
                                PressType::Released,
                                time,
                            );
                        },
                    };
                }
                SubmittedAction::VirtualKeyboard(keycodes.clone())
            },
        };
        
        self.pressed.push((key_id, submit_action));
    }
    
    pub fn handle_release(&mut self, key_id: KeyStateId, time: Timestamp) {
        let index = self.pressed.iter().position(|(id, _)| *id == key_id);
        if let Some(index) = index {
            let (_id, action) = self.pressed.remove(index);
            match action {
                // string already sent, nothing to do
                SubmittedAction::IMService => {},
                // no matter if the imservice got activated,
                // keys must be released
                SubmittedAction::VirtualKeyboard(keycodes) => {
                    let keycodes_count = keycodes.len();
                    match keycodes_count {
                        1 => {
                            let keycode = &keycodes[0];
                            self.select_keymap(keycode.keymap_idx, time);
                            self.virtual_keyboard.switch(
                                keycode.code,
                                PressType::Released,
                                time,
                            );
                        },
                        // Design choice here: submit multiple all at press time
                        // and do nothing at release time.
                        _ => {},
                    };
                },
            }
        };
    }
    
    pub fn handle_add_modifier(
        &mut self,
        key_id: KeyStateId,
        modifier: Modifier, _time: Timestamp,
    ) {
        self.modifiers_active.push((key_id, modifier));
        self.update_modifiers();
    }

    pub fn handle_drop_modifier(
        &mut self,
        key_id: KeyStateId,
        _time: Timestamp,
    ) {
        vec_remove(&mut self.modifiers_active, |(id, _)| *id == key_id);
        self.update_modifiers();
    }

    fn update_modifiers(&mut self) {
        let raw_modifiers = self.modifiers_active.iter()
            .map(|(_id, m)| match m {
                Modifier::Control => Modifiers::CONTROL,
                Modifier::Alt => Modifiers::MOD1,
            })
            .fold(Modifiers::empty(), |m, n| m | n);
        self.virtual_keyboard.set_modifiers_state(raw_modifiers);
    }

    pub fn is_modifier_active(&self, modifier: Modifier) -> bool {
        self.modifiers_active.iter()
            .position(|(_id, m)| *m == modifier)
            .is_some()
    }

    pub fn get_active_modifiers(&self) -> HashSet<Modifier> {
        HashSet::from_iter(
            self.modifiers_active.iter().map(|(_id, m)| m.clone())
        )
    }

    fn clear_all_modifiers(&mut self) {
        // Looks like an optimization,
        // but preemptive cleaning is needed before setting a new keymap,
        // so removing this check would break keymap setting.
        if self.modifiers_active.is_empty() {
            return;
        }
        self.modifiers_active = Vec::new();
        self.virtual_keyboard.set_modifiers_state(Modifiers::empty())
    }

    fn release_all_virtual_keys(&mut self, time: Timestamp) {
        let virtual_pressed = self.pressed
            .clone().into_iter()
            .filter_map(|(id, action)| {
                match action {
                    SubmittedAction::VirtualKeyboard(_) => Some(id),
                    _ => None,
                }
            });
        for id in virtual_pressed {
            self.handle_release(id, time);
        }
    }


    /// Changes keymap and clears pressed keys and modifiers.
    ///
    /// It's not obvious if clearing is the right thing to do, 
    /// but keymap update may (or may not) do that,
    /// possibly putting self.modifiers_active and self.pressed out of sync,
    /// so a consistent stance is adopted to avoid that.
    /// Alternatively, modifiers could be restored on the new keymap.
    /// That approach might be difficult
    /// due to modifiers meaning different things in different keymaps.
    fn select_keymap(&mut self, idx: usize, time: Timestamp) {
        if self.keymap_idx != Some(idx) {
            self.keymap_idx = Some(idx);
            self.clear_all_modifiers();
            self.release_all_virtual_keys(time);
            let keymap = &self.keymap_fds[idx];
            self.virtual_keyboard.update_keymap(keymap);
        }
    }
    
    pub fn use_layout(&mut self, layout: &layout::Layout, time: Timestamp) {
        self.keymap_fds = layout.keymaps.iter()
            .map(|keymap_str| vkeyboard::c::KeyMap::from_cstr(
                keymap_str.as_c_str()
            ))
            .collect();
        self.keymap_idx = None;

        // This can probably be eliminated,
        // because key presses can trigger an update anyway.
        // However, self.keymap_idx needs to become Option<>
        // in order to force update on new layouts.
        self.select_keymap(0, time);
    }
}
