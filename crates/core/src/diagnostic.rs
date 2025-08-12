use crate::{range::BufferRange, severity::Severity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    range: BufferRange,
    severity: Severity,
    description: String,
    line: u64,
}

impl Diagnostic {
    pub fn new(severity: Severity, range: BufferRange, line: u64, description: &str) -> Diagnostic {
        let description = description
            .lines()
            .map(|line| line.replace("\t", "        "))
            .collect::<Vec<String>>()
            .join(" ");

        Diagnostic {
            severity,
            range,
            description,
            line,
        }
    }

    pub fn severity(&self) -> &Severity {
        &self.severity
    }

    pub fn range(&self) -> &BufferRange {
        &self.range
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn line(&self) -> u64 {
        self.line
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
