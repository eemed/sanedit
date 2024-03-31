#[macro_use]
extern crate sanedit_macros;

pub(crate) mod actions;
pub(crate) mod common;
pub(crate) mod draw;
pub(crate) mod editor;
pub(crate) mod events;
pub(crate) mod job_runner;
pub(crate) mod runtime;
pub(crate) mod server;

use std::thread;

use runtime::TokioRuntime;
pub use server::{Address, StartOptions};
use server::{EditorHandle, CHANNEL_SIZE};
use tokio::sync::mpsc::channel;

pub fn run_sync(addrs: Vec<Address>, opts: StartOptions) -> Option<thread::JoinHandle<()>> {
    let (send, recv) = channel(CHANNEL_SIZE);
    let handle = EditorHandle {
        sender: send,
        next_id: Default::default(),
    };

    // Start listening before starting the editor
    let runtime = TokioRuntime::new(handle.clone());
    runtime.block_on(server::spawn_listeners(addrs, handle));

    thread::Builder::new()
        .name("sanedit".into())
        .spawn(move || {
            if let Err(e) = editor::main_loop(runtime, recv, opts) {
                log::error!("Editor main loop exited with error {}.", e);
            }
        })
        .ok()
}
