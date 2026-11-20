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
    if let Some(outcome) = unit_expression(session, input) {
        return Some(outcome);
    }
    let (name, args) = split_call(input)?;
    match name.as_str() {
        "arg" | "phase" | "argument" | "modulus" | "polar" | "atan2" | "convolve" | "conv"
        | "dft" | "fft" | "ifft" | "idft" | "residue" | "poles" => {}
        "subs" | "substitute" | "replace" | "evaluate" | "eval" => {
            // `subs(f, x, 3)` or `subs(f, x = 3, y = 4)`: replace whole-word variables, then evaluate.
            if args.len() < 2 {
                return None;
            }
            let mut pairs: Vec<(String, String)> = Vec::new();
            if args.len() == 3 && !args[1].contains('=') {
                pairs.push((args[1].clone(), args[2].clone()));
            } else {
                for a in &args[1..] {
                    let (v, value) = a.split_once('=')?;
                    pairs.push((v.trim().to_string(), value.trim().to_string()));
                }
            }
            if pairs
                .iter()
                .any(|(v, _)| v.is_empty() || !v.chars().all(|c| c.is_alphanumeric() || c == '_'))
            {
                return None;
            }
            // Simultaneous replacement through placeholders, so `subs(x + y, x = y, y = 1)` is not applied twice.
            let mut text = args[0].clone();
            for (k, (v, _)) in pairs.iter().enumerate() {
                text = replace_word(&text, v, &format!("§{k}§"));
            }
            for (k, (_, value)) in pairs.iter().enumerate() {
                text = text.replace(&format!("§{k}§"), &format!("({value})"));
            }
            return Some(session.eval(&text));
        }
        "degrees"
        | "rad2deg"
        | "radians"
        | "deg2rad"
        | "celsius_to_fahrenheit"
        | "fahrenheit_to_celsius"
        | "celsius_to_kelvin"
        | "kelvin_to_celsius"
        | "c_to_f"
        | "f_to_c" => {
            let [x] = args.as_slice() else { return None };
            // A typed `pi` is the symbol; the rewrite below uses the constant, so make them one.
            let x = &replace_word(x, "pi", "π");
            let rewritten = match name.as_str() {
                "degrees" | "rad2deg" => format!("({x})*180/π"),
                "radians" | "deg2rad" => format!("({x})*π/180"),
                "celsius_to_fahrenheit" | "c_to_f" => format!("({x})*9/5 + 32"),
                "fahrenheit_to_celsius" | "f_to_c" => format!("(({x}) - 32)*5/9"),
                "celsius_to_kelvin" => format!("({x}) + 5463/20"),
                _ => format!("({x}) - 5463/20"),
            };
            return Some(session.eval(&rewritten));
        }
        "hypotenuse" | "heron" | "area_triangle" | "angle_between" | "maclaurin" => {
            // Spellings that are fixed formulas of the plain commands.
            let rewritten = match (name.as_str(), args.as_slice()) {
                ("hypotenuse", [a, b]) => format!("sqrt(({a})^2 + ({b})^2)"),
                ("heron" | "area_triangle", [a, b, c]) if a.trim_start().starts_with('(') => {
                    format!("triangle_area({a}, {b}, {c})")
                }
                ("heron" | "area_triangle", [a, b, c]) => {
                    let s = format!("(({a}) + ({b}) + ({c}))/2");
                    format!("sqrt(({s})*(({s}) - ({a}))*(({s}) - ({b}))*(({s}) - ({c})))")
                }
                ("angle_between", [u, v]) => {
                    let dot = match session.eval(&format!("dot({u}, {v})")) {
                        Ok(d) => d,
                        Err(e) => return Some(Err(e)),
                    };
                    let (nu, nv) = match (
                        session.eval(&format!("norm({u})")),
                        session.eval(&format!("norm({v})")),
                    ) {
                        (Ok(a), Ok(b)) => (a, b),
                        (Err(e), _) | (_, Err(e)) => return Some(Err(e)),
                    };
                    format!("acos(({dot})/(({nu})*({nv})))")
                }
                ("maclaurin", [f, order]) => format!("taylor({f}, x, 0, {order})"),
                _ => return None,
            };
            if name == "angle_between" {
                let exact = match session.eval(&rewritten) {
                    Ok(e) => e,
                    Err(e) => return Some(Err(e)),
                };
                // Recognise an exact multiple of pi from the numeric angle.
                let theta = complex_value(session, &rewritten).ok()?.0;
                let multiple = angle_text(theta);
                if multiple.contains('π') || multiple == "0" {
                    return Some(Ok(multiple));
                }
                return Some(Ok(format!("{exact} ≈ {}", clean(theta))));
            }
            return Some(session.eval(&rewritten));
        }
        "lcm" => {
            // Integer lcm is handled elsewhere; this is the polynomial case, lcm = a b / gcd(a, b).
            if args.len() < 2 || args.iter().all(|a| a.trim().parse::<i128>().is_ok()) {
                return None;
            }
            let mut current = args[0].clone();
            for next in &args[1..] {
                let g = match session.eval(&format!("gcd({current}, {next})")) {
                    Ok(g) => g,
                    Err(e) => return Some(Err(e)),
                };
                current = match session.eval(&format!("cancel(({current})*({next})/({g}))")) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
            }
            return Some(Ok(current));
        }
        "abs" => {
            // Only a complex constant with an imaginary part: everything else is the ordinary abs.
            let [arg] = args.as_slice() else { return None };
            let (_, im) = complex_value(session, arg).ok()?;
            if im == 0.0 {
                return None;
            }
            return Some(run_named(session, "modulus", &args));
        }
        _ => return None,
    }
    Some(run_named(session, &name, &args))
}

