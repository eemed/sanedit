mod view;
mod selection;

use self::view::View;

use super::BufferId;

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,

    view: View,
}
