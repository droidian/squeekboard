#[macro_use]
extern crate bitflags;
extern crate cairo;
extern crate cairo_sys;
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate glib_sys;
extern crate gtk;
extern crate gtk_sys;
#[allow(unused_imports)]
#[macro_use] // only for tests
extern crate maplit;
extern crate serde;
extern crate xkbcommon;
extern crate zbus;
extern crate zvariant;

#[cfg(test)]
#[macro_use]
mod assert_matches;
#[macro_use]
mod logging;

mod action;
mod animation;
pub mod data;
mod debug;
mod drawing;
mod event_loop;
pub mod float_ord;
pub mod imservice;
mod keyboard;
mod layout;
mod locale;
mod main;
mod manager;
mod outputs;
mod popover;
mod resources;
mod state;
mod style;
mod submission;
pub mod tests;
pub mod util;
mod vkeyboard;
mod xdg;
