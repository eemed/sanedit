use crate::{range::BufferRange, severity::Severity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    range: BufferRange,
    severity: Severity,
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

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (
            self.range.start,
            self.range.end,
            self.severity,
            &self.description,
        )
            .partial_cmp(&(
                other.range.start,
                other.range.end,
                other.severity,
                &other.description,
            ))
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            self.range.start,
            self.range.end,
            self.severity,
            &self.description,
        )
            .cmp(&(
                other.range.start,
                other.range.end,
                other.severity,
                &other.description,
            ))
    }
}
