use super::Mouse;

#[derive(Debug, Default)]
pub(crate) struct FiletreeView {
    pub(crate) selection: usize,
    pub(crate) show: bool,
    pub(crate) mouse: Mouse,
}
