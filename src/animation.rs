/* Copyright (C) 2020 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Animation state trackers and drivers.
 * Concerns the presentation layer.
 * 
 * Documentation and comments in this module
 * are meant to be read from the top to bottom. */

use crate::logging;
use glib;
use std::cmp;
use std::sync::mpsc;
use std::time::{ Duration, Instant };

// Traits
use crate::logging::Warn;


/// The keyboard should hide after this has elapsed to prevent flickering.
const HIDING_TIMEOUT: Duration = Duration::from_millis(200);


/// Events that the state tracker processes
#[derive(Clone)]
pub enum Event {
    ClaimVisible,
    /// The panel is not needed
    ReleaseVisible,
    /// The user requested the panel to go down
    ForceHide,
    /// Event triggered because a moment in time passed.
    /// Use to animate state transitions.
    /// The value is the ideal arrival time.
    TimeoutReached(Instant),
}

/// The outwardly visible state of visibility
#[derive(PartialEq, Debug)]
pub enum Outcome {
    Visible,
    Hidden,
}


/// The actual logic of visibility animation.
/// It keeps the pael visible for a short time period after each hide request.
/// This prevents flickering on quick successive enable/disable events.
/// It does not treat user-driven hiding in a special way.
///
/// This is the "functional core".
/// All state changes return the next state and the optimal time for the next check.
///
/// This state tracker can be driven by any event loop.
#[derive(Clone, PartialEq, Debug)]
enum VisibilityTracker {
    Visible,
    /// Wait until the instant is reached and then hide immediately.
    /// Depending on the relation to current time, it means either visible or hidden.
    HiddenAfter(Instant),
}

use self::VisibilityTracker::*;

impl VisibilityTracker {
    fn apply_event(self, event: Event, now: Instant) -> Self {
        match event {
            Event::ClaimVisible => Visible,
            Event::ReleaseVisible => match self {
                Visible => HiddenAfter(now + HIDING_TIMEOUT),
                other => other,
            },
            Event::ForceHide => match self {
                // Special case to avoid unneeded state changes.
                HiddenAfter(when) => HiddenAfter(cmp::min(when, now)),
                _ => HiddenAfter(now),
            },
            // The tracker doesn't change just because time is passing.
            Event::TimeoutReached(_) => self,
        }
    }

    /// Returns the state visible to the outside
    fn get_outcome(&self, now: Instant) -> Outcome {
        let visible = match self {
            Visible => true,
            HiddenAfter(hide_after) => *hide_after > now,
        };
        if visible {
            Outcome::Visible
        } else {
            Outcome::Hidden
        }
    }

    /// Returns the next time to update the state.
    fn get_next_wake(&self, now: Instant) -> Option<Instant> {
        match self {
            HiddenAfter(next) => {
                if *next > now { Some(*next) }
                else { None }
            },
            _ => None,
        }
    }
}

/* If we performed updates in a tight loop,
 * the Tracker would have been all we need.
 * 
 * loop {
 *     event = current_event()
 *     outcome = update_state(event)
 *     window.apply(outcome)
 * }
 * 
 * This is enough to process all events,
 * and keep the window always in sync with the current state.
 * 
 * However, we're trying to be conservative,
 * and not waste time performing updates that don't change state,
 * so we have to react to events that end up influencing the state.
 * 
 * One complication from that is that animation steps
 * are not a response to events coming from the owner of the loop,
 * but are needed by the loop itself.
 *
 * This is where the rest of bugs hide:
 * too few scheduled wakeups mean missed updates and wrong visible state.
 * Too many wakeups can slow down the process, or make animation jittery.
 * The loop iteration is kept as a pure function to stay testable.
 */

/// This keeps the state of the tracker loop between iterations
#[derive(Clone)]
struct LoopState {
    state: VisibilityTracker,
    scheduled_wakeup: Option<Instant>,
}

