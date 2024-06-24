use std::{
    collections::BTreeMap,
    ffi::OsStr,
    io, mem,
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FiletreePath {
    Directory(PathBuf),
    File(PathBuf),
}

impl From<FiletreePath> for PathBuf {
    fn from(value: FiletreePath) -> Self {
        match value {
            FiletreePath::Directory(p) => p,
            FiletreePath::File(p) => p,
        }
    }
}

impl AsRef<Path> for FiletreePath {
    fn as_ref(&self) -> &Path {
        match self {
            FiletreePath::Directory(p) => p,
            FiletreePath::File(p) => p,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Filetree {
    node: Node,
    root: PathBuf,
}

impl Filetree {
    pub fn new<P: AsRef<Path>>(root: P) -> Filetree {
        let mut tree = Filetree {
            node: Node::dir(),
            root: root.as_ref().into(),
        };

        // Expand the first level
        tree.on_press(root.as_ref());

        tree
    }

    pub fn on_press(&mut self, target: &Path) {
        if let Ok(path) = target.strip_prefix(&self.root) {
            self.node.on_press(path, target);
        }
    }

    pub fn iter(&self) -> FiletreeIterator {
        let root = self.root.file_name().unwrap_or(OsStr::new(""));
        FiletreeIterator {
            stack: vec![FiletreeEntry {
                path: self.root.clone(),
                name: PathBuf::from(root),
                node: &self.node,
                level: 0,
            }],
        }
    }
}

#[derive(Debug)]
pub(crate) enum Node {
    File,
    Directory {
        expanded: bool,
        children: BTreeMap<FiletreePath, Node>,
    },
}

impl Node {
    pub fn is_dir(&self) -> bool {
        matches!(self, Node::Directory { .. })
    }

    pub fn is_dir_expanded(&self) -> bool {
        let Node::Directory { expanded, .. } = self else { return false; };
        *expanded
    }

    fn dir() -> Node {
        Node::Directory {
            expanded: false,
            children: BTreeMap::default(),
        }
    }

    fn child<'a, 'b>(&'a mut self, target: &'b Path) -> Option<(&'a mut Node, &'b Path)> {
        let Node::Directory { children, .. } = self else {
            unreachable!("Tried to open file as a directory: {:?}", target);
        };
        for (path, child) in children {
            if let Ok(ntarget) = target.strip_prefix(path) {
                return Some((child, ntarget));
            }
        }

        None
    }

    fn on_press(&mut self, stack: &Path, target: &Path) {
        let mut n = self;
        let mut t = stack;

        if t.as_os_str().is_empty() {
            n.on_press_item(target);
            return;
        }

        while let Some((node, ntarget)) = n.child(t) {
            n = node;
            t = ntarget;

            if t.as_os_str().is_empty() {
                n.on_press_item(target);
                return;
            }
        }
    }

    fn on_press_item(&mut self, target: &Path) {
        match self {
            Node::File => todo!("open file"),
            Node::Directory { expanded, .. } => {
                if *expanded {
                    self.shrink();
                } else {
                    if let Err(e) = self.expand(target) {
                        log::error!("Failed to expand {target:?}: {e}");
                    }
                }
            }
        }
    }

    fn shrink(&mut self) {
        let Node::Directory { expanded, .. } = self else {
            unreachable!("Tried to fill a file with directory entries");
        };
        log::info!("shrink");
        *expanded = false;
    }

    fn expand(&mut self, target: &Path) -> anyhow::Result<()> {
        let Node::Directory { expanded, children } = self else {
            unreachable!("Tried to fill a file with directory entries");
        };
        if mem::replace(expanded, true) {
            return Ok(());
        }

        if !children.is_empty() {
            return Ok(());
        }

        let paths = std::fs::read_dir(target)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        for path in paths {
            let local = path.strip_prefix(target)?.to_path_buf();
            if path.is_dir() {
                children.insert(FiletreePath::Directory(local), Node::dir());
            } else {
                children.insert(FiletreePath::File(local), Node::File);
            }
        }

        // TODO if single dir inside this recursively open until atleast one
        // file is found or an empty directory is found

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct FiletreeEntry<'a> {
    pub(crate) path: PathBuf,
    pub(crate) name: PathBuf,
    pub(crate) node: &'a Node,
    pub(crate) level: usize,
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
        if let Node::Directory { children, expanded } = entry.node {
            if *expanded {
                for (p, n) in children.iter().rev() {
                    let mut npath = entry.path.clone();
                    npath.push(p);

                    self.stack.push(FiletreeEntry {
                        path: npath,
                        name: p.as_ref().to_path_buf(),
                        node: n,
                        level: entry.level + 1,
                    });
                }
            }
        }

        Some(entry)
    }
}
