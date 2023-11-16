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

pub(crate) trait Clipboard {
    fn copy(&mut self, text: &str);
    fn paste(&mut self) -> Option<String>;
}

pub(crate) struct DefaultClipboard;
impl DefaultClipboard {
    #[cfg(unix)]
    pub fn new() -> Box<dyn Clipboard> {
        try_clipboard!(XClip);
        try_clipboard!(XSel);

        // Fallback
        Box::new(Internal::new())
    }
}

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

    fn paste(&mut self) -> Option<String> {
        self.content.clone()
    }
}

pub(crate) struct XClip;

impl XClip {
    pub fn new() -> Result<XClip> {
        ensure!(is_executable("xclip"), "xclip not executable");
        Ok(XClip)
    }
}

impl Clipboard for XClip {
    fn copy(&mut self, text: &str) {
        // xclip -in -selection clipboard
        // clip from stdin
        todo!()
    }

    fn paste(&mut self) -> Option<String> {
        // xclip -out -selection clipboard
        todo!()
    }
}

pub(crate) struct XSel;
impl XSel {
    pub fn new() -> Result<XSel> {
        ensure!(is_executable("xsel"), "xsel not executable");
        Ok(XSel)
    }
}

impl Clipboard for XSel {
    fn copy(&mut self, text: &str) {
        todo!()
    }

    fn paste(&mut self) -> Option<String> {
        todo!()
    }
}

fn is_executable(cmd: &str) -> bool {
    todo!()
}

fn execute(cmd: &str) -> Result<String> {
    todo!()
}
