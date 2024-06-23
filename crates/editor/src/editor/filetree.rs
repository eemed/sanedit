use std::{
    io, mem,
    path::{Path, PathBuf},
};

use rustc_hash::FxHashMap;

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
        tree.node.on_press(root.as_ref());

        tree
    }
}

#[derive(Debug)]
enum Node {
    File,
    Directory {
        expanded: bool,
        children: FxHashMap<PathBuf, Node>,
    },
}

impl Node {
    fn dir() -> Node {
        Node::Directory {
            expanded: false,
            children: FxHashMap::default(),
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

    /// On click of an entry
    fn on_press(&mut self, target: &Path) {
        let mut n = self;
        let mut t = target;

        if n.is_leaf() {
            n.on_press_leaf(target);
            return;
        }

        while let Some((node, ntarget)) = n.child(t) {
            n = node;
            t = ntarget;

            if n.is_leaf() {
                n.on_press_leaf(target);
                return;
            }
        }
    }

    fn on_press_leaf(&mut self, target: &Path) {
        // TODO
        match self {
            Node::File => todo!("open file"),
            Node::Directory { expanded, children } => {
                if *expanded {
                    self.shrink();
                } else {
                    if let Err(e) = self.expand(target) {
                        log::error!("Failed to expand {target:?}: {e}");
                    }
                }
            }
        }
        // if file => open
        // if dir unexpanded => expand
        //        expanded => shrink

        // if let Err(e) = n.fill_dir_with(target) {
        //     log::error!("Filetree error on expand: {e}");
        // }
    }

    /// Whether this node is a leaf, => a file or unexpanded directory
    fn is_leaf(&self) -> bool {
        match self {
            Node::File => true,
            Node::Directory { expanded, .. } => !expanded,
        }
    }

    fn shrink(&mut self) {
        let Node::Directory { expanded, children } = self else {
            unreachable!("Tried to fill a file with directory entries");
        };
        *expanded = false;
        children.clear();
    }

    fn expand(&mut self, target: &Path) -> std::io::Result<()> {
        let Node::Directory { expanded, children } = self else {
            unreachable!("Tried to fill a file with directory entries");
        };
        if mem::replace(expanded, true) {
            return Ok(());
        }

        let mut paths = std::fs::read_dir(target)?
            .map(|res| res.map(|e| e.path()))
            // .map(|res| res.map(|e| e.strip_prefix(target)))
            .collect::<Result<Vec<_>, io::Error>>()?;

        paths.sort();

        for path in paths {
            if path.is_dir() {
                children.insert(path, Node::dir());
            } else {
                children.insert(path, Node::File);
            }
        }

        // TODO if single dir inside this recursively open until atleast one
        // file is found or an empty directory is found

        Ok(())
    }
}
