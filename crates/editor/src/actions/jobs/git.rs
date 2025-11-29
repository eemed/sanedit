use sanedit_server::{CPUJob, ClientId, JobContext};
use sanedit_syntax::Glob;

use std::{
    any::Any,
    ffi::OsStr,
    fs::File,
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use crate::editor::{ignore::Globs, job_broker::KeepInTouch, Editor};

/// Calculate git information and set them at the appropriate places
#[derive(Debug, Clone)]
pub(crate) struct GitJob {
    pub(crate) client_id: ClientId,
    pub(crate) working_dir: PathBuf,
}

impl CPUJob for GitJob {
    fn run(&self, mut ctx: JobContext) -> anyhow::Result<()> {
        match Git::new(&self.working_dir) {
            Ok(git) => ctx.send(git),
            Err(e) => log::error!("Failed to fetch git status: {e}"),
        }

        Ok(())
    }
}

impl KeepInTouch for GitJob {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
            log::info!("on message");
        if let Ok(git) = msg.downcast::<Git>() {
            editor.ignore.git_ignore = Arc::new(git.ignores);
        }
    }

    fn on_success(&self, _editor: &mut Editor) {
        log::info!("success");
    }

    fn on_failure(&self, _editor: &mut Editor, reason: &str) {
        log::info!("failed: {reason}");
    }

    fn on_stop(&self, _editor: &mut Editor) {
        log::info!("top");
    }
}

fn git_dir() -> &'static OsStr {
    static S: OnceLock<&'static OsStr> = OnceLock::new();
    S.get_or_init(|| OsStr::new(".git"))
}

fn git_ignore() -> &'static OsStr {
    static S: OnceLock<&'static OsStr> = OnceLock::new();
    S.get_or_init(|| OsStr::new(".gitignore"))
}

#[derive(Debug)]
pub(crate) struct Git {
    root: PathBuf,
    ignores: Vec<Globs>,
}

impl Git {
    pub fn new(working_dir: &Path) -> std::io::Result<Git> {
        log::info!("NEW");
        let git_root = Self::find_root(working_dir)?;
        log::info!("ROOT: {git_root:?}");

        let mut info = Git {
            root: git_root.clone(),
            ignores: vec![Globs {
                root: git_root,
                patterns: vec![Glob::new(".git/").unwrap()],
            }],
        };

        log::info!("FIND IGNORES");
        info.find_ignores()?;
        log::info!("FIND IGNORES OK");
        Ok(info)
    }

    fn is_ignored(ignores: &[Globs], path: &Path) -> bool {
        for ignore in ignores {
            let local = path.strip_prefix(&ignore.root).unwrap_or(path);
            let bytes = local.to_string_lossy();

            for glob in &ignore.patterns {
                let opts = glob.options();
                if opts.directory_only && !path.is_dir() {
                    continue;
                }

                if glob.is_match(&bytes.as_bytes()) {
                    return !opts.negated;
                }
            }
        }
        false
    }

    fn is_ignored_single(ignore: &Globs, path: &Path) -> bool {
        let local = path.strip_prefix(&ignore.root).unwrap_or(path);
        let bytes = local.to_string_lossy();

        for glob in &ignore.patterns {
            let opts = glob.options();
            if opts.directory_only && !path.is_dir() {
                continue;
            }

            if glob.is_match(&bytes.as_bytes()) {
                return !opts.negated;
            }
        }

        false
    }

    // TODO probably could be multithreaded for better performance
    fn find_ignores(&mut self) -> std::io::Result<()> {
        let mut stack = vec![self.root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let mut current_level = vec![];

            if let Ok(mut rd) = std::fs::read_dir(&dir) {
                while let Some(Ok(entry)) = rd.next() {
                    let path = entry.path();
                    if Self::is_ignored(&self.ignores, &path) {
                        continue;
                    }

                    let Ok(metadata) = entry.metadata() else {
                        continue;
                    };

                    if metadata.is_dir() {
                        current_level.push(path);
                    } else if path.file_name() == Some(git_ignore()) {
                        let globs = Self::read_ignore(path)?;
                        current_level.retain_mut(|dir| !Self::is_ignored_single(&globs, dir));
                        self.ignores.push(globs);
                    }
                }

                stack.extend(current_level);
            }
        }

        Ok(())
    }

    fn read_ignore(path: PathBuf) -> std::io::Result<Globs> {
        let mut patterns = vec![];
        let reader = BufReader::new(File::open(&path)?);
        for line in reader.lines() {
            let line = line?;
            if line.starts_with("#") || line.is_empty() {
                continue;
            }

            match Glob::new(&line) {
                Ok(glob) => patterns.push(glob),
                Err(e) => log::error!("Failed to parse pattern: {} at {:?}: {}", line, &path, e),
            }
        }

        patterns.sort_by(|a, b| b.options().cmp(a.options()));

        Ok(Globs {
            root: path,
            patterns,
        })
    }

    fn find_root(dir: &Path) -> std::io::Result<PathBuf> {
        let mut dir = Some(dir);
        while let Some(d) = dir {
            if let Ok(mut rd) = d.read_dir() {
                while let Some(Ok(entry)) = rd.next() {
                    if entry.file_name().as_os_str() == git_dir() {
                        return Ok(d.to_path_buf());
                    }
                }
            }

            dir = d.parent();
        }

        Err(std::io::ErrorKind::NotFound.into())
    }
}
