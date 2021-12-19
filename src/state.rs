/* Copyright (C) 2021 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Application-wide state is stored here.
 * It's driven by the loop defined in the loop module. */

use crate::animation;
use crate::imservice::{ ContentHint, ContentPurpose };
use crate::main::{ Commands, PanelCommand };
use std::time::Instant;


#[derive(Clone, Copy)]
pub enum Presence {
    Present,
    Missing,
}

#[derive(Clone)]
pub struct InputMethodDetails {
    pub hint: ContentHint,
    pub purpose: ContentPurpose,
}

#[derive(Clone)]
pub enum InputMethod {
    Active(InputMethodDetails),
    InactiveSince(Instant),
}

/// Incoming events
#[derive(Clone)]
pub enum Event {
    InputMethod(InputMethod),
    Visibility(visibility::Event),
    PhysicalKeyboard(Presence),
    /// Event triggered because a moment in time passed.
    /// Use to animate state transitions.
    /// The value is the ideal arrival time.
    TimeoutReached(Instant),
}

impl From<InputMethod> for Event {
    fn from(im: InputMethod) -> Self {
        Self::InputMethod(im)
    }
}

pub mod visibility {
    #[derive(Clone)]
    pub enum Event {
        /// User requested the panel to show
        ForceVisible,
        /// The user requested the panel to go down
        ForceHidden,
    }

    #[derive(Clone, PartialEq, Debug, Copy)]
    pub enum State {
        /// Last interaction was user forcing the panel to go visible
        ForcedVisible,
        /// Last interaction was user forcing the panel to hide
        ForcedHidden,
        /// Last interaction was the input method changing active state
        NotForced,
    }
}

/// The outwardly visible state.
#[derive(Clone)]
pub struct Outcome {
    pub visibility: animation::Outcome,
    pub im: InputMethod,
}

impl Outcome {
    /// Returns the commands needed to apply changes as required by the new state.
    /// This implementation doesn't actually take the old state into account,
    /// instead issuing all the commands as needed to reach the new state.
    /// The receivers of the commands bear the burden
    /// of checking if the commands end up being no-ops.
    pub fn get_commands_to_reach(&self, new_state: &Self) -> Commands {
        let layout_hint_set = match new_state {
            Outcome {
                visibility: animation::Outcome::Visible,
                im: InputMethod::Active(hints),
            } => Some(hints.clone()),
            
            Outcome {
                visibility: animation::Outcome::Visible,
                im: InputMethod::InactiveSince(_),
            } => Some(InputMethodDetails {
                hint: ContentHint::NONE,
                purpose: ContentPurpose::Normal,
            }),
            
            Outcome {
                visibility: animation::Outcome::Hidden,
                ..
            } => None,
        };

        let (dbus_visible_set, panel_visibility) = match new_state.visibility {
            animation::Outcome::Visible => (Some(true), Some(PanelCommand::Show)),
            animation::Outcome::Hidden => (Some(false), Some(PanelCommand::Hide)),
        };

        Commands {
            panel_visibility,
            layout_hint_set,
            dbus_visible_set,
        }
    }
}

/// The actual logic of the program.
/// At this moment, limited to calculating visibility and IM hints.
///
/// It keeps the panel visible for a short time period after each hide request.
/// This prevents flickering on quick successive enable/disable events.
/// It does not treat user-driven hiding in a special way.
///
/// This is the "functional core".
/// All state changes return the next state and the optimal time for the next check.
///
/// This state tracker can be driven by any event loop.
#[derive(Clone)]
pub struct Application {
    pub im: InputMethod,
    pub visibility_override: visibility::State,
    pub physical_keyboard: Presence,
}

impl Application {
    /// A conservative default, ignoring the actual state of things.
    /// It will initially show the keyboard for a blink.
    // The ignorance might actually be desired,
    // as it allows for startup without waiting for a system check.
    // The downside is that adding actual state should not cause transitions.
    // Another acceptable alternative is to allow explicitly uninitialized parts.
    pub fn new(now: Instant) -> Self {
        Self {
            im: InputMethod::InactiveSince(now),
            visibility_override: visibility::State::NotForced,
            physical_keyboard: Presence::Missing,
        }
    }

