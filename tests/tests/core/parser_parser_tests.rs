use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;

#[test]
fn test_parse_latex_and_unicode_inputs() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);

    // Parse raw LaTeX fraction and square root
    let frac = parser.parse("\\frac{a}{b} + \\sqrt{x}").unwrap();
    assert!(!graph.is_empty());
    assert_ne!(frac, graph.integer(0));

    // Parse Unicode math symbols (superscripts, sqrt, dot product)
    let uni = parser.parse("a · x² + √(y) = 0").unwrap();
    assert!(!graph.is_empty());
    assert_ne!(uni, graph.integer(0));
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
