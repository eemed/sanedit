use super::Prompt;

#[derive(Debug)]
pub struct Search {
    prompt: Prompt,

    /// Wether to search using regex or not
    is_regex: bool,

    /// Wether to select the matches or not
    select: bool,
}

impl Search {}
