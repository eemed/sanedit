use std::{cell::RefCell, ops::Range, rc::Rc};

use sanedit_messages::redraw::PromptOption;
use sanedit_utils::sorted_vec::SortedVec;

#[derive(Debug, Default)]
struct MergedOptions {
    /// Currently processed options, cursor for each sorted vec
    cursors: Vec<usize>,
    /// Indices to Options->options
    merged: Vec<(usize, usize)>,
}

impl MergedOptions {
    pub fn clear(&mut self, n: usize) {
        self.cursors.clear();
        self.merged.clear();

        for _ in 0..n {
            self.cursors.push(0);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Options {
    options: Vec<SortedVec<SelectorOption>>,
    total: usize,
    interior: RefCell<MergedOptions>,
}

impl Options {
    pub fn new() -> Options {
        Options::default()
    }

    pub fn push(&mut self, opts: SortedVec<SelectorOption>) {
        self.total += opts.len();
        self.options.push(opts);

        let len = self.options.len();
        let mut interior = self.interior.borrow_mut();
        interior.clear(len);
    }

    pub fn len(&self) -> usize {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    pub fn get(&self, idx: usize) -> Option<&SelectorOption> {
        self.merge_until(idx);
        let interior = self.interior.borrow();
        let (list, pos) = interior.merged.get(idx)?;
        let item = &self.options[*list][*pos];
        Some(item)
    }

    /// Create one sorted list by merging the locally sorted lists
    ///
    /// To avoid always merging when options are requested.
    /// The work done is saved using interior mutability
    fn merge_until(&self, idx: usize) {
        let mut interior = self.interior.borrow_mut();
        for _ in interior.merged.len()..idx + 1 {
            let len = interior.cursors.len();
            // min element match and the list/cursor index
            let mut min: Option<(usize, &SelectorOption)> = None;

            for c in 0..len {
                let list = &self.options[c];
                let cursor = interior.cursors[c];
                if cursor >= list.len() {
                    continue;
                }
                let mat = &list[cursor];
                let replace = min
                    .as_ref()
                    .map(|(_, small_match)| mat < small_match)
                    .unwrap_or(true);

                if replace {
                    min = Some((c, mat));
                }
            }

            if let Some((idx, _)) = min {
                let pos = (idx, interior.cursors[idx]);
                interior.merged.push(pos);
                interior.cursors[idx] += 1;
            }
        }
    }
}

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
