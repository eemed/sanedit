use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_syntax::bench::ParsingMachine;

fn json(c: &mut Criterion) {
    let peg = include_str!("../pegs/json.peg");
    let content = include_str!("large.json");

    // c.bench_function("parse_large_json", |bench| {
    //     let parser = PikaParser::new(std::io::Cursor::new(peg)).unwrap();
    //     bench.iter(move || {
    //         parser.parse(content).unwrap();
    //     });
    // });
    //
    c.bench_function("parse_large_json", |bench| {
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(move || {
            assert!(parser.parse(content).is_ok());
        });
    });
}

fn toml(c: &mut Criterion) {
    let peg = include_str!("../pegs/toml.peg");
    let content = include_str!("large.toml");

    c.bench_function("parse_large_toml", |bench| {
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(move || {
            assert!(parser.parse(content).is_ok());
        });
    });

    c.bench_function("parse_large_toml_invalid", |bench| {
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(move || {
            assert!(parser.parse(&content[125..]).is_ok());
        });
    });
}

criterion_group!(benches, json, toml);
criterion_main!(benches);
