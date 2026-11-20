//! Exact roots of binomial equations `x^n = c` (`solve(x^8 = 1)`, `roots(x^6 - 64)`, `solve(x^4 = -1)`).
//!
//! The roots are `r (cos t + i sin t)` with `r = |c|^(1/n)` and `t = 2 pi k / n` (`c > 0`) or `(2k + 1) pi / n` (`c < 0`).
//! They are written exactly when the angles are multiples of pi/12, which holds for `n` in 3, 4, 6, 8, 12, 24
//! (`c > 0`) and 3, 4, 6, 12 (`c < 0`); other degrees keep the general solver.

use crate::poly::UniPoly;
use crate::session::UraeSession;
use algebra_core::parser::ExprParser;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

/// The integer `n`-th root of a non-negative integer when it is exact.
fn exact_root(value: &BigInt, n: u32) -> Option<BigInt> {
    if value.is_zero() || value.is_one() {
        return Some(value.clone());
    }
    let root = value.nth_root(n);
    (root.pow(n) == *value).then_some(root)
}

fn call_arguments(input: &str) -> Option<(String, Vec<String>)> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    let (mut depth, mut current, mut args) = (0i32, String::new(), Vec::new());
    for (i, c) in t[open + 1..t.len() - 1].char_indices() {
        let _ = i;
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            args.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    args.push(current);
    Some((
        name,
        args.into_iter().map(|a| a.trim().to_string()).collect(),
    ))
}

/// `Some(result)` when `input` solves a binomial equation of a supported degree.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let (name, args) = call_arguments(input)?;
    if name != "solve" && name != "roots" {
        return None;
    }
    let equation = args.first()?;
    let difference = match equation.split_once('=') {
        Some((l, r)) if !r.contains('=') && !l.ends_with(['<', '>', '!']) => {
            format!("({l}) - ({r})")
        }
        _ => equation.clone(),
    };
    let (expr, symbol) = {
        let graph = &session.graph;
        let id = ExprParser::new(graph).parse(&difference).ok()?;
        let mut symbols = std::collections::BTreeSet::new();
        crate::poly::collect_expr_symbols(graph, id, &mut symbols);
        let names: Vec<_> = symbols
            .into_iter()
            .filter(|s| {
                !matches!(
                    graph.symbols.resolve(*s).as_deref(),
                    Some("pi" | "π" | "e" | "i")
                )
            })
            .collect();
        let symbol = match (names.as_slice(), args.get(1)) {
            ([one], None) => *one,
            (_, Some(v)) => graph.symbols.get_or_intern(v),
            _ => return None,
        };
        (id, symbol)
    };
    let variable = session.graph.symbols.resolve(symbol)?;
    let poly = UniPoly::from_expr(&session.graph, expr, symbol).ok()?;
    // Exactly two nonzero coefficients: a x^n + b with n >= 3.
    let n = poly.degree();
    let nonzero: Vec<usize> = poly
        .coeffs
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_zero())
        .map(|(i, _)| i)
        .collect();
    if n < 3 || nonzero != [0, n] {
        return None;
    }
    let c: BigRational = -poly.coeffs[0].clone() / poly.coeffs[n].clone();
    let negative = c.is_negative();
    let allowed: &[usize] = if negative {
        &[3, 4, 6, 12]
    } else {
        &[3, 4, 6, 8, 12, 24]
    };
    if !allowed.contains(&n) {
        return None;
    }
    let magnitude = c.abs();
    let n32 = n as u32;
    // The radius |c|^(1/n): exact when numerator and denominator are n-th powers.
    let radius = match (
        exact_root(magnitude.numer(), n32),
        exact_root(magnitude.denom(), n32),
    ) {
        (Some(p), Some(q)) => {
            if q.is_one() {
                p.to_string()
            } else {
                format!("{p}/{q}")
            }
        }
        _ => format!("({}/{})^(1/{n})", magnitude.numer(), magnitude.denom()),
    };
    let mut roots = Vec::new();
    for k in 0..n {
        let angle = if negative {
            format!("(2*{k} + 1)*π/{n}")
        } else {
            format!("2*{k}*π/{n}")
        };
        let text = format!("({radius})*(cos({angle}) + i*sin({angle}))");
        match session.eval(&text) {
            Ok(r) => roots.push(r),
            Err(e) => return Some(Err(e)),
        }
    }
    Some(Ok(format!(
        "Solutions for {variable}: [{}]",
        roots.join(", ")
    )))
}
