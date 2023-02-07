use crate::{
    editor::{jobs::ListFiles, Editor},
    server::ClientId,
};

pub(crate) fn jobs_test(editor: &mut Editor, id: ClientId) {
    let jobs = editor.jobs_mut();
    jobs.new_job(ListFiles::default());
}
