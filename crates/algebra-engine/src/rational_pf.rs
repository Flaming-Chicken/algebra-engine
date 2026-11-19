//! Integration of rational functions by partial fractions over $\mathbb{Q}$, using the complete factorisation of the
//! denominator into irreducible polynomials.
//!
//! $A/B = \text{polynomial} + \sum_i \sum_{k \le m_i} N_{ik} / p_i^k$ with $\deg N_{ik} < \deg p_i$, the $N_{ik}$
//! found by solving a rational linear system. Each term integrates in closed form when $\deg p_i \le 2$:
//!
//! * linear $p = x - a$: $\ln$ for $k = 1$, a power for $k > 1$;
//! * quadratic $p = x^2 + bx + c$ with negative discriminant, $t = x + b/2$, $\delta = c - b^2/4$: the part
//!   $u t / p^k$ is a power of $p$ (or $\tfrac{u}{2}\ln p$), and $w / p^k$ uses the reduction
//!   $J_{k+1} = \frac{t}{2 k \delta p^k} + \frac{2k - 1}{2 k \delta} J_k$ with $J_1 = \frac{1}{\sqrt{\delta}} \arctan
//!   \frac{t}{\sqrt{\delta}}$;
//! * the one irreducible quartic $x^4 + 1$ (simple), by its factorisation over $\mathbb{Q}(\sqrt{2})$.
//!
//! Other irreducible factors of degree three or more need algebraic numbers and are reported as unsupported.
//! (The older Hermite-reduction code overflowed the stack on a repeated irreducible quadratic factor such as
//! `1/(x^2+1)^2`; this path is used first when the denominator has such a factor.)

use crate::poly::factor::factor_rational_polynomial;
use crate::poly::{UniPoly, expr_from_rational};
use algebra_core::{AlgebraError, AlgebraResult, ExprGraph, ExprId, ExprKind, SymbolId};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

type Q = BigRational;

fn q(n: i64) -> Q {
    Q::from_integer(n.into())
}

fn unsupported(why: impl Into<String>) -> AlgebraError {
    AlgebraError::EvaluationError(why.into())
}

fn poly_expr(graph: &ExprGraph, x: ExprId, p: &UniPoly) -> ExprId {
    let mut terms = Vec::new();
    for (k, c) in p.coeffs.iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let power = match k {
            0 => None,
            1 => Some(x),
            _ => Some(graph.pow(x, graph.integer(k as i64))),
        };
        terms.push(match power {
            None => expr_from_rational(graph, c),
            Some(p) if c.is_one() => p,
            Some(p) => graph.mul([expr_from_rational(graph, c), p]),
        });
    }
    match terms.len() {
        0 => graph.integer(0),
        1 => terms[0],
        _ => graph.add(terms),
    }
}

fn sqrt_of(graph: &ExprGraph, r: &Q) -> ExprId {
    let (n, d) = (r.numer(), r.denom());
    let (sn, sd) = (n.sqrt(), d.sqrt());
    if &(&sn * &sn) == n && &(&sd * &sd) == d {
        return expr_from_rational(graph, &Q::new(sn, sd));
    }
    graph.function("sqrt", [expr_from_rational(graph, r)])
}

/// Solve `M v = rhs` over the rationals (any solution of a consistent system; free variables are set to zero).
fn solve_linear(mut m: Vec<Vec<Q>>, mut rhs: Vec<Q>) -> Option<Vec<Q>> {
    let rows = m.len();
    let cols = m.first().map_or(0, Vec::len);
    let mut pivots = Vec::new();
    let mut r = 0;
    for c in 0..cols {
        let Some(p) = (r..rows).find(|&i| !m[i][c].is_zero()) else {
            continue;
        };
        m.swap(r, p);
        rhs.swap(r, p);
        let inv = Q::one() / m[r][c].clone();
        for j in c..cols {
            m[r][j] = &m[r][j] * &inv;
        }
        rhs[r] = &rhs[r] * &inv;
        for i in 0..rows {
            if i != r && !m[i][c].is_zero() {
                let f = m[i][c].clone();
                for j in c..cols {
                    let delta = &f * &m[r][j];
                    m[i][j] -= delta;
                }
                let delta = &f * &rhs[r];
                rhs[i] -= delta;
            }
        }
        pivots.push(c);
        r += 1;
        if r == rows {
            break;
        }
    }
    if (r..rows).any(|i| !rhs[i].is_zero()) {
        return None;
    }
    let mut v = vec![Q::zero(); cols];
    for (i, &c) in pivots.iter().enumerate() {
        v[c] = rhs[i].clone();
    }
    Some(v)
}

