/* Copyright (C) 2020 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Glue for the main loop. */

use crate::animation::Outcome as Message;
use glib::{Continue, MainContext, PRIORITY_DEFAULT, Receiver, Sender};
use std::thread;
use std::time::Duration;

mod c {
    use super::*;
    use std::os::raw::c_void;
    use std::rc::Rc;

    use ::util::c::{ ArcWrapped, Wrapped };
    
    /// ServerContextService*
    #[repr(transparent)]
    pub struct UIManager(*const c_void);
    
    /// DbusHandler*
    #[repr(transparent)]
    pub struct DBusHandler(*const c_void);
    
    /// Corresponds to main.c::channel
    #[repr(C)]
    pub struct Channel {
        sender: ArcWrapped<Sender<Message>>,
        receiver: Wrapped<Receiver<Message>>,
    }
    
    extern "C" {
        pub fn server_context_service_real_show_keyboard(imservice: *const UIManager);
        pub fn server_context_service_real_hide_keyboard(imservice: *const UIManager);
        // This should probably only get called from the gtk main loop,
        // given that dbus handler is using glib.
        pub fn dbus_handler_set_visible(dbus: *const DBusHandler, visible: u8);
    }
    
    #[no_mangle]
    pub extern "C"
    fn main_loop_channel_new() -> Channel {
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        let sender = ArcWrapped::new(sender);
        let receiver = Wrapped::new(receiver);
        let channel = Channel {
            sender,
            receiver,
        };
        
        //start_work(channel.sender.clone());
        
        channel
    }
    
    /// testing only
    fn start_work(sender: ArcWrapped<Sender<Message>>) {
        let sender = sender.clone_ref();
        thread::spawn(move || {
            let sender = sender.lock().unwrap();
            thread::sleep(Duration::from_secs(3));
            sender.send(Message::Visible).unwrap();
            thread::sleep(Duration::from_secs(3));
            sender.send(Message::Hidden).unwrap();
            thread::sleep(Duration::from_secs(3));
            sender.send(Message::Visible).unwrap();
        });
    }

    /// Places the UI loop callback in the glib main loop.
    #[no_mangle]
    pub extern "C"
    fn register_ui_loop_handler(
        receiver: Wrapped<Receiver<Message>>,
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
        msg: Message,
        ui_manager: *const UIManager,
        dbus_handler: *const DBusHandler,
    ) {
        match msg {
            Message::Visible => unsafe {
                // FIXME: reset layout to default if no IM field is active
                // Ideally: anim state stores the current IM hints,
                // Message::Visible(hints) is received here
                // and applied to layout
                server_context_service_real_show_keyboard(ui_manager);
                dbus_handler_set_visible(dbus_handler, 1);
            },
            Message::Hidden => unsafe {
                server_context_service_real_hide_keyboard(ui_manager);
                dbus_handler_set_visible(dbus_handler, 0);
            },
        };
    }
}
