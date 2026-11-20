//! Exact evaluation of discrete-mathematics functions on integer (or rational) arguments:
//! `fibonacci`, `lucas`, `catalan`, `bell`, `partitions`, `stirling1`, `stirling2`, `derangements`, `permutations`,
//! `combinations`, `multinomial`, `bernoulli`, `harmonic`, `primepi`, `prime`, `legendre`, `jacobi`, `isqrt`, `sign`,
//! `max`, `min`, `round`, `gcd`, `lcm`, `digitsum`, `digitcount`, `collatz`.
//!
//! Every function is guarded by a size limit so that a careless argument cannot run for minutes.

use num::Integer;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Q = BigRational;

fn int(n: u64) -> BigInt {
    BigInt::from(n)
}

fn as_u64(q: &Q, limit: u64) -> Option<u64> {
    if !q.is_integer() {
        return None;
    }
    q.to_integer().to_u64().filter(|&n| n <= limit)
}

fn as_integer(q: &Q) -> Option<BigInt> {
    q.is_integer().then(|| q.to_integer())
}

fn factorial(n: u64) -> BigInt {
    (2..=n).fold(BigInt::one(), |p, k| p * k)
}

fn binomial(n: u64, k: u64) -> BigInt {
    if k > n {
        return BigInt::zero();
    }
    let k = k.min(n - k);
    let mut result = BigInt::one();
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

fn fibonacci_pair(n: u64) -> (BigInt, BigInt) {
    // Fast doubling: (F(n), F(n+1)).
    if n == 0 {
        return (BigInt::zero(), BigInt::one());
    }
    let (a, b) = fibonacci_pair(n / 2);
    let c = &a * (&b * 2 - &a);
    let d = &a * &a + &b * &b;
    if n.is_multiple_of(2) {
        (c, d)
    } else {
        (d.clone(), c + d)
    }
}

fn partitions(n: u64) -> BigInt {
    // Euler's pentagonal number recurrence.
    let n = n as usize;
    let mut p: Vec<BigInt> = vec![BigInt::zero(); n + 1];
    p[0] = BigInt::one();
    for m in 1..=n {
        let mut total = BigInt::zero();
        let mut k = 1i64;
        loop {
            let (g1, g2) = (
                (k * (3 * k - 1) / 2) as usize,
                (k * (3 * k + 1) / 2) as usize,
            );
            if g1 > m {
                break;
            }
            let sign = if k % 2 == 1 { 1 } else { -1 };
            total += &p[m - g1] * sign;
            if g2 <= m {
                total += &p[m - g2] * sign;
            }
            k += 1;
        }
        p[m] = total;
    }
    p[n].clone()
}

fn bell(n: u64) -> BigInt {
    // Bell triangle.
    let n = n as usize;
    let mut row: Vec<BigInt> = vec![BigInt::one()];
    for _ in 0..n {
        let mut next = vec![row[row.len() - 1].clone()];
        for v in &row {
            let last = next[next.len() - 1].clone();
            next.push(last + v);
        }
        row = next;
    }
    row[0].clone()
}

fn stirling2(n: u64, k: u64) -> BigInt {
    let (n, k) = (n as usize, k as usize);
    let mut s = vec![vec![BigInt::zero(); k + 1]; n + 1];
    s[0][0] = BigInt::one();
    for i in 1..=n {
        for j in 1..=k.min(i) {
            s[i][j] = &s[i - 1][j - 1] + &s[i - 1][j] * j as u64;
        }
    }
    s[n][k].clone()
}

/// Signed Stirling numbers of the first kind.
fn stirling1(n: u64, k: u64) -> BigInt {
    let (n, k) = (n as usize, k as usize);
    let mut s = vec![vec![BigInt::zero(); k + 1]; n + 1];
    s[0][0] = BigInt::one();
    for i in 1..=n {
        for j in 1..=k.min(i) {
            s[i][j] = &s[i - 1][j - 1] - &s[i - 1][j] * (i as u64 - 1);
        }
    }
    s[n][k].clone()
}

fn derangements(n: u64) -> BigInt {
    let (mut a, mut b) = (BigInt::one(), BigInt::zero()); // D0, D1
    if n == 0 {
        return a;
    }
    for i in 2..=n {
        let next = (&a + &b) * (i - 1);
        a = b;
        b = next;
    }
    b
}

fn bernoulli(n: u64) -> Q {
    let n = n as usize;
    let mut b: Vec<Q> = vec![Q::one()];
    for m in 1..=n {
        let mut sum = Q::zero();
        let mut c = BigInt::one();
        for (k, bk) in b.iter().enumerate().take(m) {
            sum += Q::from_integer(c.clone()) * bk;
            c = c * BigInt::from((m + 1 - k) as u64) / BigInt::from((k + 1) as u64);
        }
        b.push(-sum / Q::from_integer(BigInt::from((m + 1) as u64)));
    }
    // The convention B_1 = -1/2.
    b[n].clone()
}

pub(crate) fn sieve(limit: usize) -> Vec<bool> {
    let mut composite = vec![false; limit + 1];
    composite[0] = true;
    if limit >= 1 {
        composite[1] = true;
    }
    let mut i = 2;
    while i * i <= limit {
        if !composite[i] {
            let mut j = i * i;
            while j <= limit {
                composite[j] = true;
                j += i;
            }
        }
        i += 1;
    }
    composite
}

fn jacobi(a: &BigInt, n: &BigInt) -> Option<i32> {
    if !n.is_positive() || n.is_even() {
        return None;
    }
    let (mut a, mut n) = (a.mod_floor(n), n.clone());
    let mut result = 1;
    while !a.is_zero() {
        while a.is_even() {
            a >>= 1;
            let r = (&n % 8u32).to_u32().unwrap_or(0);
            if r == 3 || r == 5 {
                result = -result;
            }
        }
        std::mem::swap(&mut a, &mut n);
        if (&a % 4u32).to_u32() == Some(3) && (&n % 4u32).to_u32() == Some(3) {
            result = -result;
        }
        a = a.mod_floor(&n);
    }
    Some(if n.is_one() { result } else { 0 })
}

fn mul_mod(a: u128, b: u128, m: u128) -> u128 {
    a * b % m
}

fn pow_mod(mut base: u128, mut exponent: u128, modulus: u128) -> u128 {
    let mut result = 1 % modulus;
    base %= modulus;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = mul_mod(result, base, modulus);
        }
        base = mul_mod(base, base, modulus);
        exponent >>= 1;
    }
    result
}

