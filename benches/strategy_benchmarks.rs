use algo_trader::strategy::{AdaptiveEngine, Strategy};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_on_tick(c: &mut Criterion) {
    let mut engine = AdaptiveEngine::new();
    let balance = 2500.0;
    let mut step = 0u64;
    let base_price_a = 150.0;
    let base_price_b = 100.0;

    c.bench_function("on_tick_performance", |b| {
        b.iter(|| {
            step += 1;
            let price_modifier = (step % 50) as f64 * 0.10; 
            let dynamic_price_a = base_price_a + price_modifier;
            let dynamic_price_b = base_price_b - price_modifier;
            let dynamic_volume = 100.0 + (step % 10) as f64 * 10.0;
            let dynamic_trades = 1 + (step % 3);
            
            engine.on_tick(
                black_box(dynamic_price_a),
                black_box(dynamic_price_b),
                black_box(dynamic_volume),
                black_box(dynamic_volume * 0.9),
                black_box(dynamic_trades as u64),
                black_box(dynamic_trades as u64),
                black_box(balance),
                black_box(0),
            )
        })
    });
}

criterion_group!(benches, bench_on_tick);
criterion_main!(benches);
