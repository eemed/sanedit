use sanedit_messages::redraw::Point;

use crate::{editor::Editor, server::ClientId};

pub(crate) fn new_cursor_to_point(editor: &mut Editor, id: ClientId, point: Point) {}

pub(crate) fn remove_secondary_cursors(editor: &mut Editor, id: ClientId) {}