fn factors_u64(n: u64) -> Vec<(u64, u32)> {
    crate::numbertheory::integer_prime_factors(n)
}

fn carmichael(n: u64) -> BigInt {
    use num::Integer as _;
    let mut lambda = BigInt::one();
    for (p, k) in factors_u64(n) {
        let value = if p == 2 {
            match k {
                1 => 1u64,
                2 => 2,
                _ => 1u64 << (k - 2),
            }
        } else {
            p.pow(k - 1) * (p - 1)
        };
        lambda = lambda.lcm(&BigInt::from(value));
    }
    lambda
}

/// The multiplicative order of `a` modulo `n` (`gcd(a, n) = 1`).
fn order_mod(a: u64, n: u64) -> Option<u64> {
    use num::Integer as _;
    if n < 2 || a.gcd(&n) != 1 {
        return None;
    }
    let lambda = u64::try_from(carmichael(n)).ok()?;
    // The order divides lambda: strip prime factors while the power stays 1.
    let mut order = lambda;
    for (p, _) in factors_u64(lambda) {
        while order.is_multiple_of(p) && pow_mod(a as u128, (order / p) as u128, n as u128) == 1 {
            order /= p;
        }
    }
    Some(order)
}

/// A square root of `a` modulo the odd prime `p` (the smaller one), by Tonelli-Shanks.
fn sqrt_mod(a: u64, p: u64) -> Option<u64> {
    if p == 2 {
        return Some(a % 2);
    }
    if p < 3 || !crate::numbertheory::is_prime(p) {
        return None;
    }
    let a = (a % p) as u128;
    let p128 = p as u128;
    if a == 0 {
        return Some(0);
    }
    if pow_mod(a, (p128 - 1) / 2, p128) != 1 {
        return None;
    }
    let (mut q, mut s) = (p128 - 1, 0u32);
    while q % 2 == 0 {
        q /= 2;
        s += 1;
    }
    let mut z = 2u128;
    while pow_mod(z, (p128 - 1) / 2, p128) != p128 - 1 {
        z += 1;
    }
    let (mut m, mut c) = (s, pow_mod(z, q, p128));
    let (mut t, mut r) = (pow_mod(a, q, p128), pow_mod(a, q.div_ceil(2), p128));
    while t != 1 {
        let mut i = 0;
        let mut x = t;
        while x != 1 {
            x = mul_mod(x, x, p128);
            i += 1;
        }
        let b = pow_mod(c, 1u128 << (m - i - 1), p128);
        m = i;
        c = mul_mod(b, b, p128);
        t = mul_mod(t, c, p128);
        r = mul_mod(r, b, p128);
    }
    let other = p128 - r;
    Some(r.min(other) as u64)
}

