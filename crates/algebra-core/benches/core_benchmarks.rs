#![allow(clippy::expect_used, clippy::unwrap_used)] // benchmarks may panic on a broken fixture
use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, LatexFormatter, UnicodeFormatter};
use algebra_core::interval::RealInterval;
use algebra_core::parser::ExprParser;
use algebra_core::probabilistic::ProbabilisticVerifier;
use algebra_core::vm::{BytecodeVM, DualNumber, VmCompiler};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

fn bench_expr_graph_and_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Core_ExprGraph_And_Parser");

    group.bench_function("hash_cons_hot_interning", |b| {
        let graph = ExprGraph::new();
        let x = graph.symbol("x");
        let two = graph.integer(2);
        b.iter(|| {
            let x_sq = graph.pow(black_box(x), black_box(two));
            black_box(graph.add([x_sq, x, two]))
        })
    });

    for (label, input) in [
        ("poly", "3*x^4 - 5*x^3 + 2*x^2 - 7*x + 11"),
        (
            "transcendental",
            "sin(x)^2 + cos(x)^2 + exp(-x^2) * ln(x + 1)",
        ),
        ("latex", r"\frac{x^2 + 3\cdot x - 5}{\sqrt{x + 1}}"),
    ] {
        group.bench_with_input(BenchmarkId::new("parse", label), input, |b, expr_str| {
            let graph = ExprGraph::new();
            let parser = ExprParser::new(&graph);
            b.iter(|| black_box(parser.parse(black_box(expr_str)).expect("valid expr")))
        });
    }

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let expr = parser
        .parse("(x^3 + 2*x - 1) / (x^2 + 1) + sin(x * y)")
        .expect("parse");

    group.bench_function("format_latex", |b| {
        let fmt = LatexFormatter;
        b.iter(|| black_box(fmt.format(&graph, black_box(expr)).expect("latex")))
    });

    group.bench_function("format_unicode", |b| {
        let fmt = UnicodeFormatter;
        b.iter(|| black_box(fmt.format(&graph, black_box(expr)).expect("unicode")))
    });

    group.finish();
}

fn bench_bytecode_vm_and_intervals(c: &mut Criterion) {
    let mut group = c.benchmark_group("Core_VM_And_Interval");

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let expr = parser
        .parse("sin(x) * exp(-0.1 * x^2) + cos(2 * x) / (1 + x^2)")
        .expect("parse");
    let program = VmCompiler::compile(&graph, expr, "x", &[]).expect("compile");
    let vm = BytecodeVM::new();

    group.bench_function("vm_compile", |b| {
        b.iter(|| black_box(VmCompiler::compile(&graph, expr, "x", &[]).expect("compile")))
    });

    group.bench_function("vm_eval_scalar", |b| {
        b.iter(|| {
            black_box(
                vm.eval_scalar(&program, black_box(1.25), &[])
                    .expect("eval"),
            )
        })
    });

    let xs: Vec<f64> = (0..1024).map(|i| -5.0 + (i as f64) * 0.01).collect();
    let mut ys = vec![0.0; xs.len()];
    group.bench_function("vm_eval_batch_1024", |b| {
        b.iter(|| {
            vm.eval_batch(&program, black_box(&xs), &[], &mut ys)
                .expect("batch");
            black_box(ys[512])
        })
    });

    group.bench_function("vm_eval_dual", |b| {
        let x_d = DualNumber::variable(1.25);
        b.iter(|| black_box(vm.eval_dual(&program, black_box(x_d), &[]).expect("dual")))
    });

    group.bench_function("real_interval_poly_chain", |b| {
        let i_x = RealInterval::new(1.0, 2.0);
        let i_c = RealInterval::point(3.5);
        b.iter(|| {
            let sq = black_box(i_x).mul(&i_x);
            let shifted = sq.add(&i_c);
            black_box(shifted.sub(&i_x))
        })
    });

    let lhs = parser.parse("(x - 1) * (x + 1) * (x + 2)").expect("lhs");
    let rhs = parser.parse("x^3 + 2*x^2 - x - 2").expect("rhs");
    group.bench_function("schwartz_zippel_verify_equivalence", |b| {
        b.iter(|| {
            black_box(ProbabilisticVerifier::verify_equivalence(
                &graph, lhs, rhs, 1e-12,
            ))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_expr_graph_and_parsing,
    bench_bytecode_vm_and_intervals
);
criterion_main!(benches);
