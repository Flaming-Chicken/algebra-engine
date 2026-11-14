use cat_core::ball::UniversalBall;
use cat_core::dimensions::DimVector;
use cat_egraph::*;

#[test]
fn test_two_tiered_equivalence_lattice() {
    let mut egraph = EGraph::new();

    // Node 1: x
    let n1 = egraph.add(ENode::Var("x".into()));
    // Node 2: x / 1
    let n2 = egraph.add(ENode::Var("x_div_1".into()));
    // Node 3: x + 0.0000001
    let n3 = egraph.add(ENode::Var("x_approx".into()));

    // Union n1 and n2 via Strong Exact equality
    egraph.union(
        n1,
        n2,
        EquivalenceKind::strong_exact(true),
        "Identity division by 1",
    );
    assert_eq!(egraph.find(n1), egraph.find(n2));

    // Union n1 and n3 via Weak Association with tau tolerance
    let weak_kind =
        EquivalenceKind::weak_association(1e-6, UniversalBall::real_with_radius(0.0, 1e-6));
    egraph.union(n1, n3, weak_kind, "Float tolerance match");
    assert_eq!(egraph.find(n1), egraph.find(n3));

    // Verify edge records
    assert_eq!(egraph.edges.len(), 2);
    assert!(egraph.edges[0].kind.is_exact());
    assert!(!egraph.edges[1].kind.is_exact());
}

#[test]
fn test_buckingham_pi_reynolds_number() {
    // Variables for fluid flow past a cylinder:
    // 0: Velocity v [L T^-1]
    // 1: Length L [L]
    // 2: Density \rho [M L^-3]
    // 3: Viscosity \mu [M L^-1 T^-1]
    let vel = DimVector::velocity();
    let length = DimVector::length();
    let density = DimVector::mass() / (length * length * length);
    let viscosity = DimVector::force() * DimVector::time() / (length * length); // [M L^-1 T^-1]

    let vars = vec![vel, length, density, viscosity];
    let pi_groups = BuckinghamPi::find_dimensionless_groups(&vars);

    // 4 variables, 3 independent dimensions (L, M, T) => 4 - 3 = 1 dimensionless group (Reynolds number Re!)
    assert!(
        !pi_groups.is_empty(),
        "Must find dimensionless Pi group for fluid dynamics"
    );
    let re = &pi_groups[0];

    // Verify \prod v_i^{a_i} is dimensionless
    let mut net_dim = DimVector::DIMENSIONLESS;
    for (i, &p) in re.powers.iter().enumerate() {
        net_dim = net_dim + vars[i].pow_rational(p, 1).unwrap();
    }
    assert!(net_dim.is_dimensionless());
}

#[test]
fn test_egraph_flop_optimal_extraction() {
    let mut egraph = EGraph::new();

    // Expression: x * 0 vs 0
    let var_x = egraph.add(ENode::Var("x".into()));
    let num_0 = egraph.add(ENode::Scalar(0));
    let mul_x_0 = egraph.add(ENode::Mul(smallvec::smallvec![var_x, num_0]));

    // Strong zero contraction rule: x * 0 -> 0
    egraph.union(
        mul_x_0,
        num_0,
        EquivalenceKind::strong_exact(true),
        "Zero annihilation",
    );

    // Extract optimal cost node
    let (best, cost) = egraph.extract_min_cost(mul_x_0);
    assert_eq!(best, ENode::Scalar(0));
    assert_eq!(
        cost, 0,
        "Scalar 0 has 0 FLOP cost compared to multiplication"
    );
}
