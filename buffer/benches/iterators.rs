use std::{io, path::PathBuf};

use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_buffer::piece_tree::PieceTree;

fn bytes(c: &mut Criterion) {
    c.bench_function("bytes_next", |bench| {
        // let pt = PieceTreeBytes::from_path(&PathBuf::from("benches/large.txt")).unwrap();

        let large = io::Cursor::new(include_str!("large.txt"));
        let pt = PieceTree::from_reader(large).unwrap();

        let iter = pt.bytes();
        let mut i = iter.clone();
        bench.iter(move || {
            if i.next().is_none() {
                i = iter.clone();
            }
        });
    });

    c.bench_function("bytes_prev", |bench| {
        // let pt = PieceTreeBytes::from_path(&PathBuf::from("benches/large.txt")).unwrap();

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

// fn graphemes(c: &mut Criterion) {
//     c.bench_function("grapheme_boundary_next", |bench| {
//         let large = io::Cursor::new(include_str!("large.txt"));
//         let pt = PieceTree::from_reader(large).unwrap();
//         let slice = pt.slice(..);
//         let mut pos = 0;

//         bench.iter(move || {
//             let end = next_grapheme_boundary(&slice, pos);
//             pos = end;

//             if pos == slice.len() {
//                 pos = 0
//             }
//         });
//     });

//     c.bench_function("grapheme_boundary_prev", |bench| {
//         let large = io::Cursor::new(include_str!("large.txt"));
//         let pt = PieceTree::from_reader(large).unwrap();
//         let slice = pt.slice(..);
//         let mut pos = pt.len();

//         bench.iter(move || {
//             let end = prev_grapheme_boundary(&slice, pos);
//             pos = end;

//             if pos == 0 {
//                 pos = slice.len();
//             }
//         });
//     });
// }

// fn graphemes(c: &mut Criterion) {
//     c.bench_function("graphemes_next", |bench| {
//         let large = io::Cursor::new(include_str!("large.txt"));
//         let pt = PieceTree::from_reader(large).unwrap();
//         let bytes = pt.bytes();
//         let graphemes = Graphemes::from(CodePoints::from(bytes));
//         let mut g = graphemes.clone();
//         bench.iter(move || {
//             if g.next().is_none() {
//                 g = graphemes.clone();
//             }
//         });
//     });

//     c.bench_function("graphemes_prev", |bench| {
//         let large = io::Cursor::new(include_str!("large.txt"));
//         let pt = PieceTreeBytes::new(large).unwrap();
//         let graphemes = pt.graphemes_at(pt.len());
//         let mut g = graphemes.clone();
//         bench.iter(move || {
//             if g.prev().is_none() {
//                 g = graphemes.clone();
//             }
//         });
//     });
// }

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
            pt.insert_str(0, "A");
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
            pt.insert_str(0, "A");
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
            pt.insert_str(0, "A");
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
            pt.insert_str(0, "A");
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

// criterion_group!(benches, bytes, chars, graphemes);
criterion_group!(benches, chars);
criterion_main!(benches);