/// `numerator / denominator` as polynomials in `wrt`, if both are.
fn as_polynomial_ratio(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
) -> Option<(UniPoly, UniPoly)> {
    let (num, den) = match &graph.get(expr).kind {
        ExprKind::Div(n, d) => (
            UniPoly::from_expr(graph, *n, wrt).ok()?,
            UniPoly::from_expr(graph, *d, wrt).ok()?,
        ),
        ExprKind::Pow(base, exp) => {
            let ExprKind::Number(n) = &graph.get(*exp).kind else {
                return None;
            };
            let e = crate::poly::rational_from_number(n).ok()?;
            if !e.is_integer() || !e.is_negative() {
                return None;
            }
            let k: u32 = (-e).to_integer().try_into().ok()?;
            let b = UniPoly::from_expr(graph, *base, wrt).ok()?;
            (UniPoly::constant(Q::one()), b.pow(k))
        }
        ExprKind::Mul(factors) => {
            // P * Q^(-k): numerator polynomial times an inverse power.
            let mut num = UniPoly::constant(Q::one());
            let mut den = UniPoly::constant(Q::one());
            for &f in factors {
                if let Ok(p) = UniPoly::from_expr(graph, f, wrt) {
                    num = num.mul(&p);
                } else if let Some((n, d)) = as_polynomial_ratio(graph, f, wrt) {
                    num = num.mul(&n);
                    den = den.mul(&d);
                } else {
                    return None;
                }
            }
            (num, den)
        }
        _ => return None,
    };
    (den.degree() >= 1).then_some((num, den))
}

/// Does the denominator of `expr` have a repeated irreducible factor of degree two or more? (The case the older
/// Hermite code could not handle.)
pub(crate) fn has_repeated_nonlinear_factor(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
) -> bool {
    let Some((_, den)) = as_polynomial_ratio(graph, expr, wrt) else {
        return false;
    };
    factor_rational_polynomial(&den)
        .map(|(_, factors)| factors.iter().any(|(p, m)| *m > 1 && p.degree() >= 2))
        .unwrap_or(false)
}

/// `int expr dx` for a rational function with rational coefficients.
pub(crate) fn integrate_rational_function(
    graph: &ExprGraph,
    expr: ExprId,
    wrt: SymbolId,
    x: ExprId,
) -> Option<AlgebraResult<ExprId>> {
    let (num, den) = as_polynomial_ratio(graph, expr, wrt)?;
    Some(integrate_ratio(graph, x, &num, &den))
}