fn run_named(session: &mut UraeSession, name: &str, args: &[String]) -> Result<String, String> {
    match name {
        "residue" => residue(session, args),
        "poles" => poles(session, args),
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

// ------------------------------------------------------------------------------------------------------------
// Residues and poles
// ------------------------------------------------------------------------------------------------------------

/// The expression with the variable replaced by the complex number `(re, im)`, evaluated to a complex value.
fn eval_at(
    session: &UraeSession,
    expr: algebra_core::ExprId,
    symbol: algebra_core::SymbolId,
    imaginary: algebra_core::ExprId,
    (re, im): (f64, f64),
) -> Option<(f64, f64)> {
    let g = &session.graph;
    let point = g.add([g.float(re), g.mul([g.float(im), imaginary])]);
    let replaced = g.substitute(expr, symbol, point);
    let v = g.evalf(replaced, &EvalContext::default()).ok()?;
    let (a, b) = v.as_complex();
    (a.is_finite() && b.is_finite()).then_some((a, b))
}

/// The sole variable of `expr` (or the one named), as a symbol.
fn variable_of(
    session: &UraeSession,
    expr: algebra_core::ExprId,
    named: Option<&str>,
) -> Result<algebra_core::SymbolId, String> {
    let g = &session.graph;
    if let Some(n) = named {
        return Ok(g.symbols.get_or_intern(n));
    }
    let mut symbols = std::collections::BTreeSet::new();
    crate::poly::collect_expr_symbols(g, expr, &mut symbols);
    let names: Vec<_> = symbols
        .into_iter()
        .filter(|s| !matches!(g.symbols.resolve(*s).as_deref(), Some("pi" | "e" | "i")))
        .collect();
    match names.as_slice() {
        [one] => Ok(*one),
        [] => Ok(g.symbols.get_or_intern("x")),
        _ => Err("several variables: name the one, for example `residue(f, x, 0)`".into()),
    }
}

/// `1 / (2 pi i)` times the integral of `f` over the circle of radius `r` about `a`, by the trapezoid rule
/// (it converges geometrically for a function analytic in a neighbourhood of the circle).
fn contour(
    session: &UraeSession,
    expr: algebra_core::ExprId,
    symbol: algebra_core::SymbolId,
    imaginary: algebra_core::ExprId,
    a: (f64, f64),
    r: f64,
) -> Option<(f64, f64)> {
    let n = 1024usize;
    let (mut sr, mut si) = (0.0, 0.0);
    for k in 0..n {
        let t = 2.0 * std::f64::consts::PI * (k as f64 + 0.5) / n as f64;
        let (c, s) = (t.cos(), t.sin());
        let z = (a.0 + r * c, a.1 + r * s);
        let f = eval_at(session, expr, symbol, imaginary, z)?;
        // dz / (2 pi i) = (z - a) dt / (2 pi), so the average of f * (z - a) over the circle.
        let w = (r * c, r * s);
        sr += f.0 * w.0 - f.1 * w.1;
        si += f.0 * w.1 + f.1 * w.0;
    }
    Some((sr / n as f64, si / n as f64))
}

/// A number as a small rational when it is one to about nine digits, otherwise as a decimal.
fn rational_text(x: f64) -> String {
    for q in 1..=360i64 {
        let p = (x * q as f64).round();
        if (x - p / q as f64).abs() < 1e-9 {
            let p = p as i64;
            let g = gcd(p.abs(), q).max(1);
            return if q / g == 1 {
                format!("{}", p / g)
            } else {
                format!("{}/{}", p / g, q / g)
            };
        }
    }
    clean(x)
}

fn complex_rational_text((re, im): (f64, f64)) -> String {
    let (r, i) = (rational_text(re), rational_text(im));
    match (r.as_str(), i.as_str()) {
        (_, "0") => r,
        ("0", "1") => "i".into(),
        ("0", "-1") => "-i".into(),
        ("0", _) => format!("{i}·i"),
        (_, _) if im < 0.0 => format!("{r} - {}·i", i.trim_start_matches('-')),
        _ => format!("{r} + {i}·i"),
    }
}

fn residue(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let (function, named, point) = match args {
        [f, a] => (f.as_str(), None, a.as_str()),
        [f, x, a] => (f.as_str(), Some(x.as_str()), a.as_str()),
        _ => return Err("residue(f, x, a)".into()),
    };
    let parser = ExprParser::new(&session.graph);
    let expr = parser.parse(function).map_err(|e| e.to_string())?;
    let imaginary = parser.parse("i").map_err(|e| e.to_string())?;
    let symbol = variable_of(session, expr, named)?;
    let a = complex_value(session, point)?;
    let near = "f is not defined near the point";
    let coarse = contour(session, expr, symbol, imaginary, a, 1e-2).ok_or(near)?;
    let fine = contour(session, expr, symbol, imaginary, a, 1e-3).ok_or(near)?;
    if (coarse.0 - fine.0).abs() > 1e-7 || (coarse.1 - fine.1).abs() > 1e-7 {
        return Err(
            "the point is too close to another singularity, or the singularity is not isolated"
                .into(),
        );
    }
    Ok(complex_rational_text(fine))
}

/// Top-level `numerator / denominator` split of a text with exactly one top-level division.
fn split_fraction(text: &str) -> Option<(String, String)> {
    let (mut depth, mut at) = (0i32, None);
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '/' if depth == 0 => {
                if at.is_some() {
                    return None;
                }
                at = Some(i);
            }
            _ => {}
        }
    }
    let i = at?;
    let n: String = chars[..i].iter().collect();
    let d: String = chars[i + 1..].iter().collect();
    Some((n.trim().to_string(), d.trim().to_string()))
}

