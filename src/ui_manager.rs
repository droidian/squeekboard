/* Copyright (C) 2020, 2021 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Centrally manages the shape of the UI widgets, and the choice of layout.
 * 
 * Coordinates this based on information collated from all possible sources.
 */

use std::cmp::min;
use ::outputs::c::OutputHandle;


pub mod c {
    use super::*;
    use ::util::c::Wrapped;

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
