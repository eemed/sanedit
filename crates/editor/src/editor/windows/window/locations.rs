use sanedit_server::JobId;

use super::Mouse;

/// Extra data about locations
#[derive(Debug, Default)]
pub(crate) struct LocationsView {
    pub show: bool,
    pub is_loading: bool,
    /// Backing job that loads stuff to locations
    pub job: Option<JobId>,
    pub mouse: Mouse,
}
