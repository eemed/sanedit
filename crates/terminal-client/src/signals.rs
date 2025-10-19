use std::sync::OnceLock;

use crossbeam::channel::Sender;
use libc::signal;

use crate::ClientInternalMessage;

static EXIT: OnceLock<Sender<ClientInternalMessage>> = OnceLock::new();

extern "C" fn stop(_signal: i32) {
    if let Some(exit) = EXIT.get() {
        let _ = exit.send(ClientInternalMessage::Bye);
    }
}

pub fn register_signal_handlers(exit: Sender<ClientInternalMessage>) {
    EXIT.get_or_init(|| exit);

    unsafe {
        signal(libc::SIGHUP, stop as usize);
        signal(libc::SIGINT, stop as usize);
        signal(libc::SIGTERM, stop as usize);
    }
}