    pub fn apply_event(self, event: Event, _now: Instant) -> Self {
        match event {
            Event::TimeoutReached(_) => self,

            Event::Visibility(visibility) => Self {
                visibility_override: match visibility {
                    visibility::Event::ForceHidden => visibility::State::ForcedHidden,
                    visibility::Event::ForceVisible => visibility::State::ForcedVisible,
                },
                ..self
            },

            Event::PhysicalKeyboard(presence) => Self {
                physical_keyboard: presence,
                ..self
            },

            Event::InputMethod(new_im) => match (self.im.clone(), new_im) {
                (InputMethod::Active(_old), InputMethod::Active(new_im))
                => Self {
                    im: InputMethod::Active(new_im),
                    ..self
                },
                // For changes in active state, remove user's visibility override.
                // Both cases spelled out explicitly, rather than by the wildcard,
                // to not lose the notion that it's the opposition that matters
                (InputMethod::InactiveSince(_old), InputMethod::Active(new_im))
                => Self {
                    im: InputMethod::Active(new_im),
                    visibility_override: visibility::State::NotForced,
                    ..self
                },
                (InputMethod::Active(_old), InputMethod::InactiveSince(since))
                => Self {
                    im: InputMethod::InactiveSince(since),
                    visibility_override: visibility::State::NotForced,
                    ..self
                },
                // This is a weird case, there's no need to update an inactive state.
                // But it's not wrong, just superfluous.
                (InputMethod::InactiveSince(old), InputMethod::InactiveSince(_new))
                => Self {
                    // New is going to be newer than old, so it can be ignored.
                    // It was already inactive at that moment.
                    im: InputMethod::InactiveSince(old),
                    ..self
                },
            }
        }
    }

    pub fn get_outcome(&self, now: Instant) -> Outcome {
        // FIXME: include physical keyboard presence
        Outcome {
            visibility: match (self.physical_keyboard, self.visibility_override) {
                (_, visibility::State::ForcedHidden) => animation::Outcome::Hidden,
                (_, visibility::State::ForcedVisible) => animation::Outcome::Visible,
                (Presence::Present, visibility::State::NotForced) => animation::Outcome::Hidden,
                (Presence::Missing, visibility::State::NotForced) => match self.im {
                    InputMethod::Active(_) => animation::Outcome::Visible,
                    InputMethod::InactiveSince(since) => {
                        if now < since + animation::HIDING_TIMEOUT { animation::Outcome::Visible }
                        else { animation::Outcome::Hidden }
                    },
                },
            },
            im: self.im.clone(),
        }
    }

