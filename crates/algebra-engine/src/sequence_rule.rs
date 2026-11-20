//! Guessing the rule of a sequence from its first terms: `next_term 1, 4, 9, 16`, `find_pattern 1, 1, 2, 3, 5, 8`.
//!
//! Two families are tried, in this order: a polynomial in `n` (the finite differences become zero with at least two
//! confirming terms), and a linear recurrence with constant coefficients of order at most 4 (Berlekamp-Massey over the
//! rationals, with at least two confirming terms). Terms are numbered from 1.

use algebra_core::{ExprGraph, ExprId};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

type Q = BigRational;

fn q_text(r: &Q) -> String {
    if r.is_integer() {
        r.numer().to_string()
    } else {
        format!("{}/{}", r.numer(), r.denom())
    }
}

fn list(values: &[Q]) -> String {
    values.iter().map(q_text).collect::<Vec<_>>().join(", ")
}

/// The coefficients (lowest degree first) of the polynomial through the points `(1, a_1), (2, a_2), ...`, by solving
/// the Vandermonde system exactly.
fn polynomial_through(values: &[Q]) -> Option<Vec<Q>> {
    let n = values.len();
    let mut rows: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            let x = Q::from_integer(BigInt::from(i as i64 + 1));
            let mut row = Vec::with_capacity(n + 1);
            let mut power = Q::one();
            for _ in 0..n {
                row.push(power.clone());
                power *= &x;
            }
            row.push(values[i].clone());
            row
        })
        .collect();
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

fn eval(coeffs: &[Q], x: &Q) -> Q {
    coeffs.iter().rev().fold(Q::zero(), |acc, c| acc * x + c)
}

/// The least degree `d` whose `d`-th finite differences are constant over at least two entries (three for a constant
/// sequence): the polynomial has one more term than it needs, which confirms it.
fn polynomial_degree(values: &[Q]) -> Option<usize> {
    let mut row = values.to_vec();
    for degree in 0.. {
        let needed = if degree == 0 { 3 } else { 2 };
        if row.len() < needed {
            return None;
        }
        if row.windows(2).all(|w| w[0] == w[1]) {
            return Some(degree);
        }
        row = row.windows(2).map(|w| &w[1] - &w[0]).collect();
    }
    None
}

/// Berlekamp-Massey over the rationals: the shortest `c` with `a_n = sum_{i=1}^{r} c_i a_{n-i}` for all `n > r`.
fn berlekamp_massey(a: &[Q]) -> Vec<Q> {
    let mut c: Vec<Q> = vec![Q::one()];
    let mut b: Vec<Q> = vec![Q::one()];
    let (mut l, mut m, mut bb) = (0usize, 1usize, Q::one());
    for n in 0..a.len() {
        let mut d = a[n].clone();
        for i in 1..=l {
            if i < c.len() {
                d += &c[i] * &a[n - i];
            }
        }
        if d.is_zero() {
            m += 1;
        } else if 2 * l <= n {
            let t = c.clone();
            let coefficient = &d / &bb;
            if c.len() < b.len() + m {
                c.resize(b.len() + m, Q::zero());
            }
            for (i, bi) in b.iter().enumerate() {
                c[i + m] -= &coefficient * bi;
            }
            l = n + 1 - l;
            b = t;
            bb = d;
            m = 1;
        } else {
            let coefficient = &d / &bb;
            if c.len() < b.len() + m {
                c.resize(b.len() + m, Q::zero());
            }
            for (i, bi) in b.iter().enumerate() {
                c[i + m] -= &coefficient * bi;
            }
            m += 1;
        }
    }
    c.resize(l + 1, Q::zero());
    // a_n + c_1 a_{n-1} + ... + c_l a_{n-l} = 0, so the recurrence coefficients are the negatives.
    c[1..].iter().map(|x| -x).collect()
}

/// The rule of the sequence `terms` (numbered from 1) and its next terms, as text.
pub fn describe(graph: &ExprGraph, terms: &[Q], how_many: usize) -> Result<String, String> {
    if terms.len() < 3 {
        return Err("at least three terms are needed".into());
    }
    // 1. A polynomial in n.
    if let Some(degree) = polynomial_degree(terms)
        && let Some(coeffs) = polynomial_through(&terms[..degree + 1])
    {
        let consistent = terms
            .iter()
            .enumerate()
            .all(|(i, t)| eval(&coeffs, &Q::from_integer(BigInt::from(i as i64 + 1))) == *t);
        if consistent {
            let n = graph.symbol("n");
            let mut summands: Vec<ExprId> = Vec::new();
            for (k, c) in coeffs.iter().enumerate() {
                if c.is_zero() {
                    continue;
                }
                let power = match k {
                    0 => None,
                    1 => Some(n),
                    _ => Some(graph.pow(n, graph.integer(k as i64))),
                };
                let coefficient = crate::poly::expr_from_rational(graph, c);
                summands.push(match power {
                    None => coefficient,
                    Some(p) if c.is_one() => p,
                    Some(p) => graph.mul([coefficient, p]),
                });
            }
            let expression = if summands.is_empty() {
                graph.integer(0)
            } else {
                graph.add(summands)
            };
            let shown = algebra_core::format::Formatter::format(
                &algebra_core::format::UnicodeFormatter,
                graph,
                expression,
            )
            .map_err(|e| e.to_string())?;
            let next: Vec<Q> = (0..how_many)
                .map(|i| {
                    eval(
                        &coeffs,
                        &Q::from_integer(BigInt::from((terms.len() + i + 1) as i64)),
                    )
                })
                .collect();
            return Ok(format!(
                "a(n) = {shown}   (a polynomial of degree {degree}, n = 1, 2, ...); next terms: {}",
                list(&next)
            ));
        }
    }
    // 2. A linear recurrence of order at most 4 with two confirming terms.
    let c = berlekamp_massey(terms);
    let order = c.len();
    if (1..=4).contains(&order) && 2 * order < terms.len() {
        let mut state = terms.to_vec();
        let mut next = Vec::new();
        for _ in 0..how_many {
            let n = state.len();
            let value = (1..=order).fold(Q::zero(), |s, i| s + &c[i - 1] * &state[n - i]);
            state.push(value.clone());
            next.push(value);
        }
        let mut right = String::new();
        for (i, ci) in c.iter().enumerate().filter(|(_, ci)| !ci.is_zero()) {
            let offset = order - 1 - i;
            let call = if offset == 0 {
                "a(n)".to_string()
            } else {
                format!("a(n+{offset})")
            };
            let magnitude = if ci < &Q::zero() {
                -ci.clone()
            } else {
                ci.clone()
            };
            let body = if magnitude.is_one() {
                call
            } else {
                format!("{}*{call}", q_text(&magnitude))
            };
            if right.is_empty() {
                right.push_str(&format!("{}{body}", if ci < &Q::zero() { "-" } else { "" }));
            } else {
                right.push_str(&format!(
                    " {} {body}",
                    if ci < &Q::zero() { "-" } else { "+" }
                ));
            }
        }
        let equation = format!("a(n+{order}) = {right}");
        let initial: Vec<(i64, String)> = terms[..order]
            .iter()
            .enumerate()
            .map(|(i, t)| (i as i64 + 1, q_text(t)))
            .collect();
        let closed = crate::recurrence::solve(graph, &equation, &initial)
            .map(|(_, text)| format!("; closed form a(n) = {text} (n = 1, 2, ...)"))
            .unwrap_or_default();
        return Ok(format!("{equation}{closed}; next terms: {}", list(&next)));
    }
    Err("no polynomial or low-order linear recurrence fits these terms".into())
}
