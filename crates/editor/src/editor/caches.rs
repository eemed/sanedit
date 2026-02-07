use std::path::PathBuf;

use sanedit_utils::linkedarray::LinkedArray;

use super::config::Config;

#[derive(Debug)]
pub(crate) struct Caches {
    pub(crate) files: FilesMRU,
}

impl Caches {
    pub fn new(_config: &Config) -> Self {
        Caches {
            files: FilesMRU::new(),
        }
    }
}

/// Most recently used files
#[derive(Debug)]
pub(crate) struct FilesMRU {
    items: LinkedArray<PathBuf, 2>,
}

impl FilesMRU {
    pub fn new() -> FilesMRU {
        FilesMRU {
            items: Default::default(),
        }
    }

    pub fn insert(&mut self, path: PathBuf) {
        if let Some(pos) = self.items.contains(&path) {
            self.items.move_to_front(pos);
            return;
        }

        if self.items.is_full() {
            self.items.pop_last();
        }

        self.items.push_front(path);
    }

    /// Highest position if file was recently used
    pub fn position(&self, path: &PathBuf) -> Option<usize> {
        for (i, (_, item)) in self.items.iter().enumerate() {
            if path == item {
                return Some(i);
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}
