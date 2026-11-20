//! Statistical inference on data lists: `confidence_interval(data[, level])`, `t_test(data, mu)` (one sample),
//! `t_test(a, b)` (two samples, Welch), `paired_t_test(a, b)` and `z_test(data, mu, sigma)`.
//!
//! The mean and the variance are exact rationals; the test statistic, the degrees of freedom and the p-value are
//! floating point (the t distribution comes from `distributions_f64`). All p-values are two-sided.

use crate::distributions_f64::eval as distribution;
use crate::list_utils::{list, rational};
use crate::session::UraeSession;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};

type Q = BigRational;

fn split_top(text: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn show(q: &Q) -> String {
    if q.is_integer() {
        q.numer().to_string()
    } else {
        format!("{}/{}", q.numer(), q.denom())
    }
}

/// The mean and the sample variance (divisor n - 1) of the data.
fn mean_variance(data: &[Q]) -> Option<(Q, Q)> {
    let n = Q::from_integer(BigInt::from(data.len()));
    let mean = data.iter().fold(Q::zero(), |s, x| s + x) / &n;
    if data.len() < 2 {
        return None;
    }
    let squares = data
        .iter()
        .fold(Q::zero(), |s, x| s + (x - &mean) * (x - &mean));
    Some((mean, squares / (n - Q::from_integer(BigInt::from(1)))))
}

fn float(q: &Q) -> f64 {
    q.to_f64().unwrap_or(f64::NAN)
}

fn two_sided_p(t: f64, df: f64) -> f64 {
    let upper = distribution("t_cdf", &[t.abs(), df]).unwrap_or(f64::NAN);
    (2.0 * (1.0 - upper)).clamp(0.0, 1.0)
}

/// The alternative hypothesis named by a trailing argument (`"greater"`, `less`, `two-sided`), if there is one.
#[derive(Clone, Copy, PartialEq)]
enum Alternative {
    Two,
    Greater,
    Less,
}

fn alternative(args: &[String]) -> (Alternative, usize) {
    let kind = |text: &str| match text.trim().trim_matches('"').to_ascii_lowercase().as_str() {
        "greater" | "right" | "upper" | ">" => Some(Alternative::Greater),
        "less" | "left" | "lower" | "<" => Some(Alternative::Less),
        "two-sided" | "two_sided" | "two" | "both" | "!=" => Some(Alternative::Two),
        _ => None,
    };
    match args.last().and_then(|l| kind(l)) {
        Some(alt) => (alt, args.len() - 1),
        None => (Alternative::Two, args.len()),
    }
}

fn label(alt: Alternative) -> &'static str {
    match alt {
        Alternative::Two => "two-sided",
        Alternative::Greater => "greater",
        Alternative::Less => "less",
    }
}

/// The p-value of a t statistic for the chosen alternative.
fn p_of_t(t: f64, df: f64, alt: Alternative) -> f64 {
    let below = distribution("t_cdf", &[t, df]).unwrap_or(f64::NAN);
    match alt {
        Alternative::Two => two_sided_p(t, df),
        Alternative::Greater => (1.0 - below).clamp(0.0, 1.0),
        Alternative::Less => below.clamp(0.0, 1.0),
    }
}

fn number(x: f64) -> String {
    // Six significant digits, no trailing zeros.
    let s = format!("{x:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// `Some(result)` when `input` is one of the inference commands on data lists.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    let command = match name.as_str() {
        "confidence_interval" | "conf_int" | "ci" => "ci",
        "t_test" | "ttest" | "t_test_1samp" => "t",
        "paired_t_test" | "ttest_paired" | "t_test_paired" => "paired",
        "z_test" | "ztest" => "z",
        "chi2_test" | "chisq_test" | "chi_square_test" | "chi2_independence" => "chi2",
        "proportion_test" | "prop_test" | "z_test_proportion" => "proportion",
        "anova" | "anova_oneway" | "oneway_anova" | "f_oneway" => "anova",
        "fisher_exact" | "fisher_test" | "fisher" => "fisher",
        "correlation_test" | "pearson_test" | "pearson" => "pearson",
        "spearman" | "spearman_test" | "spearmanr" => "spearman",
        "kendall" | "kendall_tau" | "kendalltau" | "kendall_test" => "kendall",
        "mann_whitney" | "mannwhitney" | "mann_whitney_u" | "wilcoxon_rank_sum" => "mann_whitney",
        "regression_test" | "regress" | "linregress" | "regression_inference" => "regress",
        "wilcoxon" | "wilcoxon_signed_rank" | "wilcoxon_test" => "wilcoxon",
        "kruskal" | "kruskal_wallis" | "kruskal_wallis_test" => "kruskal",
        _ => return None,
    };
    let mut args = split_top(&t[open + 1..t.len() - 1]);
    let (alt, count) = alternative(&args);
    args.truncate(count);
    match command {
        "chi2" => return Some(chi_square(session, &args)),
        "proportion" => return Some(proportion(session, &args, alt)),
        "anova" => return Some(anova(session, &args)),
        "fisher" => return Some(fisher(session, &args)),
        "pearson" | "spearman" => {
            return Some(correlation(session, &args, command == "spearman", alt));
        }
        "kendall" => return Some(kendall(session, &args)),
        "mann_whitney" => return Some(mann_whitney(session, &args)),
        "regress" => return Some(regress(session, &args)),
        "wilcoxon" => return Some(wilcoxon(session, &args)),
        "kruskal" => return Some(kruskal(session, &args)),
        _ => {}
    }
    let first = list(session, args.first()?)?;
    Some(evaluate(session, command, &first, &args, alt))
}

