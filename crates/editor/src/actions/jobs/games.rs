use std::time::Duration;

use crossbeam::channel::{self, Sender, TryRecvError};
use sanedit_server::{ClientId, Job};

use crate::editor::job_broker::KeepInTouch;

#[derive(Debug, Clone)]
pub(crate) struct GameTick {
    id: ClientId,
}

impl GameTick {
    pub fn new(id: ClientId) -> GameTick {
        Self { id }
    }
}

impl Job for GameTick {
    fn run(&self, ctx: sanedit_server::JobContext) -> sanedit_server::JobResult {
        let (send, mut recv) = channel::unbounded();

        let fut = async move {
            ctx.send(Start(send));

            let mut rate = recv.recv()?;
            loop {
                tokio::time::sleep(Duration::from_millis(rate)).await;

                match recv.try_recv() {
                    Ok(nrate) => {
                        if nrate == 0 {
                            let (chan_send, chan_rx) = channel::unbounded();
                            recv = chan_rx;
                            ctx.send(Start(chan_send));
                            rate = recv.recv()?;
                            continue;
                        }

                        rate = nrate;
                    }
                    Err(e) => match e {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => break,
                    },
                }

                ctx.send(Tick);
            }
            Ok(())
        };

        Box::pin(fut)
    }
}

struct Start(Sender<u64>);
struct Tick;

impl KeepInTouch for GameTick {
    fn client_id(&self) -> sanedit_server::ClientId {
        self.id
    }

    fn on_message(&self, editor: &mut crate::editor::Editor, mut msg: Box<dyn std::any::Any>) {
        if msg.downcast_mut::<Tick>().is_some() {
            let (win, _) = win_buf!(editor, self.id);
            if let Some(game) = win.game.as_mut() {
                game.tick();
            }
        }

        if let Ok(start) = msg.downcast::<Start>() {
            let (win, _) = win_buf!(editor, self.id);
            if let Some(game) = win.game.as_mut() {
                game.set_tick_sender(start.0);
            }
        }
    }
}
