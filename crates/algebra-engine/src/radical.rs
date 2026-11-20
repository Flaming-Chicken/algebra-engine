//! Exact arithmetic with radicals of rationals: sums of `r * p1^(a1) * p2^(a2) ...` with rational `r`, distinct
//! primes `p` and exponents `0 < a < 1` (so `2^(3/2)` is `2 sqrt(2)`, `1/sqrt(2)` is `sqrt(2)/2`, `(sqrt(2) + 1)^2`
//! is `3 + 2 sqrt(2)`, `1/(1 + sqrt(2))` is `sqrt(2) - 1`).
//!
//! Supported: `+ - * /`, integer powers, rational powers and roots of a single positive term, and the inverse of a
//! rational plus a rational multiple of one square root (by the conjugate).

use crate::numbertheory::integer_prime_factors;
use crate::poly::rational_from_number;
use algebra_core::{ExprGraph, ExprId, ExprKind};
use num::Integer;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::BTreeMap;

type Q = BigRational;
/// A product of prime powers with fractional exponents: `(prime, numerator, denominator)`, sorted by prime.
type Monomial = Vec<(u64, i64, i64)>;

#[derive(Clone, Debug, PartialEq)]
pub struct Radical {
    terms: BTreeMap<Monomial, Q>,
}

fn frac_add(a: (i64, i64), b: (i64, i64)) -> (i64, i64) {
    let n = a.0 * b.1 + b.0 * a.1;
    let d = a.1 * b.1;
    let g = n.gcd(&d).max(1);
    (n / g, d / g)
}

impl Radical {
    fn constant(q: Q) -> Self {
        let mut terms = BTreeMap::new();
        if !q.is_zero() {
            terms.insert(Vec::new(), q);
        }
        Radical { terms }
    }

    fn single(&self) -> Option<(&Monomial, &Q)> {
        if self.terms.len() == 1 {
            self.terms.iter().next()
        } else {
            None
        }
    }

    pub fn is_rational(&self) -> bool {
        self.terms.keys().all(|m| m.is_empty())
    }

    fn add(&self, other: &Radical) -> Radical {
        let mut terms = self.terms.clone();
        for (m, c) in &other.terms {
            let entry = terms.entry(m.clone()).or_insert_with(Q::zero);
            *entry += c;
            if entry.is_zero() {
                terms.remove(m);
            }
        }
        Radical { terms }
    }

    fn negate(&self) -> Radical {
        Radical {
            terms: self.terms.iter().map(|(m, c)| (m.clone(), -c)).collect(),
        }
    }

    fn mul(&self, other: &Radical) -> Option<Radical> {
        let mut out = Radical {
            terms: BTreeMap::new(),
        };
        for (m1, c1) in &self.terms {
            for (m2, c2) in &other.terms {
                // Merge exponents prime by prime, carrying whole powers into the coefficient.
                let mut exponents: BTreeMap<u64, (i64, i64)> = BTreeMap::new();
                for &(p, n, d) in m1.iter().chain(m2.iter()) {
                    let e = exponents.entry(p).or_insert((0, 1));
                    *e = frac_add(*e, (n, d));
                }
                let mut coefficient = c1 * c2;
                let mut monomial = Vec::new();
                for (p, (n, d)) in exponents {
                    let whole = n.div_euclid(d);
                    let rest = n.rem_euclid(d);
                    if whole != 0 {
                        let power = pow_integer(&Q::from_integer(BigInt::from(p)), whole)?;
                        coefficient *= power;
                    }
                    if rest != 0 {
                        monomial.push((p, rest, d));
                    }
                }
                let single = Radical {
                    terms: BTreeMap::from([(monomial, coefficient)]),
                };
                out = out.add(&single);
            }
        }
        Some(out)
    }

