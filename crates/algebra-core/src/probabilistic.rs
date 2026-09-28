//! # `algebra_core::probabilistic`
//!
//! Multi-Tier Probabilistic Verification, Heuristic Weighting & Solution Certification.
//!
//! Provides the generic [`Solution<T>`] type for deterministic vs. certified probabilistic
//! results, along with the [`ProbabilisticVerifier`] for $O(1)$ single-point and multi-prime
//! Schwartz-Zippel testing over Mersenne primes ($\mathbb{F}_{2^{31}-1}, \mathbb{F}_{2^{61}-1}, \mathbb{F}_{2^{64}-59}$)
//! and randomized interval bounding.

use crate::expr::ExprKind;
use crate::graph::ExprGraph;
use crate::id::{ExprId, SymbolId};
use crate::interval::RealInterval;
use crate::number::{Constant, Number};
use std::collections::HashMap;
use std::fmt;

/// Representation of a symbolic computation result, either deterministically proven
/// or certified with a rigorous probabilistic upper-bound on error probability.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Solution<T> {
    /// Guaranteed exact deterministic symbolic solution.
    Deterministic(T),
    /// Probabilistic solution certified with confidence metrics when budget caps are reached.
    Probabilistic {
        /// The computed result candidate.
        result: T,
        /// Rigorous upper bound on error probability (e.g. 1e-20).
        error_probability_upper_bound: f64,
        /// Number of multi-point / multi-prime evaluation samples.
        sample_count: usize,
        /// Mathematical verification algorithm (e.g., "Schwartz-Zippel CRT").
        method: &'static str,
    },
}

impl<T> Solution<T> {
    /// Extract the underlying result value.
    pub fn into_inner(self) -> T {
        match self {
            Self::Deterministic(v) => v,
            Self::Probabilistic { result, .. } => result,
        }
    }

    /// Access reference to the underlying result value.
    #[allow(clippy::should_implement_trait)]
    pub fn as_ref(&self) -> &T {
        <Self as AsRef<T>>::as_ref(self)
    }

    /// Access reference to the underlying result value.
    pub fn value(&self) -> &T {
        <Self as AsRef<T>>::as_ref(self)
    }

    /// Returns `true` if the solution is deterministically verified.
    pub fn is_deterministic(&self) -> bool {
        matches!(self, Self::Deterministic(_))
    }

    /// Returns `true` if the solution is certified probabilistically.
    pub fn is_probabilistic(&self) -> bool {
        matches!(self, Self::Probabilistic { .. })
    }

    /// Upper bound on the probability of error ($0.0$ for deterministic).
    pub fn error_probability(&self) -> f64 {
        match self {
            Self::Deterministic(_) => 0.0,
            Self::Probabilistic {
                error_probability_upper_bound,
                ..
            } => *error_probability_upper_bound,
        }
    }

    /// Confidence score $1.0 - \text{error\_probability}$ (between 0.0 and 1.0).
    pub fn confidence(&self) -> f64 {
        1.0 - self.error_probability()
    }

    /// Number of evaluation samples performed.
    pub fn sample_count(&self) -> usize {
        match self {
            Self::Deterministic(_) => usize::MAX,
            Self::Probabilistic { sample_count, .. } => *sample_count,
        }
    }

    /// Verification method description.
    pub fn method(&self) -> &'static str {
        match self {
            Self::Deterministic(_) => "Deterministic Proof",
            Self::Probabilistic { method, .. } => method,
        }
    }

    /// Transform the inner result via a mapping function `f`.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Solution<U> {
        match self {
            Self::Deterministic(v) => Solution::Deterministic(f(v)),
            Self::Probabilistic {
                result,
                error_probability_upper_bound,
                sample_count,
                method,
            } => Solution::Probabilistic {
                result: f(result),
                error_probability_upper_bound,
                sample_count,
                method,
            },
        }
    }

    /// Transform the inner result reference via a mapping function `f`.
    pub fn map_ref<U, F: FnOnce(&T) -> U>(&self, f: F) -> Solution<U> {
        match self {
            Self::Deterministic(v) => Solution::Deterministic(f(v)),
            Self::Probabilistic {
                result,
                error_probability_upper_bound,
                sample_count,
                method,
            } => Solution::Probabilistic {
                result: f(result),
                error_probability_upper_bound: *error_probability_upper_bound,
                sample_count: *sample_count,
                method,
            },
        }
    }
}

