//! Inverse Laplace transforms of rational functions: `inverse_laplace(1/(s^2 + 1), s, t)` is `sin(t)`.
//!
//! The denominator is factored over the rationals, the partial fractions are found exactly (a linear system over Q), and
//! each term is inverted from the table: `A / (s - a)^j` is `A t^(j-1) e^(a t) / (j-1)!`, and a term `(c1 s + c0) / q` with
//! `q = s^2 + p s + r` irreducible is `e^(-p t / 2) (c1 cos(w t) + (c0 - c1 p / 2) sin(w t) / w)` with `w^2 = r - p^2 / 4`.
//! Repeated quadratic factors and irreducible factors of degree 3 or more are reported as unsupported.

use crate::poly::UniPoly;
use crate::poly::factor::factor_rational_polynomial;
use crate::session::UraeSession;
use algebra_core::ExprKind;
use algebra_core::parser::ExprParser;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

type Q = BigRational;

fn split_top(text: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

fn q(text: &Q) -> String {
    if text.is_integer() {
        format!("({})", text.numer())
    } else {
        format!("({}/{})", text.numer(), text.denom())
    }
}

/// Solve the square system `m x = b` over the rationals by Gauss-Jordan elimination.
fn solve(mut m: Vec<Vec<Q>>, mut b: Vec<Q>) -> Option<Vec<Q>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).find(|&r| !m[r][col].is_zero())?;
        m.swap(col, pivot);
        b.swap(col, pivot);
        let p = m[col][col].clone();
        for c in col..n {
            m[col][c] = &m[col][c] / &p;
        }
        b[col] = &b[col] / &p;
        for r in 0..n {
            if r != col && !m[r][col].is_zero() {
                let f = m[r][col].clone();
                for c in col..n {
                    let delta = &f * &m[col][c];
                    m[r][c] -= delta;
                }
                let delta = &f * &b[col];
                b[r] -= delta;
            }
        }
    }
    Some(b)
}

/// `Some(result)` when `input` is an inverse Laplace call.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !matches!(
        name.as_str(),
        "inverse_laplace" | "ilaplace" | "laplace_inverse" | "invlaplace" | "inverselaplace"
    ) || !t.ends_with(')')
    {
        return None;
    }
    let args = split_top(&t[open + 1..t.len() - 1]);
    if args.is_empty() || args.len() > 3 {
        return Some(Err("inverse_laplace(F, s, t)".into()));
    }
    let (s_name, t_name) = (
        args.get(1).cloned().unwrap_or_else(|| "s".into()),
        args.get(2).cloned().unwrap_or_else(|| "t".into()),
    );
    Some(
        invert(session, &args[0], &s_name, &t_name)
            .map(|value| format!("L^-1{{{}}}({t_name}) = {value}", args[0])),
    )
}

/// The inverse Laplace transform of the rational function `f_text` of `s_name`, as an expression in `t_name`.
pub fn invert(
    session: &mut UraeSession,
    f_text: &str,
    s_name: &str,
    t_name: &str,
) -> Result<String, String> {
    invert_mode(session, f_text, s_name, t_name, false)
}

