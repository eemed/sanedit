#[macro_use]
extern crate sanedit_macros;

pub(crate) mod actions;
pub(crate) mod common;
pub(crate) mod draw;
pub(crate) mod editor;
pub(crate) mod runtime;

use std::{sync::mpsc::channel, thread};

use runtime::TokioRuntime;
use sanedit_server::{spawn_listeners, Address, EditorHandle, StartOptions};

pub fn run_sync(addrs: Vec<Address>, opts: StartOptions) -> Option<thread::JoinHandle<()>> {
    let (send, recv) = channel();
    let handle = EditorHandle {
        sender: send,
        next_id: Default::default(),
    };

    let runtime = TokioRuntime::new(handle.clone());
    runtime.block_on(spawn_listeners(addrs, handle));

    thread::Builder::new()
        .name("sanedit".into())
        .spawn(move || {
            if let Err(e) = editor::main_loop(runtime, recv, opts) {
                log::error!("Editor main loop exited with error {}.", e);
            }
        })
        .ok()
}
