/* Copyright (C) 2021 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */
 
/*! This drives the loop from the `loop` module.
 * 
 * The tracker loop needs to be driven somehow,
 * and connected to the external world,
 * both on the side of receiving and sending events.
 * 
 * That's going to be implementation-dependent,
 * connecting to some external mechanisms
 * for time, messages, and threading/callbacks.
 * 
 * This is the "imperative shell" part of the software,
 * and no longer unit-testable.
 */

use crate::event_loop;
use crate::logging;
use crate::main::Commands;
use crate::state::{ Application, Event };
use glib;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

// Traits
use crate::logging::Warn;


/// Type of the sender that waits for external events
type Sender = mpsc::Sender<Event>;
/// Type of the sender that waits for internal state changes
type UISender = glib::Sender<Commands>;

/// This loop driver spawns a new thread which updates the state in a loop,
/// in response to incoming events.
/// It sends outcomes to the glib main loop using a channel.
/// The outcomes are applied by the UI end of the channel in the `main` module.
// This could still be reasonably tested,
// by creating a glib::Sender and checking what messages it receives.
#[derive(Clone)]
pub struct Threaded {
    thread: Sender,
}

impl Threaded {
    pub fn new(ui: UISender, initial_state: Application) -> Self {
        let (sender, receiver) = mpsc::channel();
        let saved_sender = sender.clone();
        thread::spawn(move || {
            let mut state = event_loop::State::new(initial_state, Instant::now());
            loop {
                match receiver.recv() {
                    Ok(event) => {
                        state = Self::handle_loop_event(&sender, state, event, &ui);
                    },
                    Err(e) => {
                        logging::print(logging::Level::Bug, &format!("Senders hung up, aborting: {}", e));
                        return;
                    },
                };
            }
        });

        Self {
            thread: saved_sender,
        }
    }
    
    pub fn send(&self, event: Event) -> Result<(), mpsc::SendError<Event>> {
        self.thread.send(event)
    }
    
    fn handle_loop_event(loop_sender: &Sender, state: event_loop::State, event: Event, ui: &UISender)
        -> event_loop::State
    {
        let now = Instant::now();

        let (new_state, commands) = event_loop::handle_event(state.clone(), event, now);

        ui.send(commands)
            .or_warn(&mut logging::Print, logging::Problem::Bug, "Can't send to UI");

        if new_state.scheduled_wakeup != state.scheduled_wakeup {
            if let Some(when) = new_state.scheduled_wakeup {
                Self::schedule_timeout_wake(loop_sender, when);
            }
        }
        
        new_state
    }

    fn schedule_timeout_wake(loop_sender: &Sender, when: Instant) {
        let sender = loop_sender.clone();
        thread::spawn(move || {
            let now = Instant::now();
            thread::sleep(when - now);
            sender.send(Event::TimeoutReached(when))
                .or_warn(&mut logging::Print, logging::Problem::Warning, "Can't wake visibility manager");
        });
    }
}

/// For calling in only
mod c {
    use super::*;

    use crate::state::Presence;
    use crate::state::visibility;
    use crate::util::c::Wrapped;
    
    #[no_mangle]
    pub extern "C"
    fn squeek_state_send_force_visible(mgr: Wrapped<Threaded>) {
        let sender = mgr.clone_ref();
        let sender = sender.borrow();
        sender.send(Event::Visibility(visibility::Event::ForceVisible))
            .or_warn(&mut logging::Print, logging::Problem::Warning, "Can't send to state manager");
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_state_send_force_hidden(sender: Wrapped<Threaded>) {
        let sender = sender.clone_ref();
        let sender = sender.borrow();
        sender.send(Event::Visibility(visibility::Event::ForceHidden))
            .or_warn(&mut logging::Print, logging::Problem::Warning, "Can't send to state manager");
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_state_send_keyboard_present(sender: Wrapped<Threaded>, present: u32) {
        let sender = sender.clone_ref();
        let sender = sender.borrow();
        let state =
            if present == 0 { Presence::Missing }
            else { Presence::Present };
        sender.send(Event::PhysicalKeyboard(state))
            .or_warn(&mut logging::Print, logging::Problem::Warning, "Can't send to state manager");
    }
}
