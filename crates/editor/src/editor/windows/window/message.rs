#[derive(Clone, Debug, Copy, Default)]
pub(crate) enum Severity {
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Message {
    pub(crate) severity: Severity,
    pub(crate) message: String,
}
