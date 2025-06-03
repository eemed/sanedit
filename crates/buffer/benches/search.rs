use std::io;

use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_buffer::{PieceTree, Searcher, SearcherRev};

fn bmh(c: &mut Criterion) {
    c.bench_function("forward", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();
        let slice = pt.slice(..);

        let searcher = Searcher::new(b"ipsum");
        let iter = searcher.find_iter(&slice);

        bench.iter(move || {
            // Search whole file
            let mut i = iter.clone();
            while i.next().is_some() {}
        });
    });

    c.bench_function("backward", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();
        let slice = pt.slice(..);

        let searcher = SearcherRev::new(b"ipsum");
        let iter = searcher.find_iter(&slice);
        bench.iter(move || {
            // Search whole file
            let mut i = iter.clone();
            while i.next().is_some() {}
        });
    });
}

criterion_group!(benches, bmh);
criterion_main!(benches);
