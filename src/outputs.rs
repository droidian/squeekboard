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

    use ::util::c::{COpaquePtr, Wrapped};

    // Defined in C

    #[repr(transparent)]
    #[derive(Clone, PartialEq, Copy)]
    pub struct WlOutput(*const c_void);

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
    #[derive(Clone)]
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

    /// A stable reference to an output.
    #[derive(Clone)]
    #[repr(C)]
    pub struct OutputHandle {
        wl_output: WlOutput,
        outputs: COutputs,
    }

    impl OutputHandle {
        // Cannot return an Output reference
        // because COutputs is too deeply wrapped
        pub fn get_state(&self) -> Option<OutputState> {
            let outputs = self.outputs.clone_ref();
            let outputs = outputs.borrow();
            find_output(&outputs, self.wl_output.clone()).map(|o| o.current.clone())
        }
    }

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
            = find_output_mut(&mut collection, wl_output)
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
            = find_output_mut(&mut collection, wl_output)
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
        let output = find_output_mut(&mut collection, wl_output);
        match output {
            Some(output) => { output.current = output.pending.clone(); }
            None => log_print!(
                logging::Level::Warning,
                "Got done on unknown output",
            ),
        };
    }

    extern fn outputs_handle_scale(
        outputs: COutputs,
        wl_output: WlOutput,
        factor: i32,
    ) {
        let outputs = outputs.clone_ref();
        let mut collection = outputs.borrow_mut();
        let output_state: Option<&mut OutputState>
            = find_output_mut(&mut collection, wl_output)
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
    fn squeek_outputs_register(raw_collection: COutputs, output: WlOutput) {
        let collection = raw_collection.clone_ref();
        let mut collection = collection.borrow_mut();
        collection.outputs.push(Output {
            output: output.clone(),
            pending: OutputState::uninitialized(),
            current: OutputState::uninitialized(),
        });

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
    
    #[no_mangle]
    pub extern "C"
    fn squeek_outputs_get_current(raw_collection: COutputs) -> OutputHandle {
        let collection = raw_collection.clone_ref();
        let collection = collection.borrow();
        OutputHandle {
            wl_output: collection.outputs[0].output.clone(),
            outputs: raw_collection.clone(),
        }
    }

    // TODO: handle unregistration
    
    fn find_output(
        collection: &Outputs,
        wl_output: WlOutput,
    ) -> Option<&Output> {
        collection.outputs
            .iter()
            .find_map(|o|
                if o.output == wl_output { Some(o) } else { None }
            )
    }

    fn find_output_mut(
        collection: &mut Outputs,
        wl_output: WlOutput,
    ) -> Option<&mut Output> {
        collection.outputs
            .iter_mut()
            .find_map(|o|
                if o.output == wl_output { Some(o) } else { None }
            )
    }
}

/// Generic size
#[derive(Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// wl_output mode
#[derive(Clone)]
struct Mode {
    width: i32,
    height: i32,
}

#[derive(Clone)]
pub struct OutputState {
    current_mode: Option<Mode>,
    transform: Option<c::Transform>,
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

pub struct Output {
    output: c::WlOutput,
    pending: OutputState,
    current: OutputState,
}

pub struct Outputs {
    outputs: Vec<Output>,
    sender: event_loop::driver::Threaded,
}

impl Outputs {
    pub fn new(sender: event_loop::driver::Threaded) -> Outputs {
        Outputs {
            outputs: Vec::new(),
            sender,
        }
    }
}
