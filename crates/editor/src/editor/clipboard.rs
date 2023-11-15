use anyhow::ensure;
use anyhow::Result;

pub(crate) trait Clipboard {
    fn copy(&mut self, text: &str);
    fn paste(&mut self) -> Option<String>;
}

pub(crate) struct DefaultClipboard;
impl DefaultClipboard {
    pub fn new() -> Box<dyn Clipboard> {
        todo!()
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

fn is_executable(cmd: &str) -> bool {
    todo!()
}

fn execute(cmd: &str) -> Result<String> {
    todo!()
}