    /// The multiplicative inverse (single term, or `r + s sqrt(n)` by its conjugate).
    fn inverse(&self) -> Option<Radical> {
        if self.terms.is_empty() {
            return None;
        }
        if let Some((m, c)) = self.single() {
            // 1 / (c * prod p^(a)) = (1/c) prod p^(-a) = (1/c) prod p^(-1) p^(1 - a)
            let mut exponents = Vec::new();
            let mut coefficient = Q::one() / c;
            for &(p, n, d) in m {
                coefficient /= Q::from_integer(BigInt::from(p));
                exponents.push((p, d - n, d));
            }
            // p^(1 - a) with 0 < 1 - a < 1 is already normalised.
            return Some(Radical {
                terms: BTreeMap::from([(exponents, coefficient)]),
            });
        }
        // A sum of square roots (and a rational part): multiply by the conjugate of the smallest prime's root, which
        // removes that prime from the denominator; repeat on the norm.
        let prime = self
            .terms
            .keys()
            .flat_map(|m| m.iter().map(|&(p, _, _)| p))
            .min()?;
        let mut conjugate = BTreeMap::new();
        for (m, c) in &self.terms {
            if !m.iter().all(|&(_, n, d)| n == 1 && d == 2) {
                return None;
            }
            if m.iter().any(|&(p, _, _)| p == prime) {
                conjugate.insert(m.clone(), -c);
            } else {
                conjugate.insert(m.clone(), c.clone());
            }
        }
        let conjugate = Radical { terms: conjugate };
        let norm = self.mul(&conjugate)?;
        if norm.terms.is_empty() {
            return None;
        }
        // The norm no longer contains `prime`.
        if norm
            .terms
            .keys()
            .any(|m| m.iter().any(|&(p, _, _)| p == prime))
        {
            return None;
        }
        conjugate.mul(&norm.inverse()?)
    }

    /// `self^(p/q)` for a single positive term.
    fn pow_rational(&self, p: i64, q: i64) -> Option<Radical> {
        if self.terms.is_empty() {
            // 0^(p/q) is 0 for a positive exponent.
            return (p > 0).then(|| self.clone());
        }
        let (monomial, coefficient) = self.single()?;
        if !coefficient.is_positive() {
            return None;
        }
        // Result = coefficient^(p/q) * monomial^(p/q); build from prime factors of the coefficient.
        let mut exponents: BTreeMap<u64, (i64, i64)> = BTreeMap::new();
        let numerator = coefficient.numer().to_u64()?;
        let denominator = coefficient.denom().to_u64()?;
        for (prime, e) in integer_prime_factors(numerator) {
            let entry = exponents.entry(prime).or_insert((0, 1));
            *entry = frac_add(*entry, (e as i64, 1));
        }
        for (prime, e) in integer_prime_factors(denominator) {
            let entry = exponents.entry(prime).or_insert((0, 1));
            *entry = frac_add(*entry, (-(e as i64), 1));
        }
        for &(prime, n, d) in monomial {
            let entry = exponents.entry(prime).or_insert((0, 1));
            *entry = frac_add(*entry, (n, d));
        }
        let mut coefficient = Q::one();
        let mut result_monomial = Vec::new();
        for (prime, (n, d)) in exponents {
            // exponent (n/d) * (p/q)
            let (mut en, mut ed) = (n.checked_mul(p)?, d.checked_mul(q)?);
            let g = en.gcd(&ed).max(1);
            (en, ed) = (en / g, ed / g);
            let whole = en.div_euclid(ed);
            let rest = en.rem_euclid(ed);
            if whole != 0 {
                coefficient *= pow_integer(&Q::from_integer(BigInt::from(prime)), whole)?;
            }
            if rest != 0 {
                result_monomial.push((prime, rest, ed));
            }
        }
        Some(Radical {
            terms: BTreeMap::from([(result_monomial, coefficient)]),
        })
    }

