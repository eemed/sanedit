#[macro_use]
extern crate sanedit_macros;

pub(crate) mod actions;
pub(crate) mod common;
pub(crate) mod draw;
pub(crate) mod editor;
pub(crate) mod events;
pub(crate) mod grammar;
pub(crate) mod server;

pub use server::{run_sync, Address, StartOptions};