impl LoopState {
    fn new(initial_state: VisibilityTracker) -> Self {
        Self {
            state: initial_state,
            scheduled_wakeup: None,
        }
    }
}

/// A single iteration of the loop, updating its persistent state.
/// - updates tracker state,
/// - determines outcome,
/// - determines next scheduled animation wakeup,
/// and because this is a pure function, it's easily testable.
/// It returns the new state, and the optional message to send onwards.
fn handle_loop_event(
    mut loop_state: LoopState,
    event: Event,
    now: Instant,
) -> (LoopState, Option<Outcome>) {
    // Forward current public state to the consumer.
    // This doesn't take changes into account,
    // but we're only sending updates as a response to events,
    // so no-ops shouldn't dominate.
    loop_state.state = loop_state.state.apply_event(event.clone(), now);
    let outcome = loop_state.state.get_outcome(now);
    
    // Timeout events are special: they affect the scheduled timeout.
    loop_state.scheduled_wakeup = match event {
        Event::TimeoutReached(when) => {
            if when > now {
                // Special handling for scheduled events coming in early.
                // Wait at least 10 ms to avoid Zeno's paradox.
                // This is probably not needed though,
                // if the `now` contains the desired time of the event.
                // But then what about time "reversing"?
                Some(cmp::max(
                    when,
                    now + Duration::from_millis(10),
                ))
            } else {
                // There's only one timeout in flight, and it's this one.
                // It's about to complete, and then the tracker can be cleared.
                // I'm not sure if this is strictly necessary.
                None
            }
        },
        _ => loop_state.scheduled_wakeup.clone(),
    };
    
    // Reschedule timeout if the new state calls for it.
    let scheduled = &loop_state.scheduled_wakeup;
    let desired = loop_state.state.get_next_wake(now);

    loop_state.scheduled_wakeup = match (scheduled, desired) {
        (&Some(scheduled), Some(next)) => {
            if scheduled > next {
                // State wants a wake to happen before the one which is already scheduled.
                // The previous state is removed in order to only ever keep one in flight.
                // That hopefully avoids pileups,
                // e.g. because the system is busy
                // and the user keeps doing something that queues more events.
                Some(next)
            } else {
                // Not changing the case when the wanted wake is *after* scheduled,
                // because wakes are not expensive as long as they don't pile up,
                // and I can't see a pileup potential when it doesn't retrigger itself.
                // Skipping an expected event is much more dangerous.
                Some(scheduled)
            }
        },
        (None, Some(next)) => Some(next),
        // No need to change the unneeded wake - see above.
        // (Some(_), None) => ...
        (other, _) => other.clone(),
    };

    (loop_state, Some(outcome))
}


