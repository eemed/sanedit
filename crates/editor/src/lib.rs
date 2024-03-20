#[macro_use]
extern crate sanedit_macros;

macro_rules! redraw {
    () => {{
        crate::editor::REDRAW_NOTIFY.notify_one();
    }};
}

pub(crate) mod actions;
pub(crate) mod common;
pub(crate) mod draw;
pub(crate) mod editor;
pub(crate) mod events;
pub(crate) mod server;
pub(crate) mod syntax;

pub use server::{run_sync, Address, StartOptions};
