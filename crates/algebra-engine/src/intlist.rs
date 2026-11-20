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
        "primes" => {
            let n = integer(&scalar(0)?, "primes")?;
            let limit = n
                .to_usize()
                .filter(|l| *l <= 2_000_000)
                .ok_or("primes(n) needs 0 <= n <= 2000000")?;
            let composite = crate::discrete::sieve(limit);
            let primes: Vec<String> = (2..=limit)
                .filter(|&i| !composite[i])
                .map(|i| i.to_string())
                .collect();
            Ok(format!("primes({limit}) = [{}]", primes.join(", ")))
        }
        "expected_value" | "variance_discrete" => {
            let (values, probabilities) = (
                args.first().ok_or("a list of values is needed")?,
                args.get(1).ok_or("a list of probabilities is needed")?,
            );
            if values.len() != probabilities.len() || values.is_empty() {
                return Err(
                    "the values and the probabilities must be lists of the same length".into(),
                );
            }
            let total = probabilities.iter().fold(Q::zero(), |s, p| s + p);
            if total != Q::one() || probabilities.iter().any(|p| *p < Q::zero()) {
                return Err(format!(
                    "the probabilities must be non-negative and sum to 1 (they sum to {})",
                    show(&total)
                ));
            }
            let mean = values
                .iter()
                .zip(probabilities)
                .fold(Q::zero(), |s, (x, p)| s + x * p);
            if name == "expected_value" {
                return Ok(format!("expected value = {}", show(&mean)));
            }
            let second = values
                .iter()
                .zip(probabilities)
                .fold(Q::zero(), |s, (x, p)| s + x * x * p);
            Ok(format!("variance = {}", show(&(second - &mean * &mean))))
        }
        "is_square" => {
            let n = integer(&scalar(0)?, "is_square")?;
            let square = !n.is_negative() && {
                let r = n.sqrt();
                &r * &r == n
            };
            Ok(format!("is_square({n}) = {square}"))
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

// ---------------------------------------------------------------------------------------------------------------
// Radix conversion, listings, Pell and linear Diophantine equations (text arguments)
// ---------------------------------------------------------------------------------------------------------------

/// `dec2bin(10)`, `bin2dec(1010)`, `dec2hex(255)`, `hex2dec(ff)`, `dec2oct`, `oct2dec`: the argument is read as text.
pub fn radix(name: &str, argument: &str) -> Result<String, String> {
    let text = argument
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0b");
    let (from, to) = match name {
        "dec2bin" => (10, 2),
        "bin2dec" => (2, 10),
        "dec2hex" | "hex" => (10, 16),
        "hex2dec" => (16, 10),
        "dec2oct" | "oct" => (10, 8),
        "oct2dec" => (8, 10),
        other => return Err(format!("unknown conversion {other}")),
    };
    let value = BigInt::parse_bytes(text.as_bytes(), from)
        .ok_or_else(|| format!("{argument} is not a base-{from} integer"))?;
    Ok(format!(
        "{name}({}) = {}",
        argument.trim(),
        value.to_str_radix(to)
    ))
}

fn list_of(items: &[Q]) -> Vec<String> {
    items.iter().map(show).collect()
}

fn subsets_of(items: &[String]) -> Vec<Vec<String>> {
    let n = items.len();
    let mut out: Vec<Vec<String>> = (0..1usize << n)
        .map(|mask| {
            (0..n)
                .filter(|i| (mask >> i) & 1 == 1)
                .map(|i| items[i].clone())
                .collect()
        })
        .collect();
    out.sort_by_key(|s| s.len());
    out
}

fn permutations_of(items: &[String]) -> Vec<Vec<String>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut out = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let head = rest.remove(i);
        for mut tail in permutations_of(&rest) {
            tail.insert(0, head.clone());
            out.push(tail);
        }
    }
    out
}

