//! Everyday list utilities on exact rational data: `sum`, `cumsum`, `cumprod`, `diff([...])`, `sort`, `reverse`, `len`,
//! `unique`, `count`, `argmax`, `argmin`, `min([...])`, `range(n)`, `linspace`, `arange`, `sum_squares`, `geometric_mean`,
//! `harmonic_mean`, `rms`, `weighted_mean`, `moving_average`, `rank_data`, `histogram`.
//!
//! A command applies only when every argument is a rational number (or a list of them); anything else is left to the
//! other handlers (`sum(k, k, 1, n)` is a series, not a list sum).

use crate::session::UraeSession;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::BTreeMap;

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

fn show_list(items: &[Q]) -> String {
    format!(
        "[{}]",
        items.iter().map(show).collect::<Vec<_>>().join(", ")
    )
}

/// The rational value of an argument, or `None` when it is not a rational number.
pub(crate) fn rational(session: &mut UraeSession, text: &str) -> Option<Q> {
    let t = text.trim();
    if t.is_empty() || t.chars().any(|c| c.is_alphabetic() && !matches!(c, 'e')) {
        // Letters (variables, function names) mean this is not plain data. `e` alone only appears in decimals.
        if t.chars().any(|c| c.is_alphabetic()) {
            return None;
        }
    }
    let value = session.eval(t).ok()?;
    let value = value.split(" ≈ ").next()?.trim().to_string();
    match value.split_once('/') {
        Some((a, b)) => {
            let (a, b) = (
                a.trim().parse::<BigInt>().ok()?,
                b.trim().parse::<BigInt>().ok()?,
            );
            (!b.is_zero()).then(|| Q::new(a, b))
        }
        None => value.parse::<BigInt>().ok().map(Q::from_integer),
    }
}

/// A list argument `[a, b, c]` / `{a, b, c}` as numbers.
pub(crate) fn list(session: &mut UraeSession, text: &str) -> Option<Vec<Q>> {
    let t = text.trim();
    let inner = t
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .or_else(|| t.strip_prefix('{').and_then(|r| r.strip_suffix('}')))?;
    split_top(inner)
        .iter()
        .map(|item| rational(session, item))
        .collect()
}

/// The data of a call: one list argument, or several scalar arguments.
fn data(session: &mut UraeSession, args: &[String]) -> Option<Vec<Q>> {
    if let [only] = args
        && only.trim_start().starts_with(['[', '{'])
    {
        return list(session, only);
    }
    args.iter().map(|a| rational(session, a)).collect()
}

/// `Some(result)` when `input` is one of these list commands applied to rational data.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')')
        || name.is_empty()
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    let args = split_top(&t[open + 1..t.len() - 1]);
    let handled = [
        "sum",
        "cumsum",
        "cumulative_sum",
        "cumprod",
        "cumulative_product",
        "diff",
        "sort",
        "reverse",
        "length",
        "len",
        "unique",
        "count",
        "argmax",
        "argmin",
        "min",
        "range",
        "linspace",
        "arange",
        "sum_squares",
        "geometric_mean",
        "harmonic_mean",
        "rms",
        "weighted_mean",
        "moving_average",
        "rank_data",
        "histogram",
        "freq",
        "counts",
        "npv",
        "irr",
        "cagr",
        "doubling_time",
        "amortization",
        "amortisation",
        "amortization_schedule",
    ];
    if !handled.contains(&name.as_str()) || args.is_empty() {
        return None;
    }
    // `diff`, `min` and `range` already mean something else on non-lists (a derivative, a minimum, the statistical range).
    let first_is_list = args[0].trim_start().starts_with(['[', '{']);
    if (name == "diff" || name == "min") && !first_is_list {
        return None;
    }
    if name == "range" && first_is_list {
        return None;
    }
    evaluate(session, &name, &args)
}

