use libc::signal;

extern "C" fn ignore(_signal: i32) {}

pub fn register_signal_handlers() {
    // Ignore hups
    unsafe {
        signal(libc::SIGHUP, ignore as usize);
    }
}
