// Contains different jobs to run in tokio runtime
mod matcher;
mod open_file;
mod search;
mod shell;
mod syntax;
mod text;

pub(crate) const CHANNEL_SIZE: usize = 64;

pub(crate) use matcher::*;
pub(crate) use open_file::*;
pub(crate) use search::*;
pub(crate) use shell::*;
pub(crate) use syntax::*;
pub(crate) use text::*;
