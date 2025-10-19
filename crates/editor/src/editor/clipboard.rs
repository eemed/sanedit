use std::fmt;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use anyhow::anyhow;
use anyhow::ensure;
use anyhow::Result;

/// Return the clipboard provided if it is supported
macro_rules! try_clipboard {
    ( $e:ident ) => {
        match $e::new() {
            Ok(x) => return Box::new(x),
            Err(_) => {}
        }
    };
}

pub(crate) trait Clipboard: fmt::Debug {
    fn copy(&mut self, text: &str);
    fn paste(&mut self) -> Result<String>;
}

pub(crate) struct DefaultClipboard;
impl DefaultClipboard {
    #[cfg(unix)]
    pub fn new_default() -> Box<dyn Clipboard> {
        let session = std::env::var("XDG_SESSION_TYPE").ok();

        match session.as_deref() {
            Some("wayland") => {
                try_clipboard!(WaylandClipboard);
            }
            Some("x11") => {
                try_clipboard!(XClip);
                try_clipboard!(XSel);
            }
            _ => {
                try_clipboard!(WaylandClipboard);
                try_clipboard!(XClip);
                try_clipboard!(XSel);
            }
        }

        // Fallback
        Box::new(Internal::new())
    }
}

#[derive(Debug)]
pub(crate) struct Internal {
    content: Option<String>,
}

impl Internal {
    pub fn new() -> Internal {
        Internal { content: None }
    }
}

impl Clipboard for Internal {
    fn copy(&mut self, text: &str) {
        self.content = Some(text.into());
    }

    fn paste(&mut self) -> Result<String> {
        self.content.clone().ok_or(anyhow!("No content"))
    }
}

#[derive(Debug)]
pub(crate) struct XClip;

impl XClip {
    pub fn new() -> Result<XClip> {
        ensure!(is_executable("xclip"), "xclip not executable");
        Ok(XClip)
    }
}

impl Clipboard for XClip {
    fn copy(&mut self, text: &str) {
        let mut child = Command::new("xclip")
            .args(["-in", "-selection", "clipboard"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xclip process");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let _ = stdin.write_all(text.as_bytes());
        drop(stdin);
        child.wait().expect("Failed to execute xclip");
    }

    fn paste(&mut self) -> Result<String> {
        let output = Command::new("xclip")
            .args(["-out", "-selection", "clipboard"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute xclip");
        let pasted = String::from_utf8(output.stdout)?;
        Ok(pasted)
    }
}

#[derive(Debug)]
pub(crate) struct XSel;

impl XSel {
    pub fn new() -> Result<XSel> {
        ensure!(is_executable("xsel"), "xclip not executable");
        Ok(XSel)
    }
}

impl Clipboard for XSel {
    fn copy(&mut self, text: &str) {
        let mut child = Command::new("xsel")
            .args(["--input", "--clipboard"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xclip process");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let _ = stdin.write_all(text.as_bytes());
        drop(stdin);
        child.wait().expect("Failed to execute xclip");
    }

    fn paste(&mut self) -> Result<String> {
        let output = Command::new("xsel")
            .args(["--output", "--clipboard"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute xclip");
        let pasted = String::from_utf8(output.stdout)?;
        Ok(pasted)
    }
}

#[derive(Debug)]
pub(crate) struct WaylandClipboard;

impl WaylandClipboard {
    pub fn new() -> Result<WaylandClipboard> {
        ensure!(is_executable("wl-copy"), "wl-copy not executable");
        ensure!(is_executable("wl-paste"), "wl-paste not executable");
        Ok(WaylandClipboard)
    }
}

impl Clipboard for WaylandClipboard {
    fn copy(&mut self, text: &str) {
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xclip process");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let _ = stdin.write_all(text.as_bytes());
        drop(stdin);
        child.wait().expect("Failed to execute xclip");
    }

    fn paste(&mut self) -> Result<String> {
        let output = Command::new("wl-paste")
            .args(["-n"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute xclip");
        let pasted = String::from_utf8(output.stdout)?;
        Ok(pasted)
    }
}

fn is_executable(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}
