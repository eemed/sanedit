use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use tokio::{sync::Notify, time::timeout};

use crate::events::ToEditor;

use super::EditorHandle;

fn notifier() -> &'static Notify {
    static REDRAW_NOTIFY: OnceLock<Arc<Notify>> = OnceLock::new();
    REDRAW_NOTIFY.get_or_init(|| Notify::const_new().into())
}

pub(crate) fn redraw() {
    notifier().notify_one();
}

pub(crate) async fn redraw_debouncer(mut handle: EditorHandle) {
    let notif = notifier();
    loop {
        // Wait one notification, send it immediately
        notif.notified().await;
        handle.send(ToEditor::Redraw).await;

        // Then update 30fps until we hit timeout without any messages
        let target = Duration::from_millis(1000 / 30);
        let mut received_at = None;
        loop {
            let limit = received_at
                .as_ref()
                .map(|r| target.saturating_sub(Instant::now().duration_since(*r)))
                .unwrap_or(target);
            match timeout(limit, notif.notified()).await {
                Ok(_) => {
                    if received_at.is_none() {
                        received_at = Some(Instant::now());
                    }
                }
                Err(_) => {
                    if received_at.is_some() {
                        received_at = None;
                        handle.send(ToEditor::Redraw).await;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