/*
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

use std::thread;
type Sender = mpsc::Sender<Event>;
type UISender = glib::Sender<Outcome>;

/// This loop driver spawns a new thread which updates the state in a loop,
/// in response to incoming events.
/// It sends outcomes to the glib main loop using a channel.
/// The outcomes are applied by the UI end of the channel.
// This could still be reasonably tested,
// by creating a glib::Sender and checking what messages it receives.
#[derive(Clone)]
pub struct ThreadLoopDriver {
    thread: Sender,
}

impl ThreadLoopDriver {
    pub fn new(ui: UISender) -> Self {
        let (sender, receiver) = mpsc::channel();
        let saved_sender = sender.clone();
        thread::spawn(move || {
            let mut state = LoopState::new(VisibilityTracker::Visible);
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
    
    fn handle_loop_event(loop_sender: &Sender, state: LoopState, event: Event, ui: &UISender) -> LoopState {
        let now = Instant::now();

        let (new_state, outcome) = handle_loop_event(state.clone(), event, now);
        
        if let Some(outcome) = outcome {
            ui.send(outcome)
                .or_warn(&mut   logging::Print, logging::Problem::Bug, "Can't send to UI");
        }

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
    
    use util::c::{ ArcWrapped, Wrapped };
    
    #[no_mangle]
    pub extern "C"
    fn squeek_animation_visibility_manager_new(sender: ArcWrapped<UISender>)
        -> Wrapped<ThreadLoopDriver>
    {
        let sender = sender.clone_ref();
        let sender = sender.lock().unwrap();
        Wrapped::new(ThreadLoopDriver::new(sender.clone()))
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_animation_visibility_manager_send_claim_visible(mgr: Wrapped<ThreadLoopDriver>) {
        let sender = mgr.clone_ref();
        let sender = sender.borrow();
        sender.send(Event::ClaimVisible)
            .or_warn(&mut logging::Print, logging::Problem::Warning, "Can't send to visibility manager");
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_animation_visibility_manager_send_force_hide(sender: Wrapped<ThreadLoopDriver>) {
        let sender = sender.clone_ref();
        let sender = sender.borrow();
        sender.send(Event::ForceHide)
            .or_warn(&mut logging::Print, logging::Problem::Warning, "Can't send to visibility manager");
    }
}


#[cfg(test)]
mod test {
    use super::*;

    /// Test the original delay scenario: no flicker on quick switches.
    #[test]
    fn hide_show() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        let state = VisibilityTracker::Visible;
        let state = state.apply_event(Event::ReleaseVisible, now);
        // Check 100ms at 1ms intervals. It should remain visible.
        for _i in 0..100 {
            now += Duration::from_millis(1);
            assert_eq!(
                state.get_outcome(now),
                Outcome::Visible,
                "Hidden when it should remain visible: {:?}",
                now.saturating_duration_since(start),
            )
        }

        let state = state.apply_event(Event::ClaimVisible, now);

        assert_eq!(state.get_outcome(now), Outcome::Visible);
    }

    /// Make sure that hiding works when requested legitimately
    #[test]
    fn hide() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        let state = VisibilityTracker::Visible;
        let state = state.apply_event(Event::ReleaseVisible, now);

        while let Outcome::Visible = state.get_outcome(now) {
            now += Duration::from_millis(1);
            assert!(
                now < start + Duration::from_millis(250),
                "Hiding too slow: {:?}",
                now.saturating_duration_since(start),
            );
        }
    }
    
    /// Check against the false showing bug.
    /// Expectation: it will get hidden and not appear again
    #[test]
    fn false_show() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        let state = VisibilityTracker::Visible;
        // This reflects the sequence from Wayland:
        // disable, disable, enable, disable
        // all in a single batch.
        let state = state.apply_event(Event::ReleaseVisible, now);
        let state = state.apply_event(Event::ReleaseVisible, now);
        let state = state.apply_event(Event::ClaimVisible, now);
        let state = state.apply_event(Event::ReleaseVisible, now);

        while let Outcome::Visible = state.get_outcome(now) {
            now += Duration::from_millis(1);
            assert!(
                now < start + Duration::from_millis(250),
                "Still not hidden: {:?}",
                now.saturating_duration_since(start),
            );
        }
        
        // One second without appearing again
        for _i in 0..1000 {
            now += Duration::from_millis(1);
            assert_eq!(
                state.get_outcome(now),
                Outcome::Hidden,
                "Appeared unnecessarily: {:?}",
                now.saturating_duration_since(start),
            );
        }
    }

    #[test]
    fn schedule_hide() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        
        let l = LoopState::new(VisibilityTracker::Visible);
        let (l, outcome) = handle_loop_event(l, Event::ReleaseVisible, now);
        assert_eq!(outcome, Some(Outcome::Visible));
        assert_eq!(l.scheduled_wakeup, Some(now + HIDING_TIMEOUT));
        
        now += HIDING_TIMEOUT;
        
        let (l, outcome) = handle_loop_event(l, Event::TimeoutReached(now), now);
        assert_eq!(outcome, Some(Outcome::Hidden));
        assert_eq!(l.scheduled_wakeup, None);
    }
}
