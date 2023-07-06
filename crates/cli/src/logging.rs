use std::ops::Deref;
use std::panic;

use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

const LOG_FILE: &str = "/tmp/sanedit.log";
const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

pub fn setup() {
    panic::set_hook(Box::new(|panic_info| {
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
                .map(|s| *s)
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

    let config = Config::builder()
        .appender(Appender::builder().build("file-appender", Box::new(file_appender)))
        .build(Root::builder().appender("file-appender").build(LOG_LEVEL))
        .unwrap();

    let _handle = log4rs::init_config(config).unwrap();
}
