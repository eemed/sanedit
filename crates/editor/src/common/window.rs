use sanedit_messages::redraw::Point;

use crate::editor::windows::Window;

/// Return buffer position at a view point. If point is past line end, the line end position is
/// returned instead.
pub(crate) fn pos_at_point(win: &Window, point: Point) -> Option<usize> {
    if let Some(pos) = win.view().pos_at_point(point) {
        return Some(pos);
    }

    if let Some(point) = win.view().last_non_empty_cell(point.y) {
        if let Some(pos) = win.view().pos_at_point(point) {
            return Some(pos);
        }
    }

    None
}
