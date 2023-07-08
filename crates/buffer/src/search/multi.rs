use std::ops::Range;

use aho_corasick::{
    automaton::{Automaton, StateID},
    nfa::contiguous::NFA,
    Anchored,
};

use crate::{Bytes, PieceTreeSlice};

pub struct SetSearcher {
    nfa: NFA,
}

impl SetSearcher {
    pub fn new<I, P>(patterns: I) -> SetSearcher
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]>,
    {
        let nfa = NFA::new(patterns).unwrap();
        SetSearcher { nfa }
    }

    pub fn find_iter<'a, 'b: 'a>(&'a mut self, slice: &'b PieceTreeSlice) -> SetSearchIter {
        SetSearchIter::new(self, slice)
    }
}

pub struct SetSearchIter<'a, 'b> {
    nfa: &'a mut NFA,
    state: StateID,
    bytes: Bytes<'b>,
}

impl<'a, 'b> SetSearchIter<'a, 'b> {
    pub fn new(searcher: &'a mut SetSearcher, slice: &'b PieceTreeSlice) -> SetSearchIter<'a, 'b> {
        let nfa = &mut searcher.nfa;
        let state = nfa.start_state(Anchored::No).unwrap();

        SetSearchIter {
            state,
            nfa,
            bytes: slice.bytes(),
        }
    }
}

impl<'a, 'b> Iterator for SetSearchIter<'a, 'b> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let byte = self.bytes.next()?;
            self.state = self.nfa.next_state(Anchored::No, self.state, byte);

            if self.nfa.is_match(self.state) {
                let max = self.nfa.max_pattern_len();
                let mcount = self.nfa.match_len(self.state);
                for i in 0..mcount {
                    let pat = self.nfa.match_pattern(self.state, i);
                    let plen = self.nfa.pattern_len(pat);
                    let pos = self.bytes.pos();
                    println!("{i}: pos: {pos}, matches: {mcount}, max: {max}, pat: {pat:?}, plen: {plen}",);
                }
                println!("--");
            }
        }

        // loop {
        //     let byte = self.bytes.next()?;
        //     self.state = self.nfa.next_state(Anchored::No, self.state, byte);
        //     if self.nfa.is_dead(self.state) {
        //         let pos = self.bytes.pos();
        //         return Some(pos..pos);
        //     }
        // }
    }
}

#[cfg(test)]
mod test {
    use crate::PieceTree;

    use super::*;

    #[test]
    fn find_eol() {
        let bytes = b"a\nb\r\n";
        let mut pt = PieceTree::new();
        pt.insert(0, bytes);

        let slice = pt.slice(..);
        let mut searcher = SetSearcher::new(["\r\n", "\r", "\n"]);
        let mut iter = searcher.find_iter(&slice);

        while let Some(mat) = iter.next() {
            println!("Match: {mat:?}");
        }
    }

    #[test]
    fn nfa_debug() {
        let bytes = b"hello samwise doo";
        let mut pt = PieceTree::new();
        pt.insert(0, bytes);

        let slice = pt.slice(..);
        let mut searcher = SetSearcher::new(["sam", "samwise"]);
        let mut iter = searcher.find_iter(&slice);

        while let Some(mat) = iter.next() {
            println!("Match: {mat:?}");
        }
    }
}
