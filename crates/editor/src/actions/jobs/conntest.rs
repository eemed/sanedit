use std::{any::Any, time::Duration};

use sanedit_server::{ClientId, Job};

use crate::editor::{job_broker::KeepInTouch, Editor};

pub(crate) const DISCONNECT_DURATION: Duration = Duration::from_mins(10);

#[derive(Debug, Clone)]
pub(crate) struct ClientConnectionTest {
    id: ClientId,
}

impl ClientConnectionTest {
    pub fn new(id: ClientId) -> ClientConnectionTest {
        Self { id }
    }
}

impl Job for ClientConnectionTest {
    fn run(&self, mut ctx: sanedit_server::JobContext) -> sanedit_server::JobResult {
        let fut = async move {
            let mut ticker = tokio::time::interval(Duration::from_mins(8));

            loop {
                if ctx.kill.should_stop() {
                    break;
                }

                ticker.tick().await;
                ctx.send(());
            }
            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for ClientConnectionTest {
    fn client_id(&self) -> ClientId {
        self.id
    }

    fn on_message(&self, editor: &mut Editor, _msg: Box<dyn Any>) {
        editor.test_client_connections();
    }
}
