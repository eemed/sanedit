#[macro_use]
extern crate sanedit_macros;

/// Get option or return ActionResult::Failed
macro_rules! getf {
    ($thing:expr) => {{
        match $thing {
            Some(thing) => thing,
            _ => return ActionResult::Failed,
        }
    }};
}

/// Get option if Some otherwise return
macro_rules! get {
    ($thing:expr) => {{
        match $thing {
            Some(thing) => thing,
            _ => return,
        }
    }};
}

/// Quick way to borrow just window and buffer mutably from the editor.
/// Used if mutable access is needed in other parts of editor
macro_rules! win_buf {
    ($editor:ident, $id:ident) => {{
        let win = $editor
            .windows
            .get_mut($id)
            .expect("no win for cliend id {id}");
        let bid = win.buffer_id();
        let buf = $editor
            .buffers
            .get_mut(bid)
            .expect("no buffer for buffer id {bid}");
        (win, buf)
    }};
}

pub(crate) mod actions;
pub(crate) mod common;
pub(crate) mod draw;
pub(crate) mod editor;

use std::thread;

use sanedit_server::{spawn_listeners, Address, EditorHandle, StartOptions};

// works only with cargo
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run_sync(addrs: Vec<Address>, opts: StartOptions) -> Option<thread::JoinHandle<()>> {
    let (send, recv) = crossbeam::channel::unbounded();
    let handle = EditorHandle {
        sender: send,
        next_id: Default::default(),
    };

    // TODO multiworker
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .on_thread_start(|| {
            log::info!("Thread stater");
        })
        .enable_all()
        .build()
        .unwrap();

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
