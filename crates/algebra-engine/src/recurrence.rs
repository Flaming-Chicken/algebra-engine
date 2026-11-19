//! Linear recurrences with constant coefficients: `a(n+2) = a(n+1) + a(n)` with initial values gives a closed form.
//!
//! Method: the characteristic polynomial's rational roots (with multiplicities) and a remaining quadratic give the
//! homogeneous solutions `n^j r^n`; a forcing term `P(n) b^n` (rational `b`) gets a particular solution by undetermined
//! coefficients (with the resonance factor `n^mu` when `b` is a root); the constants are fixed by the initial values.
//! Every result is checked against the recurrence on eight terms before it is returned.

use crate::numeric::try_eval_exact;
use crate::simplify::{AlgebraicTransformations, SymbolicSimplifier};
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_core::parser::ExprParser;
use algebra_core::{Constant, ExprGraph, ExprId, ExprKind, SymbolId};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::HashMap;

type Q = BigRational;

fn q(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

fn exact(graph: &ExprGraph, id: ExprId) -> Option<Q> {
    try_eval_exact(graph, graph.simplify(id), &HashMap::new())
        .ok()
        .flatten()
}

/// `p(x)` for coefficients lowest degree first.
fn eval_poly(coeffs: &[Q], x: &Q) -> Q {
    coeffs.iter().rev().fold(Q::zero(), |acc, c| acc * x + c)
}

/// Divide by `(x - r)` (synthetic division); the remainder must be zero.
fn divide_root(coeffs: &[Q], r: &Q) -> Vec<Q> {
    let n = coeffs.len() - 1;
    let mut quotient = vec![Q::zero(); n];
    let mut carry = Q::zero();
    for i in (0..=n).rev() {
        let value = &coeffs[i] + &carry * r;
        if i > 0 {
            quotient[i - 1] = value.clone();
        }
        carry = value;
    }
    quotient
}

fn divisors(n: u64) -> Vec<u64> {
    crate::numbertheory::divisors_of(n.max(1))
}

/// Rational roots with multiplicities, and the polynomial that is left.
fn rational_roots(coeffs: &[Q]) -> (Vec<(Q, usize)>, Vec<Q>) {
    let mut poly = coeffs.to_vec();
    let mut roots: Vec<(Q, usize)> = Vec::new();
    loop {
        if poly.len() <= 1 {
            break;
        }
        // Clear denominators: integer coefficients.
        let lcm = poly.iter().fold(BigInt::one(), |l, c| {
            use num::Integer as _;
            l.lcm(c.denom())
        });
        let ints: Vec<BigInt> = poly
            .iter()
            .map(|c| (c * Q::from_integer(lcm.clone())).to_integer())
            .collect();
        if ints[0].is_zero() {
            // x = 0 is a root.
            let zero = Q::zero();
            poly = divide_root(&poly, &zero);
            match roots.iter_mut().find(|(r, _)| *r == zero) {
                Some(entry) => entry.1 += 1,
                None => roots.push((zero, 1)),
            }
            continue;
        }
        let (Some(a0), Some(an)) = (ints[0].abs().to_u64(), ints[ints.len() - 1].abs().to_u64())
        else {
            break;
        };
        let mut found = None;
        'search: for p in divisors(a0) {
            for d in divisors(an) {
                for sign in [1i64, -1] {
                    let candidate = Q::new(BigInt::from(sign * p as i64), BigInt::from(d as i64));
                    if eval_poly(&poly, &candidate).is_zero() {
                        found = Some(candidate);
                        break 'search;
                    }
                }
            }
        }
        match found {
            Some(r) => {
                poly = divide_root(&poly, &r);
                match roots.iter_mut().find(|(x, _)| *x == r) {
                    Some(entry) => entry.1 += 1,
                    None => roots.push((r, 1)),
                }
            }
            None => break,
        }
    }
    (roots, poly)
}

