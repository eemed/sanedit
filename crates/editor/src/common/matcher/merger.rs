use std::sync::{atomic::AtomicUsize, Arc};

use sanedit_utils::sorted_vec::SortedVec;

use super::Match;

// Hold all sorted results
// "merge sort" using a cursor once a index is requested

#[derive(Clone)]
struct Merger {
    options: Vec<SortedVec<Match>>,
    len: Arc<AtomicUsize>,
}

impl Merger {}
