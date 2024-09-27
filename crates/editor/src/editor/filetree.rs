use anyhow::Result;
use sanedit_utils::sorted_vec::SortedVec;
use std::{
    io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

trait PathExtended {
    fn is_empty(&self) -> bool;
    fn kind(&self) -> Kind;
}

impl<A: AsRef<Path> + ?Sized> PathExtended for &A {
    fn is_empty(&self) -> bool {
        self.as_ref().as_os_str().is_empty()
    }

    fn kind(&self) -> Kind {
        if self.as_ref().is_dir() {
            Kind::Directory
        } else {
            Kind::File
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) enum Kind {
    Directory,
    File,
}

/// immutable node with its absolute path
#[derive(Debug)]
pub(crate) struct TreeNode<'a> {
    _internal: &'a Node,
    absolute: PathBuf,
}

impl<'a> TreeNode<'a> {
    pub fn path(&self) -> &Path {
        &self.absolute
    }
}

/// mutable node with its absolute path
#[derive(Debug)]
pub(crate) struct TreeNodeMut<'a> {
    internal: &'a mut Node,
    absolute: PathBuf,
}

impl<'a> Deref for TreeNodeMut<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.internal
    }
}

impl<'a> DerefMut for TreeNodeMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.internal
    }
}

impl<'a> TreeNodeMut<'a> {
    pub fn expand(&mut self) -> Result<()> {
        self.internal.expand(&self.absolute)
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.internal.refresh(&self.absolute)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Node {
    /// File or directory
    kind: Kind,
    /// Local name for this node, may contain multiple path components
    local: PathBuf,
    /// Whether this entry is expanded, if directory
    expanded: bool,
    /// Entries children, if directory
    children: SortedVec<Node>,
}

impl Node {
    fn new(local: &Path, kind: Kind) -> Node {
        Node {
            local: local.into(),
            kind,
            children: SortedVec::default(),
            expanded: false,
        }
    }

    pub fn collapse(&mut self) {
        self.expanded = false;
    }

    fn expand(&mut self, absolute: &Path) -> Result<()> {
        if self.expanded || !self.children.is_empty() {
            self.expanded = true;
            return Ok(());
        }

        let paths = std::fs::read_dir(absolute)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        for path in paths {
            let local = path.strip_prefix(absolute).unwrap().to_path_buf();
            let kind = path.as_path().kind();
            let mut node = Node {
                local,
                kind,
                expanded: false,
                children: SortedVec::default(),
            };
            if node.is_dir() {
                node.add_single_directories_to_local(&path)?;
            }
            self.children.push(node);
        }

        self.expanded = true;

        Ok(())
    }

    fn add_single_directories_to_local(&mut self, absolute: &Path) -> Result<()> {
        let mut absolute = absolute.to_path_buf();
        let mut paths = std::fs::read_dir(absolute)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        while paths.len() == 1 {
            let path = paths.pop().unwrap();
            if !path.is_dir() {
                break;
            }
            let name = PathBuf::from(path.file_name().unwrap());
            self.local.push(name);
            absolute = path;

            paths = std::fs::read_dir(absolute)?
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, io::Error>>()?;
        }

        Ok(())
    }

    fn refresh(&mut self, absolute: &Path) -> Result<()> {
        let mut new_children = SortedVec::new();
        let paths = std::fs::read_dir(absolute)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        for path in paths {
            let local = path.strip_prefix(absolute).unwrap().to_path_buf();
            if let Some(index) = self.children.iter().position(|child| child.local == local) {
                let mut child = self.children.remove(index);
                if child.is_dir_expanded() {
                    child.refresh(&path)?;
                }
                new_children.push(child);
            } else {
                let kind = path.as_path().kind();
                let mut node = Node {
                    local,
                    kind,
                    expanded: false,
                    children: SortedVec::default(),
                };
                if node.is_dir() {
                    node.add_single_directories_to_local(&path)?;
                }
                new_children.push(node);
            }
        }

        self.children = new_children;

        Ok(())
    }

    fn child_mut<'a, 'b>(&'a mut self, target: &'b Path) -> Option<(&'a mut Node, &'b Path)> {
        // SAFETY: ensured by node api
        for child in unsafe { self.children.iter_mut() } {
            if let Ok(suffix) = target.strip_prefix(&child.local) {
                return Some((child, suffix));
            }
        }

        None
    }

    fn child<'a, 'b>(&'a self, target: &'b Path) -> Option<(&'a Node, &'b Path)> {
        for child in self.children.iter() {
            if let Ok(suffix) = target.strip_prefix(&child.local) {
                return Some((child, suffix));
            }
        }

        None
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Directory
    }

    pub fn is_dir_expanded(&self) -> bool {
        self.kind == Kind::Directory && self.expanded
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.kind, &self.local).cmp(&(other.kind, &other.local))
    }
}

#[derive(Debug)]
pub(crate) struct Filetree {
    absolute: PathBuf,
    root: Node,
}

impl Filetree {
    pub fn new(path: &Path) -> Filetree {
        let kind = path.kind();
        let mut absolute = path.to_path_buf();
        let name = absolute.file_name().expect("Could not create filetree");
        let local = PathBuf::from(name);
        absolute.pop();

        let root = Node::new(&local, kind);
        Filetree { absolute, root }
    }

