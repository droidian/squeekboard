/* Copyright (C) 2022 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Panel state management.
 *
 * This is effectively a mirror of the previous C code,
 * with an explicit state machine managing the panel size.
 *
 * It still relies on a callback from Wayland to accept the panel size,
 * which makes this code somewhat prone to mistakes.
 *
 * An alternative to the callback would be
 * to send a message all the way to `state::State`
 * every time the allocated size changes.
 * That would allow for a more holistic view
 * of interactions of different pieces of state.
 * 
 * However, `state::State` already has the potential to become a ball of mud,
 * tightly coupling different functionality and making it difficult to see independent units.
 * 
 * For this reason, I'm taking a light touch approach with the panel manager,
 * and moving it just a bit closer to `state::State`.
 * Hopefully ths still allows us to expose assumptions that were not stated yet
 * (e.g. can the output disappear between size request andallocation?).
 *
 * Tight coupling, e.g. a future one between presented hints and layout size,
 * will have to be taken into account later.
 */

use crate::logging;
use crate::outputs::OutputId;
use crate::util::c::Wrapped;


pub mod c {
    use super::*;
    use glib;
    use gtk::Continue;
    use std::os::raw::c_void;

    use crate::outputs::c::WlOutput;

    /// struct panel_manager*
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct PanelManager(*const c_void);
    
    extern "C" {
        #[allow(improper_ctypes)]
        pub fn panel_manager_request_widget(
            service: PanelManager,
            output: WlOutput,
            height: u32,
            // for callbacks
            panel: Wrapped<Manager>,
        );
        pub fn panel_manager_resize(service: PanelManager, height: u32);
        pub fn panel_manager_hide(service: PanelManager);
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_panel_manager_configured(panel: Wrapped<Manager>, width: u32, height: u32) {
        // This is why this needs to be moved into state::State:
        // it's getting too coupled to glib.
        glib::idle_add_local(move || {
            let panel = panel.clone_ref();
            panel.borrow_mut().set_configured(Size{width, height});
            Continue(false)
        });
    }
}


/// Size in pixels that is aware of scaling
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PixelSize {
    pub pixels: u32,
    pub scale_factor: u32,
}

fn div_ceil(a: u32, b: u32) -> u32 {
    // Given that it's for pixels on a screen, an overflow is unlikely.
    (a + b - 1) / b
}

impl PixelSize {
    pub fn as_scaled_floor(&self) -> u32 {
        self.pixels / self.scale_factor
    }

    pub fn as_scaled_ceiling(&self) -> u32 {
        div_ceil(self.pixels, self.scale_factor)
    }
}

#[derive(Clone, Debug)]
struct Size {
    width: u32,
    height: u32,
}

/// This state requests the Wayland layer shell protocol synchronization:
/// the application asks for some size,
/// and then receives a size that the compositor thought appropriate.
/// Stores raw values passed to Wayland, i.e. scaled dimensions.
#[derive(Clone, Debug)]
enum State {
    Hidden,
    SizeRequested {
        output: OutputId,
        height: u32,
        //width: u32,
    },
    SizeAllocated {
        output: OutputId,
        wanted_height: u32,
        allocated: Size,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    Show {
        output: OutputId,
        height: PixelSize,
    },
    Hide,
}

/// Tries to contain all the panel sizing duties.
pub struct Manager {
    panel: c::PanelManager,
    state: State,
}

impl Manager {
    pub fn new(panel: c::PanelManager) -> Self {
        Self {
            panel,
            state: State::Hidden,
        }
    }
    // TODO: mabe send the allocated size back to state::State,
    // to perform layout adjustments
    fn set_configured(&mut self, size: Size) {
        self.state = match self.state.clone() {
            State::Hidden => {
                // This may happen if a hide is scheduled immediately after a show.
                log_print!(
                    logging::Level::Surprise,
                    "Panel has been configured, but no request is pending. Ignoring",
                );
                State::Hidden
            },
            State::SizeAllocated{output, wanted_height, ..} => {
                log_print!(
                    logging::Level::Surprise,
                    "Panel received new configuration without asking",
                );
                State::SizeAllocated{output, wanted_height, allocated: size}
            },
            State::SizeRequested{output, height} => State::SizeAllocated {
                output,
                wanted_height: height,
                allocated: size,
            },
        };
    }

    pub fn update(mgr: Wrapped<Manager>, cmd: Command) {
        let copied = mgr.clone();

        let mgr = mgr.clone_ref();
        let mut mgr = mgr.borrow_mut();

        (*mgr).state = match (cmd, mgr.state.clone()) {
            (Command::Hide, State::Hidden) => State::Hidden,
            (Command::Hide, State::SizeAllocated{..}) => {
                unsafe { c::panel_manager_hide(mgr.panel); }
                State::Hidden
            },
            (Command::Hide, State::SizeRequested{..}) => {
                unsafe { c::panel_manager_hide(mgr.panel); }
                State::Hidden
            },
            (Command::Show{output, height}, State::Hidden) => {
                let height = height.as_scaled_ceiling();
                unsafe { c::panel_manager_request_widget(mgr.panel, output.0, height, copied); }
                State::SizeRequested{output, height}
            },
            (
                Command::Show{output, height},
                State::SizeRequested{output: req_output, height: req_height},
            ) => {
                let height = height.as_scaled_ceiling();
                if output == req_output && height == req_height {
                    State::SizeRequested{output: req_output, height: req_height}
                } else if output == req_output {
                    // I'm not sure about that.
                    // This could cause a busy loop,
                    // when two requests are being processed at the same time:
                    // one message in the compositor to allocate size A,
                    // causing the state to update to height A'
                    // the other from the state wanting height B',
                    // causing the compositor to change size to B.
                    // So better cut this short here, despite artifacts.
                    // Out of simplicty, just ignore the new request.
                    // If that causes problems, the request in flight could be stored
                    // for the purpose of handling it better somehow.
                    State::SizeRequested{output: req_output, height: req_height}
                } else {
                    // This looks weird, but should be safe.
                    // The stack seems to handle
                    // configure events on a dead surface.
                    unsafe {
                        c::panel_manager_hide(mgr.panel);
                        c::panel_manager_request_widget(mgr.panel, output.0, height, copied);
                    }
                    State::SizeRequested{output, height}
                }
            },
            (
                Command::Show{output, height},
                State::SizeAllocated{output: alloc_output, allocated, wanted_height},
            ) => {
                let height = height.as_scaled_ceiling();
                if output == alloc_output && height == wanted_height {
                    State::SizeAllocated{output: alloc_output, wanted_height, allocated}
                } else if output == alloc_output && height == allocated.height {
                    State::SizeAllocated{output: alloc_output, wanted_height: height, allocated}
                } else if output == alloc_output {
                    // Should *all* other heights cause a resize?
                    // What about those between wanted and allocated?
                    unsafe { c::panel_manager_resize(mgr.panel, height); }
                    State::SizeRequested{output, height}
                } else {
                    unsafe {
                        c::panel_manager_hide(mgr.panel);
                        c::panel_manager_request_widget(mgr.panel, output.0, height, copied);
                    }
                    State::SizeRequested{output, height}
                }
            },
        }
    }
}