/// With `z_mode` the function is `X(z) / z` and the terms are those of the inverse Z-transform: `A / (z - a)^j` gives
/// `A C(n, j-1) a^(n-j+1)`, and the poles at zero give unit impulses.
fn invert_mode(
    session: &mut UraeSession,
    f_text: &str,
    s_name: &str,
    t_name: &str,
    z_mode: bool,
) -> Result<String, String> {
    // The rational function as numerator / denominator polynomials in s.
    let together = session.eval(&format!("cancel(together({f_text}))"))?;
    if together.trim() == "0" {
        return Ok("0".into());
    }
    let (numerator, denominator) = {
        let graph = &session.graph;
        let id = ExprParser::new(graph)
            .parse(&together)
            .map_err(|e| e.to_string())?;
        let symbol = graph.symbols.get_or_intern(s_name);
        let (n_id, d_id) =
            match graph.get(id).kind.clone() {
                ExprKind::Div(n, d) => (n, d),
                _ => return Err(
                    "the transform must be a rational function of s with a nonconstant denominator"
                        .into(),
                ),
            };
        let n = UniPoly::from_expr(graph, n_id, symbol).map_err(|e| format!("numerator: {e}"))?;
        let d = UniPoly::from_expr(graph, d_id, symbol).map_err(|e| format!("denominator: {e}"))?;
        (n, d)
    };
    if denominator.degree() == 0 {
        return Err("the denominator must depend on s".into());
    }
    if numerator.degree() >= denominator.degree() && !numerator.is_zero() {
        return Err(
            "the transform is not proper (its inverse contains a delta function); divide first"
                .into(),
        );
    }
    let (constant, factors) =
        factor_rational_polynomial(&denominator).map_err(|e| e.to_string())?;
    // D = prod q_i^k_i (monic); F = N / (constant D).
    let scaled = UniPoly::from_coeffs(numerator.coeffs.iter().map(|c| c / &constant).collect());
    let total_degree: usize = factors.iter().map(|(f, k)| f.degree() * *k as usize).sum();
    // Unknowns: for each (factor i, power j) a polynomial of degree < deg q_i, multiplying D / q_i^j.
    let mut columns: Vec<UniPoly> = Vec::new();
    let mut slots: Vec<(usize, u32, usize)> = Vec::new(); // (factor index, power, coefficient index)
    for (i, (qi, ki)) in factors.iter().enumerate() {
        for j in 1..=*ki {
            let mut cofactor = UniPoly::constant(Q::one());
            for (l, (ql, kl)) in factors.iter().enumerate() {
                let power = if l == i { kl - j } else { *kl };
                cofactor = cofactor.mul(&ql.pow(power));
            }
            for c in 0..qi.degree() {
                let mut monomial = vec![Q::zero(); c + 1];
                monomial[c] = Q::one();
                columns.push(cofactor.mul(&UniPoly::from_coeffs(monomial)));
                slots.push((i, j, c));
            }
        }
    }
    let n = columns.len();
    if n != total_degree {
        return Err("internal error: partial-fraction system is not square".into());
    }
    let matrix: Vec<Vec<Q>> = (0..n)
        .map(|row| {
            columns
                .iter()
                .map(|col| col.coeffs.get(row).cloned().unwrap_or_else(Q::zero))
                .collect()
        })
        .collect();
    let rhs: Vec<Q> = (0..n)
        .map(|row| scaled.coeffs.get(row).cloned().unwrap_or_else(Q::zero))
        .collect();
    let solution = solve(matrix, rhs).ok_or("the partial-fraction system is singular")?;
    // Invert each term.
    let mut terms: Vec<String> = Vec::new();
    let mut impulses: Vec<String> = Vec::new();
    for (i, (qi, ki)) in factors.iter().enumerate() {
        for j in 1..=*ki {
            let coefficient = |c: usize| -> Q {
                slots
                    .iter()
                    .position(|s| *s == (i, j, c))
                    .map(|k| solution[k].clone())
                    .unwrap_or_else(Q::zero)
            };
            match qi.degree() {
                1 => {
                    let a = -qi.coeffs[0].clone();
                    let big_a = coefficient(0);
                    if big_a.is_zero() {
                        continue;
                    }
                    let factorial: BigInt = (1..j as i64).map(BigInt::from).product();
                    if z_mode {
                        let m = j as i64 - 1;
                        if a.is_zero() {
                            impulses.push(format!("{}*delta[{t_name}-{m}]", q(&big_a)));
                        } else {
                            let falling: Vec<String> =
                                (0..m).map(|r| format!("({t_name}-{r})")).collect();
                            let binomial = if m == 0 {
                                "1".to_string()
                            } else {
                                format!("{}/{factorial}", falling.join("*"))
                            };
                            terms.push(format!(
                                "{}*{binomial}*{}^({t_name}-{m})",
                                q(&big_a),
                                q(&a)
                            ));
                        }
                        continue;
                    }
                    terms.push(format!(
                        "{}*{t_name}^{}/{factorial}*exp({}*{t_name})",
                        q(&big_a),
                        j - 1,
                        q(&a)
                    ));
                }
                2 => {
                    if j > 1 {
                        return Err(
                            "repeated irreducible quadratic factors are not supported".into()
                        );
                    }
                    let (c1, c0) = (coefficient(1), coefficient(0));
                    if c1.is_zero() && c0.is_zero() {
                        continue;
                    }
                    let (r, p) = (qi.coeffs[0].clone(), qi.coeffs[1].clone());
                    let half = Q::new(BigInt::from(1), BigInt::from(2));
                    let w_squared = &r - &p * &p * Q::new(BigInt::from(1), BigInt::from(4));
                    let sin_coefficient = (&c0 - &c1 * &p * &half).to_string();
                    // A real irreducible quadratic (roots -p/2 +- sqrt(-w^2)) inverts to hyperbolic functions.
                    let hyperbolic = !w_squared.is_positive();
                    let magnitude = if hyperbolic {
                        -w_squared.clone()
                    } else {
                        w_squared.clone()
                    };
                    // sqrt(p / q) = sqrt(p q) / q keeps the radicand an integer (sqrt(3/4) is sqrt(3) / 2).
                    let w = format!(
                        "(sqrt({})/{})",
                        magnitude.numer() * magnitude.denom(),
                        magnitude.denom()
                    );
                    if z_mode {
                        if hyperbolic {
                            return Err("irrational real poles are not supported by the inverse Z-transform yet".into());
                        }
                        // Poles rho e^(+-i theta) with rho^2 = r, rho cos(theta) = -p/2: the term of X(z) = z (c1 z + c0) / q is
                        // rho^n (c1 cos(n theta) + (c0 - c1 p / 2) / w sin(n theta)).
                        let rho = format!("sqrt({})", q(&r));
                        let cos_theta = format!("({}/{rho})", q(&(-&p * &half)));
                        terms.push(format!(
                            "{rho}^{t_name}*({}*cos({t_name}*acos({cos_theta})) + ({})/{w}*sin({t_name}*acos({cos_theta})))",
                            q(&c1),
                            sin_coefficient
                        ));
                        continue;
                    }
                    let (cosine, sine) = if hyperbolic {
                        ("cosh", "sinh")
                    } else {
                        ("cos", "sin")
                    };
                    terms.push(format!(
                        "exp({}*{t_name})*({}*{cosine}({w}*{t_name}) + ({})/{w}*{sine}({w}*{t_name}))",
                        q(&(-&p * &half)),
                        q(&c1),
                        sin_coefficient
                    ));
                }
                d => {
                    return Err(format!(
                        "an irreducible factor of degree {d} is not supported"
                    ));
                }
            }
        }
    }
    if terms.is_empty() && impulses.is_empty() {
        return Ok("0".into());
    }
    let body = if terms.is_empty() {
        None
    } else {
        Some(session.eval(&terms.join(" + "))?)
    };
    if !z_mode {
        return Ok(body.unwrap_or_else(|| "0".into()));
    }
    // Impulses are printed as delta[n-k] (an impulse at n = k).
    let tail: Vec<String> = impulses
        .iter()
        .map(|i| i.replace("*delta", " delta").replace("-0]", "]"))
        .collect();
    Ok(match body {
        Some(b) if tail.is_empty() => b,
        Some(b) => format!("{b} + {}", tail.join(" + ")),
        None => tail.join(" + "),
    })
}

