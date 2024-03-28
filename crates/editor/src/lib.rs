#[macro_use]
extern crate sanedit_macros;

pub(crate) mod actions;
pub(crate) mod common;
pub(crate) mod draw;
pub(crate) mod editor;
pub(crate) mod events;
pub(crate) mod job_runner;
pub(crate) mod server;

use std::thread;

use lazy_static::lazy_static;

pub use server::{Address, StartOptions};
use server::{EditorHandle, CHANNEL_SIZE};
use tokio::{runtime::Runtime, sync::mpsc::channel};

lazy_static! {
    pub(crate) static ref RUNTIME: Runtime = Runtime::new().ok().unwrap();
}

pub fn run_sync(addrs: Vec<Address>, opts: StartOptions) -> Option<thread::JoinHandle<()>> {
    let (send, recv) = channel(CHANNEL_SIZE);
    let handle = EditorHandle {
        sender: send,
        next_id: Default::default(),
    };
    RUNTIME.block_on(server::spawn_listeners(addrs, handle.clone()));

    thread::Builder::new()
        .name("sanedit".into())
        .spawn(move || {
            if let Err(e) = editor::main_loop(handle, recv, opts) {
                log::error!("Editor main loop exited with error {}.", e);
            }
        })
        .ok()
}
