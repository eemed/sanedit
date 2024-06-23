use crate::editor::filetree::Filetree;

use super::DrawContext;

pub(crate) fn draw(tree: &Filetree, ctx: &mut DrawContext) {
    log::info!("FT: {tree:?}");
}
