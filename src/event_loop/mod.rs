/* Copyright (C) 2021 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! The loop abstraction for driving state changes.
 * It binds to the state tracker in `state::Application`,
 * and actually gets driven by a driver in the `driver` module.
 *
 * * * *
 * 
 * If we performed updates in a tight loop,
 * the state tracker would have been all we need.
 *
 * ``
 * loop {
 *  event = current_event()
 *  outcome = update_state(event)
 *  io.apply(outcome)
 * }
 * ``
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

pub mod driver;

// This module is tightly coupled to the shape of data passed around in this project.
// That's not a problem as long as there's only one loop.
// They can still be abstracted into Traits,
// and the loop parametrized over them.
use crate::main::Commands;
use crate::state;
use crate::state::Event;
use std::cmp;
use std::time::{ Duration, Instant };


/// This keeps the state of the tracker loop between iterations
#[derive(Clone)]
struct State {
    state: state::Application,
    scheduled_wakeup: Option<Instant>,
    last_update: Instant,
}

impl State {
    fn new(initial_state: state::Application, now: Instant) -> Self {
        Self {
            state: initial_state,
            scheduled_wakeup: None,
            last_update: now,
        }
    }
}

/// A single iteration of the loop, updating its persistent state.
/// - updates tracker state,
/// - determines outcome,
/// - determines next scheduled animation wakeup,
/// and because this is a pure function, it's easily testable.
/// It returns the new state, and the message to send onwards.
fn handle_event(
    mut loop_state: State,
    event: Event,
    now: Instant,
) -> (State, Commands) {
    // Calculate changes to send to the consumer,
    // based on publicly visible state.
    // The internal state may change more often than the publicly visible one,
    // so the resulting changes may be no-ops.
    let old_state = loop_state.state.clone();
    let last_update = loop_state.last_update;
    loop_state.state = loop_state.state.apply_event(event.clone(), now);
    loop_state.last_update = now;

    let new_outcome = loop_state.state.get_outcome(now);

    let commands = old_state.get_outcome(last_update)
        .get_commands_to_reach(&new_outcome);
    
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

    (loop_state, commands)
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::animation;
    use crate::imservice::{ ContentHint, ContentPurpose };
    use crate::main::PanelCommand;
    use crate::state::{ Application, InputMethod, InputMethodDetails, Presence, visibility };

    fn imdetails_new() -> InputMethodDetails {
        InputMethodDetails {
            purpose: ContentPurpose::Normal,
            hint: ContentHint::NONE,
        }
    }

    #[test]
    fn schedule_hide() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;

        let state = Application {
            im: InputMethod::Active(imdetails_new()),
            physical_keyboard: Presence::Missing,
            visibility_override: visibility::State::NotForced,
            ..Application::new(start)
        };
        
        let l = State::new(state, now);
        let (l, commands) = handle_event(l, InputMethod::InactiveSince(now).into(), now);
        assert_eq!(commands.panel_visibility, Some(PanelCommand::Show));
        assert_eq!(l.scheduled_wakeup, Some(now + animation::HIDING_TIMEOUT));
        
        now += animation::HIDING_TIMEOUT;
        
        let (l, commands) = handle_event(l, Event::TimeoutReached(now), now);
        assert_eq!(commands.panel_visibility, Some(PanelCommand::Hide));
        assert_eq!(l.scheduled_wakeup, None);
    }
}