fn show_list(sets: &[Vec<String>]) -> String {
    format!(
        "[{}]",
        sets.iter()
            .map(|s| format!("[{}]", s.join(", ")))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// `subsets`, `permutations_list`, `cycle_type`, `next_permutation` of a list.
pub fn listing(name: &str, items: &[Q]) -> Result<String, String> {
    let labels = list_of(items);
    match name {
        "subsets" | "powerset" | "power_set" => {
            if items.len() > 12 {
                return Err("subsets lists at most 12 elements".into());
            }
            Ok(format!("subsets = {}", show_list(&subsets_of(&labels))))
        }
        "permutations_list" => {
            if items.len() > 7 {
                return Err("permutations_list lists at most 7 elements".into());
            }
            Ok(format!(
                "permutations = {}",
                show_list(&permutations_of(&labels))
            ))
        }
        "cycle_type" => {
            // One-line notation of a permutation of 1..n.
            let n = items.len();
            let image: Vec<usize> = items
                .iter()
                .map(|q| {
                    q.to_integer()
                        .to_usize()
                        .filter(|&v| (1..=n).contains(&v) && q.is_integer())
                        .ok_or_else(|| "a permutation of 1..n is expected".to_string())
                })
                .collect::<Result<_, _>>()?;
            if image
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != n
            {
                return Err("the list is not a permutation".into());
            }
            let mut seen = vec![false; n];
            let mut cycles: Vec<(usize, Vec<usize>)> = Vec::new();
            for start in 0..n {
                if seen[start] {
                    continue;
                }
                let mut at = start;
                let mut cycle = Vec::new();
                while !seen[at] {
                    seen[at] = true;
                    cycle.push(at + 1);
                    at = image[at] - 1;
                }
                cycles.push((cycle.len(), cycle));
            }
            cycles.sort_by_key(|c| std::cmp::Reverse(c.0));
            let shown: Vec<String> = cycles
                .iter()
                .map(|(_, c)| {
                    format!(
                        "({})",
                        c.iter().map(usize::to_string).collect::<Vec<_>>().join(" ")
                    )
                })
                .collect();
            let lengths: Vec<String> = cycles.iter().map(|(l, _)| l.to_string()).collect();
            Ok(format!(
                "cycle_type = [{}], cycles {}",
                lengths.join(", "),
                shown.join("")
            ))
        }
        "next_permutation" => {
            let mut values: Vec<Q> = items.to_vec();
            let n = values.len();
            if n < 2 {
                return Ok(format!("next_permutation = [{}]", labels.join(", ")));
            }
            let Some(pivot) = (0..n - 1).rev().find(|&i| values[i] < values[i + 1]) else {
                values.sort();
                return Ok(format!(
                    "next_permutation = [{}] (wrapped to the first)",
                    list_of(&values).join(", ")
                ));
            };
            let swap = (pivot + 1..n)
                .rev()
                .find(|&j| values[j] > values[pivot])
                .unwrap_or(pivot);
            values.swap(pivot, swap);
            values[pivot + 1..].reverse();
            Ok(format!(
                "next_permutation = [{}]",
                list_of(&values).join(", ")
            ))
        }
        other => Err(format!("unknown listing {other}")),
    }
}

/// The fundamental solution of `x^2 - d y^2 = 1` (`d` not a square), by the continued fraction of `sqrt(d)`.
pub fn pell(d: &BigInt) -> Result<(BigInt, BigInt), String> {
    if !d.is_positive() {
        return Err("pell needs a positive d".into());
    }
    let a0 = d.sqrt();
    if &a0 * &a0 == *d {
        return Err(format!(
            "{d} is a perfect square: x^2 - {d} y^2 = 1 has only trivial solutions"
        ));
    }
    let (mut m, mut dd, mut a) = (BigInt::zero(), BigInt::one(), a0.clone());
    let (mut h_prev, mut h) = (BigInt::one(), a0.clone());
    let (mut k_prev, mut k) = (BigInt::zero(), BigInt::one());
    for _ in 0..100_000 {
        if &h * &h - d * &k * &k == BigInt::one() {
            return Ok((h, k));
        }
        m = &dd * &a - &m;
        dd = (d - &m * &m) / &dd;
        a = (&a0 + &m) / &dd;
        let next_h = &a * &h + &h_prev;
        let next_k = &a * &k + &k_prev;
        (h_prev, h) = (h, next_h);
        (k_prev, k) = (k, next_k);
    }
    Err("no solution found within the iteration limit".into())
}

/// The integer solutions of `a x + b y = c`.
pub fn linear_diophantine(
    a: &BigInt,
    b: &BigInt,
    c: &BigInt,
    x: &str,
    y: &str,
) -> Result<String, String> {
    if a.is_zero() && b.is_zero() {
        return Err("both coefficients are zero".into());
    }
    let (g, s, t) = egcd(a, b);
    if !(c % &g).is_zero() {
        return Err(format!(
            "no integer solutions: gcd({a}, {b}) = {g} does not divide {c}"
        ));
    }
    let scale = c / &g;
    let (x0, y0) = (&s * &scale, &t * &scale);
    let (dx, dy) = (b / &g, -(a / &g));
    // x = x0 + dx t, y = y0 + dy t
    let term = |base: &BigInt, step: &BigInt| -> String {
        match (base.is_zero(), step.is_zero()) {
            (_, true) => base.to_string(),
            (true, false) => format!("{step}·t"),
            (false, false) => format!(
                "{base} {} {}·t",
                if step.is_negative() { "-" } else { "+" },
                step.abs()
            ),
        }
    };
    Ok(format!(
        "{x} = {}, {y} = {}  (t any integer)",
        term(&x0, &dx),
        term(&y0, &dy)
    ))
}

/// `subsets` and `permutations_list` of arbitrary labels (numbers or symbols).
pub fn listing_text(name: &str, labels: &[String]) -> Result<String, String> {
    match name {
        "subsets" | "powerset" | "power_set" => {
            if labels.len() > 12 {
                return Err("subsets lists at most 12 elements".into());
            }
            Ok(format!("subsets = {}", show_list(&subsets_of(labels))))
        }
        "permutations_list" => {
            if labels.len() > 7 {
                return Err("permutations_list lists at most 7 elements".into());
            }
            Ok(format!(
                "permutations = {}",
                show_list(&permutations_of(labels))
            ))
        }
        other => Err(format!("unknown listing {other}")),
    }
}
