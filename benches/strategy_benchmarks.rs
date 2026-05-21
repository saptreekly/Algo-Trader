use criterion::{black_box, criterion_group, criterion_main, Criterion};
use algo_trader::strategy::{AdaptiveEngine, Strategy};

fn bench_on_tick(c: &mut Criterion) {
    let mut engine = AdaptiveEngine::new();
    let mut price = 100.0;

    c.bench_function("on_tick_performance", |b| {
        b.iter(|| {
            price += 0.1;
            engine.on_tick(black_box(price))
        })
    });
}

criterion_group!(benches, bench_on_tick);
criterion_main!(benches);
