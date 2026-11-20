//! First-order equations the standard solver leaves unsolved: homogeneous equations `y' = F(y/x)` (the substitution
//! `v = y/x`) and exact equations `M dx + N dy = 0`.
//!
//! The standard path runs first; this method applies only when it answers with an analysis instead of a solution.

use crate::session::UraeSession;
use algebra_core::parser::ExprParser;
use std::collections::BTreeSet;

fn symbols_of(session: &UraeSession, text: &str) -> Option<BTreeSet<String>> {
    let graph = &session.graph;
    let id = ExprParser::new(graph).parse(text).ok()?;
    let mut ids = BTreeSet::new();
    crate::poly::collect_expr_symbols(graph, id, &mut ids);
    Some(
        ids.into_iter()
            .filter_map(|s| graph.symbols.resolve(s))
            .filter(|n| !matches!(n.as_str(), "pi" | "π" | "e" | "i"))
            .collect(),
    )
}

/// The value part of an `integrate` result (`F (w.r.t x)`).
fn strip_variable_note(text: &str) -> String {
    text.split(" (w.r.t")
        .next()
        .unwrap_or(text)
        .trim()
        .to_string()
}

/// `Some(result)` when `input` is a first-order `dsolve` that the standard solver does not solve and this does.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    let body = t.strip_prefix("dsolve(")?.strip_suffix(')')?;
    if body.contains("''") || body.contains(", ") && body.contains("(0)") {
        return None;
    }
    let (left, right) = body.split_once('=')?;
    if right.contains('=') {
        return None;
    }
    // The dependent variable is the name before the first prime.
    let prime = body.find('\'')?;
    let name: String = body[..prime]
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if name.is_empty() {
        return None;
    }
    // Let the standard solver try first (and guard against asking ourselves).
    if session.in_first_order_fallback {
        return None;
    }
    session.in_first_order_fallback = true;
    let standard = session.eval(t);
    session.in_first_order_fallback = false;
    match &standard {
        Ok(text) if text.starts_with("Differential Equation Analysis") => {}
        _ => return None,
    }
    // E = left - right = M + N y'.
    let primed = format!("{name}'");
    let equation = format!("({left}) - ({right})").replace(&primed, "dydx");
    let derivative = session.eval(&format!("diff(({equation}), dydx)")).ok()?;
    let m = session
        .eval(&format!("subs(({equation}), dydx = 0)"))
        .ok()?;
    if derivative.trim() == "0" {
        return None;
    }
    let curvature = session.eval(&format!("diff(({derivative}), dydx)")).ok()?;
    if curvature.trim() != "0" {
        return clairaut(session, &equation, &name);
    }
    let n = derivative;
    let symbols = symbols_of(session, &format!("({m}) + ({n})"))?;
    let independent: Vec<&String> = symbols
        .iter()
        .filter(|s| **s != name && *s != "dydx")
        .collect();
    let [x] = independent.as_slice() else {
        return None;
    };
    let x = (*x).clone();
    let y = name;

    // Exact: M dx + N dy = 0 with dM/dy = dN/dx has the potential phi(x, y) = C1.
    let closed = session
        .eval(&format!("simplify(diff(({m}), {y}) - diff(({n}), {x}))"))
        .ok()?;
    if closed.trim() == "0" {
        let phi = potential(session, &m, &n, &x, &y)?;
        return Some(Ok(format!(
            "{phi} = C1  (implicit solution of the exact equation)"
        )));
    }

    // Homogeneous: y' = F(x, y) = -M / N with F(x, v x) free of x; v' x = F - v is separable.
    if let Some(solution) = homogeneous(session, &m, &n, &x, &y) {
        return Some(Ok(solution));
    }

    // Integrating factors: mu(x) when (M_y - N_x) / N is free of y, mu(y) when (N_x - M_y) / M is free of x.
    for (divisor, other, variable, sign) in [(&n, &y, &x, ""), (&m, &x, &y, "-")] {
        let quotient = session
            .eval(&format!("simplify({sign}({closed})/({divisor}))"))
            .ok()?;
        if symbols_of(session, &quotient)?.contains(other) {
            continue;
        }
        let integral = session
            .eval(&format!("integrate(({quotient}), {variable})"))
            .ok()?;
        let integral = strip_variable_note(&integral);
        let mu = session.eval(&format!("exp({integral})")).ok()?;
        let new_m = session.eval(&format!("expand(({mu})*({m}))")).ok()?;
        let new_n = session.eval(&format!("expand(({mu})*({n}))")).ok()?;
        let check = session
            .eval(&format!(
                "simplify(diff(({new_m}), {y}) - diff(({new_n}), {x}))"
            ))
            .ok()?;
        if check.trim() != "0" {
            continue;
        }
        let phi = potential(session, &new_m, &new_n, &x, &y)?;
        return Some(Ok(format!(
            "{phi} = C1  (implicit solution; integrating factor {mu})"
        )));
    }
    None
}

