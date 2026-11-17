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

fn exact(desc: &str) -> (EquivalenceKind, String) {
    (EquivalenceKind::strong_exact(true), desc.to_string())
}

#[test]
fn adding_an_existing_node_returns_the_existing_class() {
    let mut egraph = EGraph::new();
    let x = egraph.add(ENode::Var("x".into()));
    assert_eq!(egraph.add(ENode::Var("x".into())), x);
    let y = egraph.add(ENode::Var("y".into()));
    let sum = egraph.add(ENode::Add(smallvec::smallvec![x, y]));
    assert_eq!(egraph.add(ENode::Add(smallvec::smallvec![x, y])), sum);
    // The same operator with different children is a different class.
    assert_ne!(egraph.add(ENode::Add(smallvec::smallvec![y, x])), sum);
    // After x = y, the nodes f(x) and f(y) are the same node.
    let neg_x = egraph.add(ENode::Neg(x));
    let (kind, desc) = exact("x = y");
    egraph.union(x, y, kind, &desc);
    assert_eq!(egraph.add(ENode::Neg(y)), egraph.find(neg_x));
}

#[test]
fn rebuild_restores_congruence() {
    // f(a) and f(b) are distinct until a = b; then congruence merges them, and so on upward.
    let mut egraph = EGraph::new();
    let a = egraph.add(ENode::Var("a".into()));
    let b = egraph.add(ENode::Var("b".into()));
    let fa = egraph.add(ENode::Neg(a));
    let fb = egraph.add(ENode::Neg(b));
    let ffa = egraph.add(ENode::Neg(fa));
    let ffb = egraph.add(ENode::Neg(fb));
    assert_ne!(egraph.find(fa), egraph.find(fb));

    let (kind, desc) = exact("a = b");
    egraph.union(a, b, kind, &desc);
    // Not merged until rebuilt.
    assert_ne!(egraph.find(fa), egraph.find(fb));
    egraph.rebuild();
    assert_eq!(egraph.find(fa), egraph.find(fb));
    assert_eq!(egraph.find(ffa), egraph.find(ffb));
    // The congruence merges are recorded as strong exact equivalences.
    assert!(
        egraph
            .edges
            .iter()
            .any(|e| e.description == "congruence closure" && e.kind.is_exact())
    );
}

#[test]
fn extraction_counts_the_cost_of_the_whole_expression() {
    let mut egraph = EGraph::new();
    let x = egraph.add(ENode::Var("x".into()));
    let y = egraph.add(ENode::Var("y".into()));
    let two = egraph.add(ENode::Scalar(2));
    // (x + y) * 2: Mul over two operands plus Add over two plus three leaves.
    let sum = egraph.add(ENode::Add(smallvec::smallvec![x, y]));
    let product = egraph.add(ENode::Mul(smallvec::smallvec![sum, two]));
    let (_, cost) = egraph.extract_min_cost(product);
    // Mul = 2 + 2*2 = 6, Add = 1 + 2 = 3, x = 1, y = 1, 2 = 0.
    assert_eq!(cost, 6 + 3 + 1 + 1);

    // A shallow node-only cost would call this 6; a cheaper-looking alternative with an
    // expensive operand must not win: x*x as a power (5 + 1 + 0) vs the product (6 + 1 + 1).
    let x2 = egraph.add(ENode::Pow(x, two));
    let xx = egraph.add(ENode::Mul(smallvec::smallvec![x, x]));
    let (kind, desc) = exact("x^2 = x*x");
    egraph.union(x2, xx, kind, &desc);
    let (best, cost) = egraph.extract_min_cost(x2);
    assert_eq!(cost, 6, "Pow costs 5 + 1 (x) + 0 (2); Mul would cost 8");
    assert!(matches!(best, ENode::Pow(_, _)));
}

#[test]
fn extract_term_builds_the_cheapest_tree() {
    let mut egraph = EGraph::new();
    let x = egraph.add(ENode::Var("x".into()));
    let zero = egraph.add(ENode::Scalar(0));
    let one = egraph.add(ENode::Scalar(1));
    // (x * 0) + 1, and x * 0 = 0 makes it 0 + 1 = ... the class of the sum also equals 1.
    let x_times_zero = egraph.add(ENode::Mul(smallvec::smallvec![x, zero]));
    let sum = egraph.add(ENode::Add(smallvec::smallvec![x_times_zero, one]));
    let (kind, desc) = exact("x*0 = 0");
    egraph.union(x_times_zero, zero, kind, &desc);
    let (kind, desc) = exact("0 + 1 = 1");
    egraph.union(sum, one, kind, &desc);

    let (term, cost) = egraph.extract_term(sum).unwrap();
    assert_eq!(term, ExtractedTerm::Scalar(1));
    assert_eq!(cost, 0);

    // Without the contraction the whole tree is returned, children resolved.
    let mut plain = EGraph::new();
    let x = plain.add(ENode::Var("x".into()));
    let one = plain.add(ENode::Scalar(1));
    let sum = plain.add(ENode::Add(smallvec::smallvec![x, one]));
    let (term, cost) = plain.extract_term(sum).unwrap();
    assert_eq!(
        term,
        ExtractedTerm::Add(vec![
            ExtractedTerm::Var("x".into()),
            ExtractedTerm::Scalar(1)
        ])
    );
    // Add over two operands (3) + x (1); the scalar 1 costs nothing.
    assert_eq!(cost, 3 + 1);
}

#[test]
fn extraction_survives_cycles() {
    // x = x + 0 puts the class of x inside its own child: extraction must pick the finite form.
    let mut egraph = EGraph::new();
    let x = egraph.add(ENode::Var("x".into()));
    let zero = egraph.add(ENode::Scalar(0));
    let x_plus_zero = egraph.add(ENode::Add(smallvec::smallvec![x, zero]));
    let (kind, desc) = exact("x + 0 = x");
    egraph.union(x_plus_zero, x, kind, &desc);

    let (term, cost) = egraph.extract_term(x).unwrap();
    assert_eq!(term, ExtractedTerm::Var("x".into()));
    assert_eq!(cost, 1);
    let (best, cost) = egraph.extract_min_cost(x_plus_zero);
    assert_eq!(best, ENode::Var("x".into()));
    assert_eq!(cost, 1);
}

#[test]
fn saturation_rebuilds_after_each_round() {
    let mut egraph = EGraph::new();
    let a = egraph.add(ENode::Var("a".into()));
    let b = egraph.add(ENode::Var("b".into()));
    let fa = egraph.add(ENode::Neg(a));
    let fb = egraph.add(ENode::Neg(b));
    // One rule that merges a and b once, reporting a single rewrite.
    let merge = RewriteRule {
        name: "a=b",
        tier: RuleTier::TierAContractions,
        apply: Box::new(move |g: &mut EGraph| {
            if g.find(a) != g.find(b) {
                g.union(a, b, EquivalenceKind::strong_exact(true), "a=b");
                1
            } else {
                0
            }
        }),
    };
    let rewrites = egraph.run_saturation(&[merge], 5);
    assert_eq!(rewrites, 1);
    // Congruence followed without the rule asking for it.
    assert_eq!(egraph.find(fa), egraph.find(fb));
}