fn evaluate(
    session: &mut UraeSession,
    command: &str,
    first: &[Q],
    args: &[String],
    alt: Alternative,
) -> Result<String, String> {
    let need = "at least two data values are needed";
    match command {
        "ci" => {
            let level = match args.get(1) {
                Some(l) => {
                    float(&rational(session, l).ok_or("the level must be a number such as 0.95")?)
                }
                None => 0.95,
            };
            if !(0.0 < level && level < 1.0) {
                return Err("the confidence level must be between 0 and 1".into());
            }
            let (mean, variance) = mean_variance(first).ok_or(need)?;
            let n = first.len() as f64;
            let se = (float(&variance) / n).sqrt();
            let critical = distribution("t_quantile", &[1.0 - (1.0 - level) / 2.0, n - 1.0])
                .ok_or("t quantile failed")?;
            let m = float(&mean);
            Ok(format!(
                "{}% confidence interval for the mean: ({}, {})  (mean {}, standard error {}, t* = {}, df = {})",
                number(level * 100.0),
                number(m - critical * se),
                number(m + critical * se),
                if show(&mean) == number(m) {
                    show(&mean)
                } else {
                    format!("{} = {}", show(&mean), number(m))
                },
                number(se),
                number(critical),
                first.len() - 1
            ))
        }
        "t" => {
            let second = args.get(1).ok_or("t_test(data, mu) or t_test(a, b)")?;
            if let Some(other) = list(session, second) {
                // Two samples, Welch.
                let ((m1, v1), (m2, v2)) = (
                    mean_variance(first).ok_or(need)?,
                    mean_variance(&other).ok_or(need)?,
                );
                let (n1, n2) = (first.len() as f64, other.len() as f64);
                let (a, b) = (float(&v1) / n1, float(&v2) / n2);
                let t = (float(&m1) - float(&m2)) / (a + b).sqrt();
                let df = (a + b).powi(2) / (a * a / (n1 - 1.0) + b * b / (n2 - 1.0));
                return Ok(format!(
                    "Welch t = {}, df = {}, p = {} ({}); means {} and {}",
                    number(t),
                    number(df),
                    number(p_of_t(t, df, alt)),
                    label(alt),
                    show(&m1),
                    show(&m2)
                ));
            }
            let mu = rational(session, second).ok_or("the hypothesised mean must be a number")?;
            let (mean, variance) = mean_variance(first).ok_or(need)?;
            let n = first.len() as f64;
            let t = (float(&mean) - float(&mu)) / (float(&variance) / n).sqrt();
            let df = n - 1.0;
            Ok(format!(
                "t = {}, df = {}, p = {} ({}); mean {} against {}, sample variance {}",
                number(t),
                number(df),
                number(p_of_t(t, df, alt)),
                label(alt),
                show(&mean),
                show(&mu),
                show(&variance)
            ))
        }
        "paired" => {
            let other = list(session, args.get(1).ok_or("paired_t_test(a, b)")?)
                .ok_or("the second argument must be a list")?;
            if other.len() != first.len() {
                return Err("the two samples must have the same length".into());
            }
            let differences: Vec<Q> = first.iter().zip(&other).map(|(a, b)| a - b).collect();
            let (mean, variance) = mean_variance(&differences).ok_or(need)?;
            let n = differences.len() as f64;
            if variance.is_zero() {
                return Err("all the differences are equal: the statistic is undefined".into());
            }
            let t = float(&mean) / (float(&variance) / n).sqrt();
            Ok(format!(
                "paired t = {}, df = {}, p = {} ({}); mean difference {}",
                number(t),
                number(n - 1.0),
                number(p_of_t(t, n - 1.0, alt)),
                label(alt),
                show(&mean)
            ))
        }
        _ => {
            // z_test(data, mu, sigma): a known standard deviation.
            let (mu, sigma) = (
                rational(session, args.get(1).ok_or("z_test(data, mu, sigma)")?)
                    .ok_or("mu must be a number")?,
                rational(session, args.get(2).ok_or("z_test(data, mu, sigma)")?)
                    .ok_or("sigma must be a number")?,
            );
            if sigma.numer().to_i64().is_none_or(|s| s <= 0) {
                return Err("sigma must be positive".into());
            }
            let n = first.len() as f64;
            let mean = first.iter().fold(Q::zero(), |s, x| s + x)
                / Q::from_integer(BigInt::from(first.len()));
            let z = (float(&mean) - float(&mu)) / (float(&sigma) / n.sqrt());
            let p = normal_p(z, alt);
            Ok(format!(
                "z = {}, p = {} ({}); mean {}",
                number(z),
                number(p),
                label(alt),
                show(&mean)
            ))
        }
    }
}

