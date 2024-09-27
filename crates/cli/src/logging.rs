use std::ops::Deref;
use std::panic;

use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;

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

    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{l} {d(%H:%M:%S.%3f)} {f}:{L} {m}{n}",
        )))
        .build(LOG_FILE)
        .unwrap();

    let level = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let config = Config::builder()
        .appender(Appender::builder().build("file-appender", Box::new(file_appender)))
        .logger(Logger::builder().build("regex_cursor", LevelFilter::Off))
        .logger(Logger::builder().build("grep_regex", LevelFilter::Off))
        .logger(Logger::builder().build("globset", LevelFilter::Off))
        .build(Root::builder().appender("file-appender").build(level))
        .unwrap();

    let _handle = log4rs::init_config(config).unwrap();
}
