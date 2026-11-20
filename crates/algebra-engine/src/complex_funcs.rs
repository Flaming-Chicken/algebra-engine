//! Complex-number and discrete-transform commands on constant arguments: `arg`, `modulus`, `polar`, `atan2`,
//! `convolve`, `dft` and `fft`.

use crate::numeric::{EvalContext, NumericalEval};
use crate::session::UraeSession;
use algebra_core::parser::ExprParser;

/// Split `name(args)` (the call spans the whole input) into the lower-case name and the top-level arguments.
fn split_call(input: &str) -> Option<(String, Vec<String>)> {
    let t = input.trim();
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if name.is_empty()
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || !t.ends_with(')')
    {
        return None;
    }
    // The first parenthesis must close at the very end.
    let mut depth = 0i32;
    for (i, c) in t[open..].char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth -= 1;
                if depth == 0 && open + i != t.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    Some((name, split_top(&t[open + 1..t.len() - 1])))
}

/// A list argument as text: a literal list stays, a nested call (`fft([1, 2])`) is evaluated first.
fn list_argument(session: &mut UraeSession, text: &str) -> Option<Vec<String>> {
    if let Some(items) = list_items(text) {
        return Some(items);
    }
    if text.contains('(') {
        let value = session.eval(text).ok()?;
        return list_items(&value);
    }
    None
}