/// The p-value of a z statistic for the chosen alternative.
fn normal_p(z: f64, alt: Alternative) -> f64 {
    let below = normal_cdf(z);
    match alt {
        Alternative::Two => (2.0 * (1.0 - normal_cdf(z.abs()))).clamp(0.0, 1.0),
        Alternative::Greater => (1.0 - below).clamp(0.0, 1.0),
        Alternative::Less => below.clamp(0.0, 1.0),
    }
}

/// The standard normal distribution function (erf by its Maclaurin series).
fn normal_cdf(x: f64) -> f64 {
    // Phi(x) = (1 + erf(x / sqrt 2)) / 2 with erf from its Maclaurin series for moderate arguments.
    let z = x / std::f64::consts::SQRT_2;
    let (mut term, mut sum) = (z, z);
    for k in 1..200 {
        term *= -z * z / k as f64;
        sum += term / (2 * k + 1) as f64;
        if term.abs() < 1e-18 {
            break;
        }
    }
    0.5 * (1.0 + 2.0 / std::f64::consts::PI.sqrt() * sum)
}

/// `proportion_test(successes, n, p0[, alternative])`: the normal approximation with the null proportion in the standard error.
fn proportion(
    session: &mut UraeSession,
    args: &[String],
    alt: Alternative,
) -> Result<String, String> {
    let numbers: Vec<Q> = args
        .iter()
        .map(|a| rational(session, a))
        .collect::<Option<_>>()
        .ok_or("proportion_test(successes, n, p0) takes numbers")?;
    let [x, n, p0] = numbers.as_slice() else {
        return Err("proportion_test(successes, n, p0)".into());
    };
    let (xf, nf, p0f) = (float(x), float(n), float(p0));
    if !(x.is_integer() && n.is_integer() && xf >= 0.0 && xf <= nf && nf >= 1.0) {
        return Err("successes must be an integer between 0 and n".into());
    }
    if !(0.0 < p0f && p0f < 1.0) {
        return Err("the null proportion must be between 0 and 1".into());
    }
    let z = (xf / nf - p0f) / (p0f * (1.0 - p0f) / nf).sqrt();
    Ok(format!(
        "z = {}, p = {} ({}); sample proportion {} against {}",
        number(z),
        number(normal_p(z, alt)),
        label(alt),
        show(&(x / n)),
        show(p0)
    ))
}

/// The rows of a nested list `[[a, b], [c, d]]` as numbers.
fn table(session: &mut UraeSession, text: &str) -> Option<Vec<Vec<Q>>> {
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    let rows: Vec<Vec<Q>> = split_top(inner)
        .iter()
        .map(|row| list(session, row))
        .collect::<Option<_>>()?;
    (!rows.is_empty() && rows.iter().all(|r| r.len() == rows[0].len())).then_some(rows)
}

fn chi_square_p(statistic: f64, df: f64) -> f64 {
    (1.0 - distribution("chi2_cdf", &[statistic, df]).unwrap_or(f64::NAN)).clamp(0.0, 1.0)
}

