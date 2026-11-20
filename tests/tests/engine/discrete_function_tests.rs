//! Discrete-mathematics functions against brute force and defining identities (the oracle never uses the formula
//! under test).

use algebra_engine::session::UraeSession;

fn run(q: &str) -> String {
    let r = UraeSession::new().execute_line(q);
    assert!(!r.is_error, "{q}: {}", r.output_text);
    r.output_text
}

fn int(q: &str) -> i128 {
    let text = run(q);
    text.parse().unwrap_or_else(|_| panic!("{q} -> {text}"))
}

fn permutations(items: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
    if k == items.len() {
        out.push(items.clone());
        return;
    }
    for i in k..items.len() {
        items.swap(k, i);
        permutations(items, k + 1, out);
        items.swap(k, i);
    }
}

fn count_partitions(n: i64, largest: i64) -> i128 {
    if n == 0 {
        return 1;
    }
    (1..=largest.min(n))
        .map(|part| count_partitions(n - part, part))
        .sum()
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

#[test]
fn counting_functions_against_enumeration() {
    for n in 0..=20 {
        assert_eq!(int(&format!("partitions({n})")), count_partitions(n, n));
    }
    // Derangements: permutations with no fixed point, enumerated.
    for n in 0..=8usize {
        let mut all = Vec::new();
        permutations(&mut (0..n).collect(), 0, &mut all);
        let expected = all
            .iter()
            .filter(|p| p.iter().enumerate().all(|(i, &v)| i != v))
            .count() as i128;
        assert_eq!(int(&format!("derangements({n})")), expected, "D({n})");
    }
    // Catalan numbers count Dyck words.
    for n in 0..=10u32 {
        let mut count = 0i128;
        for mask in 0u32..(1 << (2 * n)) {
            if mask.count_ones() != n {
                continue;
            }
            let (mut height, mut ok) = (0i32, true);
            for bit in 0..2 * n {
                height += if (mask >> bit) & 1 == 1 { 1 } else { -1 };
                if height < 0 {
                    ok = false;
                    break;
                }
            }
            count += ok as i128;
        }
        assert_eq!(int(&format!("catalan({n})")), count, "C({n})");
    }
}

#[test]
fn identities() {
    for n in 1..=25i128 {
        // Row sums: Stirling numbers of the second kind give Bell numbers, unsigned first kind give n!.
        let bell_sum: i128 = (0..=n).map(|k| int(&format!("stirling2({n}, {k})"))).sum();
        assert_eq!(bell_sum, int(&format!("bell({n})")), "Bell({n})");
        let factorial: i128 = (1..=n).product();
        let first_kind: i128 = (0..=n)
            .map(|k| int(&format!("stirling1({n}, {k})")).abs())
            .sum();
        assert_eq!(first_kind, factorial, "sum |s({n}, k)|");
        let f = |m: i128| int(&format!("fibonacci({m})"));
        assert_eq!(f(n + 1), f(n) + f(n - 1));
        assert_eq!(int(&format!("lucas({n})")), f(n - 1) + f(n + 1));
        // Cassini: F(n-1) F(n+1) - F(n)^2 = (-1)^n
        assert_eq!(
            f(n - 1) * f(n + 1) - f(n) * f(n),
            if n % 2 == 0 { 1 } else { -1 }
        );
    }
    let (mut a, mut b) = (0u128, 1u128);
    for _ in 0..150 {
        (a, b) = (b, a + b);
    }
    assert_eq!(int("fibonacci(150)"), a as i128);
    assert_eq!(int("multinomial(3, 2, 2)"), 210);
    assert_eq!(int("permutations(10, 3)"), 720);
}

#[test]
fn number_theoretic_functions() {
    let is_prime = |n: i128| n > 1 && (2..).take_while(|d| d * d <= n).all(|d| n % d != 0);
    for n in [1, 2, 10, 100, 1000, 5000] {
        assert_eq!(
            int(&format!("primepi({n})")),
            (1..=n).filter(|&k| is_prime(k)).count() as i128,
            "pi({n})"
        );
    }
    let primes: Vec<i128> = (2..).filter(|&k| is_prime(k)).take(200).collect();
    for k in [1usize, 2, 10, 50, 200] {
        assert_eq!(int(&format!("prime({k})")), primes[k - 1], "p_{k}");
    }
    // Legendre symbol against Euler's criterion.
    for p in [3i128, 5, 7, 11, 13, 17, 19, 23] {
        for a in 0..p {
            let mut power = 1i128;
            for _ in 0..(p - 1) / 2 {
                power = power * a % p;
            }
            let euler = match power {
                0 => 0,
                1 => 1,
                _ => -1,
            };
            assert_eq!(int(&format!("legendre({a}, {p})")), euler, "({a}/{p})");
        }
    }
    // The Jacobi symbol is multiplicative in its bottom argument.
    for a in 1..30i128 {
        let j = |n: i128| int(&format!("jacobi({a}, {n})"));
        assert_eq!(j(105), j(3) * j(5) * j(7), "jacobi({a}, 105)");
    }
    assert_eq!(int("gcd(12, 18, 30)"), 6);
    assert_eq!(int("lcm(4, 6, 10)"), 60);
    for n in [0i128, 1, 2, 15, 16, 17, 99, 100, 10_000_000_000] {
        let r = int(&format!("isqrt({n})"));
        assert!(r * r <= n && (r + 1) * (r + 1) > n, "isqrt({n}) = {r}");
    }
    assert_eq!(int("max(1, 5, 3)"), 5);
    assert_eq!(int("digitsum(98765)"), 35);
    assert_eq!(int("collatz(27)"), 111);
}

#[test]
fn bernoulli_and_harmonic_numbers_are_exact_rationals() {
    let rational = |text: &str| -> (i128, i128) {
        let t = run(text);
        match t.split_once('/') {
            Some((a, b)) => (a.parse().unwrap(), b.parse().unwrap()),
            None => (t.parse().unwrap(), 1),
        }
    };
    let binom = |n: i128, k: i128| (0..k).fold(1i128, |p, i| p * (n - i) / (i + 1));
    // The defining recurrence: sum_{k=0}^{m} C(m+1, k) B_k = 0 for m >= 1.
    for m in 1..=12i128 {
        let terms: Vec<(i128, i128)> = (0..=m)
            .map(|k| rational(&format!("bernoulli({k})")))
            .collect();
        let lcm = terms.iter().fold(1i128, |l, (_, d)| l / gcd(l, *d) * d);
        let total: i128 = terms
            .iter()
            .enumerate()
            .map(|(k, (n, d))| binom(m + 1, k as i128) * n * (lcm / d))
            .sum();
        assert_eq!(total, 0, "m = {m}");
    }
    assert_eq!(run("bernoulli(10)"), "5/66");
    assert_eq!(run("harmonic(5)"), "137/60");
    assert_eq!(run("harmonic(10)"), "7381/2520");
}

#[test]
fn huge_arguments_are_refused_not_hung() {
    let started = std::time::Instant::now();
    for q in [
        "partitions(100000)",
        "bell(100000)",
        "primepi(10^12)",
        "fibonacci(10^9)",
    ] {
        let _ = UraeSession::new().execute_line(q);
        assert!(started.elapsed().as_secs_f64() < 60.0, "{q} took too long");
    }
}

fn ints(text: &str) -> Vec<i128> {
    let inner = text
        .rsplit_once('[')
        .map(|(_, r)| r)
        .or_else(|| text.rsplit_once('(').map(|(_, r)| r))
        .unwrap_or(text);
    inner
        .split(|c: char| !(c.is_ascii_digit() || c == '-'))
        .filter(|s| !s.is_empty() && *s != "-")
        .map(|s| s.parse().unwrap())
        .collect()
}

#[test]
fn list_valued_functions_satisfy_their_defining_identities() {
    // divmod: a = q b + r with r in [0, b) for b > 0 and in (b, 0] for b < 0.
    for (a, b) in [
        (17i128, 5i128),
        (-17, 5),
        (17, -5),
        (-17, -5),
        (0, 3),
        (5, 17),
    ] {
        let v = ints(
            &run(&format!("divmod({a}, {b})"))
                .split_once('=')
                .unwrap()
                .1
                .replace(' ', ""),
        );
        let (q, r) = (v[0], v[1]);
        assert_eq!(q * b + r, a, "divmod({a}, {b})");
        assert!(if b > 0 {
            (0..b).contains(&r)
        } else {
            (b + 1..=0).contains(&r)
        });
    }
    assert_eq!(run("17 // 5"), "17 // 5 = 3");
    assert_eq!(run("-17 // 5"), "-17 // 5 = -4");
    assert_eq!(run("7/2 // 1/2"), "7/2 // 1/2 = 7");
    // digits reassemble the number in every base.
    for (n, base) in [
        (1234i128, 10i128),
        (255, 2),
        (255, 16),
        (0, 10),
        (987_654_321_012, 7),
    ] {
        let text = run(&format!("digits({n}, {base})"));
        let d = ints(text.split_once('=').unwrap().1.trim());
        assert!(d.iter().all(|&x| (0..base).contains(&x)));
        assert_eq!(d.iter().fold(0i128, |acc, &x| acc * base + x), n, "{text}");
    }
    // egcd: Bezout's identity and gcd by an independent loop.
    for (a, b) in [(240i128, 46i128), (17, 5), (-12, 18), (0, 7), (35, 49)] {
        let text = run(&format!("egcd({a}, {b})"));
        let v = ints(
            &text
                .split_once('=')
                .unwrap()
                .1
                .split('[')
                .next()
                .unwrap()
                .replace(' ', ""),
        );
        assert_eq!(a * v[1] + b * v[2], v[0], "{text}");
        assert_eq!(v[0], gcd(a, b));
    }
    // crt against a brute-force search, including non-coprime moduli and an inconsistent system.
    for (r, m) in [
        (vec![2i128, 3, 2], vec![3i128, 5, 7]),
        (vec![1, 3], vec![4, 6]),
        (vec![0, 0], vec![6, 10]),
    ] {
        let lcm = m.iter().fold(1i128, |l, &x| l / gcd(l, x) * x);
        let expected = (0..lcm).find(|x| {
            r.iter()
                .zip(&m)
                .all(|(r, m)| x.rem_euclid(*m) == r.rem_euclid(*m))
        });
        let text = run(&format!("crt({:?}, {:?})", r, m));
        let v = ints(
            &text
                .replace(' ', "")
                .replace("crt=", "")
                .replace("(mod", ",")
                .replace(')', ""),
        );
        assert_eq!(Some(v[0]), expected, "{text}");
        assert_eq!(v[1], lcm);
    }
    assert!(
        UraeSession::new()
            .execute_line("crt([1, 2], [4, 6])")
            .is_error
    );
    // continued fractions: the terms rebuild the rational; consecutive convergents have determinant +-1.
    for (p, q) in [(415i128, 93i128), (355, 113), (-7, 3), (5, 1), (1, 7)] {
        let text = run(&format!("continued_fraction({p}/{q})"));
        let terms = ints(&text.replace(';', ","));
        let (mut n, mut d) = (terms[terms.len() - 1], 1i128);
        for &a in terms[..terms.len() - 1].iter().rev() {
            (n, d) = (a * n + d, n);
        }
        assert_eq!(n * q, p * d, "{text}");
        let convergents = run(&format!("convergents({p}/{q})"));
        let parts: Vec<(i128, i128)> = convergents
            .split_once('[')
            .unwrap()
            .1
            .trim_end_matches(']')
            .split(", ")
            .map(|s| match s.split_once('/') {
                Some((a, b)) => (a.parse().unwrap(), b.parse().unwrap()),
                None => (s.parse().unwrap(), 1),
            })
            .collect();
        for w in parts.windows(2) {
            assert_eq!(
                (w[1].0 * w[0].1 - w[0].0 * w[1].1).abs(),
                1,
                "{convergents}"
            );
        }
        let last = parts[parts.len() - 1];
        assert_eq!(last.0 * q, p * last.1);
    }
    // Integer factorisation by the plain name, and the spelling without an underscore.
    assert!(run("factor(360)").starts_with("360 = 2^3 * 3^2 * 5"));
    assert!(run("primefactors(360)").starts_with("360 = 2^3 * 3^2 * 5"));
}

#[test]
fn modular_functions_against_brute_force() {
    // carmichael(n): the least m with a^m = 1 (mod n) for every a coprime to n.
    for n in [9i128, 15, 16, 21, 24, 35, 100, 561] {
        let lambda = int(&format!("carmichael({n})"));
        let coprime: Vec<i128> = (1..n).filter(|&a| gcd(a, n) == 1).collect();
        let works = |m: i128| {
            coprime
                .iter()
                .all(|&a| (0..m).fold(1i128, |p, _| p * a % n) % n == 1 % n)
        };
        assert!(works(lambda), "carmichael({n}) = {lambda} fails");
        assert!(
            (1..lambda).all(|m| !works(m)),
            "carmichael({n}) = {lambda} is not minimal"
        );
    }
    // order_mod(a, n) is the least d >= 1 with a^d = 1 (mod n).
    for (a, n) in [(3i128, 7i128), (2, 9), (10, 17), (5, 12), (7, 40)] {
        let order = int(&format!("order_mod({a}, {n})"));
        let power = |d: i128| (0..d).fold(1i128, |p, _| p * a % n);
        assert_eq!(power(order), 1 % n);
        assert!(
            (1..order).all(|d| power(d) != 1),
            "order_mod({a}, {n}) = {order}"
        );
    }
    // sqrt_mod(a, p) squares back to a for every quadratic residue; non-residues are left unevaluated.
    for p in [7i128, 11, 13, 17, 29, 41, 101] {
        for a in 1..p {
            let residue = (1..p).any(|r| r * r % p == a);
            let out = run(&format!("sqrt_mod({a}, {p})"));
            if residue {
                let r: i128 = out.parse().unwrap();
                assert_eq!(r * r % p, a, "sqrt_mod({a}, {p}) = {r}");
            } else {
                assert!(
                    out.starts_with("sqrt_mod"),
                    "{a} is not a residue mod {p}: {out}"
                );
            }
        }
    }
    // discrete_log(g, h, p): g^x = h, and the least such x.
    for (g, p) in [(3i128, 17i128), (2, 29), (5, 23), (2, 101)] {
        for h in 1..p {
            let out = run(&format!("discrete_log({g}, {h}, {p})"));
            if let Ok(x) = out.parse::<i128>() {
                assert_eq!((0..x).fold(1i128, |v, _| v * g % p), h);
                assert!(
                    (0..x).all(|y| (0..y).fold(1i128, |v, _| v * g % p) != h),
                    "not least"
                );
            } else {
                // No solution: h is outside the subgroup generated by g.
                assert!(
                    (0..p).all(|y| (0..y).fold(1i128, |v, _| v * g % p) != h),
                    "{g}^x = {h} mod {p} has a solution"
                );
            }
        }
    }
    assert_eq!(int("radical(360)"), 30);
    assert_eq!(int("gray_code(5)"), 7);
}

#[test]
fn radix_listings_pell_and_linear_diophantine() {
    // Radix conversion against Rust's own formatting.
    for n in [0u64, 1, 10, 255, 4096, 123_456_789] {
        assert_eq!(
            run(&format!("dec2bin({n})")),
            format!("dec2bin({n}) = {n:b}")
        );
        assert_eq!(
            run(&format!("bin2dec({n:b})")),
            format!("bin2dec({n:b}) = {n}")
        );
        assert_eq!(
            run(&format!("dec2hex({n})")),
            format!("dec2hex({n}) = {n:x}")
        );
        assert_eq!(
            run(&format!("hex2dec({n:x})")),
            format!("hex2dec({n:x}) = {n}")
        );
        assert_eq!(
            run(&format!("dec2oct({n})")),
            format!("dec2oct({n}) = {n:o}")
        );
    }
    // 2^n subsets, n! permutations, all distinct.
    for n in 0..=6usize {
        let list = format!(
            "[{}]",
            (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
        );
        let subsets = run(&format!("subsets({list})"));
        assert_eq!(subsets.matches('[').count() - 1, 1 << n, "{subsets}");
        let permutations = run(&format!("permutations_list({list})"));
        let expected: usize = (1..=n).product();
        assert_eq!(
            permutations.matches('[').count() - 1,
            expected,
            "{permutations}"
        );
    }
    assert!(run("subsets([a,b])").contains("[a, b]"));
    // cycle type and the next permutation in lexicographic order.
    assert_eq!(
        run("cycle_type([2,1,4,3])"),
        "cycle_type = [2, 2], cycles (1 2)(3 4)"
    );
    assert_eq!(
        run("next_permutation([1,3,2])"),
        "next_permutation = [2, 1, 3]"
    );
    // Pell: the printed solution satisfies the equation, and no smaller y does.
    for d in [2i128, 3, 5, 6, 7, 13, 61, 109] {
        let out = run(&format!("pell({d})"));
        let pair = out.rsplit_once("= (").unwrap().1.trim_end_matches(')');
        let (x, y): (i128, i128) = {
            let (a, b) = pair.split_once(", ").unwrap();
            (a.parse().unwrap(), b.parse().unwrap())
        };
        assert_eq!(x * x - d * y * y, 1, "{out}");
        if y < 100_000 {
            assert!(
                (1..y).all(|k| {
                    let s = d * k * k + 1;
                    let r = (s as f64).sqrt() as i128;
                    !(r - 1..=r + 1).any(|c| c * c == s)
                }),
                "{out} is not fundamental"
            );
        }
    }
    assert!(UraeSession::new().execute_line("pell(9)").is_error);
    // Linear Diophantine equations: the parametrised family solves a x + b y = c for several t.
    for (a, b, c) in [(3i128, 5i128, 1i128), (6, 9, 12), (-4, 10, 6), (7, -3, 2)] {
        let out = run(&format!("diophantine({a}*x + ({b})*y = {c})"));
        // x = p + q t, y = r + s t
        let eval = |expr: &str, t: i128| -> i128 {
            let expr = expr.trim().replace('·', "*");
            let mut total = 0i128;
            let mut sign = 1i128;
            for token in expr.split_whitespace() {
                match token {
                    "+" => sign = 1,
                    "-" => sign = -1,
                    tok => {
                        let value = match tok.strip_suffix("*t") {
                            Some(k) => k.parse::<i128>().unwrap() * t,
                            None => tok.parse::<i128>().unwrap(),
                        };
                        total += sign * value;
                        sign = 1;
                    }
                }
            }
            total
        };
        let body = out.split("  (").next().unwrap();
        let (xs, ys) = body.split_once(", ").unwrap();
        let (xe, ye) = (
            xs.split_once(" = ").unwrap().1,
            ys.split_once(" = ").unwrap().1,
        );
        for t in [-3, 0, 1, 7] {
            assert_eq!(a * eval(xe, t) + b * eval(ye, t), c, "{out} at t = {t}");
        }
    }
    assert!(
        UraeSession::new()
            .execute_line("diophantine(6*x + 9*y = 4)")
            .is_error
    );
}
