// Contains different jobs to run in tokio runtime
mod directory_option_provider;
mod file_option_provider;
mod grep;
mod matcher;
mod search;
mod shell;
mod syntax;
// mod text;

pub(crate) const CHANNEL_SIZE: usize = 64;

pub(crate) use self::grep::*;
pub(crate) use file_option_provider::*;
pub(crate) use matcher::*;
pub(crate) use search::*;
pub(crate) use shell::*;
pub(crate) use syntax::*;
// pub(crate) use text::*;
