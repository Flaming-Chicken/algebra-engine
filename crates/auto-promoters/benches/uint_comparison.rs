#![allow(clippy::expect_used, clippy::unwrap_used)] // benchmarks may panic on a broken fixture
use auto_promoters::uint::Uint;
use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use num::{BigUint, One};
use std::hint::black_box as std_black_box;

fn bench_addition_and_multiplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("Uint_Arithmetic");

    let m_val = usize::MAX - 100;
    let uint_m = Uint::Machine(m_val);
    let large_b = BigUint::from(usize::MAX) + BigUint::one();
    let large_u = Uint::Promoted(large_b.clone());

    // Machine-path addition vs raw usize
    group.bench_function("add_machine_usize", |b| {
        b.iter(|| std_black_box(m_val).wrapping_add(std_black_box(1)))
    });
    group.bench_function("add_machine_uint", |b| {
        b.iter(|| black_box(uint_m.clone()) + black_box(1usize))
    });

    // Promoted-path addition vs raw BigUint
    group.bench_function("add_promoted_biguint", |b| {
        b.iter(|| black_box(large_b.clone()) + black_box(1u32))
    });
    group.bench_function("add_promoted_uint", |b| {
        b.iter(|| black_box(large_u.clone()) + black_box(1usize))
    });

    // Multiplication: machine vs auto-promoting vs already-promoted
    let small_a = Uint::Machine(1_234_567);
    let small_b = Uint::Machine(7_654_321);
    group.bench_function("mul_machine", |b| {
        b.iter(|| black_box(small_a.clone()) * black_box(small_b.clone()))
    });
    group.bench_function("mul_promoted", |b| {
        let p = Uint::Machine(1_000_000_000_000usize);
        b.iter(|| black_box(large_u.clone()) * black_box(p.clone()))
    });

    group.finish();
}

fn bench_comparisons_and_ordering(c: &mut Criterion) {
    let mut group = c.benchmark_group("Uint_Compare");
    let m1 = Uint::Machine(42);
    let m2 = Uint::Machine(usize::MAX - 5);
    let p1 = Uint::Promoted(BigUint::from(usize::MAX) + 7u32);
    let p2 = Uint::Promoted(BigUint::from(usize::MAX) + 99u32);

    for (label, lhs, rhs) in [
        ("machine_vs_machine", &m1, &m2),
        ("machine_vs_promoted", &m2, &p1),
        ("promoted_vs_promoted", &p1, &p2),
    ] {
        group.bench_with_input(
            BenchmarkId::new("ord_cmp", label),
            &(lhs, rhs),
            |b, &(x, y)| b.iter(|| black_box(x).cmp(black_box(y))),
        );
    }

    group.finish();
}

fn bench_transitions_and_div(c: &mut Criterion) {
    let mut group = c.benchmark_group("Uint_Transitions_And_Div");

    group.bench_function("trigger_promotion_add", |b| {
        b.iter_batched(
            || Uint::Machine(usize::MAX),
            |u| black_box(u) + black_box(1usize),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("trigger_demotion_sub", |b| {
        b.iter_batched(
            || Uint::Promoted(BigUint::from(usize::MAX) + 1u32),
            |u| black_box(u) - black_box(1usize),
            BatchSize::SmallInput,
        )
    });

    let dividend = Uint::Promoted(BigUint::from(usize::MAX) * BigUint::from(97u32) + 13u32);
    let divisor = Uint::Machine(97);
    group.bench_function("div_promoted_by_machine", |b| {
        b.iter(|| black_box(dividend.clone()) / black_box(divisor.clone()))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_addition_and_multiplication,
    bench_comparisons_and_ordering,
    bench_transitions_and_div
);
criterion_main!(benches);
