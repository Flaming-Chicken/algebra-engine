use cat_core::handles::ObjId;
use cat_core::hypergraph::{HyperEdge, HyperOp, OpenHypergraph};
use cat_tangent::*;

#[test]
fn test_ito_lemma_drift_and_diffusion() {
    // f(x) = x^2, so f'(x) = 2x, f''(x) = 2
    // If dX_t = \mu dt + \sigma dW_t
    // By Itô's Lemma: d(X_t^2) = (2 X_t \mu + 1/2 (2) \sigma^2) dt + 2 X_t \sigma dW_t
    // Itô drift term = 1/2 \sigma^2 f''(x) = \sigma^2
    let jet = WeilJet::var(3.0, CalculusKind::Ito);
    let sigma = 0.5;
    let (drift, diffusion) = jet.apply_ito_lemma(9.0, 6.0, 2.0, sigma);

    assert_eq!(
        drift,
        0.5 * 0.25 * 2.0,
        "Drift must equal 1/2 * sigma^2 * f'' = 0.25"
    );
    assert_eq!(
        diffusion,
        6.0 * 0.5,
        "Diffusion must equal f' * sigma = 3.0"
    );
}

#[test]
fn test_hilger_time_scale_unification() {
    // Hilger time-scale product rule:
    // (f * g)^\Delta = f^\Delta g + f g^\Delta + \mu f^\Delta g^\Delta
    // Continuous case: \mu = 0
    let j1 = WeilJet {
        primal: 2.0,
        tangent: 3.0,
        second_order: 0.0,
        kind: CalculusKind::HilgerTimeScale {
            graininess_mu_twelfths: 0,
        },
    };
    let j2 = WeilJet {
        primal: 4.0,
        tangent: 5.0,
        second_order: 0.0,
        kind: CalculusKind::HilgerTimeScale {
            graininess_mu_twelfths: 0,
        },
    };
    let cont_prod = j1.mul(&j2);
    // (2 * 5) + (3 * 4) = 10 + 12 = 22
    assert_eq!(cont_prod.tangent, 22.0);

    // Discrete case: \mu = 1 (twelfths = 12)
    let jd1 = WeilJet {
        primal: 2.0,
        tangent: 3.0,
        second_order: 0.0,
        kind: CalculusKind::HilgerTimeScale {
            graininess_mu_twelfths: 12,
        },
    };
    let jd2 = WeilJet {
        primal: 4.0,
        tangent: 5.0,
        second_order: 0.0,
        kind: CalculusKind::HilgerTimeScale {
            graininess_mu_twelfths: 12,
        },
    };
    let disc_prod = jd1.mul(&jd2);
    // 22 + (3 * 5) * 1.0 = 22 + 15 = 37 (Exact forward difference \Delta(f g)!)
    assert_eq!(disc_prod.tangent, 37.0);
}

#[test]
fn test_colombeau_distribution_multiplication() {
    // \delta(x) * \Theta(x) = 1/2 \delta(x)
    let delta = GeneralizedDistribution::DiracDelta;
    let heaviside = GeneralizedDistribution::Heaviside;

    let prod = delta.multiply(&heaviside);
    match prod {
        GeneralizedDistribution::Product(factor, dist) => {
            assert_eq!(factor, 0.5);
            assert_eq!(*dist, GeneralizedDistribution::DiracDelta);
        }
        _ => panic!("Expected regularized product 1/2 delta(x)"),
    }
}

#[test]
fn test_tangent_functor_hypergraph_differentiation() {
    let mut diag = OpenHypergraph::new();
    let u = diag.alloc_wire();
    let v = diag.alloc_wire();
    let out = diag.alloc_wire();

    diag.add_input_port(u, ObjId(0));
    diag.add_input_port(v, ObjId(0));
    diag.add_output_port(out, ObjId(0));

    // out = u * v
    diag.add_edge(HyperEdge::binary(u, v, out, HyperOp::Mul));

    let t_diag = TangentFunctor::lift_hypergraph(&diag).expect("liftable");
    // Lifted diagram must double ports to 6 (3 primal + 3 tangent)
    assert_eq!(t_diag.boundary_ports.len(), 6);
    // Tangent diagram must contain Leibniz expansion: u*dv + v*du
    assert!(t_diag.edges.len() >= 3);
}

