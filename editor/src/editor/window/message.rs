#[derive(Clone, Debug, Copy)]
pub(crate) enum Severity {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug)]
pub(crate) struct Message {
    pub(crate) severity: Severity,
    pub(crate) message: String,
}