fn evaluate(
    session: &mut UraeSession,
    name: &str,
    args: &[String],
) -> Option<Result<String, String>> {
    let zero = Q::zero();
    let one = Q::one();
    match name {
        "range" | "arange" => {
            let numbers: Vec<Q> = args
                .iter()
                .map(|a| rational(session, a))
                .collect::<Option<_>>()?;
            let (start, stop, step) = match numbers.as_slice() {
                [n] if name == "range" => (zero.clone(), n.clone(), one.clone()),
                [a, b] => (a.clone(), b.clone(), one.clone()),
                [a, b, s] => (a.clone(), b.clone(), s.clone()),
                _ => return None,
            };
            if step.is_zero() {
                return Some(Err("the step must not be zero".into()));
            }
            let (mut value, mut items) = (start, Vec::new());
            while (step.is_positive() && value < stop) || (step.is_negative() && value > stop) {
                items.push(value.clone());
                value += &step;
                if items.len() > 100_000 {
                    return Some(Err("the range has more than 100000 elements".into()));
                }
            }
            Some(Ok(show_list(&items)))
        }
        "linspace" => {
            let numbers: Vec<Q> = args
                .iter()
                .map(|a| rational(session, a))
                .collect::<Option<_>>()?;
            let [a, b, n] = numbers.as_slice() else {
                return None;
            };
            let count = n.to_integer().to_usize_checked()?;
            if !n.is_integer() || !(1..=100_000).contains(&count) {
                return Some(Err(
                    "linspace(a, b, n) needs an integer n between 1 and 100000".into(),
                ));
            }
            if count == 1 {
                return Some(Ok(show_list(std::slice::from_ref(a))));
            }
            let step = (b - a) / Q::from_integer(BigInt::from(count - 1));
            let items: Vec<Q> = (0..count)
                .map(|i| a + &step * Q::from_integer(BigInt::from(i)))
                .collect();
            Some(Ok(show_list(&items)))
        }
        "count" => {
            let [list_arg, target] = args else {
                return None;
            };
            let items = list(session, list_arg)?;
            let target = rational(session, target)?;
            Some(Ok(items
                .iter()
                .filter(|x| **x == target)
                .count()
                .to_string()))
        }
        "amortization" | "amortisation" | "amortization_schedule" => {
            // amortization(principal, annual rate, years[, rows]): a monthly schedule, exact, printed to the cent.
            let numbers: Vec<Q> = args
                .iter()
                .map(|a| rational(session, a))
                .collect::<Option<_>>()?;
            let (principal, annual, years, rows) = match numbers.as_slice() {
                [p, r, y] => (p.clone(), r.clone(), y.clone(), None),
                [p, r, y, n] => (p.clone(), r.clone(), y.clone(), Some(n.clone())),
                _ => return None,
            };
            if !years.is_integer()
                || !principal.is_positive()
                || years.is_negative()
                || years.is_zero()
            {
                return Some(Err(
                    "amortization(principal, annual rate, whole years[, rows])".into(),
                ));
            }
            let months = num_traits::ToPrimitive::to_usize(&(years.to_integer() * 12))?;
            if months > 1200 {
                return Some(Err("at most 100 years".into()));
            }
            let monthly = annual / Q::from_integer(BigInt::from(12));
            let growth = Q::one() + &monthly;
            let factor = (0..months).fold(Q::one(), |acc, _| acc * &growth);
            let payment = if monthly.is_zero() {
                &principal / Q::from_integer(BigInt::from(months))
            } else {
                &principal * &monthly / (Q::one() - factor.recip())
            };
            let shown_rows = match rows {
                Some(n) if n.is_integer() && n.is_positive() => {
                    num_traits::ToPrimitive::to_usize(&n.to_integer())?.min(months)
                }
                Some(_) => {
                    return Some(Err("the number of rows must be a positive integer".into()));
                }
                None => months.min(12),
            };
            let cents = |q: &Q| -> String {
                // Half away from zero at two decimals, on the exact value.
                let hundred = Q::from_integer(BigInt::from(100));
                let scaled = q * &hundred;
                let rounded = (scaled.abs() + Q::new(BigInt::one(), BigInt::from(2))).floor();
                let value = if scaled.is_negative() {
                    -rounded
                } else {
                    rounded
                };
                let integer = value.to_integer();
                let negative = integer.is_negative();
                let digits = integer.abs().to_string();
                let padded = format!("{digits:0>3}");
                let (whole, fraction) = padded.split_at(padded.len() - 2);
                format!("{}{whole}.{fraction}", if negative { "-" } else { "" })
            };
            let mut balance = principal.clone();
            let mut lines = vec![format!(
                "payment = {} per month; month | interest | principal | balance",
                cents(&payment)
            )];
            for month in 1..=shown_rows {
                let interest = &balance * &monthly;
                let repaid = &payment - &interest;
                balance -= &repaid;
                lines.push(format!(
                    "{month} | {} | {} | {}",
                    cents(&interest),
                    cents(&repaid),
                    cents(&balance)
                ));
            }
            Some(Ok(lines.join(
                "
",
            )))
        }
        "npv" => {
            // npv(rate, [cash flows]): the first flow is at time 0, so npv = sum cf_k / (1 + r)^k.
            let [rate, flows] = args else { return None };
            let rate = rational(session, rate)?;
            let flows = list(session, flows)?;
            let growth = Q::one() + rate;
            if growth.is_zero() {
                return Some(Err("the rate must not be -100%".into()));
            }
            let mut discount = Q::one();
            let mut total = Q::zero();
            for flow in &flows {
                total += flow / &discount;
                discount *= &growth;
            }
            Some(Ok(show(&total)))
        }
        "irr" => {
            // The rate at which the net present value is zero, by bisection on (-0.99, 100) in floating point.
            let [flows] = args else { return None };
            let flows: Vec<f64> = list(session, flows)?
                .iter()
                .map(|q| num_traits::ToPrimitive::to_f64(q).unwrap_or(f64::NAN))
                .collect();
            let value = |r: f64| -> f64 {
                flows
                    .iter()
                    .enumerate()
                    .map(|(k, f)| f / (1.0 + r).powi(k as i32))
                    .sum()
            };
            let (mut lo, mut hi) = (-0.99f64, 100.0f64);
            if value(lo) * value(hi) > 0.0 {
                return Some(Err(
                    "no sign change between -99% and 10000%: the flows have no internal rate of return".into(),
                ));
            }
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if value(lo) * value(mid) <= 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            Some(Ok(format!("{}", 0.5 * (lo + hi))))
        }
        "cagr" => {
            // (end / start)^(1 / years) - 1 as an exact expression.
            let numbers: Vec<Q> = args
                .iter()
                .map(|a| rational(session, a))
                .collect::<Option<_>>()?;
            let [start, end, years] = numbers.as_slice() else {
                return None;
            };
            if !start.is_positive() || !years.is_positive() {
                return Some(Err(
                    "cagr(start, end, years) needs a positive start and number of years".into(),
                ));
            }
            Some(session.eval(&format!(
                "({})^(1/({})) - 1",
                show(&(end / start)),
                show(years)
            )))
        }
        "doubling_time" => {
            let [rate] = args else { return None };
            let rate = rational(session, rate)?;
            if !(Q::one() + &rate).is_positive() || rate.is_zero() {
                return Some(Err("the rate must be positive for a doubling time".into()));
            }
            Some(session.eval(&format!("ln(2)/ln(1 + ({}))", show(&rate))))
        }
        "weighted_mean" => {
            let [values, weights] = args else { return None };
            let (values, weights) = (list(session, values)?, list(session, weights)?);
            if values.len() != weights.len() || values.is_empty() {
                return Some(Err(
                    "the values and the weights must have the same length".into()
                ));
            }
            let total = weights.iter().fold(Q::zero(), |s, w| s + w);
            if total.is_zero() {
                return Some(Err("the weights sum to zero".into()));
            }
            let weighted = values
                .iter()
                .zip(&weights)
                .fold(Q::zero(), |s, (x, w)| s + x * w);
            Some(Ok(show(&(weighted / total))))
        }
        "moving_average" => {
            let [values, window] = args else { return None };
            let values = list(session, values)?;
            let window = rational(session, window)?;
            let w = window.to_integer().to_usize_checked()?;
            if !window.is_integer() || w == 0 || w > values.len() {
                return Some(Err(
                    "the window must be an integer between 1 and the length of the data".into(),
                ));
            }
            let averages: Vec<Q> = values
                .windows(w)
                .map(|win| {
                    win.iter().fold(Q::zero(), |s, x| s + x) / Q::from_integer(BigInt::from(w))
                })
                .collect();
            Some(Ok(show_list(&averages)))
        }
        _ => {
            let items = data(session, args)?;
            if items.is_empty() {
                return Some(Err(format!("{name} needs at least one number")));
            }
            let n = Q::from_integer(BigInt::from(items.len()));
            match name {
                "sum" => Some(Ok(show(&items.iter().fold(Q::zero(), |s, x| s + x)))),
                "sum_squares" => Some(Ok(show(&items.iter().fold(Q::zero(), |s, x| s + x * x)))),
                "cumsum" | "cumulative_sum" => {
                    let mut total = Q::zero();
                    let running: Vec<Q> = items
                        .iter()
                        .map(|x| {
                            total += x;
                            total.clone()
                        })
                        .collect();
                    Some(Ok(show_list(&running)))
                }
                "cumprod" | "cumulative_product" => {
                    let mut total = Q::one();
                    let running: Vec<Q> = items
                        .iter()
                        .map(|x| {
                            total *= x;
                            total.clone()
                        })
                        .collect();
                    Some(Ok(show_list(&running)))
                }
                "diff" => {
                    let differences: Vec<Q> = items.windows(2).map(|w| &w[1] - &w[0]).collect();
                    Some(Ok(show_list(&differences)))
                }
                "sort" => {
                    let mut sorted = items;
                    sorted.sort();
                    Some(Ok(show_list(&sorted)))
                }
                "reverse" => {
                    let mut reversed = items;
                    reversed.reverse();
                    Some(Ok(show_list(&reversed)))
                }
                "length" | "len" => Some(Ok(items.len().to_string())),
                "unique" => {
                    let mut unique = items;
                    unique.sort();
                    unique.dedup();
                    Some(Ok(show_list(&unique)))
                }
                "min" => items.iter().min().map(|m| Ok(show(m))),
                "argmax" | "argmin" => {
                    let best = if name == "argmax" {
                        items
                            .iter()
                            .enumerate()
                            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)))
                    } else {
                        items
                            .iter()
                            .enumerate()
                            .min_by(|a, b| a.1.cmp(b.1).then(a.0.cmp(&b.0)))
                    };
                    best.map(|(i, _)| Ok(i.to_string()))
                }
                "harmonic_mean" => {
                    if items.iter().any(|x| !x.is_positive()) {
                        return Some(Err("the harmonic mean needs positive numbers".into()));
                    }
                    let reciprocals = items.iter().fold(Q::zero(), |s, x| s + x.recip());
                    Some(Ok(show(&(n / reciprocals))))
                }
                "geometric_mean" | "rms" => {
                    // (product)^(1/n) and sqrt(sum of squares / n), as exact expressions.
                    let text = if name == "geometric_mean" {
                        if items.iter().any(|x| !x.is_positive()) {
                            return Some(Err("the geometric mean needs positive numbers".into()));
                        }
                        let product = items.iter().fold(Q::one(), |p, x| p * x);
                        format!("({})^(1/{})", show(&product), items.len())
                    } else {
                        let mean_square = items.iter().fold(Q::zero(), |s, x| s + x * x) / n;
                        format!("sqrt({})", show(&mean_square))
                    };
                    Some(session.eval(&text))
                }
                "rank_data" => {
                    // Average ranks (ties share the mean of their positions), from 1.
                    let mut order: Vec<usize> = (0..items.len()).collect();
                    order.sort_by(|&a, &b| items[a].cmp(&items[b]));
                    let mut ranks = vec![Q::zero(); items.len()];
                    let mut i = 0;
                    while i < order.len() {
                        let mut j = i;
                        while j + 1 < order.len() && items[order[j + 1]] == items[order[i]] {
                            j += 1;
                        }
                        let rank = Q::new(BigInt::from(i + j + 2), BigInt::from(2));
                        for &k in &order[i..=j] {
                            ranks[k] = rank.clone();
                        }
                        i = j + 1;
                    }
                    Some(Ok(show_list(&ranks)))
                }
                "histogram" | "freq" | "counts" => {
                    let mut counts: BTreeMap<Q, usize> = BTreeMap::new();
                    for x in items {
                        *counts.entry(x).or_default() += 1;
                    }
                    let shown: Vec<String> = counts
                        .iter()
                        .map(|(v, c)| format!("{}: {c}", show(v)))
                        .collect();
                    Some(Ok(format!("{{{}}}", shown.join(", "))))
                }
                _ => None,
            }
        }
    }
}

trait ToUsizeChecked {
    fn to_usize_checked(&self) -> Option<usize>;
}

impl ToUsizeChecked for BigInt {
    fn to_usize_checked(&self) -> Option<usize> {
        num_traits::ToPrimitive::to_usize(self)
    }
}