fn integrate_ratio(
    graph: &ExprGraph,
    x: ExprId,
    num: &UniPoly,
    den: &UniPoly,
) -> AlgebraResult<ExprId> {
    // Monic denominator, polynomial part.
    let lead = den.leading_coeff();
    let den = den.monic();
    let num = UniPoly::from_coeffs(num.coeffs.iter().map(|c| c / &lead).collect());
    let (poly_part, remainder) = num.div_rem(&den)?;
    let mut terms: Vec<ExprId> = Vec::new();
    if !poly_part.is_zero() {
        let mut antiderivative = vec![Q::zero()];
        for (k, c) in poly_part.coeffs.iter().enumerate() {
            antiderivative.push(c / q(k as i64 + 1));
        }
        terms.push(poly_expr(graph, x, &UniPoly::from_coeffs(antiderivative)));
    }
    if remainder.is_zero() {
        return Ok(graph.add(terms));
    }

    // Partial fractions over Q.
    let (_, factors) = factor_rational_polynomial(&den)?;
    // Unknowns: for each factor p (degree d) and power k, d coefficients.
    let mut layout: Vec<(usize, u32, usize)> = Vec::new(); // (factor index, k, degree d)
    for (i, (p, m)) in factors.iter().enumerate() {
        for k in 1..=*m {
            layout.push((i, k, p.degree()));
        }
    }
    let n_unknowns: usize = layout.iter().map(|&(_, _, d)| d).sum();
    // Cofactors B / p^k.
    let mut columns: Vec<Vec<Q>> = Vec::new(); // each column is the coefficient vector of x^j * (B / p^k)
    for &(i, k, d) in &layout {
        let (p, _) = &factors[i];
        let divisor = p.pow(k);
        let (cofactor, rem) = den.div_rem(&divisor)?;
        debug_assert!(rem.is_zero());
        for j in 0..d {
            let shifted = cofactor.mul(&UniPoly::from_coeffs({
                let mut c = vec![Q::zero(); j];
                c.push(Q::one());
                c
            }));
            columns.push(shifted.coeffs);
        }
    }
    let size = den.degree();
    let mut matrix = vec![vec![Q::zero(); n_unknowns]; size];
    for (col, coeffs) in columns.iter().enumerate() {
        for (row, c) in coeffs.iter().enumerate().take(size) {
            matrix[row][col] = c.clone();
        }
    }
    let rhs: Vec<Q> = (0..size)
        .map(|r| remainder.coeffs.get(r).cloned().unwrap_or_else(Q::zero))
        .collect();
    let solution = solve_linear(matrix, rhs)
        .ok_or_else(|| unsupported("partial fractions: inconsistent system"))?;

    let mut cursor = 0;
    for &(i, k, d) in &layout {
        let numerator = UniPoly::from_coeffs(solution[cursor..cursor + d].to_vec());
        cursor += d;
        if numerator.is_zero() {
            continue;
        }
        let (p, _) = &factors[i];
        terms.push(integrate_term(graph, x, &numerator, p, k)?);
    }
    Ok(graph.add(terms))
}

/// `int N / p^k dx` for an irreducible monic `p`.
fn integrate_term(
    graph: &ExprGraph,
    x: ExprId,
    n: &UniPoly,
    p: &UniPoly,
    k: u32,
) -> AlgebraResult<ExprId> {
    match p.degree() {
        1 => {
            // p = x + a0; N is a constant c.
            let c = n.coeffs[0].clone();
            let pe = poly_expr(graph, x, p);
            if k == 1 {
                Ok(graph.mul([expr_from_rational(graph, &c), graph.function("ln", [pe])]))
            } else {
                // c * (x + a0)^(1 - k) / (1 - k)
                let power = graph.pow(pe, graph.integer(1 - k as i64));
                Ok(graph.mul([expr_from_rational(graph, &(c / q(1 - k as i64))), power]))
            }
        }
        2 => {
            let (b, c) = (p.coeffs[1].clone(), p.coeffs[0].clone());
            let delta = &c - &b * &b / q(4);
            if !delta.is_positive() {
                return Err(unsupported(
                    "a quadratic factor with real roots should have been factored",
                ));
            }
            let (u, v) = (
                n.coeffs.get(1).cloned().unwrap_or_else(Q::zero),
                n.coeffs[0].clone(),
            );
            let w = &v - &u * &b / q(2);
            let pe = poly_expr(graph, x, p);
            let t = if b.is_zero() {
                x
            } else {
                graph.add([x, expr_from_rational(graph, &(&b / q(2)))])
            };
            let mut parts = Vec::new();
            if !u.is_zero() {
                parts.push(if k == 1 {
                    graph.mul([
                        expr_from_rational(graph, &(u / q(2))),
                        graph.function("ln", [pe]),
                    ])
                } else {
                    graph.mul([
                        expr_from_rational(graph, &(u / (q(2) * q(1 - k as i64)))),
                        graph.pow(pe, graph.integer(1 - k as i64)),
                    ])
                });
            }
            if !w.is_zero() {
                parts.push(graph.mul([
                    expr_from_rational(graph, &w),
                    j_integral(graph, t, p, &delta, k),
                ]));
            }
            Ok(graph.add(parts))
        }
        4 if k == 1 && is_x4_plus_1(p) => integrate_x4_plus_1(graph, x, n),
        d => Err(unsupported(format!(
            "the denominator has an irreducible factor of degree {d} over the rationals; its closed form needs algebraic numbers"
        ))),
    }
}

