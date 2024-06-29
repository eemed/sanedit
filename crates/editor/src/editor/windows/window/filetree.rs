#[derive(Debug)]
pub(crate) struct FiletreeView {
    pub(crate) selection: usize,
    pub(crate) scroll: usize,
}

impl Default for FiletreeView {
    fn default() -> Self {
        FiletreeView {
            selection: 0,
            scroll: 0,
        }
    }
}
