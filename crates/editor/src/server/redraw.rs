use std::time::{Duration, Instant};

use tokio::time::timeout;

use crate::{editor::REDRAW_NOTIFY, events::ToEditor};

use super::EditorHandle;

pub(crate) async fn redraw_debouncer(mut handle: EditorHandle) {
    let target = Duration::from_millis(1000 / 30);
    let mut received_at = None;
    loop {
        let limit = received_at
            .as_ref()
            .map(|r| target.saturating_sub(Instant::now().duration_since(*r)))
            .unwrap_or(target);

        match timeout(limit, REDRAW_NOTIFY.notified()).await {
            Ok(_) => {
                log::info!("notified");
                if received_at.is_none() {
                    received_at = Some(Instant::now());
                }
            }
            Err(_) => {
                if received_at.is_some() {
                    log::info!("redraw");
                    received_at = None;
                    handle.send(ToEditor::Redraw).await;
                }
            }
        }
    }
}