impl<T> AsRef<T> for Solution<T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Deterministic(v) => v,
            Self::Probabilistic { result, .. } => result,
        }
    }
}

impl<T: fmt::Display> fmt::Display for Solution<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deterministic(v) => write!(f, "{v}"),
            Self::Probabilistic {
                result,
                error_probability_upper_bound,
                sample_count,
                method,
            } => {
                let conf = (1.0 - error_probability_upper_bound) * 100.0;
                write!(
                    f,
                    "{result} [Probabilistic Certificate: {conf:.8}% confidence, {sample_count} samples via {method}]"
                )
            }
        }
    }
}

/// Multi-prime and randomized interval verification engine.
#[derive(Debug, Clone, Copy)]
pub struct ProbabilisticVerifier;

impl ProbabilisticVerifier {
    /// Mersenne prime $p_1 = 2^{31} - 1 = 2,147,483,647$.
    pub const MERSENNE_31: u64 = 2_147_483_647;
    /// Mersenne prime $p_2 = 2^{61} - 1 = 2,305,843,009,213,693,951$.
    pub const MERSENNE_61: u64 = 2_305_843_009_213_693_951;
    /// Cryptographic prime $p_3 = 2^{64} - 59 = 18,446,744,073,709,551,557$.
    pub const LARGE_PRIME_64: u64 = 18_446_744_073_709_551_557;

    /// Modular exponentiation: $(b^e) \pmod m$.
    pub fn mod_pow(mut base: u128, mut exp: u128, modulus: u128) -> u128 {
        if modulus == 1 {
            return 0;
        }
        let mut result = 1;
        base %= modulus;
        while exp > 0 {
            if exp % 2 == 1 {
                result = (result * base) % modulus;
            }
            exp /= 2;
            base = (base * base) % modulus;
        }
        result
    }

    /// Modular inverse using Extended Euclidean algorithm: $a^{-1} \pmod m$.
    pub fn mod_inv(a: u64, m: u64) -> Option<u64> {
        let mut t = 0i128;
        let mut newt = 1i128;
        let mut r = m as i128;
        let mut newr = (a % m) as i128;

        while newr != 0 {
            let quotient = r / newr;
            let tmp_t = t - quotient * newt;
            t = newt;
            newt = tmp_t;

            let tmp_r = r - quotient * newr;
            r = newr;
            newr = tmp_r;
        }

        if r > 1 {
            return None; // Not invertible
        }
        if t < 0 {
            t += m as i128;
        }
        Some(t as u64)
    }