fn poles(session: &mut UraeSession, args: &[String]) -> Result<String, String> {
    let (function, named) = match args {
        [f] => (f.as_str(), None),
        [f, x] => (f.as_str(), Some(x.as_str())),
        _ => return Err("poles(f)".into()),
    };
    let parser = ExprParser::new(&session.graph);
    let expr = parser.parse(function).map_err(|e| e.to_string())?;
    let imaginary = parser.parse("i").map_err(|e| e.to_string())?;
    let symbol = variable_of(session, expr, named)?;
    let var = session.graph.symbols.resolve(symbol).unwrap_or_default();
    let together = session.eval(&format!("cancel(together({function}))"))?;
    let Some((_, denominator)) = split_fraction(&together) else {
        return Ok("no poles".into());
    };
    let solved = session.eval(&format!("solve({denominator} = 0, {var})"))?;
    let (Some(open), Some(close)) = (solved.find('['), solved.rfind(']')) else {
        return Err(format!("could not solve the denominator: {solved}"));
    };
    let mut found = Vec::new();
    for root in split_top(&solved[open + 1..close]) {
        let a = complex_value(session, &root)?;
        // The order from how fast |f| grows: |f(a + h)| ~ h^-m.
        let magnitude = |h: f64| {
            eval_at(session, expr, symbol, imaginary, (a.0 + h, a.1))
                .map(|v| (v.0 * v.0 + v.1 * v.1).sqrt())
        };
        let order = match (magnitude(1e-3), magnitude(1e-4)) {
            (Some(p), Some(q)) if p > 0.0 && q > 0.0 => (q / p).log10().round() as i64,
            _ => 1,
        };
        found.push(format!("{var} = {root} (order {})", order.max(1)));
    }
    if found.is_empty() {
        Ok("no poles".into())
    } else {
        Ok(found.join(", "))
    }
}