    /// Written like the rest of the engine's output: `3 + 2 · √(2)`.
    pub fn format(&self) -> String {
        if self.terms.is_empty() {
            return "0".into();
        }
        let monomial_text = |m: &Monomial| -> String {
            let halves: Vec<u64> = m
                .iter()
                .filter(|&&(_, n, d)| n == 1 && d == 2)
                .map(|&(p, _, _)| p)
                .collect();
            let mut parts = Vec::new();
            if !halves.is_empty() {
                parts.push(format!("√({})", halves.iter().product::<u64>()));
            }
            for &(p, n, d) in m {
                if !(n == 1 && d == 2) {
                    parts.push(format!("{p}^({n}/{d})"));
                }
            }
            parts.join(" · ")
        };
        let mut out = String::new();
        for (index, (m, c)) in self.terms.iter().enumerate() {
            let magnitude = c.abs();
            let body = if m.is_empty() {
                show(&magnitude)
            } else if magnitude.is_one() {
                monomial_text(m)
            } else {
                format!("{} · {}", show(&magnitude), monomial_text(m))
            };
            if index == 0 {
                out.push_str(&format!("{}{body}", if c.is_negative() { "-" } else { "" }));
            } else {
                out.push_str(&format!(
                    " {} {body}",
                    if c.is_negative() { "-" } else { "+" }
                ));
            }
        }
        out
    }
}

