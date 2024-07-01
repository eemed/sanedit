#[derive(Debug)]
pub(crate) struct FiletreeView {
    pub(crate) selection: usize,
}

impl Default for FiletreeView {
    fn default() -> Self {
        FiletreeView { selection: 0 }
    }
}