/// `text` with every whole-word occurrence of the identifier `word` replaced by `with`.
fn replace_word(text: &str, word: &str, with: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let w: Vec<char> = word.chars().collect();
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let (mut out, mut i) = (String::new(), 0usize);
    while i < chars.len() {
        let boundary_before = i == 0 || !is_ident(chars[i - 1]);
        if boundary_before
            && chars[i..].starts_with(&w)
            && chars.get(i + w.len()).is_none_or(|c| !is_ident(*c))
        {
            out.push_str(with);
            i += w.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------------------
// Quantities with units written as a sentence: `5 km to m`, `3 kg + 500 g to g`, `convert 1 h + 30 min to s`.
// ------------------------------------------------------------------------------------------------------------

/// `number unit` as `(number text, unit text)`.
fn quantity_term(term: &str) -> Option<(String, String)> {
    let t = term.trim();
    let split = t.find(char::is_whitespace)?;
    let (number, unit) = (t[..split].trim(), t[split..].trim());
    let numeric = !number.is_empty()
        && number
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '/' | '-'))
        && number.chars().any(|c| c.is_ascii_digit());
    let unit_ok = !unit.is_empty()
        && !unit.contains(char::is_whitespace)
        && unit
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '°' || c == '%');
    (numeric && unit_ok).then(|| (number.to_string(), unit.to_string()))
}

/// Split `a + b - c` at ` + ` and ` - ` (with their signs).
fn signed_terms(text: &str) -> Vec<(f64, String)> {
    let (mut out, mut sign, mut current) = (Vec::new(), 1.0, String::new());
    let words: Vec<&str> = text.split(' ').collect();
    let mut i = 0;
    while i < words.len() {
        match words[i] {
            "+" | "-" if !current.is_empty() => {
                out.push((sign, std::mem::take(&mut current)));
                sign = if words[i] == "-" { -1.0 } else { 1.0 };
            }
            w => {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(w);
            }
        }
        i += 1;
    }
    if !current.is_empty() {
        out.push((sign, current));
    }
    out
}

fn unit_expression(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let body = t.strip_prefix("convert ").unwrap_or(t);
    let (lhs, target) = [" to ", " in ", " as "]
        .iter()
        .filter_map(|w| body.rfind(w).map(|at| (at, w.len())))
        .max_by_key(|(at, _)| *at)
        .map(|(at, len)| (&body[..at], body[at + len..].trim()))?;
    if target.is_empty() || target.contains(char::is_whitespace) {
        return None;
    }
    let terms = signed_terms(lhs);
    let quantities: Vec<(f64, (String, String))> = terms
        .iter()
        .map(|(s, term)| quantity_term(term).map(|q| (*s, q)))
        .collect::<Option<_>>()?;
    if quantities.is_empty() {
        return None;
    }
    // A single quantity without the word `convert`: hand it to the conversion command.
    if quantities.len() == 1 {
        if t.starts_with("convert ") {
            return None;
        }
        let (_, (number, unit)) = &quantities[0];
        return Some(session.eval(&format!("convert {number} {unit} to {target}")));
    }
    let mut total = 0.0;
    let mut shown = Vec::new();
    for (sign, (number, unit)) in &quantities {
        let converted = match session.eval(&format!("convert {number} {unit} to {target}")) {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };
        let value: f64 = converted
            .rsplit(" = ")
            .next()
            .and_then(|r| r.split(" [").next())
            .and_then(|n| n.trim().parse().ok())?;
        total += sign * value;
        let term = format!("{number} [{unit}]");
        shown.push(if shown.is_empty() {
            term
        } else {
            format!("{} {term}", if *sign < 0.0 { "-" } else { "+" })
        });
    }
    Some(Ok(format!(
        "{} = {} [{target}]",
        shown.join(" "),
        clean(total)
    )))
}
