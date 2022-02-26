/* Copyright (C) 2019-2022 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Managing Wayland outputs */

use std::vec::Vec;
use crate::event_loop;
use ::logging;

// traits
use ::logging::Warn;

/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;
    
    use std::os::raw::{ c_char, c_void };
    use std::ptr;

    use ::util::c::{COpaquePtr, Wrapped};

    // Defined in C

    #[repr(transparent)]
    #[derive(Clone, PartialEq, Copy, Debug, Eq, Hash)]
    pub struct WlOutput(*const c_void);

    impl WlOutput {
        fn null() -> Self {
            Self(ptr::null())
        }
    }

    #[repr(C)]
    struct WlOutputListener<T: COpaquePtr> {
        geometry: extern fn(
            T, // data
            WlOutput,
            i32, // x
            i32, // y
            i32, // physical_width
            i32, // physical_height
            i32, // subpixel
            *const c_char, // make
            *const c_char, // model
            i32, // transform
        ),
        mode: extern fn(
            T, // data
            WlOutput,
            u32, // flags
            i32, // width
            i32, // height
            i32, // refresh
        ),
        done: extern fn(
            T, // data
            WlOutput,
        ),
        scale: extern fn(
            T, // data
            WlOutput,
            i32, // factor
        ),
    }
    
    bitflags!{
        /// Map to `wl_output.mode` values
        pub struct Mode: u32 {
            const NONE = 0x0;
            const CURRENT = 0x1;
            const PREFERRED = 0x2;
        }
    }

    /// Map to `wl_output.transform` values
    #[derive(Clone, Copy, Debug)]
    pub enum Transform {
        Normal = 0,
        Rotated90 = 1,
        Rotated180 = 2,
        Rotated270 = 3,
        Flipped = 4,
        FlippedRotated90 = 5,
        FlippedRotated180 = 6,
        FlippedRotated270 = 7,
    }
    
    impl Transform {
        fn from_u32(v: u32) -> Option<Transform> {
            use self::Transform::*;
            match v {
                0 => Some(Normal),
                1 => Some(Rotated90),
                2 => Some(Rotated180),
                3 => Some(Rotated270),
                4 => Some(Flipped),
                5 => Some(FlippedRotated90),
                6 => Some(FlippedRotated180),
                7 => Some(FlippedRotated270),
                _ => None,
            }
        }
    }

    extern "C" {
        // Rustc wrongly assumes
        // that COutputs allows C direct access to the underlying RefCell
        #[allow(improper_ctypes)]
        fn squeek_output_add_listener(
            wl_output: WlOutput,
            listener: *const WlOutputListener<COutputs>,
            data: COutputs,
        ) -> i32;
    }

    /// Wrapping Outputs is required for calling its methods from C
    type COutputs = Wrapped<Outputs>;

    // Defined in Rust

    // Callbacks from the output listener follow

    extern fn outputs_handle_geometry(
        outputs: COutputs,
        wl_output: WlOutput,
        _x: i32, _y: i32,
        _phys_width: i32, _phys_height: i32,
        _subpixel: i32,
        _make: *const c_char, _model: *const c_char,
        transform: i32,
    ) {
        let transform = Transform::from_u32(transform as u32)
            .or_print(
                logging::Problem::Warning,
                "Received invalid wl_output.transform value",
            ).unwrap_or(Transform::Normal);

        let outputs = outputs.clone_ref();
        let mut collection = outputs.borrow_mut();
        let output_state: Option<&mut OutputState>
            = collection
                .find_output_mut(wl_output)
                .map(|o| &mut o.pending);
        match output_state {
            Some(state) => { state.transform = Some(transform) },
            None => log_print!(
                logging::Level::Warning,
                "Got geometry on unknown output",
            ),
        };
    }

    extern fn outputs_handle_mode(
        outputs: COutputs,
        wl_output: WlOutput,
        flags: u32,
        width: i32,
        height: i32,
        _refresh: i32,
    ) {
        let flags = Mode::from_bits(flags)
            .or_print(
                logging::Problem::Warning,
                "Received invalid wl_output.mode flags",
            ).unwrap_or(Mode::NONE);

        let outputs = outputs.clone_ref();
        let mut collection = outputs.borrow_mut();
        let output_state: Option<&mut OutputState>
            = collection
                .find_output_mut(wl_output)
                .map(|o| &mut o.pending);
        match output_state {
            Some(state) => {
                if flags.contains(Mode::CURRENT) {
                    state.current_mode = Some(super::Mode { width, height});
                }
            },
            None => log_print!(
                logging::Level::Warning,
                "Got mode on unknown output",
            ),
        };
    }

    extern fn outputs_handle_done(
        outputs: COutputs,
        wl_output: WlOutput,
    ) {
        let outputs = outputs.clone_ref();
        let mut collection = outputs.borrow_mut();
        let output = collection
            .find_output_mut(wl_output);
        let event = match output {
            Some(output) => {
                output.current = output.pending.clone();
                Some(Event {
                    output: OutputId(wl_output),
                    change: ChangeType::Altered(output.current),
                })
            },
            None => {
                log_print!(
                    logging::Level::Warning,
                    "Got done on unknown output",
                );
                None
            }
        };
        if let Some(event) = event {
            collection.send_event(event);
        }
    }

    extern fn outputs_handle_scale(
        outputs: COutputs,
        wl_output: WlOutput,
        factor: i32,
    ) {
        let outputs = outputs.clone_ref();
        let mut collection = outputs.borrow_mut();
        let output_state: Option<&mut OutputState>
            = collection
                .find_output_mut(wl_output)
                .map(|o| &mut o.pending);
        match output_state {
            Some(state) => { state.scale = factor; }
            None => log_print!(
                logging::Level::Warning,
                "Got scale on unknown output",
            ),
        };
    }

    // End callbacks

    #[no_mangle]
    pub extern "C"
    fn squeek_outputs_free(outputs: COutputs) {
        unsafe { outputs.unwrap() }; // gets dropped
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_outputs_register(raw_collection: COutputs, output: WlOutput, id: u32) {
        let collection = raw_collection.clone_ref();
        let mut collection = collection.borrow_mut();
        collection.outputs.push((
            Output {
                output: output.clone(),
                pending: OutputState::uninitialized(),
                current: OutputState::uninitialized(),
            },
            id,
        ));

        unsafe { squeek_output_add_listener(
            output,
            &WlOutputListener {
                geometry: outputs_handle_geometry,
                mode: outputs_handle_mode,
                done: outputs_handle_done,
                scale: outputs_handle_scale,
            } as *const WlOutputListener<COutputs>,
            raw_collection,
        )};
    }

    /// This will try to unregister the output, if the id matches a registered one.
    #[no_mangle]
    pub extern "C"
    fn squeek_outputs_try_unregister(raw_collection: COutputs, id: u32) -> WlOutput {
        let collection = raw_collection.clone_ref();
        let mut collection = collection.borrow_mut();
        collection.remove_output_by_global(id)
            .map_err(|e| log_print!(
                logging::Level::Debug,
                "Tried to remove global {:x} but it is not registered as an output: {:?}",
                id, e,
            ))
            .unwrap_or(WlOutput::null())
    }

    // TODO: handle unregistration
}

