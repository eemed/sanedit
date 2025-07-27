use std::sync::Arc;

use sanedit_messages::redraw::window::WindowGrid;

#[derive(Debug)]
pub(crate) struct WindowBuffers {
    bufs: [Arc<WindowGrid>; 3],
    active: usize,
}

impl Default for WindowBuffers {
    fn default() -> Self {
        WindowBuffers {
            bufs: [
                Arc::new(WindowGrid::default()),
                Arc::new(WindowGrid::default()),
                Arc::new(WindowGrid::default()),
            ],
            active: 0,
        }
    }
}

impl WindowBuffers {
    pub fn get(&self) -> Arc<WindowGrid> {
        self.bufs[self.active].clone()
    }

    pub fn next_mut(&mut self) -> &mut WindowGrid {
        self.active = (self.active + 1) % self.bufs.len();
        let elem = &mut self.bufs[self.active];
        Arc::make_mut(elem)
    }
}
