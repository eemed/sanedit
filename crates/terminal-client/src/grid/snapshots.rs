use std::cmp::max;

use sanedit_messages::redraw::snapshots::Snapshots;

use crate::{
    grid::{
        drawable::{DrawCursor, Drawable, Subgrid},
        Rect, Split,
    },
    ui::UIContext,
};

#[derive(Debug)]
pub(crate) struct CustomSnapshots {
    pub(crate) snapshots: Snapshots,
    pub(crate) scroll: usize,
}

impl CustomSnapshots {
    pub fn new(snapshots: Snapshots) -> CustomSnapshots {
        CustomSnapshots {
            snapshots,
            scroll: 0,
        }
    }

    pub fn split_off(&self, win: &mut Rect) -> Rect {
        const MIN: usize = 30;
        // Each level is indented by 2, and root starts at indent 2, +1 for possible directory marker
        // let max_item_width = self
        //     .items
        //     .items
        //     .iter()
        //     .map(|item| (item.level + 1) * 2 + item.name.chars().count() + 1)
        //     .max()
        //     .unwrap_or(0)
        //     + 1;
        let max_screen = max(MIN, win.width / 3);
        // let width = max_item_width.clamp(MIN, max_screen);
        win.split_off(Split::left_size(max_screen))
    }

    pub fn update_scroll_position(&mut self, rect: &Rect) {
        let height = rect.height;
        let sel = self.snapshots.selected;
        let at_least = sel.saturating_sub(height.saturating_sub(1));
        self.scroll = max(self.scroll, at_least);

        if self.scroll > sel {
            self.scroll = sel;
        }

        if self.scroll + height < sel {
            self.scroll = sel - (height / 2);
        }
    }
}

impl Drawable for CustomSnapshots {
    fn draw(&self, ctx: &UIContext, cells: Subgrid) {
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        if self.snapshots.in_focus {
            DrawCursor::Hide
        } else {
            DrawCursor::Ignore
        }
    }
}
