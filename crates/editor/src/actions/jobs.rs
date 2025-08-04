// Contains different jobs to run in tokio runtime
mod directory_option_provider;
mod file_option_provider;
mod games;
mod grep;
mod lsp;
mod matcher;
mod search;
mod shell;
mod syntax;
// mod text;

pub(crate) const CHANNEL_SIZE: usize = 64;

pub(crate) use self::grep::*;
pub(crate) use directory_option_provider::*;
pub(crate) use file_option_provider::*;
pub(crate) use games::*;
pub(crate) use lsp::*;
pub(crate) use matcher::*;
pub(crate) use search::*;
pub(crate) use syntax::*;
// pub(crate) use text::*;