fn rational_expr(graph: &ExprGraph, q: &Q) -> ExprId {
    crate::poly::expr_from_rational(graph, q)
}

/// The terms of a sum with nested sums flattened (and `-(a + b)` distributed).
fn flatten_terms(graph: &ExprGraph, id: ExprId) -> Vec<ExprId> {
    match graph.get(id).kind.clone() {
        ExprKind::Add(ts) => ts.iter().flat_map(|&t| flatten_terms(graph, t)).collect(),
        ExprKind::Sub(a, b) => {
            let mut out = flatten_terms(graph, a);
            out.extend(flatten_terms(graph, b).into_iter().map(|t| graph.neg(t)));
            out
        }
        ExprKind::Neg(a) if matches!(graph.get(a).kind, ExprKind::Add(_) | ExprKind::Sub(..)) => {
            flatten_terms(graph, a)
                .into_iter()
                .map(|t| graph.neg(t))
                .collect()
        }
        _ => vec![id],
    }
}

/// The factors of a term with negations and nested products flattened.
fn flatten_factors(graph: &ExprGraph, id: ExprId) -> Vec<ExprId> {
    match graph.get(id).kind.clone() {
        ExprKind::Neg(a) => {
            let mut out = vec![graph.integer(-1)];
            out.extend(flatten_factors(graph, a));
            out
        }
        ExprKind::Mul(fs) => fs.iter().flat_map(|&f| flatten_factors(graph, f)).collect(),
        _ => vec![id],
    }
}

/// A forcing term `c n^k b^n`.
struct Term {
    coefficient: Q,
    power: u32,
    base: Q,
}

/// Parse a forcing function into terms `c n^k b^n` with rational `c`, integer `k` and rational `b`.
fn forcing_terms(graph: &ExprGraph, expr: ExprId, n: SymbolId) -> Result<Vec<Term>, String> {
    let expanded = AlgebraicTransformations::expand(graph, expr);
    let terms: Vec<ExprId> = flatten_terms(graph, expanded);
    let n_expr = graph.symbol(&graph.symbols.resolve(n).unwrap_or_default());
    let mut out = Vec::new();
    for term in terms {
        if exact(graph, term).is_some_and(|v| v.is_zero()) {
            continue;
        }
        let factors: Vec<ExprId> = flatten_factors(graph, term);
        let (mut coefficient, mut power, mut base) = (Q::one(), 0u32, Q::one());
        for f in factors {
            if !graph.has_symbol(f, n) {
                coefficient *=
                    exact(graph, f).ok_or("the forcing term has a non-rational coefficient")?;
                continue;
            }
            if f == n_expr {
                power += 1;
                continue;
            }
            match graph.get(f).kind.clone() {
                ExprKind::Pow(b, e) if b == n_expr => {
                    let k = exact(graph, e)
                        .filter(|q| q.is_integer() && !q.is_negative())
                        .and_then(|q| q.to_integer().to_u32())
                        .ok_or("only non-negative integer powers of n are supported in the forcing term")?;
                    power += k;
                }
                ExprKind::Pow(b, e) if e == n_expr && !graph.has_symbol(b, n) => {
                    base *=
                        exact(graph, b).ok_or("the base of the forcing term must be rational")?;
                }
                _ => {
                    return Err(
                        "the forcing term must be a sum of terms c n^k b^n with rational c and b"
                            .into(),
                    );
                }
            }
        }
        out.push(Term {
            coefficient,
            power,
            base,
        });
    }
    Ok(out)
}

/// Solve a small square linear system over the rationals; `None` if singular.
fn solve_q(mut rows: Vec<Vec<Q>>, n: usize) -> Option<Vec<Q>> {
    for col in 0..n {
        let pivot = (col..n).find(|&r| !rows[r][col].is_zero())?;
        rows.swap(col, pivot);
        let p = rows[col][col].clone();
        for c in col..=n {
            rows[col][c] = &rows[col][c] / &p;
        }
        for r in 0..n {
            if r != col && !rows[r][col].is_zero() {
                let f = rows[r][col].clone();
                for c in col..=n {
                    let delta = &f * &rows[col][c];
                    rows[r][c] -= delta;
                }
            }
        }
    }
    Some((0..n).map(|r| rows[r][n].clone()).collect())
}

