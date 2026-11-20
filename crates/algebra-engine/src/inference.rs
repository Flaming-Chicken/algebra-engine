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
        _ => return None,
    };
    let mut args = split_top(&t[open + 1..t.len() - 1]);
    let (alt, count) = alternative(&args);
    args.truncate(count);
    match command {
        "chi2" => return Some(chi_square(session, &args)),
        "proportion" => return Some(proportion(session, &args, alt)),
        "anova" => return Some(anova(session, &args)),
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
