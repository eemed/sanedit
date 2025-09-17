use std::{
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use sanedit_server::{BoxFuture, ClientId, Kill};
use sanedit_utils::appendlist::Appendlist;

use crate::{common::choice::Choice, editor::Editor};

use super::MatcherMessage;

/// Provides options to match
pub(crate) trait OptionProvider: fmt::Debug + Sync + Send {
    fn provide(
        &self,
        sender: Appendlist<Arc<Choice>>,
        kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()>;
}

impl OptionProvider for Arc<Vec<String>> {
    fn provide(
        &self,
        sender: Appendlist<Arc<Choice>>,
        _kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            let mut n = 0;
            for opt in items.iter() {
                n += 1;
                sender.append(Choice::from_text(opt.clone()));
            }

            done.store(n, Ordering::Release);
        };

        Box::pin(fut)
    }
}

impl OptionProvider for Arc<Vec<&'static str>> {
    fn provide(
        &self,
        sender: Appendlist<Arc<Choice>>,
        _kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            let mut n = 0;
            for opt in items.iter() {
                n += 1;
                sender.append(Choice::from_text(opt.to_string()));
            }

            done.store(n, Ordering::Release);
        };

        Box::pin(fut)
    }
}

impl OptionProvider for Arc<Vec<Arc<Choice>>> {
    fn provide(
        &self,
        sender: Appendlist<Arc<Choice>>,
        _kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            let mut n = 0;
            for opt in items.iter() {
                n += 1;
                sender.append(opt.clone());
            }

            done.store(n, Ordering::Release);
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
    fn provide(
        &self,
        _sender: Appendlist<Arc<Choice>>,
        _kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            done.store(0, Ordering::Release);
        })
    }
}
