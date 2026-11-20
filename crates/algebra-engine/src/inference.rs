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
use num_traits::{ToPrimitive, Zero};

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
        _ => return None,
    };
    let args = split_top(&t[open + 1..t.len() - 1]);
    let first = list(session, args.first()?)?;
    Some(evaluate(session, command, &first, &args))
}

fn evaluate(
    session: &mut UraeSession,
    command: &str,
    first: &[Q],
    args: &[String],
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
                    "Welch t = {}, df = {}, p = {} (two-sided); means {} and {}",
                    number(t),
                    number(df),
                    number(two_sided_p(t, df)),
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
                "t = {}, df = {}, p = {} (two-sided); mean {} against {}, sample variance {}",
                number(t),
                number(df),
                number(two_sided_p(t, df)),
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
                "paired t = {}, df = {}, p = {} (two-sided); mean difference {}",
                number(t),
                number(n - 1.0),
                number(two_sided_p(t, n - 1.0)),
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
            let p = (2.0
                * (1.0
                    - distribution("normal_cdf", &[z.abs()])
                        .unwrap_or_else(|| normal_cdf(z.abs()))))
            .clamp(0.0, 1.0);
            Ok(format!(
                "z = {}, p = {} (two-sided); mean {}",
                number(z),
                number(p),
                show(&mean)
            ))
        }
    }
}

/// The standard normal distribution function (Abramowitz and Stegun 26.2.17 is too coarse: erf by a series).
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