    pub fn get_mut(&mut self, mut target: &Path) -> Option<TreeNodeMut> {
        let absolute = target.to_path_buf();
        target = target.strip_prefix(&self.absolute).unwrap_or(target);
        let mut node = &mut self.root;

        if let Ok(suffix) = target.strip_prefix(&node.local) {
            if suffix.is_empty() {
                return Some(TreeNodeMut {
                    internal: node,
                    absolute,
                });
            }
            target = suffix;
        }

        while let Some((child, suffix)) = node.child_mut(target) {
            if suffix.is_empty() {
                return Some(TreeNodeMut {
                    internal: child,
                    absolute,
                });
            }

            node = child;
            target = suffix;
        }

        None
    }

    pub fn parent_of(&self, mut target: &Path) -> Option<TreeNode> {
        let mut absolute = target.to_path_buf();
        target = target.strip_prefix(&self.absolute).unwrap_or(target);
        let mut node = &self.root;

        if let Ok(suffix) = target.strip_prefix(&node.local) {
            if suffix.is_empty() {
                return Some(TreeNode {
                    _internal: node,
                    absolute,
                });
            }
            target = suffix;
        }

        while let Some((child, suffix)) = node.child(target) {
            if suffix.is_empty() {
                for _ in 0..target.components().count() {
                    absolute.pop();
                }
                return Some(TreeNode {
                    _internal: node,
                    absolute,
                });
            }

            node = child;
            target = suffix;
        }

        None
    }

    pub fn iter(&self) -> FiletreeIterator {
        let absolute = self.absolute.join(&self.root.local);
        let entry = FiletreeEntry {
            node: &self.root,
            absolute,
            level: 0,
        };
        FiletreeIterator { stack: vec![entry] }
    }
}

#[derive(Debug)]
pub(crate) struct FiletreeEntry<'a> {
    node: &'a Node,
    absolute: PathBuf,
    level: usize,
}

impl<'a> Deref for FiletreeEntry<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

impl<'a> FiletreeEntry<'a> {
    pub fn name_to_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        self.node.local.to_string_lossy()
    }

    pub fn path(&self) -> &Path {
        &self.absolute
    }

    pub fn node(&self) -> &Node {
        self.node
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
                let absolute = entry.absolute.join(&child.local);
                let child_entry = FiletreeEntry {
                    node: child,
                    absolute,
                    level: entry.level + 1,
                };
                self.stack.push(child_entry);
            }
        }
        Some(entry)
    }
}
