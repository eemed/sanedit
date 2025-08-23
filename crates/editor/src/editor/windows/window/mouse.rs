use std::time::{Duration, Instant};

use sanedit_messages::redraw::Point;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub(crate) struct Mouse {
    last_click: Option<(Point, Instant)>,
    clicks: MouseClick,
}

impl Mouse {
    const DOUBLE_CLICK_DELAY: Duration = Duration::from_millis(200);

    pub fn on_click(&mut self, point: Point) {
        if let Some((ppoint, time)) = std::mem::take(&mut self.last_click) {
            if ppoint == point && time.elapsed() < Self::DOUBLE_CLICK_DELAY {
                self.clicks.next();
            } else {
                self.clicks = MouseClick::Single;
            }
        }

        self.last_click = Some((point, Instant::now()));
    }

    pub fn clicks(&self) -> &MouseClick {
        &self.clicks
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub(crate) enum MouseClick {
    #[default]
    Single,
    Double,
    Triple,
}

impl MouseClick {
    pub fn next(&mut self) {
        *self = match self {
            MouseClick::Single => MouseClick::Double,
            MouseClick::Double => MouseClick::Triple,
            MouseClick::Triple => MouseClick::Triple,
        };
    }
}
