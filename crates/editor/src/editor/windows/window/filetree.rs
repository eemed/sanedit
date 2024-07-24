#[derive(Debug)]
pub(crate) struct FiletreeView {
    pub(crate) selection: usize,
    pub(crate) show: bool,
}

impl Default for FiletreeView {
    fn default() -> Self {
        FiletreeView {
            selection: 0,
            show: false,
        }
    }
}