/// `chi2_test(observed, expected)` (goodness of fit) or `chi2_test([[a, b], [c, d]])` (independence).
fn chi_square(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    match args {
        [observed, expected] => {
            let (o, e) = (
                list(session, observed).ok_or("observed counts must be a list")?,
                list(session, expected).ok_or("expected counts must be a list")?,
            );
            if o.len() != e.len() || o.len() < 2 {
                return Err("observed and expected need the same length, at least 2".into());
            }
            if e.iter().any(|x| !x.is_positive()) {
                return Err("every expected count must be positive".into());
            }
            let statistic = o
                .iter()
                .zip(&e)
                .fold(Q::zero(), |s, (o, e)| s + (o - e) * (o - e) / e);
            let df = (o.len() - 1) as f64;
            Ok(format!(
                "chi-square = {} ({}), df = {}, p = {}",
                number(float(&statistic)),
                show(&statistic),
                df,
                number(chi_square_p(float(&statistic), df))
            ))
        }
        [one] => {
            let rows =
                table(session, one).ok_or("chi2_test([[a, b], [c, d]]) takes a table of counts")?;
            let (r, c) = (rows.len(), rows[0].len());
            if r < 2 || c < 2 {
                return Err("the table needs at least two rows and two columns".into());
            }
            let row_totals: Vec<Q> = rows
                .iter()
                .map(|row| row.iter().fold(Q::zero(), |s, x| s + x))
                .collect();
            let column_totals: Vec<Q> = (0..c)
                .map(|j| rows.iter().fold(Q::zero(), |s, row| s + &row[j]))
                .collect();
            let total = row_totals.iter().fold(Q::zero(), |s, x| s + x);
            if total.is_zero() {
                return Err("the table is empty".into());
            }
            let mut statistic = Q::zero();
            for i in 0..r {
                for j in 0..c {
                    let expected = &row_totals[i] * &column_totals[j] / &total;
                    if expected.is_zero() {
                        return Err("a row or column total is zero".into());
                    }
                    statistic += (&rows[i][j] - &expected) * (&rows[i][j] - &expected) / expected;
                }
            }
            let df = ((r - 1) * (c - 1)) as f64;
            Ok(format!(
                "chi-square = {} ({}), df = {}, p = {} (test of independence)",
                number(float(&statistic)),
                show(&statistic),
                df,
                number(chi_square_p(float(&statistic), df))
            ))
        }
        _ => Err("chi2_test(observed, expected) or chi2_test(table)".into()),
    }
}

/// `anova(a, b, c, ...)`: one-way analysis of variance of two or more groups.
fn anova(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let groups: Vec<Vec<Q>> = args
        .iter()
        .map(|a| list(session, a))
        .collect::<Option<_>>()
        .ok_or("anova takes lists of numbers, one per group")?;
    if groups.len() < 2 || groups.iter().any(|g| g.is_empty()) {
        return Err("anova needs at least two non-empty groups".into());
    }
    let total_n: usize = groups.iter().map(Vec::len).sum();
    if total_n <= groups.len() {
        return Err("anova needs more observations than groups".into());
    }
    let grand = groups.iter().flatten().fold(Q::zero(), |s, x| s + x)
        / Q::from_integer(BigInt::from(total_n));
    let mut between = Q::zero();
    let mut within = Q::zero();
    for g in &groups {
        let mean = g.iter().fold(Q::zero(), |s, x| s + x) / Q::from_integer(BigInt::from(g.len()));
        between += Q::from_integer(BigInt::from(g.len())) * (&mean - &grand) * (&mean - &grand);
        within += g
            .iter()
            .fold(Q::zero(), |s, x| s + (x - &mean) * (x - &mean));
    }
    let (df1, df2) = ((groups.len() - 1) as f64, (total_n - groups.len()) as f64);
    if within.is_zero() {
        return Err("every group is constant: the F statistic is undefined".into());
    }
    let f = float(&between) / df1 / (float(&within) / df2);
    let p = (1.0 - distribution("f_cdf", &[f, df1, df2]).unwrap_or(f64::NAN)).clamp(0.0, 1.0);
    Ok(format!(
        "F = {}, df = ({}, {}), p = {}; between {} within {}",
        number(f),
        df1,
        df2,
        number(p),
        show(&between),
        show(&within)
    ))
}

/// `n choose k` as an exact integer.
fn choose(n: u64, k: u64) -> BigInt {
    if k > n {
        return BigInt::zero();
    }
    (1..=k.min(n - k)).fold(BigInt::from(1), |acc, i| {
        acc * BigInt::from(n - i + 1) / BigInt::from(i)
    })
}

/// `fisher_exact([[a, b], [c, d]])`: the exact two-sided p-value of a 2x2 table (the sum of the probabilities, under the
/// hypergeometric distribution with the margins fixed, of the tables no more probable than the observed one).
fn fisher(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let rows = table(
        session,
        args.first().ok_or("fisher_exact([[a, b], [c, d]])")?,
    )
    .ok_or("fisher_exact takes a 2x2 table of counts")?;
    if rows.len() != 2 || rows[0].len() != 2 {
        return Err("fisher_exact takes a 2x2 table".into());
    }
    let counts: Vec<u64> = rows
        .iter()
        .flatten()
        .map(|q| {
            if q.is_integer() && !q.is_negative() {
                q.to_integer().to_u64()
            } else {
                None
            }
        })
        .collect::<Option<_>>()
        .ok_or("the counts must be non-negative integers")?;
    let (a, b, c, d) = (counts[0], counts[1], counts[2], counts[3]);
    let (row1, row2, col1, n) = (a + b, c + d, a + c, a + b + c + d);
    if n == 0 {
        return Err("the table is empty".into());
    }
    let denominator = choose(n, col1);
    let weight = |k: u64| choose(row1, k) * choose(row2, col1 - k.min(col1));
    let observed = weight(a);
    let lowest = col1.saturating_sub(row2);
    let highest = col1.min(row1);
    let mut total = BigInt::zero();
    for k in lowest..=highest {
        let w = weight(k);
        if w <= observed {
            total += w;
        }
    }
    let p = Q::new(total, denominator);
    let odds = if b == 0 || c == 0 {
        "infinite".to_string()
    } else {
        number((a as f64 * d as f64) / (b as f64 * c as f64))
    };
    Ok(format!(
        "p = {} = {} (two-sided, exact); sample odds ratio {}",
        number(float(&p)),
        show(&p),
        odds
    ))
}