/// Closed form of the recurrence `equation` (an equation in `a(n + k)` terms) with the given initial values
/// `(index, value)`.
pub fn solve(
    graph: &ExprGraph,
    equation: &str,
    initial: &[(i64, String)],
) -> Result<(String, String), String> {
    let parser = ExprParser::new(graph);
    let eq = parser.parse(equation).map_err(|e| e.to_string())?;
    let ExprKind::Relational { lhs, rhs, .. } = graph.get(eq).kind.clone() else {
        return Err("a recurrence such as `a(n+2) = a(n+1) + a(n)` is needed".into());
    };
    let f = AlgebraicTransformations::expand(graph, graph.simplify(graph.sub(lhs, rhs)));
    // The sequence function and the index variable.
    fn find_call(graph: &ExprGraph, id: ExprId) -> Option<(String, ExprId)> {
        match graph.get(id).kind.clone() {
            ExprKind::Function { name, args } if args.len() == 1 => {
                let n = graph.symbols.resolve(name)?;
                if matches!(
                    n.as_str(),
                    "exp" | "sin" | "cos" | "ln" | "sqrt" | "factorial"
                ) {
                    return find_call(graph, args[0]);
                }
                Some((n, args[0]))
            }
            ExprKind::Add(ts) | ExprKind::Mul(ts) => ts.iter().find_map(|&t| find_call(graph, t)),
            ExprKind::Sub(a, b) => find_call(graph, a).or_else(|| find_call(graph, b)),
            ExprKind::Neg(a) => find_call(graph, a),
            ExprKind::Pow(a, _) => find_call(graph, a),
            _ => None,
        }
    }
    let (name, sample_arg) =
        find_call(graph, f).ok_or("no sequence term such as a(n+1) was found")?;
    let mut symbols = std::collections::BTreeSet::new();
    crate::poly::collect_expr_symbols(graph, sample_arg, &mut symbols);
    let n = *symbols
        .iter()
        .next()
        .ok_or("the index must be a variable")?;
    let n_expr = graph.symbol(&graph.symbols.resolve(n).unwrap_or_default());

    // Split into coefficients of a(n + s) and the rest.
    let terms: Vec<ExprId> = flatten_terms(graph, f);
    let mut by_shift: HashMap<i64, Q> = HashMap::new();
    let mut rest: Vec<ExprId> = Vec::new();
    for term in terms {
        let factors: Vec<ExprId> = flatten_factors(graph, term);
        let mut shift = None;
        let mut coefficient = Q::one();
        let mut is_sequence_term = false;
        for factor in &factors {
            match graph.get(*factor).kind.clone() {
                ExprKind::Function { name: fname, args }
                    if args.len() == 1
                        && graph.symbols.resolve(fname).as_deref() == Some(name.as_str()) =>
                {
                    let s = exact(graph, graph.sub(args[0], n_expr))
                        .filter(|q| q.is_integer())
                        .and_then(|q| q.to_integer().to_i64())
                        .ok_or("the arguments must be n plus an integer")?;
                    shift = Some(s);
                    is_sequence_term = true;
                }
                _ => match exact(graph, *factor) {
                    Some(c) => coefficient *= c,
                    None => {
                        coefficient = Q::zero();
                        is_sequence_term = false;
                        shift = None;
                        break;
                    }
                },
            }
        }
        if is_sequence_term && let Some(s) = shift {
            *by_shift.entry(s).or_insert_with(Q::zero) += coefficient;
        } else {
            rest.push(term);
        }
    }
    let s0 = *by_shift.keys().min().ok_or("no sequence terms")?;
    let s1 = *by_shift.keys().max().ok_or("no sequence terms")?;
    let order = (s1 - s0) as usize;
    if order == 0 {
        return Err("the recurrence has order 0".into());
    }
    let c: Vec<Q> = (0..=order as i64)
        .map(|j| by_shift.get(&(s0 + j)).cloned().unwrap_or_else(Q::zero))
        .collect();
    if c[order].is_zero() || c[0].is_zero() {
        return Err("the first and last coefficients must be non-zero".into());
    }
    if order > 6 {
        return Err("recurrences of order above 6 are not supported".into());
    }
    if initial.len() < order {
        return Err(format!(
            "{order} initial values are needed for a recurrence of order {order}, got {}",
            initial.len()
        ));
    }

    // Characteristic polynomial and its roots.
    let (rational, remainder) = rational_roots(&c);
    let mut basis: Vec<ExprId> = Vec::new(); // functions of n
    let multiplicity: Vec<(Q, usize)> = rational.clone();
    for (r, m) in &rational {
        for j in 0..*m {
            let power = if r.is_one() {
                graph.integer(1)
            } else {
                graph.pow(rational_expr(graph, r), n_expr)
            };
            basis.push(if j == 0 {
                power
            } else {
                graph.mul([graph.pow(n_expr, graph.integer(j as i64)), power])
            });
        }
    }
    match remainder.len() {
        1 => {}
        3 => {
            let (a, b, cc) = (&remainder[2], &remainder[1], &remainder[0]);
            let disc = b * b - q(4) * a * cc;
            let two_a = q(2) * a;
            let sqrt_d = if disc.is_negative() {
                graph.mul([
                    graph.constant(Constant::I),
                    graph.function("sqrt", [rational_expr(graph, &-disc.clone())]),
                ])
            } else {
                graph.function("sqrt", [rational_expr(graph, &disc)])
            };
            for sign in [1i64, -1] {
                let numerator = graph.add([
                    rational_expr(graph, &-b.clone()),
                    graph.mul([graph.integer(sign), sqrt_d]),
                ]);
                let root = graph.div(numerator, rational_expr(graph, &two_a));
                basis.push(graph.pow(root, n_expr));
            }
        }
        _ => {
            return Err(
                "the characteristic polynomial has an irreducible factor of degree above 2".into(),
            );
        }
    }
    if basis.len() != order {
        return Err(
            "internal error: the number of homogeneous solutions differs from the order".into(),
        );
    }

    // Particular solution for the forcing term h(t) = -(rest)(n = t - s0).
    let mut particular = graph.integer(0);
    if !rest.is_empty() {
        let g = graph.add(rest.clone());
        let h = graph.neg(graph.substitute(g, n, graph.sub(n_expr, graph.integer(s0))));
        for term in forcing_terms(graph, h, n)? {
            let b = term.base.clone();
            let mu = multiplicity
                .iter()
                .find(|(r, _)| *r == b)
                .map(|(_, m)| *m)
                .unwrap_or(0);
            let degree = term.power as usize;
            // Unknowns u_0..u_degree: sum_j c_j b^j (t + j)^mu sum_i u_i (t + j)^i = coefficient t^power for t = 1..degree+1.
            let mut rows = vec![vec![Q::zero(); degree + 2]; degree + 1];
            for (row, t) in (1..=(degree as i64 + 1)).enumerate() {
                for i in 0..=degree {
                    let mut total = Q::zero();
                    for (j, cj) in c.iter().enumerate() {
                        let at = q(t + j as i64);
                        let mut bj = Q::one();
                        for _ in 0..j {
                            bj *= &b;
                        }
                        let mut shifted_power = Q::one();
                        for _ in 0..(mu + i) {
                            shifted_power *= &at;
                        }
                        total += cj * bj * shifted_power;
                    }
                    rows[row][i] = total;
                }
                let mut target = term.coefficient.clone();
                for _ in 0..term.power {
                    target *= q(t);
                }
                rows[row][degree + 1] = target;
            }
            let u = solve_q(rows, degree + 1)
                .ok_or("the undetermined coefficients have no solution")?;
            let mut poly_terms = Vec::new();
            for (i, ui) in u.iter().enumerate() {
                if ui.is_zero() {
                    continue;
                }
                poly_terms.push(graph.mul([
                    rational_expr(graph, ui),
                    graph.pow(n_expr, graph.integer((mu + i) as i64)),
                ]));
            }
            if poly_terms.is_empty() {
                continue;
            }
            let poly = graph.add(poly_terms);
            let geometric = if b.is_one() {
                poly
            } else {
                graph.mul([poly, graph.pow(rational_expr(graph, &b), n_expr)])
            };
            particular = graph.add([particular, geometric]);
        }
    }

    // Fit the constants to the initial values.
    let k = order;
    let initial = &initial[..k];
    let mut matrix = Vec::with_capacity(k * k);
    let mut rhs = Vec::with_capacity(k);
    for (index, value) in initial {
        let at = graph.integer(*index);
        for &b in &basis {
            matrix.push(graph.simplify(graph.substitute(b, n, at)));
        }
        let v = parser.parse(value).map_err(|e| e.to_string())?;
        let p = graph.substitute(particular, n, at);
        rhs.push(graph.simplify(graph.sub(v, p)));
    }
    let m = crate::matrix::SymbolicMatrix::new(k, k, matrix).map_err(|e| e.to_string())?;
    let inverse = m
        .inverse(graph)
        .map_err(|e| format!("the initial values do not determine the solution ({e})"))?;
    let mut solution = particular;
    for (row, &basis_fn) in basis.iter().enumerate() {
        let mut constant = Vec::new();
        for (col, &r) in rhs.iter().enumerate() {
            constant.push(graph.mul([inverse.elements[row * k + col], r]));
        }
        let constant = graph.simplify(graph.add(constant));
        // Constants with radicals (the golden ratio) in their normal form: 1/sqrt(5), not a fraction of nested sums.
        let constant = match crate::radical::evaluate(graph, constant) {
            Some(radical) => radical.to_expr(graph),
            None => constant,
        };
        solution = graph.add([solution, graph.mul([constant, basis_fn])]);
    }
    let solution = graph.simplify(solution);

    // Verify against the recurrence on eight terms.
    let ctx = crate::numeric::EvalContext::default();
    use crate::numeric::NumericalEval as _;
    let value_at = |t: i64| -> Result<f64, String> {
        graph
            .evalf(graph.substitute(solution, n, graph.integer(t)), &ctx)
            .map(|v| v.to_f64())
            .map_err(|e| e.to_string())
    };
    let first = initial.iter().map(|(i, _)| *i).min().unwrap_or(0);
    for t in first..first + 8 {
        let lhs: f64 = (0..=order as i64)
            .map(|j| c[j as usize].to_f64().unwrap_or(0.0) * value_at(t + j).unwrap_or(f64::NAN))
            .sum();
        let forcing = if rest.is_empty() {
            0.0
        } else {
            let g = graph.add(rest.clone());
            let at = graph.integer(t - s0);
            -graph
                .evalf(graph.substitute(g, n, at), &ctx)
                .map(|v| v.to_f64())
                .map_err(|e| e.to_string())?
        };
        let scale = lhs.abs().max(forcing.abs()).max(1.0);
        let residual = (lhs - forcing).abs();
        if residual.is_nan() || residual > 1e-6 * scale {
            return Err(
                "internal check failed: the closed form does not satisfy the recurrence".into(),
            );
        }
    }
    let text = UnicodeFormatter
        .format(graph, solution)
        .map_err(|e| e.to_string())?;
    Ok((name, text))
}