/// Generic size
#[derive(Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// wl_output mode
#[derive(Clone, Copy, Debug)]
pub struct Mode {
    width: i32,
    height: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct OutputState {
    pub current_mode: Option<Mode>,
    pub transform: Option<c::Transform>,
    pub scale: i32,
}

impl OutputState {
    // More properly, this would have been a builder kind of struct,
    // with wl_output gradually adding properties to it
    // before it reached a fully initialized state,
    // when it would transform into a struct without all (some?) of the Options.
    // However, it's not clear which state is fully initialized,
    // and whether it would make things easier at all anyway.
    fn uninitialized() -> OutputState {
        OutputState {
            current_mode: None,
            transform: None,
            scale: 1,
        }
    }

    pub fn get_pixel_size(&self) -> Option<Size> {
        use self::c::Transform;
        match self {
            OutputState {
                current_mode: Some(Mode { width, height } ),
                transform: Some(transform),
                scale: _,
            } => Some(
                match transform {
                    Transform::Normal
                    | Transform::Rotated180
                    | Transform::Flipped
                    | Transform::FlippedRotated180 => Size {
                        width: *width as u32,
                        height: *height as u32,
                    },
                    _ => Size {
                        width: *height as u32,
                        height: *width as u32,
                    },
                }
            ),
            _ => None,
        }
    }
}

/// Not guaranteed to exist,
/// but can be used to look up state.
#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub struct OutputId(pub c::WlOutput);

// WlOutput is a pointer,
// but in the public interface,
// we're only using it as a lookup key.
unsafe impl Send for OutputId {}

struct Output {
    output: c::WlOutput,
    pending: OutputState,
    current: OutputState,
}

#[derive(Debug)]
struct NotFound;

/// Wayland global ID type
type GlobalId = u32;

/// The outputs manager
pub struct Outputs {
    outputs: Vec<(Output, GlobalId)>,
    sender: event_loop::driver::Threaded,
}

impl Outputs {
    pub fn new(sender: event_loop::driver::Threaded) -> Outputs {
        Outputs {
            outputs: Vec::new(),
            sender,
        }
    }

    fn send_event(&self, event: Event) {
        self.sender.send(event.into()).unwrap()
    }

    fn remove_output_by_global(&mut self, id: GlobalId)
        -> Result<c::WlOutput, NotFound>
    {
        let index = self.outputs.iter()
            .position(|(_o, global_id)| *global_id == id);
        if let Some(index) = index {
            let (output, _id) = self.outputs.remove(index);
            self.send_event(Event {
                change: ChangeType::Removed,
                output: OutputId(output.output),
            });
            Ok(output.output)
        } else {
            Err(NotFound)
        }
    }

    fn find_output_mut(&mut self, wl_output: c::WlOutput)
        -> Option<&mut Output>
    {
        self.outputs
            .iter_mut()
            .find_map(|(o, _global)|
                if o.output == wl_output { Some(o) } else { None }
            )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ChangeType {
    /// Added or changed
    Altered(OutputState),
    Removed,
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub output: OutputId,
    pub change: ChangeType,
}