/// Average ranks (ties share the mean of their positions), from 1.
fn average_ranks(values: &[Q]) -> Vec<Q> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].cmp(&values[b]));
    let mut ranks = vec![Q::zero(); values.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && values[order[j + 1]] == values[order[i]] {
            j += 1;
        }
        let rank = Q::new(BigInt::from(i + j + 2), BigInt::from(2));
        for &k in &order[i..=j] {
            ranks[k] = rank.clone();
        }
        i = j + 1;
    }
    ranks
}

/// `pearson(x, y)` / `spearman(x, y)`: the correlation coefficient with the t test of zero correlation.
fn correlation(
    session: &mut UraeSession,
    args: &[String],
    spearman: bool,
    alt: Alternative,
) -> Result<String, String> {
    let x =
        list(session, args.first().ok_or("correlation_test(x, y)")?).ok_or("x must be a list")?;
    let y =
        list(session, args.get(1).ok_or("correlation_test(x, y)")?).ok_or("y must be a list")?;
    if x.len() != y.len() || x.len() < 3 {
        return Err("x and y need the same length, at least 3".into());
    }
    let (x, y) = if spearman {
        (average_ranks(&x), average_ranks(&y))
    } else {
        (x, y)
    };
    let n = Q::from_integer(BigInt::from(x.len()));
    let (mean_x, mean_y) = (
        x.iter().fold(Q::zero(), |s, v| s + v) / &n,
        y.iter().fold(Q::zero(), |s, v| s + v) / &n,
    );
    let sxx = x
        .iter()
        .fold(Q::zero(), |s, v| s + (v - &mean_x) * (v - &mean_x));
    let syy = y
        .iter()
        .fold(Q::zero(), |s, v| s + (v - &mean_y) * (v - &mean_y));
    let sxy = x
        .iter()
        .zip(&y)
        .fold(Q::zero(), |s, (a, b)| s + (a - &mean_x) * (b - &mean_y));
    if sxx.is_zero() || syy.is_zero() {
        return Err("a variable is constant: the correlation is undefined".into());
    }
    // r^2 is rational; r carries the sign of the covariance.
    let r_squared = &sxy * &sxy / (&sxx * &syy);
    let r = float(&r_squared).sqrt() * if sxy.is_negative() { -1.0 } else { 1.0 };
    let df = x.len() as f64 - 2.0;
    let t = if r_squared == Q::from_integer(BigInt::from(1)) {
        f64::INFINITY * r.signum()
    } else {
        r * (df / (1.0 - r * r)).sqrt()
    };
    let p = if t.is_infinite() {
        0.0
    } else {
        p_of_t(t, df, alt)
    };
    Ok(format!(
        "{} = {} (r^2 = {}), t = {}, df = {}, p = {} ({})",
        if spearman {
            "Spearman rho"
        } else {
            "Pearson r"
        },
        number(r),
        show(&r_squared),
        number(t),
        df,
        number(p),
        label(alt)
    ))
}

