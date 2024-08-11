use std::{path::Path, str::FromStr};

pub fn path_to_uri(path: &Path) -> lsp_types::Uri {
    let uri = format!("file://{}", path.to_string_lossy());
    lsp_types::Uri::from_str(&uri).unwrap()
}
