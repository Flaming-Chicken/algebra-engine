use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;

#[test]
fn test_parse_latex_and_unicode_inputs() {
    use algebra_core::{ExprKind, RelOp};
    use algebra_engine::numeric::{EvalContext, NumericalEval};
    use std::collections::HashMap;

    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);

    // Parse raw LaTeX fraction and square root: evaluate numerically at (a=6, b=3, x=9) => 6/3 + 3 = 5.0
    let frac = parser.parse("\\frac{a}{b} + \\sqrt{x}").unwrap();
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), 6.0);
    vars.insert("b".to_string(), 3.0);
    vars.insert("x".to_string(), 9.0);
    let ctx = EvalContext::with_bindings(53, vars);
    let frac_val = NumericalEval::evalf(&graph, frac, &ctx).unwrap().to_f64();
    assert!(
        (frac_val - 5.0).abs() < 1e-12,
        "Expected 6/3 + sqrt(9) = 5.0, got {frac_val}"
    );

    // Parse Unicode math symbols (superscripts, sqrt, dot product) and verify LHS evaluation
    let uni = parser.parse("a · x² + √(y) = 0").unwrap();
    match &graph.get(uni).kind {
        ExprKind::Relational {
            op: RelOp::Equal,
            lhs,
            rhs,
        } => {
            let mut uvars = HashMap::new();
            uvars.insert("a".to_string(), 2.0);
            uvars.insert("x".to_string(), 3.0);
            uvars.insert("y".to_string(), 16.0);
            let uctx = EvalContext::with_bindings(53, uvars);
            let lhs_val = NumericalEval::evalf(&graph, *lhs, &uctx).unwrap().to_f64();
            let rhs_val = NumericalEval::evalf(&graph, *rhs, &uctx).unwrap().to_f64();
            assert!(
                (lhs_val - 22.0).abs() < 1e-12,
                "Expected 2*9 + 4 = 22, got {lhs_val}"
            );
            assert!(rhs_val.abs() < 1e-12);
        }
        other => panic!("Expected Equal relation, got {other:?}"),
    }
}

#[test]
fn test_permissive_intent_parsing() {
    use algebra_core::parser::{PermissiveIntent, parse_permissive_intent};

    // 1. Solve commands
    assert_eq!(
        parse_permissive_intent("given g(x) = 3 find x"),
        Some(PermissiveIntent::Solve {
            equation: "g(x) = 3".to_string(),
            variable: "x".to_string()
        })
    );

    // 2. Differentiate commands
    assert_eq!(
        parse_permissive_intent("differentiate x^3 wrt x"),
        Some(PermissiveIntent::Differentiate {
            expression: "x^3".to_string(),
            variable: Some("x".to_string()),
            is_total: false,
        })
    );

    // 3. Integrate commands
    assert_eq!(
        parse_permissive_intent("integral of cos(x) wrt x"),
        Some(PermissiveIntent::Integrate {
            expression: "cos(x)".to_string(),
            variable: Some("x".to_string()),
            lower: None,
            upper: None,
        })
    );

    // 4. Distribution commands
    assert_eq!(
        parse_permissive_intent("X ~ Normal(0, 1)"),
        Some(PermissiveIntent::Distribution {
            name: "X".to_string(),
            dist_type: "Normal".to_string(),
            params: vec!["0".to_string(), "1".to_string()],
        })
    );

    // 5. Comparison / Equivalence commands
    assert_eq!(
        parse_permissive_intent("are f(x) and g(x) equal?"),
        Some(PermissiveIntent::Compare {
            left: "f(x)".to_string(),
            right: "g(x)".to_string(),
            comparison_type: "equal".to_string(),
        })
    );

    // 6. Substitution / Alteration commands
    assert_eq!(
        parse_permissive_intent("evaluate x^2 + 3 at x = 2"),
        Some(PermissiveIntent::Substitute {
            expression: "x^2 + 3".to_string(),
            variable: "x".to_string(),
            value: "2".to_string(),
        })
    );

    // 7. Limit commands
    assert_eq!(
        parse_permissive_intent("limit of sin(x)/x as x -> 0"),
        Some(PermissiveIntent::Limit {
            expression: "sin(x)/x".to_string(),
            variable: "x".to_string(),
            point: "0".to_string(),
            direction: None,
        })
    );
    assert_eq!(
        parse_permissive_intent("lim_{x->0+} 1/x"),
        Some(PermissiveIntent::Limit {
            expression: "1/x".to_string(),
            variable: "x".to_string(),
            point: "0".to_string(),
            direction: Some("+".to_string()),
        })
    );

    // 8. Summation commands
    assert_eq!(
        parse_permissive_intent("sum of k from k=1 to n"),
        Some(PermissiveIntent::Sum {
            expression: "k".to_string(),
            variable: "k".to_string(),
            lower: "1".to_string(),
            upper: "n".to_string(),
        })
    );
}