/// `mann_whitney(a, b)`: the rank-sum test. The p-value is exact (all assignments of the pooled ranks, ties included) for up
/// to 40 observations and the normal approximation with a continuity correction beyond that.
fn mann_whitney(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let a = list(session, args.first().ok_or("mann_whitney(a, b)")?).ok_or("a must be a list")?;
    let b = list(session, args.get(1).ok_or("mann_whitney(a, b)")?).ok_or("b must be a list")?;
    if a.is_empty() || b.is_empty() {
        return Err("both samples need at least one value".into());
    }
    let (n1, n2) = (a.len(), b.len());
    let pooled: Vec<Q> = a.iter().chain(&b).cloned().collect();
    let ranks = average_ranks(&pooled);
    let two = Q::from_integer(BigInt::from(2));
    // Ranks doubled are integers: the sum over the first sample, W, and U = W - n1 (n1 + 1) / 2.
    let doubled: Vec<usize> = ranks
        .iter()
        .map(|r| (r * &two).to_integer().to_usize().unwrap_or(0))
        .collect();
    let observed_w2: usize = doubled[..n1].iter().sum();
    let u = Q::new(BigInt::from(observed_w2), BigInt::from(2))
        - Q::new(BigInt::from(n1 * (n1 + 1)), BigInt::from(2));
    let mean_u = Q::new(BigInt::from(n1 * n2), BigInt::from(2));
    if n1 + n2 <= 40 {
        // count[k][s]: the number of k-subsets with doubled-rank sum s.
        let top: usize = doubled.iter().sum();
        let mut count = vec![vec![BigInt::zero(); top + 1]; n1 + 1];
        count[0][0] = BigInt::from(1);
        for &v in &doubled {
            for k in (0..n1).rev() {
                for s in (0..=top - v).rev() {
                    if !count[k][s].is_zero() {
                        let add = count[k][s].clone();
                        count[k + 1][s + v] += add;
                    }
                }
            }
        }
        let total = choose((n1 + n2) as u64, n1 as u64);
        let observed_distance = (&u - &mean_u).abs();
        let mut extreme = BigInt::zero();
        for s in 0..=top {
            if count[n1][s].is_zero() {
                continue;
            }
            let candidate = Q::new(BigInt::from(s), BigInt::from(2))
                - Q::new(BigInt::from(n1 * (n1 + 1)), BigInt::from(2));
            if (&candidate - &mean_u).abs() >= observed_distance {
                extreme += &count[n1][s];
            }
        }
        let p = Q::new(extreme, total);
        return Ok(format!(
            "U = {}, p = {} = {} (two-sided, exact); U1 = {}, U2 = {}",
            show(&u),
            number(float(&p)),
            show(&p),
            show(&u),
            show(&(Q::from_integer(BigInt::from(n1 * n2)) - &u))
        ));
    }
    let (n1f, n2f) = (n1 as f64, n2 as f64);
    let sigma = (n1f * n2f * (n1f + n2f + 1.0) / 12.0).sqrt();
    let correction = if float(&(&u - &mean_u)) > 0.0 {
        -0.5
    } else {
        0.5
    };
    let z = (float(&(&u - &mean_u)) + correction) / sigma;
    Ok(format!(
        "U = {}, z = {}, p = {} (two-sided, normal approximation)",
        show(&u),
        number(z),
        number(normal_p(z, Alternative::Two))
    ))
}

/// `regress(x, y)`: the least-squares line with its inference (standard errors, t statistics, p-values, R^2 and 95% intervals).
fn regress(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let x = list(session, args.first().ok_or("regress(x, y)")?).ok_or("x must be a list")?;
    let y = list(session, args.get(1).ok_or("regress(x, y)")?).ok_or("y must be a list")?;
    if x.len() != y.len() || x.len() < 3 {
        return Err("x and y need the same length, at least 3".into());
    }
    let n = Q::from_integer(BigInt::from(x.len()));
    let (mean_x, mean_y) = (
        x.iter().fold(Q::zero(), |s, v| s + v) / &n,
        y.iter().fold(Q::zero(), |s, v| s + v) / &n,
    );
    let sxx = x
        .iter()
        .fold(Q::zero(), |s, v| s + (v - &mean_x) * (v - &mean_x));
    let syy = y
        .iter()
        .fold(Q::zero(), |s, v| s + (v - &mean_y) * (v - &mean_y));
    let sxy = x
        .iter()
        .zip(&y)
        .fold(Q::zero(), |s, (a, b)| s + (a - &mean_x) * (b - &mean_y));
    if sxx.is_zero() {
        return Err("x is constant: the slope is undefined".into());
    }
    let slope = &sxy / &sxx;
    let intercept = &mean_y - &slope * &mean_x;
    let sse = &syy - &slope * &sxy;
    let df = x.len() as f64 - 2.0;
    let variance = &sse / (n.clone() - Q::from_integer(BigInt::from(2)));
    let se_slope = (float(&variance) / float(&sxx)).sqrt();
    let se_intercept =
        (float(&variance) * (1.0 / float(&n) + float(&(&mean_x * &mean_x / &sxx)))).sqrt();
    let r_squared = if syy.is_zero() {
        Q::from_integer(BigInt::from(1))
    } else {
        &sxy * &sxy / (&sxx * &syy)
    };
    let critical = distribution("t_quantile", &[0.975, df]).ok_or("t quantile failed")?;
    let describe = |name: &str, estimate: &Q, se: f64| -> String {
        let value = float(estimate);
        if se == 0.0 {
            return format!("{name} = {} (exact fit)", show(estimate));
        }
        let t = value / se;
        format!(
            "{name} = {} (se {}, t = {}, p = {}, 95% CI ({}, {}))",
            show(estimate),
            number(se),
            number(t),
            number(p_of_t(t, df, Alternative::Two)),
            number(value - critical * se),
            number(value + critical * se)
        )
    };
    Ok(format!(
        "{}; {}; R^2 = {}, residual standard error {} on {} df",
        describe("slope", &slope, se_slope),
        describe("intercept", &intercept, se_intercept),
        show(&r_squared),
        number(float(&variance).sqrt()),
        df
    ))
}

