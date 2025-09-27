use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_buffer::PieceTree;

const CAP: usize = 10_000_000;
const LARGE: &str = include_str!("large.txt");

fn insert(c: &mut Criterion) {
    c.bench_function("insert_start", |bench| {
        let mut pt = PieceTree::new();
        bench.iter(move || {
            if pt.piece_count() >= CAP {
                pt = PieceTree::new();
            }

            pt.insert(0, b"a")
        });
    });

    c.bench_function("insert_middle", |bench| {
        let mut pt = PieceTree::new();
        bench.iter(move || {
            if pt.piece_count() >= CAP {
                pt = PieceTree::new();
            }

            pt.insert((pt.len() + 1) / 2, b"a")
        });
    });

    c.bench_function("insert_end", |bench| {
        let mut pt = PieceTree::new();
        bench.iter(move || {
            if pt.piece_count() >= CAP {
                pt = PieceTree::new();
            }

            pt.insert(pt.len(), b"a")
        });
    });
}

fn full_pt() -> PieceTree {
    let mut pt = PieceTree::new();
    while pt.len() < CAP as u64 {
        pt.insert(0, LARGE.as_bytes());
    }
    pt
}

fn remove(c: &mut Criterion) {
    c.bench_function("remove_start", |bench| {
        let mut pt = full_pt();

        bench.iter(move || {
            if pt.is_empty() {
                pt = full_pt();
            }

            pt.remove(0..1)
        });
    });

    c.bench_function("remove_middle", |bench| {
        let mut pt = full_pt();

        bench.iter(move || {
            let mid = (pt.len() + 1) / 2;
            if pt.is_empty() || mid + 1 > pt.len() {
                pt = full_pt();
            }

            pt.remove(mid..mid + 1)
        });
    });

    c.bench_function("remove_end", |bench| {
        let mut pt = full_pt();

        bench.iter(move || {
            if pt.is_empty() {
                pt = full_pt();
            }

            pt.remove(pt.len() - 1..pt.len())
        });
    });
}

criterion_group!(benches, insert, remove);
criterion_main!(benches);
