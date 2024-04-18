use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_parser::PikaParser;

fn json(c: &mut Criterion) {
    let peg = include_str!("../pegs/json.peg");
    let content = include_str!("large.json");

    c.bench_function("parse_large_json", |bench| {
        let parser = PikaParser::new(std::io::Cursor::new(peg)).unwrap();
        bench.iter(move || {
            parser.parse(content).unwrap();
        });
    });
}

criterion_group!(benches, json);
criterion_main!(benches);
