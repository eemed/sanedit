use criterion::{criterion_group, criterion_main, Criterion};
// use regex::Regex;

fn compilation(_c: &mut Criterion) {
    //     c.bench_function("compile_car?", |bench| {
    //         bench.iter(move || {
    //             let regex = Regex::new("car?");
    //         });
    //     });
}

criterion_group!(benches, compilation);
criterion_main!(benches);
