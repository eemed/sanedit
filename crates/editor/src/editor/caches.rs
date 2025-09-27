use std::{collections::LinkedList, path::PathBuf};

use rustc_hash::FxHashSet;

use super::config::Config;

#[derive(Debug)]
pub(crate) struct Caches {
    pub(crate) files: FilesMRU,
}

impl Caches {
    pub fn new(config: &Config) -> Self {
        Caches {
            files: FilesMRU::new(config.window.max_prompt_completions),
        }
    }
}

/// Most recently used files
#[derive(Debug)]
pub(crate) struct FilesMRU {
    paths: FxHashSet<PathBuf>,
    order: LinkedList<PathBuf>,
    cap: usize,
}

impl FilesMRU {
    pub fn new(size: usize) -> FilesMRU {
        FilesMRU {
            paths: Default::default(),
            order: Default::default(),
            cap: size,
        }
    }

    pub fn insert(&mut self, path: PathBuf) {
        if self.paths.contains(&path) {
            // TODO for what ever reason linked list does not support middle removal
            // So do this the shitty way. Should fix when remove is available
            // Also self.paths could be a hashmap Pathbuf -> pointer to linked list node, for faster deletion
            // But that wont probably be supported either if removal is already this hard..
            // Would need to implement own doubly linked list which is hard because rust.
            let mut found = 0;
            for (i, p) in self.order.iter().enumerate() {
                if p == &path {
                    found = i;
                    break;
                }
            }
            let mut rest = self.order.split_off(found);
            rest.pop_front();
            rest.push_back(path);
            self.order.append(&mut rest);
        } else {
            if self.paths.len() >= self.cap {
                if let Some(elem) = self.order.pop_front() {
                    self.paths.remove(&elem);
                }
            }

            self.paths.insert(path.clone());
            self.order.push_back(path);
        }
    }

    #[allow(dead_code)]
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.paths.contains(path)
    }

    /// Highest position if file was recently used
    pub fn position(&self, path: &PathBuf) -> Option<usize> {
        if !self.paths.contains(path) {
            return None;
        }

        let mut found = 0;
        for (i, p) in self.order.iter().enumerate() {
            if p == path {
                found = i;
                break;
            }
        }

        Some(found)
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    #[allow(dead_code)]
    pub fn cap(&self) -> usize {
        self.cap
    }
}