// --- B17: products of distributions, checked by integrating against a test function -----------------

/// `integral f(x) phi(x) dx` by the trapezoid rule on a fine grid over [-12, 12] (the integrands are smooth
/// Gaussians and error functions, and phi decays fast).
fn weak(f: impl Fn(f64) -> f64, phi: impl Fn(f64) -> f64) -> f64 {
    let (lo, hi, n) = (-12.0_f64, 12.0_f64, 480_000usize);
    let h = (hi - lo) / n as f64;
    (0..=n)
        .map(|i| {
            let x = lo + i as f64 * h;
            let w = if i == 0 || i == n { 0.5 } else { 1.0 };
            w * f(x) * phi(x)
        })
        .sum::<f64>()
        * h
}

#[test]
fn products_of_distributions_are_associated_with_the_right_limits() {
    use cat_tangent::colombeau::GeneralizedDistribution as G;
    let phi = |x: f64| (-0.5 * (x - 0.3).powi(2)).exp(); // a Gaussian test function, phi(0) = exp(-0.045)
    let phi0 = phi(0.0);
    let (delta, theta) = (G::DiracDelta, G::Heaviside);

    let mut errors = Vec::new();
    for eps in [0.05, 0.02] {
        // delta * theta -> phi(0) / 2: the generalized product is (1/2) delta.
        let product = delta.multiply(&theta);
        let numeric = weak(
            |x| delta.eval_mollified(x, eps) * theta.eval_mollified(x, eps),
            phi,
        );
        let symbolic = weak(|x| product.eval_mollified(x, eps), phi);
        errors.push((numeric - 0.5 * phi0).abs());
        assert!(
            (numeric - 0.5 * phi0).abs() < 0.1 * eps, // the weak limit is reached at rate O(eps)
            "delta*theta: {numeric}"
        );
        assert!(
            (symbolic - 0.5 * phi0).abs() < 0.1 * eps,
            "(1/2) delta: {symbolic}"
        );

        // delta^2: the integral grows like 1 / (2 sqrt(pi) eps) times phi(0): not a multiple of delta
        // (the old rule said 1 * delta, whose integral stays at phi(0)).
        let square = delta.multiply(&delta);
        let numeric = weak(|x| delta.eval_mollified(x, eps).powi(2), phi);
        let symbolic = weak(|x| square.eval_mollified(x, eps), phi);
        let predicted = phi0 / (2.0 * std::f64::consts::PI.sqrt() * eps);
        assert!(
            (numeric - predicted).abs() < 1e-2 * predicted,
            "delta^2: {numeric} vs {predicted}"
        );
        assert!(
            (symbolic - numeric).abs() < 1e-2 * numeric,
            "scaled delta: {symbolic} vs {numeric}"
        );
        assert!(
            matches!(
                square,
                G::Scaled {
                    epsilon_power: -1,
                    ..
                }
            ),
            "{square:?}"
        );

        // theta^2 -> theta weakly (the old rule said 0).
        let heavy = theta.multiply(&theta);
        assert_eq!(heavy, G::Heaviside);
        let numeric = weak(|x| theta.eval_mollified(x, eps).powi(2), phi);
        let target = weak(|x| if x > 0.0 { 1.0 } else { 0.0 }, phi);
        // The difference theta_eps^2 - theta_eps integrates to -eps phi(0) / sqrt(pi) to first order, because
        // the integral of theta (1 - theta) over the real line is 1 / sqrt(pi): it tends to 0 with epsilon.
        let first_order = -eps * phi0 / std::f64::consts::PI.sqrt();
        assert!(
            (numeric - target - first_order).abs() < 0.1 * first_order.abs(),
            "theta^2: {numeric} vs {target} (first-order correction {first_order})"
        );
    }
    assert!(
        errors[1] < errors[0],
        "the error of delta * theta shrinks with epsilon: {errors:?}"
    );
}

