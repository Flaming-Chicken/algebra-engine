use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, LatexFormatter};

#[test]
fn test_latex_formatting() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let two = graph.integer(2);
    let pow = graph.pow(x, two);
    let fn_sin = graph.function("sin", [pow]);

    let latex = LatexFormatter.format(&graph, fn_sin).unwrap();
    assert_eq!(latex, "\\operatorname{sin}\\left({x}^{2}\\right)");
}

// --- Operator precedence and grouping ---------------------------------------

use algebra_core::format::{MathMLFormatter, UnicodeFormatter};
use algebra_core::parser::ExprParser;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use std::collections::HashMap;

fn unicode(graph: &ExprGraph, id: algebra_core::ExprId) -> String {
    UnicodeFormatter.format(graph, id).unwrap()
}

fn latex(graph: &ExprGraph, id: algebra_core::ExprId) -> String {
    LatexFormatter.format(graph, id).unwrap()
}

#[test]
fn product_of_sums_keeps_its_parentheses() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let a = graph.sub(x, graph.integer(3));
    let b = graph.sub(x, graph.integer(2));
    let product = graph.mul([a, b]);

    assert_eq!(unicode(&graph, product), "(x - 3) · (x - 2)");
    assert_eq!(
        latex(&graph, product),
        "\\left(x - 3\\right) \\cdot \\left(x - 2\\right)"
    );
}

#[test]
fn power_of_a_sum_groups_the_base() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let base = graph.add([x, graph.integer(1)]);
    let pow = graph.pow(base, graph.integer(5));

    assert_eq!(unicode(&graph, pow), "(x + 1)^5");
    assert_eq!(latex(&graph, pow), "{\\left(x + 1\\right)}^{5}");
}

#[test]
fn nested_powers_are_unambiguous() {
    let graph = ExprGraph::new();
    let (x, a, b) = (graph.symbol("x"), graph.symbol("a"), graph.symbol("b"));

    // (x^a)^b needs grouping; x^(a^b) is what a bare chain already means.
    let left = graph.pow(graph.pow(x, a), b);
    let right = graph.pow(x, graph.pow(a, b));
    assert_eq!(unicode(&graph, left), "(x^a)^b");
    assert_eq!(unicode(&graph, right), "x^a^b");
}

#[test]
fn fractional_and_negative_exponents_are_grouped() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");

    let root = graph.pow(graph.integer(2), graph.rational(1, 2));
    assert_eq!(unicode(&graph, root), "2^(1/2)");

    let inverse = graph.pow(x, graph.integer(-1));
    assert_eq!(unicode(&graph, inverse), "x^(-1)");

    let negative_base = graph.pow(graph.integer(-2), x);
    assert_eq!(unicode(&graph, negative_base), "(-2)^x");
}

#[test]
fn quotients_group_only_what_needs_it() {
    let graph = ExprGraph::new();
    let (a, b, c, d) = (
        graph.symbol("a"),
        graph.symbol("b"),
        graph.symbol("c"),
        graph.symbol("d"),
    );

    let simple = graph.div(a, b);
    assert_eq!(unicode(&graph, simple), "a / b");

    let sums = graph.div(graph.add([a, b]), graph.sub(c, d));
    assert_eq!(unicode(&graph, sums), "(a + b) / (c - d)");

    let product_below = graph.div(a, graph.mul([b, c]));
    assert_eq!(unicode(&graph, product_below), "a / (b · c)");

    let squared_below = graph.div(a, graph.pow(graph.sub(c, d), graph.integer(2)));
    assert_eq!(unicode(&graph, squared_below), "a / (c - d)^2");
}

#[test]
fn negative_terms_print_as_subtraction() {
    let graph = ExprGraph::new();
    let (x, y) = (graph.symbol("x"), graph.symbol("y"));

    let with_neg = graph.add([x, graph.neg(y)]);
    assert_eq!(unicode(&graph, with_neg), "x - y");
    assert_eq!(latex(&graph, with_neg), "x - y");

    let with_literal = graph.add([x, graph.integer(-2)]);
    assert_eq!(unicode(&graph, with_literal), "x - 2");
}

#[test]
fn subtracting_a_sum_or_a_negative_is_grouped() {
    let graph = ExprGraph::new();
    let (a, b, c) = (graph.symbol("a"), graph.symbol("b"), graph.symbol("c"));

    let minus_sum = graph.sub(a, graph.add([b, c]));
    assert_eq!(unicode(&graph, minus_sum), "a - (b + c)");

    let minus_negative = graph.sub(a, graph.neg(b));
    assert_eq!(unicode(&graph, minus_negative), "a - (-b)");

    let negated_sum = graph.neg(graph.add([a, b]));
    assert_eq!(unicode(&graph, negated_sum), "-(a + b)");
}

#[test]
fn mathml_power_has_exactly_two_children() {
    let graph = ExprGraph::new();
    let x = graph.symbol("x");
    let pow = graph.pow(graph.add([x, graph.integer(1)]), graph.integer(2));

    assert_eq!(
        MathMLFormatter.format(&graph, pow).unwrap(),
        "<msup><mrow><mo>(</mo><mi>x</mi><mo>+</mo><mn>1</mn><mo>)</mo></mrow><mrow><mn>2</mn></mrow></msup>"
    );
}

/// Printing must not change meaning: parse, print, parse again, and compare
/// the two expressions numerically at a fixed point.
#[test]
fn unicode_output_round_trips_through_the_parser() {
    let cases = [
        "(x + 1)^5",
        "(x - 3)*(x - 2)",
        "(a + b)/(c - y)^2 - x",
        "(x^a)^b",
        "x^(a^b)",
        "x^(a + 1)",
        "a - (b - c)",
        "a - (b + c)",
        "-(a + b)*c",
        "a/(b*c)",
        "a/b/c",
        "a/(b/c)",
        "(a/b)^c",
        "2^(1/2) + 3^(-1/2)",
        "(a + b)*(c + x)*(y - 1)",
        "sin(x + y)^2 + cos(x*y)",
        "1/(1 + x^2)",
        "x*(x + 1)^(a - 1)",
        "-x^2",
        "(-x)^2",
        "a - -b",
        "6*(x + 1/3)*(x + 3/2)",
        "exp(x)*(x - 1)",
        "(1/2)/(x - 1) + (-1/2)/(x + 1)",
        "sqrt(x + 1)/(2*sqrt(y))",
    ];
    let bindings: HashMap<String, f64> =
        [("x", 1.3), ("y", 0.7), ("a", 2.1), ("b", 0.4), ("c", 1.9)]
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect();
    let ctx = EvalContext::with_bindings(15, bindings);

    for src in cases {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);
        let original = parser.parse(src).unwrap();
        let printed = unicode(&graph, original);
        let reparsed = parser.parse(&printed).unwrap_or_else(|e| {
            panic!("`{src}` printed as `{printed}`, which does not parse: {e}")
        });

        let want = graph.evalf(original, &ctx).unwrap().to_f64();
        let got = graph.evalf(reparsed, &ctx).unwrap().to_f64();
        assert!(
            (want - got).abs() <= 1e-9 * want.abs().max(1.0),
            "`{src}` printed as `{printed}`: {want} became {got}"
        );
    }
}
