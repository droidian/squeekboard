/* Copyright (C) 2020,2022 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Glue for the main loop. */
use crate::outputs::OutputId;
use crate::state;
use glib::{Continue, MainContext, PRIORITY_DEFAULT, Receiver};


mod c {
    use super::*;
    use std::os::raw::c_void;
    use std::ptr;
    use std::rc::Rc;
    use std::time::Instant;

    use crate::event_loop::driver;
    use crate::imservice::IMService;
    use crate::imservice::c::InputMethod;
    use crate::outputs::Outputs;
    use crate::outputs::c::WlOutput;
    use crate::state;
    use crate::submission::Submission;
    use crate::util::c::Wrapped;
    use crate::vkeyboard::c::ZwpVirtualKeyboardV1;
    
    /// ServerContextService*
    #[repr(transparent)]
    pub struct UIManager(*const c_void);
    
    /// DbusHandler*
    #[repr(transparent)]
    pub struct DBusHandler(*const c_void);
    
    /// Holds the Rust structures that are interesting from C.
    #[repr(C)]
    pub struct RsObjects {
        /// The handle to which Commands should be sent
        /// for processing in the main loop.
        receiver: Wrapped<Receiver<Commands>>,
        state_manager: Wrapped<driver::Threaded>,
        submission: Wrapped<Submission>,
        /// Not wrapped, because C needs to access this.
        wayland: *mut Wayland,
    }

    /// Corresponds to wayland.h::squeek_wayland.
    /// Fields unused by Rust are marked as generic data types.
    #[repr(C)]
    pub struct Wayland {
        layer_shell: *const c_void,
        virtual_keyboard_manager: *const c_void,
        input_method_manager: *const c_void,
        outputs: Wrapped<Outputs>,
        seat: *const c_void,
        input_method: InputMethod,
        virtual_keyboard: ZwpVirtualKeyboardV1,
    }

    impl Wayland {
        fn new(outputs_manager: Outputs) -> Self {
            Wayland {
                layer_shell: ptr::null(),
                virtual_keyboard_manager: ptr::null(),
                input_method_manager: ptr::null(),
                outputs: Wrapped::new(outputs_manager),
                seat: ptr::null(),
                input_method: InputMethod::null(),
                virtual_keyboard: ZwpVirtualKeyboardV1::null(),
            }
        }
    }
    
    extern "C" {
        #[allow(improper_ctypes)]
        fn init_wayland(wayland: *mut Wayland);
        fn server_context_service_update_keyboard(service: *const UIManager, output: WlOutput, scaled_height: u32);
        fn server_context_service_real_hide_keyboard(service: *const UIManager);
        fn server_context_service_set_hint_purpose(service: *const UIManager, hint: u32, purpose: u32);
        // This should probably only get called from the gtk main loop,
        // given that dbus handler is using glib.
        fn dbus_handler_set_visible(dbus: *const DBusHandler, visible: u8);
    }

    /// Creates what's possible in Rust to eliminate as many FFI calls as possible,
    /// because types aren't getting checked across their boundaries,
    /// and that leads to suffering.
    #[no_mangle]
    pub extern "C"
    fn squeek_init() -> RsObjects {
        // Set up channels
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        let now = Instant::now();
        let state_manager = driver::Threaded::new(sender, state::Application::new(now));

        let outputs = Outputs::new(state_manager.clone());
        let mut wayland = Box::new(Wayland::new(outputs));
        let wayland_raw = &mut *wayland as *mut _;
        unsafe { init_wayland(wayland_raw); }

        let vk = wayland.virtual_keyboard;

        let imservice = if wayland.input_method.is_null() {
            None
        } else {
            Some(IMService::new(wayland.input_method, state_manager.clone()))
        };
        let submission = Submission::new(vk, imservice);
        
        RsObjects {
            submission: Wrapped::new(submission),
            state_manager: Wrapped::new(state_manager),
            receiver: Wrapped::new(receiver),
            wayland: Box::into_raw(wayland),
        }
    }

    /// Places the UI loop callback in the glib main loop.
    #[no_mangle]
    pub extern "C"
    fn register_ui_loop_handler(
        receiver: Wrapped<Receiver<Commands>>,
        ui_manager: *const UIManager,
        dbus_handler: *const DBusHandler,
    ) {
        let receiver = unsafe { receiver.unwrap() };
        let receiver = Rc::try_unwrap(receiver).expect("References still present");
        let receiver = receiver.into_inner();
        let ctx = MainContext::default();
        let _acqu = ctx.acquire();
        receiver.attach(
            Some(&ctx),
            move |msg| {
                main_loop_handle_message(msg, ui_manager, dbus_handler);
                Continue(true)
            },
        );
        #[cfg(not(feature = "glib_v0_14"))]
        ctx.release();
    }

    /// A single iteration of the UI loop.
    /// Applies state outcomes to external portions of the program.
    /// This is the outest layer of the imperative shell,
    /// and doesn't lend itself to testing other than integration.
    fn main_loop_handle_message(
        msg: Commands,
        ui_manager: *const UIManager,
        dbus_handler: *const DBusHandler,
    ) {
        match msg.panel_visibility {
            Some(PanelCommand::Show { output, height }) => unsafe {
                server_context_service_update_keyboard(ui_manager, output.0, height.as_scaled_ceiling());
            },
            Some(PanelCommand::Hide) => unsafe {
                server_context_service_real_hide_keyboard(ui_manager);
            },
            None => {},
        };

        if let Some(visible) = msg.dbus_visible_set {
            if dbus_handler != std::ptr::null() {
                unsafe { dbus_handler_set_visible(dbus_handler, visible as u8) };
            }
        }

        if let Some(hints) = msg.layout_hint_set {
            unsafe {
                server_context_service_set_hint_purpose(
                    ui_manager,
                    hints.hint.bits(),
                    hints.purpose.clone() as u32,
                )
            };
        }
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

#[derive(Clone, PartialEq, Debug)]
pub enum PanelCommand {
    Show {
        output: OutputId,
        height: PixelSize,
    },
    Hide,
}

/// The commands consumed by the main loop,
/// to be sent out to external components.
#[derive(Clone)]
pub struct Commands {
    pub panel_visibility: Option<PanelCommand>,
    pub layout_hint_set: Option<state::InputMethodDetails>,
    pub dbus_visible_set: Option<bool>,
}
