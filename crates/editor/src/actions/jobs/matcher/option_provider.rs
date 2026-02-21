use std::{fmt, sync::Arc};

use crossbeam::channel::Sender;
use sanedit_server::{BoxFuture, ClientId};

use crate::{common::choice::Choice, editor::Editor};

use super::MatcherMessage;

/// Provides options to match
pub(crate) trait OptionProvider: fmt::Debug + Sync + Send {
    fn provide(&self, sender: Sender<Arc<Choice>>) -> BoxFuture<'static, ()>;
}

impl OptionProvider for Arc<Vec<String>> {
    fn provide(&self, sender: Sender<Arc<Choice>>) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            for opt in items.iter() {
                if sender.send(Choice::from_text(opt.clone())).is_err() {
                    break;
                }
            }
        };

        Box::pin(fut)
    }
}

impl OptionProvider for Arc<Vec<&'static str>> {
    fn provide(&self, sender: Sender<Arc<Choice>>) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            for opt in items.iter() {
                if sender.send(Choice::from_text(opt.to_string())).is_err() {
                    break;
                }
            }
        };

        Box::pin(fut)
    }
}

impl OptionProvider for Arc<Vec<Arc<Choice>>> {
    fn provide(&self, sender: Sender<Arc<Choice>>) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            for opt in items.iter() {
                if sender.send(opt.clone()).is_err() {
                    break;
                }
            }
        };

        Box::pin(fut)
    }
}

#[derive(Debug)]
pub(crate) struct Empty;
impl Empty {
    pub fn none_result_handler(_editor: &mut Editor, _id: ClientId, _msg: MatcherMessage) {}
}
impl OptionProvider for Empty {
    fn provide(&self, _sender: Sender<Arc<Choice>>) -> BoxFuture<'static, ()> {
        Box::pin(async move {})
    }
}
