use algo_trader::strategy::{AdaptiveEngine, Strategy};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_on_tick(c: &mut Criterion) {
    let mut engine = AdaptiveEngine::new();
    let price_a = 100.0;
    let price_b = 100.0;
    let vol_a = 1000.0;
    let vol_b = 1000.0;
    let trades_a = 10;
    let trades_b = 10;
    let balance = 100.0;

    c.bench_function("on_tick_performance", |b| {
        b.iter(|| {
            engine.on_tick(
                black_box(price_a),
                black_box(price_b),
                black_box(vol_a),
                black_box(vol_b),
                black_box(trades_a),
                black_box(trades_b),
                black_box(balance),
                black_box(0),
            )
        })
    });
}

criterion_group!(benches, bench_on_tick);
criterion_main!(benches);
