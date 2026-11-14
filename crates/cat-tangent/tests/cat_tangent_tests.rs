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

    assert_eq!(drift, 0.5 * 0.25 * 2.0, "Drift must equal 1/2 * sigma^2 * f'' = 0.25");
    assert_eq!(diffusion, 6.0 * 0.5, "Diffusion must equal f' * sigma = 3.0");
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
        kind: CalculusKind::HilgerTimeScale { graininess_mu_twelfths: 0 },
    };
    let j2 = WeilJet {
        primal: 4.0,
        tangent: 5.0,
        second_order: 0.0,
        kind: CalculusKind::HilgerTimeScale { graininess_mu_twelfths: 0 },
    };
    let cont_prod = j1.mul(&j2);
    // (2 * 5) + (3 * 4) = 10 + 12 = 22
    assert_eq!(cont_prod.tangent, 22.0);

    // Discrete case: \mu = 1 (twelfths = 12)
    let jd1 = WeilJet {
        primal: 2.0,
        tangent: 3.0,
        second_order: 0.0,
        kind: CalculusKind::HilgerTimeScale { graininess_mu_twelfths: 12 },
    };
    let jd2 = WeilJet {
        primal: 4.0,
        tangent: 5.0,
        second_order: 0.0,
        kind: CalculusKind::HilgerTimeScale { graininess_mu_twelfths: 12 },
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

    let t_diag = TangentFunctor::lift_hypergraph(&diag);
    // Lifted diagram must double ports to 6 (3 primal + 3 tangent)
    assert_eq!(t_diag.boundary_ports.len(), 6);
    // Tangent diagram must contain Leibniz expansion: u*dv + v*du
    assert!(t_diag.edges.len() >= 3);
}