#[test]
fn scalar_and_scaled_factors_multiply_through() {
    use cat_tangent::colombeau::GeneralizedDistribution as G;
    let (delta, theta) = (G::DiracDelta, G::Heaviside);
    // (3 delta) * theta = 3/2 delta; scalars multiply; the order of the factors does not matter.
    let three_delta = delta.multiply(&G::Scalar(3.0));
    let p = three_delta.multiply(&theta);
    assert_eq!(p, G::Product(1.5, Box::new(G::DiracDelta)));
    assert_eq!(theta.multiply(&three_delta), p);
    assert_eq!(G::Scalar(2.0).multiply(&G::Scalar(5.0)), G::Scalar(10.0));
    // (delta^2) * theta keeps the epsilon scaling; delta^3 has order epsilon^-2.
    let sq_theta = delta.multiply(&delta).multiply(&theta);
    assert!(
        matches!(
            sq_theta,
            G::Scaled {
                epsilon_power: -1,
                ..
            }
        ),
        "{sq_theta:?}"
    );
    let cube = delta.multiply(&delta).multiply(&delta);
    assert!(
        matches!(
            cube,
            G::Scaled {
                epsilon_power: -2,
                ..
            }
        ),
        "{cube:?}"
    );
}

// --- B18: the tangent functor on every operation it supports, checked against finite differences -----

use cat_core::handles::{ScalarId, WireId};
use std::collections::HashMap;

/// Evaluate a diagram of scalar operations given the values on some wires (a fixed point over the edges, so the
/// order in which the edges are stored does not matter).
fn evaluate(diag: &OpenHypergraph, known: &[(WireId, f64)]) -> HashMap<WireId, f64> {
    let mut values: HashMap<WireId, f64> = known.iter().copied().collect();
    loop {
        let mut progressed = false;
        for edge in &diag.edges {
            if edge.outputs.iter().all(|w| values.contains_key(w)) {
                continue;
            }
            let inputs: Option<Vec<f64>> =
                edge.inputs.iter().map(|w| values.get(w).copied()).collect();
            let Some(inputs) = inputs else { continue };
            let value = match &edge.op {
                HyperOp::Add => inputs.iter().sum(),
                HyperOp::Mul | HyperOp::CliffordGeometricProduct => inputs.iter().product(),
                HyperOp::Neg => -inputs[0],
                HyperOp::Inv => 1.0 / inputs[0],
                HyperOp::Identity(_) => inputs[0],
                HyperOp::Nullary(ScalarId(0)) => 0.0,
                HyperOp::Nullary(ScalarId(1)) => 1.0,
                // The commutator of scalars is zero.
                HyperOp::LieBracket => 0.0,
                // An operation the evaluator does not know leaves its output undefined, and the caller's
                // assertions on that wire then fail.
                _ => continue,
            };
            values.insert(edge.outputs[0], value);
            progressed = true;
        }
        if !progressed {
            return values;
        }
    }
}

