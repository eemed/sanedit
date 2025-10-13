use std::io;

use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_buffer::{
    utf8::{next_grapheme_boundary, prev_grapheme_boundary},
    PieceTree,
};

fn bytes(c: &mut Criterion) {
    c.bench_function("bytes_next", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();

        let iter = pt.bytes();
        let mut i = iter.clone();
        // Measures a single step, and clones = restarts search after finising
        bench.iter(move || {
            if i.next().is_none() {
                i = iter.clone();
            }
        });
    });

    c.bench_function("bytes_prev", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();

        let iter = pt.bytes_at(pt.len());
        let mut i = iter.clone();
        bench.iter(move || {
            if i.prev().is_none() {
                i = iter.clone();
            }
        });
    });
}

fn chars(c: &mut Criterion) {
    c.bench_function("chars_next", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();

        let iter = pt.chars();
        let mut i = iter.clone();
        bench.iter(move || {
            if i.next().is_none() {
                i = iter.clone();
            }
        });
    });

    c.bench_function("chars_prev", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();

        let iter = pt.chars_at(pt.len());
        let mut i = iter.clone();
        bench.iter(move || {
            if i.prev().is_none() {
                i = iter.clone();
            }
        });
    });

}

fn graphemes(c: &mut Criterion) {
    c.bench_function("grapheme_iterator_next", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();
        let slice = pt.slice(..);
        let iter = slice.graphemes_at(0);
        let mut graphemes = iter.clone();

        bench.iter(move || {
            if graphemes.next().is_none() {
                graphemes = iter.clone();
            }
        });
    });

    c.bench_function("grapheme_iterator_prev", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();
        let slice = pt.slice(..);
        let iter = slice.graphemes_at(slice.len());
        let mut graphemes = iter.clone();

        bench.iter(move || {
            if graphemes.prev().is_none() {
                graphemes = iter.clone();
            }
        });
    });

    c.bench_function("grapheme_boundary_next", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();
        let slice = pt.slice(..);
        let mut pos = 0;

        bench.iter(move || {
            let end = next_grapheme_boundary(&slice, pos);
            pos = end;

            if pos == slice.len() {
                pos = 0
            }
        });
    });

    c.bench_function("grapheme_boundary_prev", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();
        let slice = pt.slice(..);
        let mut pos = pt.len();

        bench.iter(move || {
            let end = prev_grapheme_boundary(&slice, pos);
            pos = end;

            if pos == 0 {
                pos = slice.len();
            }
        });
    });
}

fn chunks(c: &mut Criterion) {
    c.bench_function("chunks_next", |bench| {
        // let pt = PieceTreeBytes::from_path(&PathBuf::from("benches/large.txt")).unwrap();

        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();

        let iter = pt.chunks();
        let mut i = iter.clone();
        bench.iter(move || {
            if i.next().is_none() {
                i = iter.clone();
            }
        });
    });

    c.bench_function("chunks_prev", |bench| {
        // let pt = PieceTreeBytes::from_path(&PathBuf::from("benches/large.txt")).unwrap();

        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();

        let iter = pt.chunks_at(pt.len());
        let mut i = iter.clone();
        bench.iter(move || {
            if i.prev().is_none() {
                i = iter.clone();
            }
        });
    });

    c.bench_function("chunks_next_10_000", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let mut pt = PieceTree::from_reader(large).unwrap();
        for _ in 0..10_000 {
            pt.insert(0, "A");
        }
        let chunks = pt.chunks();
        let mut chks = chunks.clone();
        bench.iter(move || {
            if chks.next().is_none() {
                chks = chunks.clone();
            }
        });
    });

    c.bench_function("chunks_prev_10_000", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let mut pt = PieceTree::from_reader(large).unwrap();
        for _ in 0..10_000 {
            pt.insert(0, "A");
        }
        let chunks = pt.chunks();
        let mut chks = chunks.clone();
        bench.iter(move || {
            if chks.prev().is_none() {
                chks = chunks.clone();
            }
        });
    });

    c.bench_function("chunks_next_100_000", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let mut pt = PieceTree::from_reader(large).unwrap();
        for _ in 0..100_000 {
            pt.insert(0, "A");
        }
        let chunks = pt.chunks();
        let mut chks = chunks.clone();
        bench.iter(move || {
            if chks.next().is_none() {
                chks = chunks.clone();
            }
        });
    });

    c.bench_function("chunks_prev_100_000", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let mut pt = PieceTree::from_reader(large).unwrap();
        for _ in 0..100_000 {
            pt.insert(0, "A");
        }
        let chunks = pt.chunks();
        let mut chks = chunks.clone();
        bench.iter(move || {
            if chks.prev().is_none() {
                chks = chunks.clone();
            }
        });
    });
}

fn create(c: &mut Criterion) {
    c.bench_function("create_bytes_iter_10_000", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let mut pt = PieceTree::from_reader(large).unwrap();
        for _ in 0..10_000 {
            pt.insert(0, "A");
        }

        bench.iter(move || {
            let _iter = pt.bytes();
        });
    });

    c.bench_function("create_chars_iter_10_000", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let mut pt = PieceTree::from_reader(large).unwrap();
        for _ in 0..10_000 {
            pt.insert(0, "A");
        }

        bench.iter(move || {
            let _iter = pt.chars();
        });
    });

    c.bench_function("create_graphemes_iter_10_000", |bench| {
        let large = io::Cursor::new(include_str!("large.txt"));
        let mut pt = PieceTree::from_reader(large).unwrap();
        for _ in 0..10_000 {
            pt.insert(0, "A");
        }

        bench.iter(move || {
            let _iter = pt.graphemes();
        });
    });
}

// criterion_group!(benches, chunks, bytes, chars, graphemes);
criterion_group!(benches, create);
criterion_main!(benches);
