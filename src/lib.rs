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
extern crate regex;
extern crate serde;
extern crate xkbcommon;

#[macro_use]
mod logging;

mod action;
mod animation;
pub mod data;
mod drawing;
pub mod float_ord;
pub mod imservice;
mod keyboard;
mod layout;
mod locale;
mod manager;
mod outputs;
mod popover;
mod resources;
mod style;
mod submission;
pub mod tests;
pub mod util;
mod ui_manager;
mod vkeyboard;
mod xdg;
