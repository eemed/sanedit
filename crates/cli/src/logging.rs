use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::panic;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use log::LevelFilter;

pub(crate) fn init_panic() {
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
}

pub(crate) fn init_logger(debug: bool) {
    static LOGGER: OnceLock<Option<Logger>> = OnceLock::new();
    let logger = LOGGER.get_or_init(|| {
        let level = if debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };

        // let ignore = vec![]
        //     .into_iter()
        //     .map(String::from)
        //     .collect();
        // Logger::new(level, LOG_FILE, ignore)
        let tmp = sanedit_core::tmp_dir()?;
        let log_file = tmp.join("sanedit.log");

        Some(Logger::new(level, log_file, vec![]))
    });

    if let Some(logger) = logger {
        log::set_max_level(logger.level);
        let _ = log::set_logger(logger);
    }
}

struct Logger {
    level: LevelFilter,
    output_file: Arc<Mutex<File>>,
    ignore_crates: Vec<String>,
}

impl Logger {
    pub fn new(level: LevelFilter, path: PathBuf, ignore: Vec<String>) -> Logger {
        Logger {
            level,
            output_file: Arc::new(Mutex::new(
                File::create(path).expect("Failed to open log file"),
            )),
            ignore_crates: ignore,
        }
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
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = chrono::Local::now().format("%H:%M:%S.%3f");
        let mut file = self.output_file.lock().expect("Failed to lock logger");
        let _ = writeln!(
            file,
            "{} {} {}:{} {}",
            record.level(),
            timestamp,
            record.file().unwrap_or(""),
            record
                .line()
                .map(|n| n.to_string())
                .unwrap_or(String::new()),
            record.args()
        );
    }

    fn flush(&self) {
        let _ = self
            .output_file
            .lock()
            .expect("Failed to lock logger")
            .flush();
    }
}
