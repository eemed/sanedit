use std::{any::Any, path::PathBuf};

pub trait Choice: Send + Sync + std::fmt::Debug {
    fn description(&self) -> &str;
    fn text(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}

impl std::hash::Hash for dyn Choice {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.text(), self.description()).hash(state)
    }
}

impl PartialEq for dyn Choice {
    fn eq(&self, other: &Self) -> bool {
        self.text().eq(other.text())
    }
}

impl Eq for dyn Choice {}

impl PartialOrd for dyn Choice {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.text().partial_cmp(other.text())
    }
}

impl Ord for dyn Choice {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.text().cmp(other.text())
    }
}

impl Choice for String {
    fn description(&self) -> &str {
        ""
    }

    fn text(&self) -> &str {
        self.as_str()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Choice for (String, String) {
    fn description(&self) -> &str {
        self.1.as_str()
    }

    fn text(&self) -> &str {
        self.0.as_str()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
