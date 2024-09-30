use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::panic;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use log::LevelFilter;
// use log4rs::append::file::FileAppender;
// use log4rs::config::{Appender, Config, Logger, Root};
// use log4rs::encode::pattern::PatternEncoder;

const LOG_FILE: &str = "/tmp/sanedit.log";

pub fn setup(debug: bool) {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();
        log::error!("{backtrace}");

        let (filename, line) = panic_info
            .location()
            .map(|loc| (loc.file(), loc.line()))
            .unwrap_or(("<unknown>", 0));

        let cause = panic_info
            .payload()
            .downcast_ref::<String>()
            .map(String::deref);

        let cause = cause.unwrap_or_else(|| {
            panic_info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .unwrap_or("<cause unknown>")
        });

        log::error!("A panic occurred at {}:{}: {}", filename, line, cause);
    }));

        let level= if debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };

        let ignore = vec!["regex_cursor", "grep_regex", "globset"]
            .into_iter()
            .map(String::from)
            .collect();
    let logger = Logger::new(level, LOG_FILE, ignore);

    logger.init();
}

struct Logger {
    level: LevelFilter,
    output_file: Rc<RefCell<File>>,
    ignore_crates: Vec<String>,
}

impl Logger {
    pub fn new(level: LevelFilter, path: &str, ignore: Vec<String>) -> Logger {
    }

    pub fn init(self) {
        log::set_max_level(self.level);
        let _ = log::set_boxed_logger(Box::new(self));
    }

    fn is_ignored(&self, module: &str) -> bool {
        for c in &self.ignore_crates {
            if c == module {
                return true;
            }
        }

        false
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level && !self.is_ignored(metadata.target())
    }

    fn log(&self, record: &log::Record) {
        todo!()
    }

    fn flush(&self) {
        self.output_file.flush()
    }
}
