#![allow(clippy::expect_used, clippy::unwrap_used)] // benchmarks may panic on a broken fixture
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use urae::api::sym;
use urae::prelude::*;

fn bench_urae_facade_and_session(c: &mut Criterion) {
    let mut group = c.benchmark_group("Urae_Facade_Workflows");

    group.bench_function("session_execute_diff_and_solve", |b| {
        let mut session = UraeSession::new();
        b.iter(|| {
            let r1 = session.execute_line("diff x^3 * exp(x), x");
            let r2 = session.execute_line("solve x^2 - 5*x + 6 = 0, x");
            black_box((r1.is_error, r2.is_error))
        })
    });

    let graph = ExprGraph::new();
    let ctx = ExecutionContext::new();
    group.bench_function("query_engine_multi_pod", |b| {
        b.iter(|| {
            black_box(QueryEngine::query(
                &graph,
                black_box("integrate x * sin(x)"),
                &ctx,
            ))
        })
    });

    group.bench_function("fluent_sym_expr_pipeline", |b| {
        let g = ExprGraph::new();
        let x = sym(&g, "x");
        b.iter(|| {
            let f = (x * x + 2 * x + 1).diff(&x).simplify();
            black_box(f)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_urae_facade_and_session);
criterion_main!(benches);
