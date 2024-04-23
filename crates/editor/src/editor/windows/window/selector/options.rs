use std::ops::Range;

use sanedit_messages::redraw::PromptOption;
use sanedit_utils::sorted_vec::SortedVec;

#[derive(Debug, Default)]
pub(crate) struct Options {
    chunks: Vec<SortedVec<SelectorOption>>,
    total: usize,
}

impl Options {
    pub fn new() -> Options {
        Options::default()
    }

    pub fn push(&mut self, opts: SortedVec<SelectorOption>) {
        self.total += opts.len();
        self.chunks.push(opts);
    }

    pub fn get(&self, idx: usize) -> Option<&SelectorOption> {
        self.iter().skip(idx).next()
    }

    pub fn len(&self) -> usize {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    pub fn iter(&self) -> OptionIter {
        OptionIter::new(&self.chunks)
    }
}

#[derive(Debug)]
pub(crate) struct OptionIter<'a> {
    cursors: Vec<usize>,
    chunks: &'a [SortedVec<SelectorOption>],
}

impl<'a> OptionIter<'a> {
    pub fn new(chunks: &[SortedVec<SelectorOption>]) -> OptionIter {
        let cursors = vec![0; chunks.len()];
        OptionIter { cursors, chunks }
    }
}

impl<'a> Iterator for OptionIter<'a> {
    type Item = &'a SelectorOption;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.cursors.len();
        // min element match and the list/cursor index
        let mut min: Option<(usize, &SelectorOption)> = None;

        for c in 0..len {
            let list = &self.chunks[c];
            let cursor = self.cursors[c];
            let mat = &list[cursor];

            let replace = min
                .as_ref()
                .map(|(_, small_match)| mat < small_match)
                .unwrap_or(true);

            if replace {
                min = Some((c, mat));
            }
        }

        let (idx, mat) = min?;
        self.cursors[idx] += 1;

        Some(mat)
    }
}

// pub(crate) type Options = SortedVec<SelectorOption>;

#[derive(Debug, Clone, Default)]
pub(crate) struct SelectorOption {
    pub(super) score: u32,
    /// Underlying option
    pub(super) value: String,

    /// Matched ranges
    pub(super) matches: Vec<Range<usize>>,

    /// Additional description of the option
    pub(crate) description: String,
}

impl SelectorOption {
    pub fn new(opt: String, matches: Vec<Range<usize>>, score: u32) -> SelectorOption {
        SelectorOption {
            value: opt.clone(),
            score,
            description: String::new(),
            matches,
        }
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }
}

impl PartialEq for SelectorOption {
    fn eq(&self, other: &Self) -> bool {
        (self.score, &self.value) == (other.score, &other.value)
    }
}

impl Eq for SelectorOption {}

impl PartialOrd for SelectorOption {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.score, &self.value).partial_cmp(&(other.score, &other.value))
    }
}

impl Ord for SelectorOption {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.score, &self.value).cmp(&(other.score, &other.value))
    }
}

impl From<SelectorOption> for PromptOption {
    fn from(opt: SelectorOption) -> Self {
        PromptOption {
            name: opt.value,
            description: opt.description,
            matches: opt.matches,
        }
    }
}