fn split_top(text: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
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

fn complex_value(session: &UraeSession, text: &str) -> Result<(f64, f64), String> {
    let parser = ExprParser::new(&session.graph);
    let id = parser.parse(text).map_err(|e| e.to_string())?;
    let value = session
        .graph
        .evalf(id, &EvalContext::default())
        .map_err(|e| format!("{text} is not a constant: {e}"))?;
    let (re, im) = value.as_complex();
    if re.is_finite() && im.is_finite() {
        Ok((re, im))
    } else {
        Err(format!("{text} is not a finite number"))
    }
}

/// `p*pi/q` text for an angle that is a rational multiple of pi with a small denominator.
fn angle_text(theta: f64) -> String {
    let ratio = theta / std::f64::consts::PI;
    for q in [1i64, 2, 3, 4, 6, 8, 12] {
        let p = (ratio * q as f64).round();
        if (ratio * q as f64 - p).abs() < 1e-12 {
            let p = p as i64;
            let g = gcd(p.abs(), q).max(1);
            let (p, q) = (p / g, q / g);
            return match (p, q) {
                (0, _) => "0".into(),
                (1, 1) => "π".into(),
                (-1, 1) => "-π".into(),
                (p, 1) => format!("{p}*π"),
                (1, q) => format!("π/{q}"),
                (-1, q) => format!("-π/{q}"),
                (p, q) => format!("{p}*π/{q}"),
            };
        }
    }
    format!("{theta}")
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn clean(x: f64) -> String {
    let r = x.round();
    if (x - r).abs() < 1e-10 * (1.0 + x.abs()) {
        format!("{}", r as i64)
    } else {
        let s = format!("{x:.12}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn complex_text((re, im): (f64, f64)) -> String {
    let (r, i) = (clean(re), clean(im));
    if i == "0" {
        r
    } else if r == "0" {
        format!("{i}i")
    } else if im < 0.0 {
        format!("{r} - {}i", clean(-im))
    } else {
        format!("{r} + {i}i")
    }
}

fn list_items(text: &str) -> Option<Vec<String>> {
    let t = text.trim();
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(split_top(inner))
}

/// Discrete Fourier transform (radix 2 when the length is a power of two, the direct sum otherwise).
fn dft(x: &[(f64, f64)], inverse: bool) -> Vec<(f64, f64)> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    if n > 1 && n.is_power_of_two() {
        let even: Vec<_> = x.iter().step_by(2).copied().collect();
        let odd: Vec<_> = x.iter().skip(1).step_by(2).copied().collect();
        let (e, o) = (dft(&even, inverse), dft(&odd, inverse));
        let mut out = vec![(0.0, 0.0); n];
        for k in 0..n / 2 {
            let a = sign * 2.0 * std::f64::consts::PI * k as f64 / n as f64;
            let (c, s) = (a.cos(), a.sin());
            let t = (c * o[k].0 - s * o[k].1, c * o[k].1 + s * o[k].0);
            out[k] = (e[k].0 + t.0, e[k].1 + t.1);
            out[k + n / 2] = (e[k].0 - t.0, e[k].1 - t.1);
        }
        return out;
    }
    (0..n)
        .map(|k| {
            x.iter().enumerate().fold((0.0, 0.0), |acc, (j, v)| {
                let a = sign * 2.0 * std::f64::consts::PI * ((j * k) % n) as f64 / n as f64;
                let (c, s) = (a.cos(), a.sin());
                (acc.0 + c * v.0 - s * v.1, acc.1 + c * v.1 + s * v.0)
            })
        })
        .collect()
}

/// `Some(result)` when `input` is one of the commands above applied to constants.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let (name, args) = split_call(input)?;
    match name.as_str() {
        "arg" | "phase" | "argument" | "modulus" | "polar" | "atan2" | "convolve" | "conv"
        | "dft" | "fft" | "ifft" | "idft" => {}
        _ => return None,
    }
    Some(run_named(session, &name, &args))
}

fn run_named(session: &mut UraeSession, name: &str, args: &[String]) -> Result<String, String> {
    match name {
        "atan2" => {
            if args.len() != 2 {
                return Err("atan2(y, x)".into());
            }
            let y = complex_value(session, &args[0])?.0;
            let x = complex_value(session, &args[1])?.0;
            if x == 0.0 && y == 0.0 {
                return Ok("undefined".into());
            }
            Ok(angle_text(y.atan2(x)))
        }
        "arg" | "phase" | "argument" | "modulus" | "polar" => {
            if args.len() != 1 {
                return Err(format!("{name}(z) takes one complex number"));
            }
            let (re, im) = complex_value(session, &args[0])?;
            let re_t = session.eval(&format!("re({})", args[0]))?;
            let im_t = session.eval(&format!("im({})", args[0]))?;
            let modulus = session.eval(&format!("sqrt(({re_t})^2 + ({im_t})^2)"))?;
            let theta = im.atan2(re);
            let zero = re == 0.0 && im == 0.0;
            match name {
                "modulus" => Ok(modulus),
                "polar" if zero => Ok("0".into()),
                "polar" => Ok(format!("{modulus}·e^(i·{})", angle_text(theta))),
                _ if zero => Ok("undefined".into()),
                _ => Ok(angle_text(theta)),
            }
        }
        "convolve" | "conv" => {
            if args.len() != 2 {
                return Err("convolve([a, b, ...], [c, d, ...])".into());
            }
            let a = list_argument(session, &args[0]).ok_or("convolve needs two lists")?;
            let b = list_argument(session, &args[1]).ok_or("convolve needs two lists")?;
            if a.is_empty() || b.is_empty() {
                return Err("convolve needs non-empty lists".into());
            }
            let ints =
                |v: &[String]| -> Option<Vec<i128>> { v.iter().map(|s| s.parse().ok()).collect() };
            if let (Some(x), Some(y)) = (ints(&a), ints(&b)) {
                let mut out = vec![0i128; x.len() + y.len() - 1];
                for (i, p) in x.iter().enumerate() {
                    for (j, q) in y.iter().enumerate() {
                        out[i + j] += p * q;
                    }
                }
                let shown: Vec<String> = out.iter().map(|v| v.to_string()).collect();
                return Ok(format!("[{}]", shown.join(", ")));
            }
            let x: Vec<(f64, f64)> = a
                .iter()
                .map(|s| complex_value(session, s))
                .collect::<Result<_, _>>()?;
            let y: Vec<(f64, f64)> = b
                .iter()
                .map(|s| complex_value(session, s))
                .collect::<Result<_, _>>()?;
            let mut out = vec![(0.0, 0.0); x.len() + y.len() - 1];
            for (i, p) in x.iter().enumerate() {
                for (j, q) in y.iter().enumerate() {
                    out[i + j].0 += p.0 * q.0 - p.1 * q.1;
                    out[i + j].1 += p.0 * q.1 + p.1 * q.0;
                }
            }
            let shown: Vec<String> = out.into_iter().map(complex_text).collect();
            Ok(format!("[{}]", shown.join(", ")))
        }
        _ => {
            let inverse = name == "ifft" || name == "idft";
            let items = args
                .first()
                .and_then(|a| list_argument(session, a))
                .ok_or(format!("{name}([x0, x1, ...])"))?;
            let x: Vec<(f64, f64)> = items
                .iter()
                .map(|s| complex_value(session, s))
                .collect::<Result<_, _>>()?;
            let n = x.len() as f64;
            let out = dft(&x, inverse);
            let shown: Vec<String> = out
                .into_iter()
                .map(|(r, i)| if inverse { (r / n, i / n) } else { (r, i) })
                .map(complex_text)
                .collect();
            Ok(format!("[{}]", shown.join(", ")))
        }
    }
}