    /// Returns the next time to update the outcome.
    pub fn get_next_wake(&self, now: Instant) -> Option<Instant> {
        match self {
            Self {
                visibility_override: visibility::State::NotForced,
                im: InputMethod::InactiveSince(since),
                ..
            } => {
                let anim_end = *since + animation::HIDING_TIMEOUT;
                if now < anim_end { Some(anim_end) }
                else { None }
            }
            _ => None,
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    use std::time::Duration;

    fn imdetails_new() -> InputMethodDetails {
        InputMethodDetails {
            purpose: ContentPurpose::Normal,
            hint: ContentHint::NONE,
        }
    }

    /// Test the original delay scenario: no flicker on quick switches.
    #[test]
    fn avoid_hide() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        let state = Application {
            im: InputMethod::Active(imdetails_new()),
            physical_keyboard: Presence::Missing,
            visibility_override: visibility::State::NotForced,
        };

        let state = state.apply_event(Event::InputMethod(InputMethod::InactiveSince(now)), now);
        // Check 100ms at 1ms intervals. It should remain visible.
        for _i in 0..100 {
            now += Duration::from_millis(1);
            assert_eq!(
                state.get_outcome(now).visibility,
                animation::Outcome::Visible,
                "Hidden when it should remain visible: {:?}",
                now.saturating_duration_since(start),
            )
        }

        let state = state.apply_event(Event::InputMethod(InputMethod::Active(imdetails_new())), now);

        assert_eq!(state.get_outcome(now).visibility, animation::Outcome::Visible);
    }

    /// Make sure that hiding works when input method goes away
    #[test]
    fn hide() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        let state = Application {
            im: InputMethod::Active(imdetails_new()),
            physical_keyboard: Presence::Missing,
            visibility_override: visibility::State::NotForced,
        };
        
        let state = state.apply_event(Event::InputMethod(InputMethod::InactiveSince(now)), now);

        while let animation::Outcome::Visible = state.get_outcome(now).visibility {
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
        let state = Application {
            im: InputMethod::Active(imdetails_new()),
            physical_keyboard: Presence::Missing,
            visibility_override: visibility::State::NotForced,
        };
        // This reflects the sequence from Wayland:
        // disable, disable, enable, disable
        // all in a single batch.
        let state = state.apply_event(Event::InputMethod(InputMethod::InactiveSince(now)), now);
        let state = state.apply_event(Event::InputMethod(InputMethod::InactiveSince(now)), now);
        let state = state.apply_event(Event::InputMethod(InputMethod::Active(imdetails_new())), now);
        let state = state.apply_event(Event::InputMethod(InputMethod::InactiveSince(now)), now);

        while let animation::Outcome::Visible = state.get_outcome(now).visibility {
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
                state.get_outcome(now).visibility,
                animation::Outcome::Hidden,
                "Appeared unnecessarily: {:?}",
                now.saturating_duration_since(start),
            );
        }
    }

    #[test]
    fn force_visible() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        let state = Application {
            im: InputMethod::InactiveSince(now),
            physical_keyboard: Presence::Missing,
            visibility_override: visibility::State::NotForced,
        };
        now += Duration::from_secs(1);

        let state = state.apply_event(Event::Visibility(visibility::Event::ForceVisible), now);
        assert_eq!(
            state.get_outcome(now).visibility,
            animation::Outcome::Visible,
            "Failed to show: {:?}",
            now.saturating_duration_since(start),
        );
        
        now += Duration::from_secs(1);
        let state = state.apply_event(Event::InputMethod(InputMethod::Active(imdetails_new())), now);
        now += Duration::from_secs(1);
        let state = state.apply_event(Event::InputMethod(InputMethod::InactiveSince(now)), now);
        now += Duration::from_secs(1);

        assert_eq!(
            state.get_outcome(now).visibility,
            animation::Outcome::Hidden,
            "Failed to release forced visibility: {:?}",
            now.saturating_duration_since(start),
        );
    }

    #[test]
    fn keyboard_present() {
        let start = Instant::now(); // doesn't matter when. It would be better to have a reproducible value though
        let mut now = start;
        let state = Application {
            im: InputMethod::Active(imdetails_new()),
            physical_keyboard: Presence::Missing,
            visibility_override: visibility::State::NotForced,
        };
        now += Duration::from_secs(1);

        let state = state.apply_event(Event::PhysicalKeyboard(Presence::Present), now);
        assert_eq!(
            state.get_outcome(now).visibility,
            animation::Outcome::Hidden,
            "Failed to hide: {:?}",
            now.saturating_duration_since(start),
        );
        
        now += Duration::from_secs(1);
        let state = state.apply_event(Event::InputMethod(InputMethod::InactiveSince(now)), now);
        now += Duration::from_secs(1);
        let state = state.apply_event(Event::InputMethod(InputMethod::Active(imdetails_new())), now);

        assert_eq!(
            state.get_outcome(now).visibility,
            animation::Outcome::Hidden,
            "Failed to remain hidden: {:?}",
            now.saturating_duration_since(start),
        );

        now += Duration::from_secs(1);
        let state = state.apply_event(Event::PhysicalKeyboard(Presence::Missing), now);

        assert_eq!(
            state.get_outcome(now).visibility,
            animation::Outcome::Visible,
            "Failed to appear: {:?}",
            now.saturating_duration_since(start),
        );

    }
}
