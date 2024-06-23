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
        Filetree {
            node: Node::dir(),
            root: root.as_ref().into(),
        }
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
        let Node::Directory {  children, .. } = self else { unreachable!("Tried to open file as a directory: {:?}", target); };
        for (path, child) in children {
            if let Ok(ntarget) = target.strip_prefix(path) {
                return Some((child, ntarget));
            }
        }

        None
    }

    /// On click of an entry
    fn on_click(&mut self, target: &Path) {
        let mut n = self;
        let mut t = target;

        while let Some((node, ntarget)) = n.child(t) {
            n = node;
            t = ntarget;

            if n.is_leaf() {
                n.on_click_leaf(target);
                return;
            }
        }
    }

    fn on_click_leaf(&mut self, target: &Path) {
        // TODO
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
            Node::Directory { expanded, .. } => *expanded,
        }
    }

    fn expand(&mut self, target: &Path) -> std::io::Result<()> {
        let Node::Directory { expanded, children } = self else { unreachable!("Tried to fill a file with directory entries"); };
        if mem::replace(expanded, true) {
            return Ok(());
        }

        let mut paths = std::fs::read_dir(target)?
            .map(|res| res.map(|e| e.path()))
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
