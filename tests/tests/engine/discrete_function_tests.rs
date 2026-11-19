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
