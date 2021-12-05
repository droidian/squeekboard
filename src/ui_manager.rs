/* Copyright (C) 2020 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Centrally manages the shape of the UI widgets, and the choice of layout.
 * 
 * Coordinates this based on information collated from all possible sources.
 * 
 * Somewhat obsoleted by the `animation` module
 * (except keyboard presence calculation),
 * and could be folded into that tracker loop as another piece of state.
 */

use crate::animation::{
    ThreadLoopDriver as Receiver,
    Event as ReceiverMessage,
};
use crate::logging;
use std::cmp::min;
use ::outputs::c::OutputHandle;


use crate::logging::Warn;


pub mod c {
    use super::*;
    use ::util::c::Wrapped;

    use ::util::CloneOwned;
    
    #[no_mangle]
    pub extern "C"
    fn squeek_visman_new(receiver: Wrapped<Receiver>) -> Wrapped<VisibilityManager> {
        Wrapped::new(VisibilityManager {
            receiver: receiver.clone_owned(),
            visibility_state: VisibilityFactors {
                im_active: false,
                physical_keyboard_present: false,
            }
        })
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_visman_set_keyboard_present(visman: Wrapped<VisibilityManager>, present: u32) {
        let visman = visman.clone_ref();
        let mut visman = visman.borrow_mut();
        visman.set_keyboard_present(present != 0)
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_uiman_new() -> Wrapped<Manager> {
        Wrapped::new(Manager { output: None })
    }

    /// Used to size the layer surface containing all the OSK widgets.
    #[no_mangle]
    pub extern "C"
    fn squeek_uiman_get_perceptual_height(
        uiman: Wrapped<Manager>,
    ) -> u32 {
        let uiman = uiman.clone_ref();
        let uiman = uiman.borrow();
        // TODO: what to do when there's no output?
        uiman.get_perceptual_height().unwrap_or(0)
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_uiman_set_output(
        uiman: Wrapped<Manager>,
        output: OutputHandle,
    ) {
        let uiman = uiman.clone_ref();
        let mut uiman = uiman.borrow_mut();
        uiman.output = Some(output);
    }
}

/// Stores current state of all things influencing what the UI should look like.
pub struct Manager {
    /// Shared output handle, current state updated whenever it's needed.
    // TODO: Stop assuming that the output never changes.
    // (There's no way for the output manager to update the ui manager.)
    // FIXME: Turn into an OutputState and apply relevant connections elsewhere.
    // Otherwise testability and predictablity is low.
    output: Option<OutputHandle>,
    //// Pixel size of the surface. Needs explicit updating.
    //surface_size: Option<Size>,
}

impl Manager {
    fn get_perceptual_height(&self) -> Option<u32> {
        let output_info = (&self.output).as_ref()
            .and_then(|o| o.get_state())
            .map(|os| (os.scale as u32, os.get_pixel_size()));
        match output_info {
            Some((scale, Some(px_size))) => Some({
                let height = if (px_size.width < 720) & (px_size.width > 0) {
                    px_size.width * 7 / 12 // to match 360×210
                } else if px_size.width < 1080 {
                    360 + (1080 - px_size.width) * 60 / 360 // smooth transition
                } else {
                    360
                };

                // Don't exceed half the display size
                min(height, px_size.height / 2) / scale
            }),
            Some((scale, None)) => Some(360 / scale),
            None => None,
        }
    }
}

#[derive(PartialEq, Debug)]
enum Visibility {
    Hidden,
    Visible,
}

#[derive(Debug)]
enum VisibilityTransition {
    /// Hide immediately
    Hide,
    /// Hide if no show request comes soon
    Release,
    /// Show instantly
    Show,
    /// Don't do anything
    NoTransition,
}

/// Contains visibility policy
#[derive(Clone, Debug)]
struct VisibilityFactors {
    im_active: bool,
    physical_keyboard_present: bool,
}

impl VisibilityFactors {
    /// Static policy.
    /// Use when transitioning from an undefined state (e.g. no UI before).
    fn desired(&self) -> Visibility {
        match self {
            VisibilityFactors {
                im_active: true,
                physical_keyboard_present: false,
            } => Visibility::Visible,
            _ => Visibility::Hidden,
        }
    }
    /// Stateful policy
    fn transition_to(&self, next: &Self) -> VisibilityTransition {
        use self::Visibility::*;
        let im_deactivation = self.im_active && !next.im_active;
        match (self.desired(), next.desired(), im_deactivation) {
            (Visible, Hidden, true) => VisibilityTransition::Release,
            (Visible, Hidden, _) => VisibilityTransition::Hide,
            (Hidden, Visible, _) => VisibilityTransition::Show,
            _ => VisibilityTransition::NoTransition,
        }
    }
}

pub struct VisibilityManager {
    /// Forward changes there.
    receiver: Receiver,
    visibility_state: VisibilityFactors,
}

impl VisibilityManager {
    fn apply_changes(&mut self, new: Self) {
        let request = match self.visibility_state.transition_to(&new.visibility_state) {
            VisibilityTransition::Hide => Some(ReceiverMessage::ForceHide),
            VisibilityTransition::Show => Some(ReceiverMessage::ClaimVisible),
            VisibilityTransition::Release => Some(ReceiverMessage::ReleaseVisible),
            VisibilityTransition::NoTransition => None,
        };
        
        if let Some(request) = request {
            new.receiver.send(request)
                .or_warn(&mut logging::Print, logging::Problem::Warning, "Can't send to animation manager");
        }
        *self = new;
    }

    pub fn set_im_active(&mut self, im_active: bool) {
        let new = VisibilityManager {
            visibility_state: VisibilityFactors {
                im_active,
                ..self.visibility_state.clone()
            },
            ..unsafe { self.clone() }
        };
        self.apply_changes(new);
    }

    pub fn set_keyboard_present(&mut self, keyboard_present: bool) {
        let new = VisibilityManager {
            visibility_state: VisibilityFactors {
                physical_keyboard_present: keyboard_present,
                ..self.visibility_state.clone()
            },
            ..unsafe { self.clone() }
        };
        self.apply_changes(new);
    }

    /// This is only a helper for getting desired visibility.
    unsafe fn clone(&self) -> Self {
        VisibilityManager {
            receiver: self.receiver.clone(),
            visibility_state: self.visibility_state.clone(),
        }
    }
}