/// `inverse_ztransform(X(z), z, n)`: the sequence of a rational `X(z)` (right-sided, real poles).
pub fn run_z(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !matches!(
        name.as_str(),
        "inverse_ztransform" | "iztransform" | "inverse_z" | "izt" | "inverse_z_transform"
    ) || !t.ends_with(')')
    {
        return None;
    }
    let mut args = split_top(&t[open + 1..t.len() - 1]);
    // An optional last argument `left` selects the left-sided (anticausal) sequence, the region of convergence |z| < smallest pole.
    let left = args.len() > 1
        && matches!(
            args.last()
                .map(|a| a.trim().trim_matches('"').to_ascii_lowercase())
                .as_deref(),
            Some("left" | "left_sided" | "anticausal")
        );
    if left {
        args.pop();
    }
    if args.is_empty() || args.len() > 3 {
        return Some(Err("inverse_ztransform(X, z, n[, left])".into()));
    }
    let z_name = args.get(1).cloned().unwrap_or_else(|| "z".into());
    let n_name = args.get(2).cloned().unwrap_or_else(|| "n".into());
    let divided = format!("({})/({z_name})", args[0]);
    Some(
        invert_mode(session, &divided, &z_name, &n_name, true).and_then(|value| {
            if !left {
                return Ok(format!("Z^-1{{{}}}[{n_name}] = {value}", args[0]));
            }
            if value.contains("delta[") {
                return Err(
                    "a left-sided sequence needs X(0) = 0 (no pole at zero in X(z)/z)".into(),
                );
            }
            // Each causal term A a^n u[n] of the partial fractions becomes -A a^n u[-n-1].
            let negated = session.eval(&format!("-({value})"))?;
            Ok(format!(
                "Z^-1{{{}}}[{n_name}] = {negated} for {n_name} <= -1 (left-sided)",
                args[0]
            ))
        }),
    )
}
