#![allow(clippy::expect_used, clippy::unwrap_used)] // benchmarks may panic on a broken fixture
use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;
use algebra_engine::autodiff::HyperDual;
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::matrix::SymbolicMatrix;
use algebra_engine::ode::{NumericalOdeConfig, NumericalOdeMethod, NumericalOdeSolver};
use algebra_engine::pde::{FeaBoundaryCondition, FeaEngine, FeaMesh, PhysicsDiscipline};
use algebra_engine::pinn::PinnResidual;
use algebra_engine::poly::grobner::GrobnerBasis;
use algebra_engine::poly::multivariate::{MonomialOrder, MultiPoly};
use algebra_engine::simplify::Simplifier;
use algebra_engine::solver::SymbolicSolver;
use algebra_engine::stream::StreamProcessor;
use algebra_engine::symplectic::{KeplerOrbit2D, SymplecticIntegrator, SymplecticOrder};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::io::Cursor;

fn bench_symbolic_cas(c: &mut Criterion) {
    let mut group = c.benchmark_group("Engine_Symbolic_CAS");

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let x_sym = graph.symbols.get_or_intern("x");

    let trig_expr = parser
        .parse("sin(x)^2 + cos(x)^2 + (x + 0) * 1")
        .expect("parse");
    let rat_expr = parser.parse("(x^2 - 1) / (x - 1)").expect("parse");

    group.bench_function("simplify_trig_identity", |b| {
        b.iter(|| black_box(Simplifier::simplify(&graph, trig_expr).expect("simplify")))
    });

    group.bench_function("simplify_rational", |b| {
        b.iter(|| black_box(Simplifier::simplify(&graph, rat_expr).expect("simplify")))
    });

    let diff_target = parser.parse("x^3 * sin(x) + exp(-x^2)").expect("parse");
    group.bench_function("diff_product_chain", |b| {
        b.iter(|| black_box(graph.diff(diff_target, x_sym)))
    });

    let int_target = parser.parse("x * sin(x) + 3 * x^2").expect("parse");
    group.bench_function("integrate_by_parts_poly", |b| {
        b.iter(|| black_box(graph.integrate(int_target, x_sym).expect("integrate")))
    });

    let quad_eq = parser.parse("x^2 - 5*x + 6").expect("parse");
    group.bench_function("solveset_quadratic", |b| {
        b.iter(|| black_box(graph.solveset(quad_eq, x_sym).expect("solve")))
    });

    // Multivariate Gröbner basis on circle-parabola system
    let f1 = parser.parse("x^2 + y^2 - 4").expect("f1");
    let f2 = parser.parse("y - x^2 + 1").expect("f2");
    let vars = vec!["x".to_string(), "y".to_string()];
    let p1 = MultiPoly::from_expr(&graph, f1, &vars, MonomialOrder::GrevLex).expect("p1");
    let p2 = MultiPoly::from_expr(&graph, f2, &vars, MonomialOrder::GrevLex).expect("p2");
    group.bench_function("grobner_buchberger_2var", |b| {
        b.iter(|| {
            black_box(GrobnerBasis::buchberger(black_box(&[
                p1.clone(),
                p2.clone(),
            ])))
        })
    });

    // 4x4 Symbolic Matrix determinant & inverse
    let mut elems = Vec::with_capacity(16);
    for r in 0..4 {
        for col in 0..4 {
            let v = if r == col { 4 + r as i64 } else { 1 };
            elems.push(graph.integer(v));
        }
    }
    let mat4 = SymbolicMatrix::new(4, 4, elems).expect("mat4");
    group.bench_function("matrix_det_4x4", |b| {
        b.iter(|| black_box(mat4.determinant(&graph).expect("det")))
    });
    group.bench_function("matrix_inv_4x4", |b| {
        b.iter(|| black_box(mat4.inverse(&graph).expect("inv")))
    });

    group.finish();
}