#[test]
fn the_tangent_diagram_computes_the_derivative_for_every_supported_operation() {
    // f(x, y) = ((x * y + x) * inv(y + 1)) * (-x) * y + 3 (a constant), built from Add, Mul, Neg, Inv, Nullary.
    let mut d = OpenHypergraph::new();
    let (x, y) = (d.alloc_wire(), d.alloc_wire());
    let (one, two) = (d.alloc_wire(), d.alloc_wire());
    d.add_edge(HyperEdge::new(
        smallvec::smallvec![],
        smallvec::smallvec![one],
        HyperOp::Nullary(ScalarId(1)),
    ));
    d.add_edge(HyperEdge::binary(one, one, two, HyperOp::Add));
    let (xy, xy_x, y1, inv_y1, quotient, neg_x, prod3, out) = (
        d.alloc_wire(),
        d.alloc_wire(),
        d.alloc_wire(),
        d.alloc_wire(),
        d.alloc_wire(),
        d.alloc_wire(),
        d.alloc_wire(),
        d.alloc_wire(),
    );
    d.add_edge(HyperEdge::binary(x, y, xy, HyperOp::Mul));
    d.add_edge(HyperEdge::binary(xy, x, xy_x, HyperOp::Add));
    d.add_edge(HyperEdge::binary(y, one, y1, HyperOp::Add));
    d.add_edge(HyperEdge::new(
        smallvec::smallvec![y1],
        smallvec::smallvec![inv_y1],
        HyperOp::Inv,
    ));
    d.add_edge(HyperEdge::binary(xy_x, inv_y1, quotient, HyperOp::Mul));
    d.add_edge(HyperEdge::new(
        smallvec::smallvec![x],
        smallvec::smallvec![neg_x],
        HyperOp::Neg,
    ));
    // a three-factor product (n-ary Leibniz rule): quotient * (-x) * y
    d.add_edge(HyperEdge::new(
        smallvec::smallvec![quotient, neg_x, y],
        smallvec::smallvec![prod3],
        HyperOp::Mul,
    ));
    d.add_edge(HyperEdge::binary(prod3, two, out, HyperOp::Add));

    // Ports identify the wires (the lifted diagram is canonicalised, which may renumber wires): each port of the
    // original becomes a primal and a tangent port, in this order.
    d.add_input_port(x, ObjId(0));
    d.add_input_port(y, ObjId(0));
    d.add_output_port(out, ObjId(0));
    d.add_output_port(two, ObjId(0));
    let lifted =
        TangentFunctor::lift_hypergraph(&d).expect("every operation here has a tangent rule");
    let f = |vx: f64, vy: f64| evaluate(&d, &[(x, vx), (y, vy)])[&out];
    let ports: Vec<WireId> = lifted.boundary_ports.iter().map(|p| p.wire).collect();
    assert_eq!(ports.len(), 8);
    let (px, tx, py, ty, po, to, t_two) = (
        ports[0], ports[1], ports[2], ports[3], ports[4], ports[5], ports[7],
    );
    for &(vx, vy, dx, dy) in &[
        (0.7, 1.3, 1.0, 0.0),
        (0.7, 1.3, 0.0, 1.0),
        (-1.2, 2.5, 0.8, -0.6),
        (2.0, 0.4, 1.0, 1.0),
    ] {
        let values = evaluate(&lifted, &[(px, vx), (tx, dx), (py, vy), (ty, dy)]);
        // The primal output equals the original function.
        assert!(
            (values[&po] - f(vx, vy)).abs() < 1e-12,
            "primal at ({vx}, {vy})"
        );
        // The tangent output is the directional derivative: compare with a central finite difference.
        let h = 1e-6;
        let numeric = (f(vx + h * dx, vy + h * dy) - f(vx - h * dx, vy - h * dy)) / (2.0 * h);
        assert!(
            (values[&to] - numeric).abs() < 1e-6 * (1.0 + numeric.abs()),
            "tangent at ({vx}, {vy}) along ({dx}, {dy}): {} vs {numeric}",
            values[&to]
        );
    }
    // A constant has tangent zero, even though no input reaches it.
    let values = evaluate(&lifted, &[(px, 1.0), (tx, 1.0), (py, 2.0), (ty, 1.0)]);
    assert_eq!(values[&t_two], 0.0);
}

#[test]
fn unsupported_operations_are_refused_not_silently_dropped() {
    let mut d = OpenHypergraph::new();
    let (a, b) = (d.alloc_wire(), d.alloc_wire());
    d.add_edge(HyperEdge::new(
        smallvec::smallvec![a],
        smallvec::smallvec![b],
        HyperOp::External(7),
    ));
    let err = TangentFunctor::lift_hypergraph(&d)
        .expect_err("an external function has no known derivative");
    assert!(
        matches!(err, TangentError::UnsupportedOperation(_)),
        "{err}"
    );
    assert!(err.to_string().contains("External"));
    // A malformed edge (an output wire that does not exist) is an error as well.
    let mut bad = OpenHypergraph::new();
    let w = bad.alloc_wire();
    bad.add_edge(HyperEdge::new(
        smallvec::smallvec![w],
        smallvec::smallvec![WireId(99)],
        HyperOp::Neg,
    ));
    assert!(matches!(
        TangentFunctor::lift_hypergraph(&bad),
        Err(TangentError::Malformed(_))
    ));
}
