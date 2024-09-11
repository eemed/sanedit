use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_syntax::Parser;

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
        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        bench.iter(move || {
            parser.parse(content);
        });
    });
}

fn toml(c: &mut Criterion) {
    let peg = include_str!("../pegs/toml.peg");
    let content = include_str!("large.toml");

    c.bench_function("parse_large_toml", |bench| {
        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        bench.iter(move || {
            parser.parse(content);
        });
    });

    c.bench_function("parse_large_toml_invalid", |bench| {
        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        bench.iter(move || {
            parser.parse(&content[125..]);
        });
    });
}

// criterion_group!(benches, json, toml);
criterion_group!(benches, toml);
criterion_main!(benches);
