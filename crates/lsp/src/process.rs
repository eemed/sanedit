use std::collections::BTreeMap;

use crate::error::LSPError;
use crate::jsonrpc::{read_from, JsonNotification, JsonRequest, JsonResponse};

use sanedit_utils::either::Either;
use serde_json::Value;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::{io::BufReader, process::Child};

#[derive(Debug)]
pub(crate) enum ServerRequest {
    Request {
        json: JsonRequest,
        answer: Sender<Result<Value, String>>,
    },
    Notification {
        json: JsonNotification,
    },
}

pub(crate) struct ProcessHandler {
    pub(crate) _process: Child,
    pub(crate) stdin: ChildStdin,
    pub(crate) stdout: BufReader<ChildStdout>,
    pub(crate) _stderr: BufReader<ChildStderr>,

    pub(crate) notification_sender: Sender<JsonNotification>,
    pub(crate) receiver: Receiver<ServerRequest>,
    pub(crate) in_flight: BTreeMap<u32, Sender<Result<Value, String>>>,
}

impl ProcessHandler {
    pub async fn run(mut self) -> Result<(), LSPError> {
        //--------------------
        // let mut buf = vec![];
        // use tokio::io::AsyncBufReadExt;
        //--------------------

        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    match msg {
                        Some(ServerRequest::Request { json, answer }) => self.handle_request(json, answer).await?,
                        Some(ServerRequest::Notification { json }) => self.handle_notification(json).await?,
                        None => return Err(LSPError::Receive),
                    }
                }


                // Ok(read) = self._stderr.read_until(b'\n', &mut buf) => {
                //     if let Ok(res) = std::str::from_utf8(&buf[..read]) {
                //         log::error!("lsp: {res}");
                //     }
                // }
                json = read_from(&mut self.stdout) => {
                    match json? {
                        Either::Right(notification) =>
                            self.handle_response_notification(notification).await?,
                        Either::Left(response) => self.handle_response(response).await?,
                    }
                }
            };
        }

    }

    async fn handle_request(
        &mut self,
        json: JsonRequest,
        answer: Sender<Result<Value, String>>,
    ) -> Result<(), LSPError> {
        let id = json.id();
        json.write_to(&mut self.stdin).await?;
        self.in_flight.insert(id, answer);
        Ok(())
    }

    async fn handle_notification(&mut self, json: JsonNotification) -> Result<(), LSPError> {
        json.write_to(&mut self.stdin).await?;
        Ok(())
    }

    async fn handle_response_notification(
        &mut self,
        notif: JsonNotification,
    ) -> Result<(), LSPError> {
        let _ = self.notification_sender.send(notif).await;
        Ok(())
    }

    async fn handle_response(&mut self, response: JsonResponse) -> Result<(), LSPError> {
        if response.result.is_none() && response.error.is_none() {
            return Ok(());
        }

        let sender = self
            .in_flight
            .remove(&response.id)
            .ok_or(LSPError::ResponseToNonexistentRequest)?;

        let result = response.result.ok_or(format!("{:?}", response.error));
        let _ = sender.send(result).await;
        Ok(())
    }
}