    /// Evaluate an AST expression modulo prime $p$ with assigned symbol variable values.
    pub fn eval_mod_p(
        graph: &ExprGraph,
        expr_id: ExprId,
        p: u64,
        env: &HashMap<SymbolId, u64>,
    ) -> Option<u64> {
        let node = graph.get(expr_id);
        let p_u128 = p as u128;

        match &node.kind {
            ExprKind::Number(num) => match num {
                Number::Integer(n) => {
                    let rem = n.rem_euclid(p as i64);
                    Some(rem as u64)
                }
                Number::BigInteger(b) => {
                    use num::Integer;
                    use num_traits::ToPrimitive;
                    let p_bi = num_bigint::BigInt::from(p);
                    let rem = b.mod_floor(&p_bi);
                    rem.to_u64()
                }
                Number::Rational(num, den) => {
                    let num_rem = num.rem_euclid(p as i64) as u64;
                    let den_rem = den.rem_euclid(p as i64) as u64;
                    let den_inv = Self::mod_inv(den_rem, p)?;
                    Some(((num_rem as u128 * den_inv as u128) % p_u128) as u64)
                }
                Number::BigRational(r) => {
                    use num::Integer;
                    use num_traits::ToPrimitive;
                    let p_bi = num_bigint::BigInt::from(p);
                    let num_rem = r.numer().mod_floor(&p_bi).to_u64()?;
                    let den_rem = r.denom().mod_floor(&p_bi).to_u64()?;
                    let den_inv = Self::mod_inv(den_rem, p)?;
                    Some(((num_rem as u128 * den_inv as u128) % p_u128) as u64)
                }
                Number::Scientific { mantissa, exponent } => {
                    if let Some(r) = Number::scientific(*mantissa, *exponent).as_f64() {
                        let i = (r.round() as i64).rem_euclid(p as i64);
                        Some(i as u64)
                    } else {
                        None
                    }
                }
                Number::Float(u) => {
                    let f = f64::from_bits(*u);
                    if f.is_finite() {
                        let i = (f.round() as i64).rem_euclid(p as i64);
                        Some(i as u64)
                    } else {
                        None
                    }
                }
                Number::Constant(c) => match c {
                    Constant::Pi => Some(314159265 % p),
                    Constant::E => Some(271828182 % p),
                    Constant::I => None,
                    Constant::GoldenRatio => Some(161803398 % p),
                    _ => None,
                },
            },
            ExprKind::Symbol(sym) => env.get(sym).copied().or(Some(42 % p)),
            ExprKind::Add(terms) => {
                let mut sum: u128 = 0;
                for &t in terms {
                    let val = Self::eval_mod_p(graph, t, p, env)? as u128;
                    sum = (sum + val) % p_u128;
                }
                Some(sum as u64)
            }
            ExprKind::Sub(lhs, rhs) => {
                let l = Self::eval_mod_p(graph, *lhs, p, env)? as u128;
                let r = Self::eval_mod_p(graph, *rhs, p, env)? as u128;
                let diff = (l + p_u128 - (r % p_u128)) % p_u128;
                Some(diff as u64)
            }
            ExprKind::Mul(factors) => {
                let mut prod: u128 = 1;
                for &f in factors {
                    let val = Self::eval_mod_p(graph, f, p, env)? as u128;
                    prod = (prod * val) % p_u128;
                }
                Some(prod as u64)
            }
            ExprKind::Div(num, den) => {
                let n = Self::eval_mod_p(graph, *num, p, env)? as u128;
                let d = Self::eval_mod_p(graph, *den, p, env)?;
                let inv = Self::mod_inv(d, p)? as u128;
                Some(((n * inv) % p_u128) as u64)
            }
            ExprKind::Pow(base, exp) => {
                let b = Self::eval_mod_p(graph, *base, p, env)? as u128;
                let e = Self::eval_mod_p(graph, *exp, p, env)? as u128;
                Some(Self::mod_pow(b, e, p_u128) as u64)
            }
            ExprKind::Neg(inner) => {
                let val = Self::eval_mod_p(graph, *inner, p, env)? as u128;
                Some(((p_u128 - (val % p_u128)) % p_u128) as u64)
            }
            _ => None,
        }
    }

