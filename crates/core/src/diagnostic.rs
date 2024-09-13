use crate::{range::BufferRange, severity::Severity};

#[derive(Debug, Clone)]
pub struct Diagnostic {
    severity: Severity,
    range: BufferRange,
    description: String,
}

impl Diagnostic {
    pub fn new(severity: Severity, range: BufferRange, description: &str) -> Diagnostic {
        Diagnostic {
            severity,
            range,
            description: description.into(),
        }
    }

    pub fn severity(&self) -> &Severity {
        &self.severity
    }

    pub fn range(&self) -> BufferRange {
        self.range.clone()
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}