/// `wilcoxon(a[, b])`: the signed-rank test of the differences (zeros dropped), exact over all sign assignments up to 30 pairs.
fn wilcoxon(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let first = list(
        session,
        args.first()
            .ok_or("wilcoxon(differences) or wilcoxon(a, b)")?,
    )
    .ok_or("the data must be a list")?;
    let differences: Vec<Q> = match args.get(1) {
        Some(second) => {
            let other = list(session, second).ok_or("the second argument must be a list")?;
            if other.len() != first.len() {
                return Err("the two samples must have the same length".into());
            }
            first.iter().zip(&other).map(|(a, b)| a - b).collect()
        }
        None => first,
    };
    let nonzero: Vec<Q> = differences.into_iter().filter(|d| !d.is_zero()).collect();
    if nonzero.is_empty() {
        return Err("all the differences are zero".into());
    }
    let n = nonzero.len();
    let absolute: Vec<Q> = nonzero.iter().map(|d| d.abs()).collect();
    let ranks = average_ranks(&absolute);
    let two = Q::from_integer(BigInt::from(2));
    let doubled: Vec<usize> = ranks
        .iter()
        .map(|r| (r * &two).to_integer().to_usize().unwrap_or(0))
        .collect();
    let w_plus2: usize = nonzero
        .iter()
        .zip(&doubled)
        .filter(|(d, _)| d.is_positive())
        .map(|(_, r)| *r)
        .sum();
    let total2: usize = doubled.iter().sum();
    let w_plus = Q::new(BigInt::from(w_plus2), BigInt::from(2));
    if n > 30 {
        let nf = n as f64;
        let mean = nf * (nf + 1.0) / 4.0;
        let sigma = (nf * (nf + 1.0) * (2.0 * nf + 1.0) / 24.0).sqrt();
        let z = (float(&w_plus) - mean - 0.5 * (float(&w_plus) - mean).signum()) / sigma;
        return Ok(format!(
            "W+ = {}, n = {}, z = {}, p = {} (two-sided, normal approximation)",
            show(&w_plus),
            n,
            number(z),
            number(normal_p(z, Alternative::Two))
        ));
    }
    // count[s]: the number of sign assignments whose positive ranks sum (doubled) to s.
    let mut count = vec![BigInt::zero(); total2 + 1];
    count[0] = BigInt::from(1);
    for &v in &doubled {
        for s in (0..=total2 - v).rev() {
            if !count[s].is_zero() {
                let add = count[s].clone();
                count[s + v] += add;
            }
        }
    }
    let distance = |s: usize| -> i64 { (2 * s as i64 - total2 as i64).abs() };
    let observed = distance(w_plus2);
    let mut extreme = BigInt::zero();
    for (s, c) in count.iter().enumerate() {
        if distance(s) >= observed {
            extreme += c;
        }
    }
    let p = Q::new(extreme, BigInt::from(1) << n);
    Ok(format!(
        "W+ = {}, n = {}, p = {} = {} (two-sided, exact)",
        show(&w_plus),
        n,
        number(float(&p)),
        show(&p)
    ))
}

/// `kruskal(a, b, c, ...)`: the Kruskal-Wallis H test with the tie correction and the chi-square p-value.
fn kruskal(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let groups: Vec<Vec<Q>> = args
        .iter()
        .map(|a| list(session, a))
        .collect::<Option<_>>()
        .ok_or("kruskal takes lists of numbers, one per group")?;
    if groups.len() < 2 || groups.iter().any(|g| g.is_empty()) {
        return Err("kruskal needs at least two non-empty groups".into());
    }
    let pooled: Vec<Q> = groups.iter().flatten().cloned().collect();
    let big_n = pooled.len() as f64;
    let ranks = average_ranks(&pooled);
    let mut offset = 0;
    let mut sum = 0.0;
    for g in &groups {
        let rank_sum: f64 = ranks[offset..offset + g.len()].iter().map(float).sum();
        sum += rank_sum * rank_sum / g.len() as f64;
        offset += g.len();
    }
    let mut h = 12.0 / (big_n * (big_n + 1.0)) * sum - 3.0 * (big_n + 1.0);
    // Tie correction: 1 - sum (t^3 - t) / (N^3 - N).
    let mut sorted = pooled.clone();
    sorted.sort();
    let (mut ties, mut i) = (0.0, 0);
    while i < sorted.len() {
        let mut j = i;
        while j + 1 < sorted.len() && sorted[j + 1] == sorted[i] {
            j += 1;
        }
        let t = (j - i + 1) as f64;
        ties += t * t * t - t;
        i = j + 1;
    }
    let correction = 1.0 - ties / (big_n * big_n * big_n - big_n);
    if correction <= 0.0 {
        return Err("all the values are equal".into());
    }
    h /= correction;
    let df = (groups.len() - 1) as f64;
    Ok(format!(
        "H = {}, df = {}, p = {}",
        number(h),
        df,
        number(chi_square_p(h, df))
    ))
}

