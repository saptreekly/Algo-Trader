use algo_trader::strategy::{AdaptiveEngine, Strategy};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_on_tick(c: &mut Criterion) {
    let mut engine = AdaptiveEngine::new();
    let mut price_a = 100.0;
    let mut price_b = 100.0;

    c.bench_function("on_tick_performance", |b| {
        b.iter(|| {
            price_a += 0.1;
            price_b += 0.09;
            engine.on_tick(black_box(price_a), black_box(price_b))
        })
    });
}

criterion_group!(benches, bench_on_tick);
criterion_main!(benches);
