use std::{
    any::Any, path::PathBuf, sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    }
};

use sanedit_core::Group;
use sanedit_server::{ClientId, Job, JobContext, JobResult};
use sanedit_syntax::GitGlob;
use sanedit_utils::appendlist::Appendlist;

use crate::{
    actions::jobs::OptionProvider,
    common::Choice,
    editor::{job_broker::KeepInTouch, Editor},
};

#[derive(Debug, Clone)]
pub struct LocationsGlobAdd {
    client_id: ClientId,
    glob: Arc<GitGlob>,
    opts: Arc<dyn OptionProvider>,
}

impl LocationsGlobAdd {
    pub fn new(
        client_id: ClientId,
        pattern: &str,
        opts: Arc<dyn OptionProvider>,
    ) -> anyhow::Result<LocationsGlobAdd> {
        let glob = GitGlob::new(pattern)?;
        Ok(LocationsGlobAdd {
            client_id,
            glob: Arc::new(glob),
            opts,
        })
    }

    pub async fn send_results(
        glob: Arc<GitGlob>,
        mut ctx: JobContext,
        reader: Appendlist<Arc<Choice>>,
        write_done: Arc<AtomicUsize>,
    ) {
        let mut taken = 0;

        while !ctx.kill.should_stop() {
            let total = write_done.load(Ordering::Acquire);
            let available = reader.len();
            let fully_read = available == total;

            if fully_read && available == taken {
                break;
            }

            while available > taken {
                match reader.get(taken) {
                    Some(choice) => {
                        taken += 1;
                        if let Choice::Path {  path, .. } = choice.as_ref() {
                            let lossy = path.to_string_lossy();
                            let bytes = lossy.as_bytes();
                            if glob.is_match(bytes) {
                                ctx.send(path.to_path_buf());
                            }
                        }
                    }
                    None => {
                        ctx.kill.stop();
                        return;
                    }
                }
            }

            tokio::task::yield_now().await;
        }
    }
}

impl Job for LocationsGlobAdd {
    fn run(&self, ctx: JobContext) -> JobResult {
        let glob = self.glob.clone();
        let opts = self.opts.clone();

        let fut = async move {
            let list = Appendlist::<Arc<Choice>>::new();
            let write_done = Arc::new(AtomicUsize::new(usize::MAX));
            let kill = ctx.kill.clone();
            tokio::join!(
                opts.provide(list.clone(), kill, write_done.clone()),
                Self::send_results(glob, ctx, list, write_done)
            );

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for LocationsGlobAdd {
    fn client_id(&self) -> sanedit_server::ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(path) = msg.downcast::<PathBuf>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            let groups = win.locations.groups();
            for group in groups {
                if group.path() == path.as_path() {
                    return;
                }
            }
            win.locations.push(Group::new(path.as_path()));
        }
    }
}