/// Kendall's tau-b of two paired samples: concordant minus discordant pairs over the tie-adjusted pair count.
fn kendall_stats(x: &[Q], y: &[Q]) -> (i64, f64) {
    let n = x.len();
    let mut s = 0i64;
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = ((&x[i]).cmp(&x[j]), (&y[i]).cmp(&y[j]));
            use std::cmp::Ordering::Equal;
            if a != Equal && b != Equal {
                s += if a == b { 1 } else { -1 };
            }
        }
    }
    let pairs = (n * (n - 1) / 2) as i64;
    let ties_x = pairs - tie_pairs(x);
    let ties_y = pairs - tie_pairs(y);
    (s, s as f64 / ((ties_x as f64) * (ties_y as f64)).sqrt())
}

fn tie_pairs(v: &[Q]) -> i64 {
    // The number of pairs that are not tied.
    let n = v.len();
    let mut count = 0;
    for i in 0..n {
        for j in i + 1..n {
            if v[i] != v[j] {
                count += 1;
            }
        }
    }
    (n * (n - 1) / 2) as i64 - count
}

/// `kendall(x, y)`: tau-b with an exact permutation p-value up to 8 pairs and the tie-corrected normal approximation beyond.
fn kendall(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let x = list(session, args.first().ok_or("kendall(x, y)")?).ok_or("x must be a list")?;
    let y = list(session, args.get(1).ok_or("kendall(x, y)")?).ok_or("y must be a list")?;
    let n = x.len();
    if n != y.len() || n < 3 {
        return Err("x and y need the same length, at least 3".into());
    }
    let (s, tau) = kendall_stats(&x, &y);
    if tau.is_nan() {
        return Err("a sample is constant: tau is undefined".into());
    }
    if n <= 8 {
        // Every rearrangement of y against x, counting those at least as extreme.
        let mut perm: Vec<usize> = (0..n).collect();
        let (mut extreme, mut total) = (0u64, 0u64);
        let mut c = vec![0usize; n];
        let mut check = |perm: &[usize]| {
            let permuted: Vec<Q> = perm.iter().map(|&i| y[i].clone()).collect();
            let (s2, _) = kendall_stats(&x, &permuted);
            total += 1;
            if s2.abs() >= s.abs() {
                extreme += 1;
            }
        };
        check(&perm);
        let mut i = 0;
        while i < n {
            if c[i] < i {
                if i % 2 == 0 {
                    perm.swap(0, i);
                } else {
                    perm.swap(c[i], i);
                }
                check(&perm);
                c[i] += 1;
                i = 0;
            } else {
                c[i] = 0;
                i += 1;
            }
        }
        return Ok(format!(
            "tau-b = {}, S = {}, n = {}, p = {} = {}/{} (two-sided, exact)",
            number(tau),
            s,
            n,
            number(extreme as f64 / total as f64),
            extreme,
            total
        ));
    }
    let nf = n as f64;
    let group = |v: &[Q]| -> (f64, f64, f64) {
        let mut sorted = v.to_vec();
        sorted.sort();
        let (mut a, mut b, mut c, mut i) = (0.0, 0.0, 0.0, 0);
        while i < sorted.len() {
            let mut j = i;
            while j + 1 < sorted.len() && sorted[j + 1] == sorted[i] {
                j += 1;
            }
            let t = (j - i + 1) as f64;
            a += t * (t - 1.0) * (2.0 * t + 5.0);
            b += t * (t - 1.0) * (t - 2.0);
            c += t * (t - 1.0);
            i = j + 1;
        }
        (a, b, c)
    };
    let (ax, bx, cx) = group(&x);
    let (ay, by, cy) = group(&y);
    let variance = (nf * (nf - 1.0) * (2.0 * nf + 5.0) - ax - ay) / 18.0
        + bx * by / (9.0 * nf * (nf - 1.0) * (nf - 2.0))
        + cx * cy / (2.0 * nf * (nf - 1.0));
    let z = s as f64 / variance.sqrt();
    Ok(format!(
        "tau-b = {}, S = {}, n = {}, z = {}, p = {} (two-sided, normal approximation)",
        number(tau),
        s,
        n,
        number(z),
        number(normal_p(z, Alternative::Two))
    ))
}
