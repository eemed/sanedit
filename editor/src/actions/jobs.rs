use crate::{editor::Editor, server::ClientId};

pub(crate) fn jobs_test(editor: &mut Editor, id: ClientId) {
    editor.jobs_mut().test();
}