/// The potential `phi` of an exact `M dx + N dy`: `phi_x = M`, `phi_y = N`.
fn potential(session: &mut UraeSession, m: &str, n: &str, x: &str, y: &str) -> Option<String> {
    let phi_x = session.eval(&format!("integrate(({m}), {x})")).ok()?;
    let phi_x = strip_variable_note(&phi_x);
    let remainder = session
        .eval(&format!("expand(({n}) - diff(({phi_x}), {y}))"))
        .ok()?;
    let h = session
        .eval(&format!("integrate(({remainder}), {y})"))
        .ok()?;
    let h = strip_variable_note(&h);
    session.eval(&format!("({phi_x}) + ({h})")).ok()
}

/// The solution of a homogeneous equation as text, or `None` when the equation is not homogeneous or cannot be integrated.
fn homogeneous(session: &mut UraeSession, m: &str, n: &str, x: &str, y: &str) -> Option<String> {
    let f = session.eval(&format!("simplify(-({m})/({n}))")).ok()?;
    let substituted = session.eval(&format!("subs(({f}), {y} = v*{x})")).ok()?;
    let g = session.eval(&format!("simplify({substituted})")).ok()?;
    if symbols_of(session, &g)?.contains(x) {
        return None;
    }
    let integrand = session
        .eval(&format!("cancel(together(1/(({g}) - v)))"))
        .ok()?;
    let integral = session.eval(&format!("integrate(({integrand}), v)")).ok()?;
    let integral = strip_variable_note(&integral);
    // Try to isolate v: G(v) = ln x + C1.
    if let Ok(solved) = session.eval(&format!("solve(({integral}) = ln({x}) + C1, v)"))
        && !solved.contains("RootOf")
        && let (Some(open), Some(close)) = (solved.find('['), solved.rfind(']'))
    {
        let roots: Vec<String> = solved[open + 1..close]
            .split(", ")
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty())
            .collect();
        let mut solutions = Vec::new();
        for root in &roots {
            let value = session.eval(&format!("simplify({x}*({root}))")).ok()?;
            solutions.push(format!("{y}({x}) = {value}"));
        }
        if !solutions.is_empty() {
            return Some(solutions.join("  or  "));
        }
    }
    let implicit = session
        .eval(&format!("subs(({integral}), v = {y}/{x})"))
        .ok()?;
    Some(format!(
        "{implicit} = ln({x}) + C1  (implicit solution: y cannot be isolated in closed form)"
    ))
}

/// Clairaut's equation `y = x y' + f(y')` (in any rearrangement): the lines `y = C x + f(C)` and the singular envelope
/// `x = -f'(p)`, `y = p x + f(p)`.
fn clairaut(session: &mut UraeSession, equation: &str, y: &str) -> Option<Result<String, String>> {
    let k = session.eval(&format!("diff(({equation}), {y})")).ok()?;
    let k = k.trim().to_string();
    if k == "0" || !symbols_of(session, &k)?.is_empty() {
        return None;
    }
    let symbols = symbols_of(session, equation)?;
    let independent: Vec<&String> = symbols.iter().filter(|s| *s != y && *s != "dydx").collect();
    let [x] = independent.as_slice() else {
        return None;
    };
    let x = (*x).clone();
    // E / k = y - x p - f(p), so f = y - x p - E / k must be free of x and y.
    let f = session
        .eval(&format!("simplify({y} - {x}*dydx - ({equation})/({k}))"))
        .ok()?;
    let used = symbols_of(session, &f)?;
    if used.contains(&x) || used.contains(y) {
        return None;
    }
    let general = session.eval(&format!("subs(({f}), dydx = C1)")).ok()?;
    let singular_x = session
        .eval(&format!("simplify(diff(-({f}), dydx))"))
        .ok()?;
    let singular_y = session
        .eval(&format!("simplify(dydx*({singular_x}) + ({f}))"))
        .ok()?;
    let line = format!("{y} = C1*{x} + ({general})");
    Some(Ok(format!(
        "{line}  (Clairaut equation; singular solution {x} = {singular_x}, {y} = {singular_y}, parameter dydx)"
    )))
}
