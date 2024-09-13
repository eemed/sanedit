use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub enum Kind {
    Path,
    String,
}

/// An option that can be matched using matcher.
/// Contains bytes and a description of those bytes
#[derive(Debug, Clone)]
pub struct MatchOption {
    /// Match option data
    value: Vec<u8>,

    // TODO value and what is required to match could be just different fields?
    /// Offset into value that should be used in matching
    /// this allows to ignore any prefix during matching
    /// Useful for matching paths, without a certain prefix.
    offset: usize,

    /// How to represent the match option data
    kind: Kind,
    description: String,
}

impl MatchOption {
    pub fn new(option: &[u8], description: &str, offset: usize, kind: Kind) -> MatchOption {
        MatchOption {
            value: option.into(),
            offset,
            kind,
            description: description.into(),
        }
    }

    pub fn with_description(option: &str, description: &str) -> MatchOption {
        MatchOption {
            value: option.into(),
            offset: 0,
            kind: Kind::String,
            description: description.into(),
        }
    }

    /// Return the bytes required to match this option interpreted as utf8
    pub fn to_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        match self.kind {
            Kind::Path => {
                let os = unsafe { OsStr::from_encoded_bytes_unchecked(self.bytes_to_match()) };
                os.to_string_lossy()
            }
            Kind::String => unsafe { std::str::from_utf8_unchecked(self.bytes_to_match()) }.into(),
        }
    }

    /// Return path if this match option represents a path
    pub fn path(&self) -> Option<PathBuf> {
        match self.kind {
            Kind::Path => {
                let os = unsafe { OsStr::from_encoded_bytes_unchecked(&self.value) };
                PathBuf::from(os).into()
            }
            Kind::String => None,
        }
    }

    /// Returns the bytes used to match this option
    fn bytes_to_match(&self) -> &[u8] {
        &self.value[self.offset..]
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn bytes(&self) -> &[u8] {
        &self.value
    }
}

impl std::hash::Hash for MatchOption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl PartialEq for MatchOption {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl Eq for MatchOption {}

impl From<&Path> for MatchOption {
    fn from(value: &Path) -> Self {
        MatchOption {
            value: value.as_os_str().as_encoded_bytes().into(),
            offset: 0,
            kind: Kind::Path,
            description: String::new(),
        }
    }
}

impl From<PathBuf> for MatchOption {
    fn from(value: PathBuf) -> Self {
        MatchOption {
            value: value.as_os_str().as_encoded_bytes().into(),
            offset: 0,
            kind: Kind::Path,
            description: String::new(),
        }
    }
}

impl From<String> for MatchOption {
    fn from(value: String) -> Self {
        MatchOption {
            value: value.into(),
            offset: 0,
            kind: Kind::String,
            description: String::new(),
        }
    }
}

impl From<&str> for MatchOption {
    fn from(value: &str) -> Self {
        MatchOption {
            value: value.into(),
            offset: 0,
            kind: Kind::String,
            description: String::new(),
        }
    }
}

// /// A matched and scored option
// #[derive(Debug, Clone)]
// pub(crate) struct Match {
//     /// Matched value
//     pub(crate) opt: MatchOption,
//     /// Score of the match
//     pub(crate) score: u32,

//     /// Ranges of value string that were matched
//     pub(crate) ranges: Vec<Range<usize>>,
// }

// impl Match {
//     pub fn score(&self) -> u32 {
//         self.score
//     }

//     pub fn ranges(&self) -> &[Range<usize>] {
//         &self.ranges
//     }
// }

// impl PartialEq for Match {
//     fn eq(&self, other: &Self) -> bool {
//         (self.score, &self.opt.value) == (other.score, &other.opt.value)
//     }
// }

// impl Eq for Match {}

// impl PartialOrd for Match {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         (self.score, &self.opt.value).partial_cmp(&(other.score, &other.opt.value))
//     }
// }

// impl Ord for Match {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         (self.score, &self.opt.value).cmp(&(other.score, &other.opt.value))
//     }
// }

// impl From<Match> for SelectorOption {
//     fn from(mat: Match) -> Self {
//         SelectorOption::new(&mat.opt.value, mat.ranges, mat.score, &mat.opt.description)
//     }
// }

// impl From<&Match> for SelectorOption {
//     fn from(mat: &Match) -> Self {
//         SelectorOption::new(
//             &mat.opt.value,
//             mat.ranges.clone(),
//             mat.score,
//             &mat.opt.description,
//         )
//     }
// }
