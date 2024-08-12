use sanedit_messages::redraw::Severity;

use super::BufferRange;

#[derive(Debug)]
pub(crate) struct Diagnostic {
    severity: Severity,
    range: BufferRange,
    description: String,
}

impl Diagnostic {
    pub fn new(severity: Severity, range: BufferRange, description: String) -> Diagnostic {
        Diagnostic {
            severity,
            range,
            description,
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
