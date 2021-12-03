/* Copyright (C) 2020 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Glue for the main loop. */

use crate::state;
use glib::{Continue, MainContext, PRIORITY_DEFAULT, Receiver};


mod c {
    use super::*;
    use std::os::raw::c_void;
    use std::rc::Rc;
    use std::time::Instant;

    use crate::event_loop::driver;
    use crate::imservice::IMService;
    use crate::imservice::c::InputMethod;
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
        receiver: Wrapped<Receiver<Commands>>,
        state_manager: Wrapped<driver::Threaded>,
        submission: Wrapped<Submission>,
    }
    
    extern "C" {
        fn server_context_service_real_show_keyboard(service: *const UIManager);
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
    fn squeek_rsobjects_new(
        im: *mut InputMethod,
        vk: ZwpVirtualKeyboardV1,
    ) -> RsObjects {
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        
        let now = Instant::now();
        let state_manager = driver::Threaded::new(sender, state::Application::new(now));

        let imservice = if im.is_null() {
            None
        } else {
            Some(IMService::new(im, state_manager.clone()))
        };
        let submission = Submission::new(vk, imservice);
        
        RsObjects {
            submission: Wrapped::new(submission),
            state_manager: Wrapped::new(state_manager),
            receiver: Wrapped::new(receiver),
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
        ctx.acquire();
        receiver.attach(
            Some(&ctx),
            move |msg| {
                main_loop_handle_message(msg, ui_manager, dbus_handler);
                Continue(true)
            },
        );
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
            Some(PanelCommand::Show) => unsafe {
                server_context_service_real_show_keyboard(ui_manager);
            },
            Some(PanelCommand::Hide) => unsafe {
                server_context_service_real_hide_keyboard(ui_manager);
            },
            None => {},
        };

        if let Some(visible) = msg.dbus_visible_set {
            unsafe { dbus_handler_set_visible(dbus_handler, visible as u8) };
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

#[derive(Clone, PartialEq, Debug)]
pub enum PanelCommand {
    Show,
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
