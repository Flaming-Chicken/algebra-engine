#![allow(clippy::expect_used, clippy::unwrap_used)] // benchmarks may panic on a broken fixture
use algebra_advanced::clifford::{CliffordMultivector, CliffordSignature};
use algebra_advanced::crypto::InformationTheory;
use algebra_advanced::padic::{HenselLifter, PadicNumber};
use algebra_advanced::proof::{ProofTrace, UnificationEngine};
use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_clifford_and_padic(c: &mut Criterion) {
    let mut group = c.benchmark_group("Advanced_Clifford_And_Padic");

    let sig = CliffordSignature::pga3d();
    let e1 = CliffordMultivector::basis_vector(0, sig).expect("e1");
    let e2 = CliffordMultivector::basis_vector(1, sig).expect("e2");
    let bivector = e1.geometric_product(&e2).expect("e1e2");
    let rotor =
        CliffordMultivector::make_rotor_3d(&bivector, std::f64::consts::FRAC_PI_4).expect("rotor");

    group.bench_function("clifford_3d_geometric_product", |b| {
        b.iter(|| black_box(e1.geometric_product(black_box(&e2)).expect("gp")))
    });

    group.bench_function("clifford_3d_rotor_sandwich", |b| {
        b.iter(|| black_box(e1.rotor_sandwich(black_box(&rotor)).expect("sandwich")))
    });

    let p_a = PadicNumber::from_rational(2, 3, 7, 16).expect("p_a");
    let p_b = PadicNumber::from_rational(5, 11, 7, 16).expect("p_b");
    group.bench_function("padic_q7_add_prec16", |b| {
        b.iter(|| black_box(p_a.add(black_box(&p_b))))
    });

    group.bench_function("hensel_lift_x2_minus_2_mod_7_prec12", |b| {
        // Lift x^2 - 2 = 0 mod 7 from r0 = 3 to precision 7^12
        b.iter(|| black_box(HenselLifter::lift_root(&[1, 0, -2], 3, 7, 12).expect("hensel")))
    });

    group.finish();
}

fn bench_proof_and_entropy(c: &mut Criterion) {
    let mut group = c.benchmark_group("Advanced_Proof_And_InfoTheory");

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let a = parser.parse("(x + 1)^2").expect("a");
    let b_expr = parser.parse("x^2 + 2*x + 1").expect("b");

    let mut trace = ProofTrace::new();
    trace.add_step("expand_binomial", a, b_expr);

    group.bench_function("proof_trace_verify_steps", |b| {
        b.iter(|| black_box(trace.verify_steps(&graph).expect("verify")))
    });

    let x_sym = graph.symbol("x");
    let forty_two = graph.integer(42);
    let unifier = UnificationEngine::new();
    group.bench_function("unification_atom", |b| {
        b.iter(|| black_box(unifier.unify(&graph, x_sym, forty_two).expect("unify")))
    });

    let info = InformationTheory;
    let probs = [
        graph.rational(1, 2),
        graph.rational(1, 4),
        graph.rational(1, 4),
    ];
    group.bench_function("symbolic_shannon_entropy", |b| {
        b.iter(|| black_box(info.entropy(&graph, &probs)))
    });

    group.finish();
}

criterion_group!(benches, bench_clifford_and_padic, bench_proof_and_entropy);
criterion_main!(benches);
