use algebra_advanced::quantum::QuantumOperator;
use algebra_core::ExprGraph;
use algebra_engine::SymbolicCalculus;

#[test]
fn test_canonical_commutation_relation() {
    let graph = ExprGraph::new();
    let mode_a = graph.symbols.get_or_intern("mode_0");

    let a = QuantumOperator::annihilation(mode_a);
    let a_dag = QuantumOperator::creation(mode_a);

    // [a, a^\dagger] = 1
    let comm = a.ccr_commutator(&graph, &a_dag).unwrap();
    assert_eq!(comm, graph.integer(1));
}

#[test]
fn test_symbolic_commutator_bracket() {
    use algebra_core::format::{Formatter, UnicodeFormatter};

    let graph = ExprGraph::new();
    let x = graph.symbol("A");
    let y = graph.symbol("B");

    // [A, B] = A*B - B*A
    let comm = QuantumOperator::commutator(&graph, x, y);
    let ab = graph.mul([x, y]);
    let ba = graph.mul([y, x]);
    assert_eq!(comm, graph.sub(ab, ba));

    let s = UnicodeFormatter.format(&graph, comm).unwrap();
    assert!(s.contains("A") && s.contains("B"), "got {s}");
}

#[test]
fn test_braket_gauge_and_dyson_series() {
    use algebra_advanced::quantum::{BraKet, DysonSeries, GaugeTheory};
    use algebra_core::format::{Formatter, UnicodeFormatter};

    let graph = ExprGraph::new();
    let psi_sym = graph.symbols.get_or_intern("psi");
    let phi_sym = graph.symbols.get_or_intern("phi");
    let op_a = graph.symbol("A");
    let fmt = UnicodeFormatter;

    // Bra-Ket <psi| A |phi>
    let bk = BraKet::matrix_element(psi_sym, op_a, phi_sym);
    let bk_expr = bk.to_expr(&graph);
    let bk_str = fmt.format(&graph, bk_expr).unwrap();
    assert!(
        bk_str.contains("psi") && bk_str.contains('A') && bk_str.contains("phi"),
        "got {bk_str}"
    );

    // Gauge Theory Field Strength Tensor F_{\mu\nu}^a = d_mu A_nu - d_nu A_mu
    let a_mu = graph.symbol("A_mu");
    let a_nu = graph.symbol("A_nu");
    let x_mu = graph.symbols.get_or_intern("x_mu");
    let x_nu = graph.symbols.get_or_intern("x_nu");

    let f_tensor = GaugeTheory::field_strength_tensor(&graph, a_mu, a_nu, x_mu, x_nu, None, None);
    assert_ne!(f_tensor, a_mu);
    assert_eq!(
        f_tensor,
        graph.sub(graph.diff(a_nu, x_mu), graph.diff(a_mu, x_nu))
    );

    // Dyson Series first-order term: check -i * \int H_I dt factor
    let h_int = graph.symbol("H_I");
    let t_sym = graph.symbols.get_or_intern("t");
    let dyson = DysonSeries::first_order_term(&graph, h_int, t_sym);
    let d_str = fmt.format(&graph, dyson).unwrap();
    assert!(d_str.contains("H_I") && d_str.contains('i'), "got {d_str}");
}
