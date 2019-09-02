use std::{panic, thread};

/// Install a panic handler which logs panics on errors.
/// Adapted from: https://github.com/sfackler/rust-log-panics/blob/master/src/lib.rs
pub fn panic_logger() {
    panic::set_hook(Box::new(|info| {
        let thread = thread::current();
        let thread = thread.name().unwrap_or("unnamed");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "?",
            },
        };

        match info.location() {
            Some(location) => {
                log::error!(
                    target: "panic", "thread '{}' panicked at '{}': {}:{}",
                    thread,
                    msg,
                    location.file(),
                    location.line(),
                );
            }
            None => {
                log::error!(
                    target: "panic",
                    "thread '{}' panicked at '{}'",
                    thread,
                    msg,
                );
            }
        }

        log::error!("Since the process panicked it will now shut down :(");
        std::process::abort();
    }));
}