/// The least `x >= 0` with `g^x = h (mod p)`, by baby-step giant-step (`p` up to about 10^12).
fn discrete_log(g: u64, h: u64, p: u64) -> Option<u64> {
    if !(2..=1_000_000_000_000).contains(&p) {
        return None;
    }
    let (g, h, m) = (g as u128 % p as u128, h as u128 % p as u128, p as u128);
    let n = (p as f64).sqrt().ceil() as u128 + 1;
    let mut table = std::collections::HashMap::new();
    let mut value = 1u128;
    for j in 0..n {
        table.entry(value).or_insert(j);
        value = mul_mod(value, g, m);
    }
    let factor = pow_mod(g, m - 2, m); // g^-1 for a prime modulus
    let giant = pow_mod(factor, n, m);
    let mut gamma = h;
    for i in 0..n {
        if let Some(&j) = table.get(&gamma) {
            return Some((i * n + j) as u64);
        }
        gamma = mul_mod(gamma, giant, m);
    }
    None
}

/// Evaluate `name(args)` exactly, or `None` if the function is not one of these or the arguments are out of range.
pub fn evaluate(name: &str, args: &[Q]) -> Option<Q> {
    let integer_result = |n: BigInt| Some(Q::from_integer(n));
    let lowered = name.to_ascii_lowercase();
    match (lowered.as_str(), args) {
        ("fibonacci" | "fib", [n]) => integer_result(fibonacci_pair(as_u64(n, 200_000)?).0),
        ("lucas", [n]) => {
            let n = as_u64(n, 200_000)?;
            let (a, b) = fibonacci_pair(n);
            // L(n) = 2 F(n+1) - F(n)
            integer_result(b * 2 - a)
        }
        ("double_factorial" | "factorial2" | "semifactorial", [n]) => {
            let n = n
                .to_integer()
                .to_i64()
                .filter(|v| (-1..=200_000).contains(v))?;
            if !n_is_int(&args[0]) {
                return None;
            }
            integer_result((2..=n).rev().step_by(2).fold(BigInt::one(), |p, v| p * v))
        }
        ("binomial_cdf", [n, p, k]) => {
            let n = as_u64(n, 5000)?;
            if *p < Q::zero() || *p > Q::one() || !k.is_integer() {
                return None;
            }
            let k = k.to_integer().to_i64()?;
            if k < 0 {
                return Some(Q::zero());
            }
            let k = (k as u64).min(n);
            let q = Q::one() - p;
            let power = |base: &Q, e: u64| (0..e).fold(Q::one(), |acc, _| acc * base);
            Some((0..=k).fold(Q::zero(), |sum, i| {
                sum + Q::from_integer(binomial(n, i)) * power(p, i) * power(&q, n - i)
            }))
        }
        ("hypergeometric_pmf", [k, total, successes, draws]) => {
            let (k, total, successes, draws) = (
                as_u64(k, 100_000)?,
                as_u64(total, 100_000)?,
                as_u64(successes, 100_000)?,
                as_u64(draws, 100_000)?,
            );
            if successes > total
                || draws > total
                || k > successes
                || draws < k
                || draws - k > total - successes
            {
                return Some(Q::zero());
            }
            Some(Q::new(
                binomial(successes, k) * binomial(total - successes, draws - k),
                binomial(total, draws),
            ))
        }
        ("z_score" | "zscore", [x, mean, sd]) if !sd.is_zero() => Some((x - mean) / sd),
        ("bayes", [prior, sensitivity, false_positive]) => {
            let joint = prior * sensitivity;
            let denominator = &joint + (Q::one() - prior) * false_positive;
            (!denominator.is_zero()).then(|| joint / denominator)
        }
        ("tribonacci", [n]) => {
            let n = as_u64(n, 100_000)?;
            let (mut a, mut b, mut c) = (BigInt::zero(), BigInt::zero(), BigInt::one());
            for _ in 0..n {
                (a, b, c) = (b.clone(), c.clone(), &a + &b + &c);
            }
            integer_result(a)
        }
        ("compositions", [n]) => {
            let n = as_u64(n, 100_000)?;
            if n == 0 {
                return integer_result(BigInt::one());
            }
            integer_result(BigInt::one() << (n - 1) as usize)
        }
        ("compositions", [n, k]) => {
            let (n, k) = (as_u64(n, 100_000)?, as_u64(k, 100_000)?);
            if k == 0 || k > n {
                return Some(Q::from_integer(BigInt::from((n == 0 && k == 0) as u8)));
            }
            integer_result(binomial(n - 1, k - 1))
        }
        ("catalan", [n]) => {
            let n = as_u64(n, 20_000)?;
            integer_result(binomial(2 * n, n) / (n + 1))
        }
        ("bell", [n]) => integer_result(bell(as_u64(n, 1500)?)),
        ("partitions" | "partition_number", [n]) => integer_result(partitions(as_u64(n, 5000)?)),
        ("stirling2", [n, k]) => integer_result(stirling2(as_u64(n, 600)?, as_u64(k, 600)?)),
        ("stirling1", [n, k]) => integer_result(stirling1(as_u64(n, 600)?, as_u64(k, 600)?)),
        ("derangements" | "subfactorial", [n]) => integer_result(derangements(as_u64(n, 20_000)?)),
        ("permutations" | "perm" | "npr", [n, k]) => {
            let (n, k) = (as_u64(n, 100_000)?, as_u64(k, 100_000)?);
            if k > n {
                return Some(Q::zero());
            }
            integer_result((n - k + 1..=n).fold(BigInt::one(), |p, v| p * v))
        }
        ("combinations" | "choose" | "binomial" | "ncr", [n, k]) => {
            let (n, k) = (as_u64(n, 200_000)?, as_u64(k, 200_000)?);
            integer_result(binomial(n, k))
        }
        ("multinomial", ks) if !ks.is_empty() => {
            let parts: Vec<u64> = ks
                .iter()
                .map(|k| as_u64(k, 20_000))
                .collect::<Option<_>>()?;
            let total: u64 = parts.iter().sum();
            let denominator = parts.iter().fold(BigInt::one(), |p, &k| p * factorial(k));
            integer_result(factorial(total) / denominator)
        }
        ("bernoulli", [n]) => Some(bernoulli(as_u64(n, 400)?)),
        ("harmonic", [n]) => {
            let n = as_u64(n, 20_000)?;
            Some((1..=n).fold(Q::zero(), |s, k| s + Q::new(BigInt::one(), int(k))))
        }
        ("primepi" | "prime_pi", [n]) => {
            let n = as_u64(n, 20_000_000)? as usize;
            Some(Q::from_integer(BigInt::from(
                sieve(n).iter().filter(|c| !**c).count(),
            )))
        }
        ("prime" | "nthprime", [n]) => {
            let n = as_u64(n, 1_000_000)? as usize;
            if n == 0 {
                return None;
            }
            // p_n < n (ln n + ln ln n) for n >= 6.
            let bound = if n < 6 {
                15
            } else {
                (n as f64 * ((n as f64).ln() + (n as f64).ln().ln())).ceil() as usize + 10
            };
            let composite = sieve(bound);
            let p = composite
                .iter()
                .enumerate()
                .filter(|(_, c)| !**c)
                .map(|(i, _)| i)
                .nth(n - 1)?;
            integer_result(BigInt::from(p))
        }
        ("legendre" | "jacobi", [a, n]) => {
            let (a, n) = (as_integer(a)?, as_integer(n)?);
            Some(Q::from_integer(BigInt::from(jacobi(&a, &n)?)))
        }
        ("carmichael", [n]) => integer_result(carmichael(as_u64(n, u64::MAX >> 1)?.max(1))),
        ("radical" | "rad", [n]) => {
            let n = as_u64(n, u64::MAX >> 1)?;
            if n == 0 {
                return None;
            }
            integer_result(
                factors_u64(n)
                    .iter()
                    .fold(BigInt::one(), |r, &(p, _)| r * p),
            )
        }
        ("order_mod" | "multiplicative_order", [a, n]) => integer_result(BigInt::from(order_mod(
            as_u64(a, u64::MAX >> 1)?,
            as_u64(n, u64::MAX >> 1)?,
        )?)),
        ("sqrt_mod", [a, p]) => integer_result(BigInt::from(sqrt_mod(
            as_u64(a, u64::MAX >> 1)?,
            as_u64(p, u64::MAX >> 1)?,
        )?)),
        ("discrete_log" | "dlog", [g, h, p]) => integer_result(BigInt::from(discrete_log(
            as_u64(g, u64::MAX >> 1)?,
            as_u64(h, u64::MAX >> 1)?,
            as_u64(p, u64::MAX >> 1)?,
        )?)),
        ("gray_code" | "gray", [n]) => {
            let n = as_u64(n, u64::MAX >> 1)?;
            integer_result(BigInt::from(n ^ (n >> 1)))
        }
        ("isqrt", [n]) => {
            let n = as_integer(n)?;
            if n.is_negative() {
                return None;
            }
            integer_result(num::integer::Roots::sqrt(&n))
        }
        ("sign", [x]) => Some(if x.is_zero() {
            Q::zero()
        } else if x.is_positive() {
            Q::one()
        } else {
            -Q::one()
        }),
        ("max", xs) if xs.len() >= 2 => xs.iter().max().cloned(),
        ("min", xs) if xs.len() >= 2 => xs.iter().min().cloned(),
        ("round", [x]) => {
            // Nearest integer, halves away from zero.
            let half = Q::new(BigInt::one(), int(2));
            let shifted = if x.is_negative() {
                x - &half
            } else {
                x + &half
            };
            Some(Q::from_integer(shifted.trunc().to_integer()))
        }
        ("gcd", xs) if xs.len() >= 3 => {
            let ints: Vec<BigInt> = xs.iter().map(as_integer).collect::<Option<_>>()?;
            integer_result(ints.iter().fold(BigInt::zero(), |g, v| g.gcd(v)))
        }
        ("lcm", xs) if xs.len() >= 2 => {
            let ints: Vec<BigInt> = xs.iter().map(as_integer).collect::<Option<_>>()?;
            integer_result(ints.iter().fold(BigInt::one(), |l, v| {
                if v.is_zero() {
                    BigInt::zero()
                } else {
                    l.lcm(v)
                }
            }))
        }
        ("digitsum" | "digit_sum" | "sum_of_digits", [n]) => {
            let n = as_integer(n)?.abs();
            integer_result(BigInt::from(
                n.to_string()
                    .bytes()
                    .map(|b| (b - b'0') as u64)
                    .sum::<u64>(),
            ))
        }
        ("digitcount" | "ndigits", [n]) => {
            let n = as_integer(n)?.abs();
            integer_result(BigInt::from(n.to_string().len()))
        }
        ("collatz", [n]) => {
            let mut v = as_integer(n)?;
            if !v.is_positive() {
                return None;
            }
            let mut steps = 0u64;
            while !v.is_one() {
                v = if v.is_even() { v / 2 } else { v * 3 + 1 };
                steps += 1;
                if steps > 100_000 {
                    return None;
                }
            }
            integer_result(BigInt::from(steps))
        }
        _ => None,
    }
}

fn n_is_int(q: &Q) -> bool {
    q.is_integer()
}