fn bench_numerical_physics_and_fea(c: &mut Criterion) {
    let mut group = c.benchmark_group("Engine_Numerical_Physics");

    // 1. Adaptive & implicit ODE solvers (relating to physics-informed benchmark doc 07)
    for (name, method) in [
        ("tsitouras54", NumericalOdeMethod::Tsitouras54),
        ("dopri5", NumericalOdeMethod::DormandPrince54),
        ("radau_iia5", NumericalOdeMethod::RadauIIA5),
        (
            "self_optimizing_meta",
            NumericalOdeMethod::SelfOptimizingMeta,
        ),
    ] {
        let cfg = NumericalOdeConfig {
            rtol: 1e-6,
            atol: 1e-9,
            initial_step: Some(0.05),
            max_step: 0.2,
            min_step: 1e-12,
            max_steps: 20_000,
            method,
        };
        group.bench_with_input(
            BenchmarkId::new("ode_harmonic_oscillator", name),
            &cfg,
            |b, cfg| {
                b.iter(|| {
                    black_box(
                        NumericalOdeSolver::solve(
                            |_t, y| vec![y[1], -y[0]],
                            (0.0, 10.0),
                            &[1.0, 0.0],
                            cfg,
                        )
                        .expect("ode"),
                    )
                })
            },
        );
    }

    // 2. Symplectic Hamiltonian integrators (Kepler 2-Body orbit, doc 05)
    let kepler = KeplerOrbit2D::new(1.0, 1.0);
    for (label, order) in [
        ("verlet2_1000steps", SymplecticOrder::VerletOrder2),
        ("yoshida4_1000steps", SymplecticOrder::YoshidaOrder4),
    ] {
        group.bench_function(label, |b| {
            b.iter(|| {
                black_box(SymplecticIntegrator::integrate(
                    &kepler,
                    &[1.0, 0.0],
                    &[0.0, 1.0],
                    0.01,
                    1000,
                    order,
                ))
            })
        });
    }

    // 3. PINN HyperDual PDE residual evaluation
    group.bench_function("pinn_burgers_hyperdual_64pts", |b| {
        b.iter(|| {
            let mut acc = 0.0;
            for i in 0..64 {
                let x = (i as f64) * 0.05;
                let r = PinnResidual::burgers_residual(
                    |hx: HyperDual<f64>, ht: HyperDual<f64>| {
                        -(hx * (HyperDual::constant(1.0) / ht.exp())).sin()
                    },
                    x,
                    0.25,
                    0.01,
                );
                acc += r * r;
            }
            black_box(acc)
        })
    });

    // 4. FEA 2D Cantilever Linear Elasticity
    let mesh = FeaMesh::plate_2d(10.0, 2.0, 10, 4);
    let physics = PhysicsDiscipline::LinearElasticity {
        youngs_modulus: 200_000.0,
        poissons_ratio: 0.3,
        density: 7850.0,
        body_force: vec![0.0, 0.0],
        plane_stress: true,
    };
    let bcs = vec![
        FeaBoundaryCondition::FixedDisplacement {
            group_name: "left".into(),
            dof_index: 0,
            value: 0.0,
        },
        FeaBoundaryCondition::FixedDisplacement {
            group_name: "left".into(),
            dof_index: 1,
            value: 0.0,
        },
        FeaBoundaryCondition::PointForce {
            group_name: "right".into(),
            force_vector: vec![0.0, -500.0],
        },
    ];
    group.bench_function("fea_2d_plate_10x4", |b| {
        b.iter(|| black_box(FeaEngine::solve(&mesh, &physics, &bcs).expect("fea")))
    });

    // 5. CSV BytecodeVM stream batch (500 rows)
    let stream_graph = ExprGraph::new();
    let mut csv_buf = String::from("x,y\n");
    for i in 0..500 {
        csv_buf.push_str(&format!("{},{}\n", i as f64 * 0.1, i as f64 * 0.2));
    }
    group.bench_function("stream_eval_csv_500rows", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(8192);
            black_box(
                StreamProcessor::eval_csv_stream(
                    &stream_graph,
                    "x^2 + sin(y)",
                    "z",
                    Cursor::new(csv_buf.as_bytes()),
                    &mut out,
                )
                .expect("csv"),
            )
        })
    });

    group.finish();
}

criterion_group!(benches, bench_symbolic_cas, bench_numerical_physics_and_fea);
criterion_main!(benches);