fn show(q: &Q) -> String {
    if q.is_integer() {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

fn pow_integer(base: &Q, exponent: i64) -> Option<Q> {
    if exponent.abs() > 512 {
        return None;
    }
    let mut result = Q::one();
    for _ in 0..exponent.abs() {
        result *= base;
    }
    if exponent < 0 {
        if result.is_zero() {
            return None;
        }
        Some(Q::one() / result)
    } else {
        Some(result)
    }
}

fn from_integer_power(r: &Radical, k: i64) -> Option<Radical> {
    if k.abs() > 64 {
        return None;
    }
    let mut result = Radical::constant(Q::one());
    let base = if k < 0 { r.inverse()? } else { r.clone() };
    for _ in 0..k.abs() {
        result = result.mul(&base)?;
    }
    Some(result)
}

/// Evaluate a constant expression as a radical expression, or `None` if it is outside the family.
pub fn evaluate(graph: &ExprGraph, id: ExprId) -> Option<Radical> {
    match graph.get(id).kind.clone() {
        ExprKind::Number(n) => rational_from_number(&n).ok().map(Radical::constant),
        ExprKind::Neg(a) => Some(evaluate(graph, a)?.negate()),
        ExprKind::Add(ts) => {
            let mut total = Radical::constant(Q::zero());
            for t in ts.iter() {
                total = total.add(&evaluate(graph, *t)?);
            }
            Some(total)
        }
        ExprKind::Sub(a, b) => Some(evaluate(graph, a)?.add(&evaluate(graph, b)?.negate())),
        ExprKind::Mul(fs) => {
            let mut product = Radical::constant(Q::one());
            for f in fs.iter() {
                product = product.mul(&evaluate(graph, *f)?)?;
            }
            Some(product)
        }
        ExprKind::Div(a, b) => {
            if let Some(v) = logarithm_value(graph, id) {
                return Some(v);
            }
            evaluate(graph, a)?.mul(&evaluate(graph, b)?.inverse()?)
        }
        ExprKind::Pow(base, exponent)
            if is_e(graph, base) && log_sum(graph, exponent).is_some() =>
        {
            exp_of_log_sum(&log_sum(graph, exponent)?)
        }
        ExprKind::Pow(base, exponent) => {
            let base = evaluate(graph, base)?;
            let e = evaluate(graph, exponent)?;
            if !e.is_rational() {
                return None;
            }
            let e = e.terms.get(&Vec::new()).cloned().unwrap_or_else(Q::zero);
            let (p, q) = (e.numer().to_i64()?, e.denom().to_i64()?);
            if q == 1 {
                from_integer_power(&base, p)
            } else {
                base.pow_rational(p, q)
            }
        }
        ExprKind::Function { name, args } => {
            let fname = graph.symbols.resolve(name)?;
            match (fname.as_str(), args.len()) {
                ("sin" | "cos" | "tan" | "cot" | "sec" | "csc", 1) => {
                    trig_value(graph, &fname, args[0])
                }
                ("exp", 1) => exp_of_log_sum(&log_sum(graph, args[0])?),
                ("log2" | "log10", 1) | ("log", 2) => logarithm_value(graph, id),
                ("sqrt", 1) => evaluate(graph, args[0])?.pow_rational(1, 2),
                ("cbrt", 1) => evaluate(graph, args[0])?.pow_rational(1, 3),
                ("root", 2) => {
                    let k = evaluate(graph, args[1])?;
                    let k = k.terms.get(&Vec::new())?.to_integer().to_i64()?;
                    if k <= 0 {
                        return None;
                    }
                    evaluate(graph, args[0])?.pow_rational(1, k)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether `id` contains a root or a fractional power (so that the radical form is worth printing).
pub fn has_radicals(graph: &ExprGraph, id: ExprId) -> bool {
    match graph.get(id).kind.clone() {
        ExprKind::Pow(a, b) => {
            matches!(
                graph.get(b).kind,
                ExprKind::Number(algebra_core::Number::Rational(..))
            ) || has_radicals(graph, a)
                || has_radicals(graph, b)
        }
        ExprKind::Function { name, args } => {
            matches!(
                graph.symbols.resolve(name).as_deref(),
                Some(
                    "sqrt"
                        | "cbrt"
                        | "root"
                        | "exp"
                        | "log2"
                        | "log10"
                        | "ln"
                        | "log"
                        | "sin"
                        | "cos"
                        | "tan"
                        | "cot"
                        | "sec"
                        | "csc"
                )
            ) || args.iter().any(|&a| has_radicals(graph, a))
        }
        ExprKind::Add(ts) | ExprKind::Mul(ts) => ts.iter().any(|&t| has_radicals(graph, t)),
        ExprKind::Sub(a, b) | ExprKind::Div(a, b) => {
            has_radicals(graph, a) || has_radicals(graph, b)
        }
        ExprKind::Neg(a) => has_radicals(graph, a),
        _ => false,
    }
}

/// A sum of rational multiples of `ln p` over primes `p`: the logarithm of a positive rational number, or a rational
/// combination of such logarithms. `ln(8) = 3 ln 2`, `ln(3/4) = ln 3 - 2 ln 2`.
type LogSum = BTreeMap<u64, Q>;

fn log_of_rational(q: &Q) -> Option<LogSum> {
    if !q.is_positive() {
        return None;
    }
    let mut sum = LogSum::new();
    for (p, e) in integer_prime_factors(q.numer().to_u64()?) {
        *sum.entry(p).or_insert_with(Q::zero) += Q::from_integer(BigInt::from(e));
    }
    for (p, e) in integer_prime_factors(q.denom().to_u64()?) {
        *sum.entry(p).or_insert_with(Q::zero) -= Q::from_integer(BigInt::from(e));
    }
    sum.retain(|_, c| !c.is_zero());
    Some(sum)
}

fn rational_value(graph: &ExprGraph, id: ExprId) -> Option<Q> {
    let r = evaluate(graph, id)?;
    if r.is_rational() {
        Some(r.terms.get(&Vec::new()).cloned().unwrap_or_else(Q::zero))
    } else {
        None
    }
}

fn is_e(graph: &ExprGraph, id: ExprId) -> bool {
    match &graph.get(id).kind {
        ExprKind::Number(algebra_core::Number::Constant(algebra_core::Constant::E)) => true,
        ExprKind::Symbol(s) => graph.symbols.resolve(*s).as_deref() == Some("e"),
        _ => false,
    }
}

/// `expr` as a combination of logarithms of rationals, if it is one.
fn log_sum(graph: &ExprGraph, id: ExprId) -> Option<LogSum> {
    let scale = |s: LogSum, k: &Q| -> LogSum {
        s.into_iter()
            .map(|(p, c)| (p, c * k))
            .filter(|(_, c)| !c.is_zero())
            .collect()
    };
    let merge = |a: LogSum, b: LogSum| -> LogSum {
        let mut out = a;
        for (p, c) in b {
            *out.entry(p).or_insert_with(Q::zero) += c;
        }
        out.retain(|_, c| !c.is_zero());
        out
    };
    match graph.get(id).kind.clone() {
        ExprKind::Function { name, args } if args.len() == 1 => {
            match graph.symbols.resolve(name).as_deref() {
                Some("ln" | "log") => log_of_rational(&rational_value(graph, args[0])?),
                _ => None,
            }
        }
        ExprKind::Neg(a) => Some(scale(log_sum(graph, a)?, &-Q::one())),
        ExprKind::Add(ts) => {
            let mut total = LogSum::new();
            for t in ts.iter() {
                total = merge(total, log_sum(graph, *t)?);
            }
            Some(total)
        }
        ExprKind::Sub(a, b) => Some(merge(
            log_sum(graph, a)?,
            scale(log_sum(graph, b)?, &-Q::one()),
        )),
        ExprKind::Mul(fs) => {
            let (mut logs, mut k) = (None, Q::one());
            for f in fs.iter() {
                if let Some(v) = rational_value(graph, *f) {
                    k *= v;
                } else if logs.is_none() {
                    logs = Some(log_sum(graph, *f)?);
                } else {
                    return None;
                }
            }
            Some(scale(logs?, &k))
        }
        ExprKind::Div(a, b) => Some(scale(
            log_sum(graph, a)?,
            &(Q::one() / rational_value(graph, b)?),
        )),
        _ => None,
    }
}

/// The rational `t` with `a = t b` for two log sums, if there is one.
fn log_ratio(a: &LogSum, b: &LogSum) -> Option<Q> {
    let (&p, cb) = b.iter().next()?;
    let t = a.get(&p).cloned().unwrap_or_else(Q::zero) / cb;
    let proportional = a.len() == b.len()
        && b.iter()
            .all(|(q, c)| a.get(q).is_some_and(|x| *x == &t * c));
    proportional.then_some(t)
}

/// `exp(sum)` as a radical: the product of `p^c`.
fn exp_of_log_sum(sum: &LogSum) -> Option<Radical> {
    let mut product = Radical::constant(Q::one());
    for (&p, c) in sum {
        let base = Radical::constant(Q::from_integer(BigInt::from(p)));
        let power = if c.is_integer() {
            from_integer_power(&base, c.to_integer().to_i64()?)?
        } else {
            base.pow_rational(c.numer().to_i64()?, c.denom().to_i64()?)?
        };
        product = product.mul(&power)?;
    }
    Some(product)
}

/// `log2(r)`, `log10(r)`, `log(r, b)` and `ln(r) / ln(b)` of rationals, when the result is a rational.
fn logarithm_value(graph: &ExprGraph, id: ExprId) -> Option<Radical> {
    let of = |x: ExprId| rational_value(graph, x).and_then(|q| log_of_rational(&q));
    let base = |n: i64| log_of_rational(&Q::from_integer(n.into()));
    match graph.get(id).kind.clone() {
        ExprKind::Function { name, args } => {
            match (graph.symbols.resolve(name).as_deref()?, args.len()) {
                ("log2", 1) => Some(Radical::constant(log_ratio(&of(args[0])?, &base(2)?)?)),
                ("log10", 1) => Some(Radical::constant(log_ratio(&of(args[0])?, &base(10)?)?)),
                ("log", 2) => Some(Radical::constant(log_ratio(&of(args[0])?, &of(args[1])?)?)),
                _ => None,
            }
        }
        ExprKind::Div(a, b) => {
            let (la, lb) = (log_sum(graph, a)?, log_sum(graph, b)?);
            if lb.is_empty() {
                return None;
            }
            Some(Radical::constant(log_ratio(&la, &lb)?))
        }
        _ => None,
    }
}

impl Radical {
    /// The same value as an expression: `3 + 2 sqrt(2)`.
    pub fn to_expr(&self, graph: &ExprGraph) -> ExprId {
        if self.terms.is_empty() {
            return graph.integer(0);
        }
        let mut sum = Vec::new();
        for (monomial, coefficient) in &self.terms {
            let mut factors = Vec::new();
            if !coefficient.is_one() || monomial.is_empty() {
                factors.push(crate::poly::expr_from_rational(graph, coefficient));
            }
            let halves: Vec<u64> = monomial
                .iter()
                .filter(|&&(_, n, d)| n == 1 && d == 2)
                .map(|&(p, _, _)| p)
                .collect();
            if !halves.is_empty() {
                let product: u64 = halves.iter().product();
                factors.push(graph.function("sqrt", [graph.integer(product as i64)]));
            }
            for &(p, n, d) in monomial {
                if !(n == 1 && d == 2) {
                    factors.push(graph.pow(graph.integer(p as i64), graph.rational(n, d)));
                }
            }
            sum.push(if factors.len() == 1 {
                factors[0]
            } else {
                graph.mul(factors)
            });
        }
        if sum.len() == 1 {
            sum[0]
        } else {
            graph.add(sum)
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Exact trigonometric values at multiples of pi/12
// ---------------------------------------------------------------------------------------------------------------

fn is_pi(graph: &ExprGraph, id: ExprId) -> bool {
    match &graph.get(id).kind {
        ExprKind::Number(algebra_core::Number::Constant(algebra_core::Constant::Pi)) => true,
        ExprKind::Symbol(s) => matches!(graph.symbols.resolve(*s).as_deref(), Some("pi" | "π")),
        _ => false,
    }
}

/// `expr = r pi` for a rational `r`.
fn pi_multiple(graph: &ExprGraph, id: ExprId) -> Option<Q> {
    if is_pi(graph, id) {
        return Some(Q::one());
    }
    match graph.get(id).kind.clone() {
        ExprKind::Number(n) => {
            let q = rational_from_number(&n).ok()?;
            q.is_zero().then_some(q)
        }
        ExprKind::Neg(a) => Some(-pi_multiple(graph, a)?),
        ExprKind::Add(ts) => {
            let mut total = Q::zero();
            for t in ts.iter() {
                total += pi_multiple(graph, *t)?;
            }
            Some(total)
        }
        ExprKind::Sub(a, b) => Some(pi_multiple(graph, a)? - pi_multiple(graph, b)?),
        ExprKind::Mul(fs) => {
            let (mut k, mut multiple) = (Q::one(), None);
            for f in fs.iter() {
                if let Some(v) = rational_value(graph, *f) {
                    k *= v;
                } else if multiple.is_none() {
                    multiple = Some(pi_multiple(graph, *f)?);
                } else {
                    return None;
                }
            }
            Some(k * multiple?)
        }
        ExprKind::Div(a, b) => Some(pi_multiple(graph, a)? / rational_value(graph, b)?),
        _ => None,
    }
}

/// `sin`, `cos`, `tan`, `cot`, `sec` or `csc` of `k pi / 12`, exactly.
fn trig_value(graph: &ExprGraph, name: &str, argument: ExprId) -> Option<Radical> {
    let r = pi_multiple(graph, argument)?;
    let twelfths = r * Q::from_integer(12.into());
    if !twelfths.is_integer() {
        return None;
    }
    let k = twelfths
        .to_integer()
        .mod_floor(&BigInt::from(24))
        .to_i64()?;
    let root = |n: i64| Radical::constant(Q::from_integer(n.into())).pow_rational(1, 2);
    let quarter = Radical::constant(Q::new(1.into(), 4.into()));
    let (r6, r2) = (root(6)?, root(2)?);
    let sin_unit = quarter.mul(&r6.add(&r2.negate()))?;
    let cos_unit = quarter.mul(&r6.add(&r2))?;
    let (mut s, mut c) = (Radical::constant(Q::zero()), Radical::constant(Q::one()));
    for _ in 0..k {
        let next_s = s.mul(&cos_unit)?.add(&c.mul(&sin_unit)?);
        let next_c = c.mul(&cos_unit)?.add(&s.mul(&sin_unit)?.negate());
        (s, c) = (next_s, next_c);
    }
    match name {
        "sin" => Some(s),
        "cos" => Some(c),
        "tan" => s.mul(&c.inverse()?),
        "cot" => c.mul(&s.inverse()?),
        "sec" => c.inverse(),
        "csc" => s.inverse(),
        _ => None,
    }
}