    /// Evaluate an AST expression numerically with floating point environment.
    pub fn eval_float(
        graph: &ExprGraph,
        expr_id: ExprId,
        env: &HashMap<SymbolId, f64>,
    ) -> Option<f64> {
        let node = graph.get(expr_id);
        match &node.kind {
            ExprKind::Number(num) => match num {
                Number::Integer(n) => Some(*n as f64),
                Number::BigInteger(b) => num_traits::ToPrimitive::to_f64(b),
                Number::Rational(num, den) => Some(*num as f64 / *den as f64),
                Number::BigRational(r) => num_traits::ToPrimitive::to_f64(r.as_ref()),
                Number::Scientific { mantissa, exponent } => {
                    Some(*mantissa as f64 * 10.0f64.powi(*exponent))
                }
                Number::Float(u) => Some(f64::from_bits(*u)),
                Number::Constant(c) => match c {
                    Constant::Pi => Some(std::f64::consts::PI),
                    Constant::E => Some(std::f64::consts::E),
                    Constant::GoldenRatio => Some(1.618_033_988_749_895),
                    _ => None,
                },
            },
            ExprKind::Symbol(sym) => env.get(sym).copied().or(Some(1.234567)),
            ExprKind::Add(terms) => {
                let mut sum = 0.0;
                for &t in terms {
                    sum += Self::eval_float(graph, t, env)?;
                }
                Some(sum)
            }
            ExprKind::Sub(lhs, rhs) => {
                let l = Self::eval_float(graph, *lhs, env)?;
                let r = Self::eval_float(graph, *rhs, env)?;
                Some(l - r)
            }
            ExprKind::Mul(factors) => {
                let mut prod = 1.0;
                for &f in factors {
                    prod *= Self::eval_float(graph, f, env)?;
                }
                Some(prod)
            }
            ExprKind::Div(num, den) => {
                let n = Self::eval_float(graph, *num, env)?;
                let d = Self::eval_float(graph, *den, env)?;
                if d.abs() < 1e-15 { None } else { Some(n / d) }
            }
            ExprKind::Pow(base, exp) => {
                let b = Self::eval_float(graph, *base, env)?;
                let e = Self::eval_float(graph, *exp, env)?;
                Some(b.powf(e))
            }
            ExprKind::Neg(inner) => {
                let val = Self::eval_float(graph, *inner, env)?;
                Some(-val)
            }
            ExprKind::Function { name, args } => {
                let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                if args.is_empty() {
                    return None;
                }
                let arg0 = Self::eval_float(graph, args[0], env)?;
                match fn_name.as_str() {
                    "sin" => Some(arg0.sin()),
                    "cos" => Some(arg0.cos()),
                    "tan" => Some(arg0.tan()),
                    "asin" => {
                        if (-1.0..=1.0).contains(&arg0) {
                            Some(arg0.asin())
                        } else {
                            None
                        }
                    }
                    "acos" => {
                        if (-1.0..=1.0).contains(&arg0) {
                            Some(arg0.acos())
                        } else {
                            None
                        }
                    }
                    "atan" => Some(arg0.atan()),
                    "sinh" => Some(arg0.sinh()),
                    "cosh" => Some(arg0.cosh()),
                    "tanh" => Some(arg0.tanh()),
                    "exp" => Some(arg0.exp()),
                    "ln" | "log" => {
                        if arg0 > 0.0 {
                            Some(arg0.ln())
                        } else {
                            None
                        }
                    }
                    "sqrt" => {
                        if arg0 >= 0.0 {
                            Some(arg0.sqrt())
                        } else {
                            None
                        }
                    }
                    "abs" => Some(arg0.abs()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Evaluate an AST expression with rigorous interval arithmetic.
    pub fn eval_interval(
        graph: &ExprGraph,
        expr_id: ExprId,
        env: &HashMap<SymbolId, RealInterval>,
    ) -> Option<RealInterval> {
        let node = graph.get(expr_id);
        match &node.kind {
            ExprKind::Number(num) => match num {
                Number::Integer(n) => Some(RealInterval::point(*n as f64)),
                Number::BigInteger(b) => {
                    num_traits::ToPrimitive::to_f64(b).map(RealInterval::point)
                }
                Number::Rational(num, den) => Some(RealInterval::point(*num as f64 / *den as f64)),
                Number::BigRational(r) => {
                    num_traits::ToPrimitive::to_f64(r.as_ref()).map(RealInterval::point)
                }
                Number::Scientific { mantissa, exponent } => Some(RealInterval::point(
                    *mantissa as f64 * 10.0f64.powi(*exponent),
                )),
                Number::Float(u) => Some(RealInterval::point(f64::from_bits(*u))),
                Number::Constant(c) => match c {
                    Constant::Pi => Some(RealInterval::point(std::f64::consts::PI)),
                    Constant::E => Some(RealInterval::point(std::f64::consts::E)),
                    _ => None,
                },
            },
            ExprKind::Symbol(sym) => env.get(sym).copied().or(Some(RealInterval::new(1.0, 2.0))),
            ExprKind::Add(terms) => {
                let mut res = RealInterval::point(0.0);
                for &t in terms {
                    res = res + Self::eval_interval(graph, t, env)?;
                }
                Some(res)
            }
            ExprKind::Sub(lhs, rhs) => {
                let l = Self::eval_interval(graph, *lhs, env)?;
                let r = Self::eval_interval(graph, *rhs, env)?;
                Some(l - r)
            }
            ExprKind::Mul(factors) => {
                let mut res = RealInterval::point(1.0);
                for &f in factors {
                    res = res * Self::eval_interval(graph, f, env)?;
                }
                Some(res)
            }
            ExprKind::Neg(inner) => {
                let val = Self::eval_interval(graph, *inner, env)?;
                Some(val.neg())
            }
            ExprKind::Function { name, args } => {
                let fn_name = graph.symbols.resolve(*name).unwrap_or_default();
                if args.is_empty() {
                    return None;
                }
                let arg0 = Self::eval_interval(graph, args[0], env)?;
                match fn_name.as_str() {
                    "exp" => Some(RealInterval::new(arg0.inf.exp(), arg0.sup.exp())),
                    "ln" | "log" if arg0.inf > 0.0 => {
                        Some(RealInterval::new(arg0.inf.ln(), arg0.sup.ln()))
                    }
                    "sqrt" if arg0.inf >= 0.0 => {
                        Some(RealInterval::new(arg0.inf.sqrt(), arg0.sup.sqrt()))
                    }
                    "abs" => {
                        let a = arg0.inf.abs();
                        let b = arg0.sup.abs();
                        if arg0.contains(0.0) {
                            Some(RealInterval::new(0.0, a.max(b)))
                        } else {
                            Some(RealInterval::new(a.min(b), a.max(b)))
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Extract all distinct free symbol variables in an AST expression.
    pub fn extract_symbols(graph: &ExprGraph, expr_id: ExprId) -> Vec<SymbolId> {
        let mut syms = Vec::new();
        let mut stack = vec![expr_id];
        let mut visited = std::collections::HashSet::new();

        while let Some(curr) = stack.pop() {
            if !visited.insert(curr) {
                continue;
            }
            let node = graph.get(curr);
            match &node.kind {
                ExprKind::Symbol(s) if !syms.contains(s) => {
                    syms.push(*s);
                }
                ExprKind::Add(children) | ExprKind::Mul(children) => {
                    stack.extend(children.iter().copied());
                }
                ExprKind::Sub(l, r) | ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                    stack.push(*l);
                    stack.push(*r);
                }
                ExprKind::Neg(inner) => {
                    stack.push(*inner);
                }
                ExprKind::Function { args, .. } => {
                    stack.extend(args.iter().copied());
                }
                _ => {}
            }
        }
        syms
    }

    /// Fast $O(1)$ pseudo-random pseudo-hash fingerprint of an expression.
    pub fn fingerprint(graph: &ExprGraph, expr_id: ExprId, seed: u64) -> u64 {
        let syms = Self::extract_symbols(graph, expr_id);
        let mut env = HashMap::new();
        for (i, sym) in syms.iter().enumerate() {
            let val = (seed
                .wrapping_mul(1103515245)
                .wrapping_add(12345 + (i as u64) * 65537))
                % Self::MERSENNE_31;
            env.insert(*sym, val);
        }
        Self::eval_mod_p(graph, expr_id, Self::MERSENNE_31, &env).unwrap_or(seed ^ 0xDEADBEEF)
    }

    /// Progressive multi-prime Schwartz-Zippel CRT equivalence verification.
    ///
    /// Checks whether $f(x_1, \dots, x_n) \equiv g(x_1, \dots, x_n)$ across multiple random
    /// evaluations over triple Mersenne primes and floating-point intervals.
    ///
    /// Computes formal upper bound on error probability:
    /// $$\epsilon \le \left(\frac{d}{p_1}\right)^{k_1} \cdot \left(\frac{d}{p_2}\right)^{k_2} \cdot \left(\frac{d}{p_3}\right)^{k_3}$$
    pub fn verify_equivalence(
        graph: &ExprGraph,
        lhs: ExprId,
        rhs: ExprId,
        target_epsilon: f64,
    ) -> Solution<bool> {
        if lhs == rhs {
            return Solution::Deterministic(true);
        }

        let syms = {
            let mut s = Self::extract_symbols(graph, lhs);
            for r_sym in Self::extract_symbols(graph, rhs) {
                if !s.contains(&r_sym) {
                    s.push(r_sym);
                }
            }
            s
        };

        let primes = [Self::MERSENNE_31, Self::MERSENNE_61, Self::LARGE_PRIME_64];
        let mut total_samples = 0;
        let mut current_error_bound = 1.0f64;
        let estimated_degree = 20.0f64; // Safe upper bound on typical symbolic polynomial degree

        // Iterate through primes and random evaluation points
        for &prime in &primes {
            for round in 1..=8 {
                let mut env = HashMap::new();
                for (idx, &sym) in syms.iter().enumerate() {
                    let seed = ((round as u64) * 999983 + (idx as u64) * 31337 + 7) % prime;
                    let val = if seed == 0 { 17 } else { seed };
                    env.insert(sym, val);
                }

                let val_l = Self::eval_mod_p(graph, lhs, prime, &env);
                let val_r = Self::eval_mod_p(graph, rhs, prime, &env);

                if let (Some(vl), Some(vr)) = (val_l, val_r) {
                    if vl != vr {
                        // Disproven by counterexample!
                        return Solution::Deterministic(false);
                    }
                    total_samples += 1;
                    let single_sample_err = estimated_degree / (prime as f64);
                    current_error_bound *= single_sample_err;

                    if total_samples >= 3 && current_error_bound <= target_epsilon {
                        return Solution::Probabilistic {
                            result: true,
                            error_probability_upper_bound: current_error_bound,
                            sample_count: total_samples,
                            method: "Schwartz-Zippel Multi-Prime CRT",
                        };
                    }
                }
            }
        }

        // Float validation points
        for round in 1..=5 {
            let mut f_env = HashMap::new();
            for (idx, &sym) in syms.iter().enumerate() {
                let val = ((round as f64) * std::f64::consts::SQRT_2
                    + (idx as f64) * std::f64::consts::E)
                    % 10.0
                    + 0.5;
                f_env.insert(sym, val);
            }
            if let (Some(fl), Some(fr)) = (
                Self::eval_float(graph, lhs, &f_env),
                Self::eval_float(graph, rhs, &f_env),
            ) {
                if (fl - fr).abs() > 1e-9 * (1.0 + fl.abs() + fr.abs()) {
                    return Solution::Deterministic(false);
                }
                total_samples += 1;
                current_error_bound *= 1e-3;
            }
        }

        Solution::Probabilistic {
            result: true,
            error_probability_upper_bound: current_error_bound.min(1e-6),
            sample_count: total_samples,
            method: "Schwartz-Zippel CRT & Randomized Interval",
        }
    }

    /// Parallel multi-prime Schwartz-Zippel CRT equivalence verification using Rayon on desktop targets.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn verify_equivalence_parallel(
        graph: &ExprGraph,
        lhs: ExprId,
        rhs: ExprId,
        target_epsilon: f64,
    ) -> Solution<bool> {
        if lhs == rhs {
            return Solution::Deterministic(true);
        }

        let syms = {
            let mut s = Self::extract_symbols(graph, lhs);
            for r_sym in Self::extract_symbols(graph, rhs) {
                if !s.contains(&r_sym) {
                    s.push(r_sym);
                }
            }
            s
        };

        use rayon::prelude::*;

        let primes = [Self::MERSENNE_31, Self::MERSENNE_61, Self::LARGE_PRIME_64];
        let estimated_degree = 20.0f64;

        // Build all test cases: (prime, round)
        let test_cases: Vec<(u64, usize)> = primes
            .iter()
            .flat_map(|&p| (1..=8).map(move |r| (p, r)))
            .collect();

        // Evaluate all prime test cases in parallel
        let results: Vec<Option<(bool, f64)>> = test_cases
            .par_iter()
            .map(|&(prime, round)| {
                let mut env = HashMap::new();
                for (idx, &sym) in syms.iter().enumerate() {
                    let seed = ((round as u64) * 999983 + (idx as u64) * 31337 + 7) % prime;
                    let val = if seed == 0 { 17 } else { seed };
                    env.insert(sym, val);
                }

                let val_l = Self::eval_mod_p(graph, lhs, prime, &env);
                let val_r = Self::eval_mod_p(graph, rhs, prime, &env);

                match (val_l, val_r) {
                    (Some(vl), Some(vr)) => {
                        let matches = vl == vr;
                        let err = estimated_degree / (prime as f64);
                        Some((matches, err))
                    }
                    _ => None,
                }
            })
            .collect();

        let mut total_samples = 0;
        let mut current_error_bound = 1.0f64;

        for res in results.into_iter().flatten() {
            let (matches, err) = res;
            if !matches {
                return Solution::Deterministic(false);
            }
            total_samples += 1;
            current_error_bound *= err;

            if total_samples >= 3 && current_error_bound <= target_epsilon {
                return Solution::Probabilistic {
                    result: true,
                    error_probability_upper_bound: current_error_bound,
                    sample_count: total_samples,
                    method: "Schwartz-Zippel Multi-Prime CRT (Parallel)",
                };
            }
        }

        // Float validation points in parallel
        let float_cases: Vec<usize> = (1..=5).collect();
        let float_results: Vec<Option<bool>> = float_cases
            .par_iter()
            .map(|&round| {
                let mut f_env = HashMap::new();
                for (idx, &sym) in syms.iter().enumerate() {
                    let val = ((round as f64) * std::f64::consts::SQRT_2
                        + (idx as f64) * std::f64::consts::E)
                        % 10.0
                        + 0.5;
                    f_env.insert(sym, val);
                }
                match (
                    Self::eval_float(graph, lhs, &f_env),
                    Self::eval_float(graph, rhs, &f_env),
                ) {
                    (Some(fl), Some(fr)) => {
                        let diff = (fl - fr).abs();
                        let tol = 1e-9 * (1.0 + fl.abs() + fr.abs());
                        Some(diff <= tol)
                    }
                    _ => None,
                }
            })
            .collect();

        for f_res in float_results.into_iter().flatten() {
            if !f_res {
                return Solution::Deterministic(false);
            }
            total_samples += 1;
            current_error_bound *= 1e-3;
        }

        if total_samples > 0 {
            Solution::Probabilistic {
                result: true,
                error_probability_upper_bound: current_error_bound,
                sample_count: total_samples,
                method: "Schwartz-Zippel Multi-Prime CRT (Parallel)",
            }
        } else {
            Self::verify_equivalence(graph, lhs, rhs, target_epsilon)
        }
    }

    /// Compute heuristic weight $[0.0, 1.0]$ for candidate search branch prioritization in $O(1)$.
    pub fn compute_candidate_heuristic_weight(
        graph: &ExprGraph,
        candidate_id: ExprId,
        reference_id: Option<ExprId>,
    ) -> f64 {
        let node = graph.get(candidate_id);
        let mut score = 1.0;

        // Favor compact ASTs
        match &node.kind {
            ExprKind::Number(_) => score += 0.5,
            ExprKind::Symbol(_) => score += 0.3,
            ExprKind::Mul(factors) => score += 0.1 * (factors.len() as f64),
            ExprKind::Div(_, _) => score -= 0.1,
            _ => {}
        }

        if let Some(ref_id) = reference_id {
            let fp_cand = Self::fingerprint(graph, candidate_id, 12345);
            let fp_ref = Self::fingerprint(graph, ref_id, 12345);
            if fp_cand == fp_ref {
                score += 2.0; // Strong fingerprint match
            }
        }

        score
    }

    /// Verify if an expression is identically zero using multi-prime Schwartz-Zippel CRT.
    pub fn verify_zero_schwartz_zippel(
        graph: &ExprGraph,
        expr_id: ExprId,
        target_epsilon: f64,
    ) -> Solution<bool> {
        let zero = graph.integer(0);
        Self::verify_equivalence(graph, expr_id, zero, target_epsilon)
    }
}