/// `J_k = int dt / (t^2 + delta)^k`, with `p = t^2 + delta`.
fn j_integral(graph: &ExprGraph, t: ExprId, p: &UniPoly, delta: &Q, k: u32) -> ExprId {
    let _ = p;
    let root = sqrt_of(graph, delta);
    let base = graph.div(graph.function("atan", [graph.div(t, root)]), root);
    if k == 1 {
        return base;
    }
    // J_k = t / (2 (k-1) delta (t^2 + delta)^(k-1)) + (2k - 3) / (2 (k-1) delta) J_(k-1)
    let previous = j_integral(graph, t, p, delta, k - 1);
    let kk = q(k as i64 - 1);
    let first = graph.div(
        t,
        graph.mul([
            expr_from_rational(graph, &(q(2) * &kk * delta)),
            graph.pow(
                graph.add([
                    graph.pow(t, graph.integer(2)),
                    expr_from_rational(graph, delta),
                ]),
                graph.integer(k as i64 - 1),
            ),
        ]),
    );
    let second = graph.mul([
        expr_from_rational(graph, &(q(2 * k as i64 - 3) / (q(2) * &kk * delta))),
        previous,
    ]);
    graph.add([first, second])
}

fn is_x4_plus_1(p: &UniPoly) -> bool {
    p.coeffs.len() == 5
        && p.coeffs[0].is_one()
        && p.coeffs[1].is_zero()
        && p.coeffs[2].is_zero()
        && p.coeffs[3].is_zero()
        && p.coeffs[4].is_one()
}

/// `int (n0 + n1 x + n2 x^2 + n3 x^3) / (x^4 + 1) dx`.
fn integrate_x4_plus_1(graph: &ExprGraph, x: ExprId, n: &UniPoly) -> AlgebraResult<ExprId> {
    let coeff = |i: usize| n.coeffs.get(i).cloned().unwrap_or_else(Q::zero);
    let (n0, n1, n2, n3) = (coeff(0), coeff(1), coeff(2), coeff(3));
    let two = graph.integer(2);
    let root2 = graph.function("sqrt", [two]);
    let x2 = graph.pow(x, two);
    let a = graph.add([x2, graph.mul([root2, x]), graph.integer(1)]); // x^2 + sqrt2 x + 1
    let b = graph.add([x2, graph.neg(graph.mul([root2, x])), graph.integer(1)]); // x^2 - sqrt2 x + 1
    let log_ratio = graph.function("ln", [graph.div(a, b)]);
    let atans = graph.add([
        graph.function(
            "atan",
            [graph.add([graph.mul([root2, x]), graph.integer(1)])],
        ),
        graph.function("atan", [graph.sub(graph.mul([root2, x]), graph.integer(1))]),
    ]);
    // int 1/(x^4+1) = (1/(4 sqrt2)) ln(a/b) + (1/(2 sqrt2)) atans
    let i0 = graph.add([
        graph.div(log_ratio, graph.mul([graph.integer(4), root2])),
        graph.div(atans, graph.mul([graph.integer(2), root2])),
    ]);
    // int x^2/(x^4+1) = -(1/(4 sqrt2)) ln(a/b) + (1/(2 sqrt2)) atans
    let i2 = graph.add([
        graph.neg(graph.div(log_ratio, graph.mul([graph.integer(4), root2]))),
        graph.div(atans, graph.mul([graph.integer(2), root2])),
    ]);
    // int x/(x^4+1) = (1/2) atan(x^2); int x^3/(x^4+1) = (1/4) ln(x^4+1)
    let i1 = graph.mul([graph.rational(1, 2), graph.function("atan", [x2])]);
    let x4p1 = graph.add([graph.pow(x, graph.integer(4)), graph.integer(1)]);
    let i3 = graph.mul([graph.rational(1, 4), graph.function("ln", [x4p1])]);
    let mut terms = Vec::new();
    for (c, i) in [(n0, i0), (n1, i1), (n2, i2), (n3, i3)] {
        if !c.is_zero() {
            terms.push(graph.mul([expr_from_rational(graph, &c), i]));
        }
    }
    Ok(graph.add(terms))
}
