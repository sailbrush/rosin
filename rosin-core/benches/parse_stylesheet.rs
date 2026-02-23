use std::{hint::black_box, str::FromStr};

use criterion::{Criterion, criterion_group, criterion_main};
use rosin_core::css::Stylesheet;

const CSS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tests/css/test.css"));

fn bench_parse_stylesheet(c: &mut Criterion) {
    c.bench_function("stylesheet_parse", |b| {
        b.iter(|| {
            let data = black_box(CSS);
            let _ = Stylesheet::from_str(data);
        })
    });
}

criterion_group!(benches, bench_parse_stylesheet);
criterion_main!(benches);
