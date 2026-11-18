//! Limits and Taylor series against known values (found the `(1+x)^(1/x)` NaN and an i64
//! overflow in constant folding of high-order series).

use algebra_core::ExprGraph;
use algebra_core::parser::ExprParser;
use algebra_engine::calculus::SymbolicCalculus;
use algebra_engine::numeric::{EvalContext, NumericalEval};
use std::collections::HashMap;

#[test]
fn limits_and_series_agree_with_known_values() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let x_sym = graph.symbols.get_or_intern("x");
    let cases: &[(&str, &str, f64)] = &[
        ("(1-cos(x))/x^2", "0", 0.5),
        ("(exp(x)-1-x)/x^2", "0", 0.5),
        ("(tan(x)-sin(x))/x^3", "0", 0.5),
        ("(sin(x)-x)/x^3", "0", -1.0 / 6.0),
        ("x*ln(x)", "0", 0.0),
        ("(1+x)^(1/x)", "0", std::f64::consts::E),
        ("x^x", "0", 1.0),
        ("(x^2-4)/(x-2)", "2", 4.0),
        ("(sqrt(x+1)-1)/x", "0", 0.5),
        ("(exp(x)-exp(-x))/(2*x)", "0", 1.0),
        ("(x^3-1)/(x-1)", "1", 3.0),
        ("(ln(1+x)-x)/x^2", "0", -0.5),
        ("(atan(x)-x)/x^3", "0", -1.0 / 3.0),
        ("x/sin(x)", "0", 1.0),
        ("(2^x-1)/x", "0", std::f64::consts::LN_2),
        ("(cosh(x)-1)/x^2", "0", 0.5),
        ("(1+2*x)^(1/x)", "0", 2.0_f64.exp()),
        ("(1+x)^(3/x)", "0", 3.0_f64.exp()),
    ];
    for (e, p, want) in cases {
        let ex = parser.parse(e).unwrap();
        let pt = parser.parse(p).unwrap();
        match graph.limit(ex, x_sym, pt) {
            Ok(l) => match graph.evalf(l, &EvalContext::default()) {
                Ok(v) => {
                    let v = v.to_f64();
                    assert!((v - want).abs() < 1e-8, "lim {e} at {p} = {v}, want {want}");
                }
                Err(err) => panic!("lim {e} at {p} is not numeric: {err:?}"),
            },
            Err(err) => panic!("lim {e} at {p} failed: {err:?}"),
        }
    }
    // Series: compare the truncated series with the function near 0.
    let fns = ["exp(x)", "sin(x)", "cos(x)", "1/(1-x)", "ln(1+x)", "atan(x)", "sqrt(1+x)", "tan(x)", "exp(x)*cos(x)", "sinh(x)", "1/(1+x^2)"];
    for f in fns {
        let ex = parser.parse(f).unwrap();
        let zero = parser.parse("0").unwrap();
        let s = graph.taylor_series(ex, x_sym, zero, 8);
        {
            {
                let mut worst: f64 = 0.0; // truncation error of order 8 at |x| <= 0.1 is far below 1e-8
                for xv in [0.05, -0.05, 0.1] {
                    let mut b = HashMap::new();
                    b.insert("x".to_string(), xv);
                    let ctx = EvalContext::with_bindings(15, b);
                    let a = graph.evalf(s, &ctx).map(|v| v.to_f64());
                    let t = graph.evalf(ex, &ctx).map(|v| v.to_f64());
                    if let (Ok(a), Ok(t)) = (a, t) { worst = worst.max((a - t).abs()); } else { worst = f64::NAN; }
                }
                assert!(worst < 1e-8, "series of {f}: worst error {worst:e}");
            }
        }
    }
}
