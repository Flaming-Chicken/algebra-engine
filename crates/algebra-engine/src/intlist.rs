//! List-valued integer functions: `divmod`, `digits`, `egcd`, `crt`, `continued_fraction`, `convergents`.
//!
//! All arithmetic is exact (`BigInt`). The result is text written as an equation, like the other integer functions.

use num::Integer;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Q = BigRational;

fn integer(q: &Q, what: &str) -> Result<BigInt, String> {
    if q.is_integer() {
        Ok(q.to_integer())
    } else {
        Err(format!("{what} must be an integer, got {}", show(q)))
    }
}

fn show(q: &Q) -> String {
    if q.is_integer() {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

fn join<T: ToString>(items: &[T]) -> String {
    items
        .iter()
        .map(T::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Extended Euclid: `(g, x, y)` with `a x + b y = g = gcd(a, b) >= 0`.
pub fn egcd(a: &BigInt, b: &BigInt) -> (BigInt, BigInt, BigInt) {
    let (mut old_r, mut r) = (a.clone(), b.clone());
    let (mut old_s, mut s) = (BigInt::one(), BigInt::zero());
    let (mut old_t, mut t) = (BigInt::zero(), BigInt::one());
    while !r.is_zero() {
        let q = &old_r / &r;
        (old_r, r) = (r.clone(), &old_r - &q * &r);
        (old_s, s) = (s.clone(), &old_s - &q * &s);
        (old_t, t) = (t.clone(), &old_t - &q * &t);
    }
    if old_r.is_negative() {
        (-old_r, -old_s, -old_t)
    } else {
        (old_r, old_s, old_t)
    }
}

/// Continued-fraction terms of a rational (floor convention, last term greater than one unless it is the only term).
pub fn continued_fraction_terms(q: &Q) -> Vec<BigInt> {
    let (mut n, mut d) = (q.numer().clone(), q.denom().clone());
    let mut terms = Vec::new();
    while !d.is_zero() {
        let (quotient, remainder) = n.div_mod_floor(&d);
        terms.push(quotient);
        n = d;
        d = remainder;
    }
    terms
}

/// The successive convergents `h_k / k_k` of a continued fraction.
pub fn convergents_of(terms: &[BigInt]) -> Vec<(BigInt, BigInt)> {
    let (mut h_prev, mut h) = (BigInt::zero(), BigInt::one());
    let (mut k_prev, mut k) = (BigInt::one(), BigInt::zero());
    let mut out = Vec::new();
    for a in terms {
        let next_h = a * &h + &h_prev;
        let next_k = a * &k + &k_prev;
        (h_prev, h) = (h, next_h);
        (k_prev, k) = (k, next_k);
        out.push((h.clone(), k.clone()));
    }
    out
}

/// Combine `x ≡ r_i (mod m_i)` (the moduli need not be coprime). `None` if the system has no solution.
pub fn crt(residues: &[BigInt], moduli: &[BigInt]) -> Option<(BigInt, BigInt)> {
    let (mut x, mut m) = (BigInt::zero(), BigInt::one());
    for (r, n) in residues.iter().zip(moduli) {
        let (g, p, _) = egcd(&m, n);
        let difference = r - &x;
        if !(&difference % &g).is_zero() {
            return None;
        }
        let lcm = &m / &g * n;
        let step = (&difference / &g * p).mod_floor(&(n / &g));
        x = (&x + &m * step).mod_floor(&lcm);
        m = lcm;
    }
    Some((x, m))
}

/// Evaluate a list-valued integer function. `args` are already-evaluated exact numbers or lists of them.
pub fn run(
    name: &str,
    args: &[Vec<Q>],
    scalar: impl Fn(usize) -> Result<Q, String>,
) -> Result<String, String> {
    match name {
        "divmod" => {
            let (a, b) = (
                integer(&scalar(0)?, "divmod")?,
                integer(&scalar(1)?, "divmod")?,
            );
            if b.is_zero() {
                return Err("divmod: division by zero".into());
            }
            let (q, r) = a.div_mod_floor(&b);
            Ok(format!("divmod({a}, {b}) = ({q}, {r})"))
        }
        "floordiv" => {
            let (a, b) = (scalar(0)?, scalar(1)?);
            if b.is_zero() {
                return Err("division by zero".into());
            }
            Ok(format!(
                "{} // {} = {}",
                show(&a),
                show(&b),
                (a / b).floor().to_integer()
            ))
        }
        "digits" => {
            let n = integer(&scalar(0)?, "digits")?;
            let base = if args.len() > 1 {
                integer(&scalar(1)?, "the base")?
            } else {
                BigInt::from(10)
            };
            let base_small = base
                .to_u32()
                .filter(|b| (2..=1_000_000).contains(b))
                .ok_or("the base must be an integer between 2 and 1000000")?;
            let base = BigInt::from(base_small);
            let negative = n.is_negative();
            let mut m = n.abs();
            let mut digits = Vec::new();
            if m.is_zero() {
                digits.push(BigInt::zero());
            }
            while !m.is_zero() {
                let (q, r) = m.div_rem(&base);
                digits.push(r);
                m = q;
            }
            digits.reverse();
            Ok(format!(
                "digits({}{}) = [{}]",
                if negative { "-" } else { "" },
                scalar(0)?.to_integer().abs(),
                join(&digits)
            ))
        }
        "egcd" => {
            let (a, b) = (integer(&scalar(0)?, "egcd")?, integer(&scalar(1)?, "egcd")?);
            let (g, x, y) = egcd(&a, &b);
            Ok(format!(
                "egcd({a}, {b}) = ({g}, {x}, {y})   [{a}·({x}) + {b}·({y}) = {g}]"
            ))
        }
        "continued_fraction" => {
            let q = scalar(0)?;
            let terms = continued_fraction_terms(&q);
            let (head, tail) = terms.split_first().ok_or("empty continued fraction")?;
            Ok(format!(
                "continued_fraction({}) = [{head}{}]",
                show(&q),
                if tail.is_empty() {
                    String::new()
                } else {
                    format!("; {}", join(tail))
                }
            ))
        }
        "convergents" => {
            let q = scalar(0)?;
            let list = convergents_of(&continued_fraction_terms(&q));
            let shown: Vec<String> = list
                .iter()
                .map(|(h, k)| {
                    if k.is_one() {
                        h.to_string()
                    } else {
                        format!("{h}/{k}")
                    }
                })
                .collect();
            Ok(format!(
                "convergents({}) = [{}]",
                show(&q),
                shown.join(", ")
            ))
        }
        "crt" => {
            let (residues, moduli) = match args {
                [r, m] if r.len() == m.len() && !r.is_empty() => (r, m),
                _ => {
                    return Err(
                        "crt([r1, r2, ...], [m1, m2, ...]) takes two lists of equal length".into(),
                    );
                }
            };
            let residues: Vec<BigInt> = residues
                .iter()
                .map(|q| integer(q, "a residue"))
                .collect::<Result<_, _>>()?;
            let moduli: Vec<BigInt> = moduli
                .iter()
                .map(|q| {
                    integer(q, "a modulus").and_then(|m| {
                        if m.is_positive() {
                            Ok(m)
                        } else {
                            Err("the moduli must be positive".to_string())
                        }
                    })
                })
                .collect::<Result<_, _>>()?;
            match crt(&residues, &moduli) {
                Some((x, m)) => Ok(format!("crt = {x} (mod {m})")),
                None => Err("the congruences are inconsistent: no solution".into()),
            }
        }
        other => Err(format!("unknown integer function {other}")),
    }
}
