use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_ucd::grapheme_break;

fn grapheme_break_lookup(c: &mut Criterion) {
    c.bench_function("lookup", |bench| {
        bench.iter(move || {
            grapheme_break('a');
        });
    });
}

criterion_group!(benches, grapheme_break_lookup);
criterion_main!(benches);
