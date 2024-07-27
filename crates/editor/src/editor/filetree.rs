use anyhow::Result;
use sanedit_utils::sorted_vec::SortedVec;
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) enum Kind {
    Directory,
    File,
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone)]
pub(crate) struct Node {
    kind: Kind,
    absolute: PathBuf,
    local: PathBuf,
    expanded: bool,
    children: SortedVec<Node>,
}

impl Node {
    pub fn new(absolute: &Path) -> Result<Node> {
        let kind = if absolute.is_dir() {
            Kind::Directory
        } else {
            Kind::File
        };
        let local = PathBuf::from(absolute.file_name().unwrap());

        Ok(Node {
            absolute: absolute.to_path_buf(),
            local,
            kind,
            children: SortedVec::default(),
            expanded: false,
        })
    }

    pub fn collapse(&mut self) {
        self.expanded = false;
    }

    pub fn expand(&mut self) -> Result<()> {
        if self.expanded || !self.children.is_empty() {
            self.expanded = true;
            return Ok(());
        }

        let paths = std::fs::read_dir(&self.absolute)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        for path in paths {
            let local = path.strip_prefix(&self.absolute).unwrap().to_path_buf();
            let kind = if path.is_dir() {
                Kind::Directory
            } else {
                Kind::File
            };
            let node = Node {
                absolute: path,
                local,
                kind,
                expanded: false,
                children: SortedVec::default(),
            };
            self.children.push(node);
        }

        self.expanded = true;

        Ok(())
    }

    pub fn refresh(&mut self) -> Result<()> {
        let mut new_children = SortedVec::new();
        let paths = std::fs::read_dir(&self.absolute)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        for path in paths {
            if let Some(index) = self
                .children
                .iter()
                .position(|child| child.absolute == path)
            {
                let mut child = self.children.remove(index);
                if child.is_dir_expanded() {
                    child.refresh()?;
                }
                new_children.push(child);
            } else {
                let local = path.strip_prefix(&self.absolute).unwrap().to_path_buf();
                let kind = if path.is_dir() {
                    Kind::Directory
                } else {
                    Kind::File
                };
                let node = Node {
                    absolute: path,
                    local,
                    kind,
                    expanded: false,
                    children: SortedVec::default(),
                };
                new_children.push(node);
            }
        }

        self.children = new_children;

        Ok(())
    }

    fn child_mut(&mut self, target: &Path) -> Option<(&mut Node, bool)> {
        for child in self.children.iter_mut() {
            if let Ok(suffix) = target.strip_prefix(&child.absolute) {
                let full_match = suffix.as_os_str().is_empty();
                return Some((child, full_match));
            }
        }

        None
    }

    fn child(&self, target: &Path) -> Option<(&Node, bool)> {
        for child in self.children.iter() {
            if let Ok(suffix) = target.strip_prefix(&child.absolute) {
                let full_match = suffix.as_os_str().is_empty();
                return Some((child, full_match));
            }
        }

        None
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn path(&self) -> &Path {
        &self.absolute
    }

    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Directory
    }

    pub fn is_dir_expanded(&self) -> bool {
        self.kind == Kind::Directory && self.expanded
    }
}

#[derive(Debug)]
pub(crate) struct Filetree {
    root: Node,
}

impl Filetree {
    pub fn new(path: &Path) -> Filetree {
        let root = Node::new(path).expect("could not create filetree");
        Filetree { root }
    }

    pub fn get_mut(&mut self, target: &Path) -> Option<&mut Node> {
        let mut node = &mut self.root;

        if let Ok(suffix) = target.strip_prefix(&node.absolute) {
            if suffix.as_os_str().is_empty() {
                return Some(node);
            }
        }

        while let Some((child, full_match)) = node.child_mut(target) {
            if full_match {
                return Some(child);
            }

            node = child;
        }

        None
    }

    pub fn parent_of(&self, target: &Path) -> Option<&Node> {
        let mut node = &self.root;

        while let Some((child, full_match)) = node.child(target) {
            if full_match {
                return Some(node);
            }

            node = child;
        }

        None
    }

    pub fn iter(&self) -> FiletreeIterator {
        let entry = FiletreeEntry {
            node: &self.root,
            level: 0,
        };
        FiletreeIterator { stack: vec![entry] }
    }
}

#[derive(Debug)]
pub(crate) struct FiletreeEntry<'a> {
    node: &'a Node,
    level: usize,
}

impl<'a> FiletreeEntry<'a> {
    pub fn name(&self) -> &Path {
        &self.node.local
    }

    pub fn name_to_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        self.node.local.to_string_lossy()
    }

    pub fn path(&self) -> &Path {
        &self.node.absolute
    }

    pub fn node(&self) -> &Node {
        &self.node
    }

    pub fn level(&self) -> usize {
        self.level
    }
}

/// Iterator over filetree in displayed order
#[derive(Debug)]
pub(crate) struct FiletreeIterator<'a> {
    stack: Vec<FiletreeEntry<'a>>,
}

impl<'a> Iterator for FiletreeIterator<'a> {
    type Item = FiletreeEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.stack.pop()?;
        let n = entry.node;
        if Kind::Directory == n.kind && n.expanded {
            for child in n.children.iter().rev() {
                let child_entry = FiletreeEntry {
                    node: child,
                    level: entry.level + 1,
                };
                self.stack.push(child_entry);
            }
        }
        Some(entry)
    }
}
