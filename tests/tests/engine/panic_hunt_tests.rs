//! Panic hunting: hostile and random text goes through the parser and the main symbolic operations, and none of
//! it may panic (an error is fine, a wrong answer is another test's business). Deterministic, so a failure
//! reproduces.

use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprKind};
use algebra_engine::{Simplifier, SymbolicCalculus};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }
}

const ATOMS: &[&str] = &[
    "x",
    "y",
    "0",
    "1",
    "2",
    "-1",
    "3.5",
    "1/0",
    "0^0",
    "pi",
    "e",
    "i",
    "99999999999999999999",
    "1e308",
    "1e-320",
    "0.0000001",
    "x^x",
    "inf",
    "nan",
];
const FUNCS: &[&str] = &[
    "sin",
    "cos",
    "tan",
    "exp",
    "ln",
    "log",
    "sqrt",
    "abs",
    "asin",
    "acos",
    "atan",
    "sinh",
    "cosh",
    "tanh",
    "gamma",
    "erf",
    "floor",
    "ceil",
    "sign",
    "factorial",
];
const BINOPS: &[&str] = &["+", "-", "*", "/", "^", "=", "<", ">=", "%"];

fn random_expr(rng: &mut Lcg, depth: usize) -> String {
    if depth == 0 || rng.below(4) == 0 {
        return ATOMS[rng.below(ATOMS.len())].to_string();
    }
    match rng.below(5) {
        0 => format!(
            "{}({})",
            FUNCS[rng.below(FUNCS.len())],
            random_expr(rng, depth - 1)
        ),
        1 => format!("({})", random_expr(rng, depth - 1)),
        2 => format!("-{}", random_expr(rng, depth - 1)),
        _ => format!(
            "{} {} {}",
            random_expr(rng, depth - 1),
            BINOPS[rng.below(BINOPS.len())],
            random_expr(rng, depth - 1)
        ),
    }
}

/// Noise: random bytes drawn from characters a parser is most likely to trip on.
fn random_noise(rng: &mut Lcg) -> String {
    const CHARS: &[char] = &[
        '(', ')', '[', ']', '{', '}', '\\', '^', '_', '/', '*', '+', '-', '=', '<', '>', '|', '!',
        ',', ';', '.', ' ', '0', '1', '9', 'x', 'a', 'z', 'é', 'π', '∫', '∑', '\u{0}', '\n', '\t',
        '$', '%', '&', '#', '@', '~', '\'', '"',
    ];
    let len = rng.below(40);
    (0..len).map(|_| CHARS[rng.below(CHARS.len())]).collect()
}

fn latex_fragments(rng: &mut Lcg) -> String {
    const PIECES: &[&str] = &[
        "\\frac{",
        "}{",
        "}",
        "\\sqrt{",
        "\\sqrt[3]{",
        "\\int_0^1",
        "\\sum_{n=1}^{",
        "\\lim_{x\\to 0}",
        "\\sin",
        "\\cos",
        "x",
        "^{2}",
        "_{i}",
        "\\left(",
        "\\right)",
        "\\begin{matrix}",
        "\\end{matrix}",
        "&",
        "\\\\",
        "\\cdot",
        "\\pi",
        "\\infty",
        "\\partial",
        "\\nabla",
        "{",
        "(",
        "1",
        "2",
        "dx",
    ];
    let n = 1 + rng.below(10);
    (0..n)
        .map(|_| PIECES[rng.below(PIECES.len())])
        .collect::<Vec<_>>()
        .join("")
}

fn run_pipeline(input: &str) {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let Ok(id) = parser.parse(input) else { return };
    let x = graph.symbol("x");
    let _ = Simplifier::simplify(&graph, id);
    if let ExprKind::Symbol(sym) = graph.get(x).kind {
        let d = graph.diff(id, sym);
        let _ = Simplifier::simplify(&graph, d);
        let _ = graph.integrate(id, sym);
    }
}

fn hunt(label: &str, cases: Vec<String>) {
    let mut panics: Vec<String> = Vec::new();
    // Silence the default hook so a hit does not flood the log; the message is recorded instead.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for case in &cases {
        let result = catch_unwind(AssertUnwindSafe(|| run_pipeline(case)));
        if result.is_err() {
            panics.push(case.clone());
        }
    }
    std::panic::set_hook(previous);
    assert!(
        panics.is_empty(),
        "{label}: {} of {} inputs panicked; first few: {:?}",
        panics.len(),
        cases.len(),
        &panics[..panics.len().min(8)]
    );
}

#[test]
fn random_well_formed_expressions_never_panic() {
    let mut rng = Lcg(12345);
    let cases: Vec<String> = (0..1500).map(|_| random_expr(&mut rng, 4)).collect();
    hunt("well-formed", cases);
}

#[test]
fn random_noise_never_panics() {
    let mut rng = Lcg(777);
    let cases: Vec<String> = (0..3000).map(|_| random_noise(&mut rng)).collect();
    hunt("noise", cases);
}

#[test]
fn random_latex_fragments_never_panic() {
    let mut rng = Lcg(2024);
    let cases: Vec<String> = (0..3000).map(|_| latex_fragments(&mut rng)).collect();
    hunt("latex", cases);
}

#[test]
fn known_nasty_inputs_never_panic() {
    let cases: Vec<String> = [
        "",
        " ",
        "(",
        ")",
        "((((((((((((((((((((((((((((((((((((((((x",
        "x^^^^2",
        "1/0",
        "0/0",
        "0^0",
        "ln(0)",
        "ln(-1)",
        "sqrt(-1)",
        "x^99999999999999999999",
        "99999999999999999999999999999999999999999999^99999999999",
        "\\frac{1}{",
        "\\sqrt[0]{x}",
        "\\sum_{n=1}^{}",
        "\\int",
        "x_{",
        "∫∫∫",
        "9223372036854775807 + 1",
        "-9223372036854775808 * -1",
        "9223372036854775807 * 9223372036854775807",
        "x^(1/0)",
        "(-8)^(1/3)",
        "1e999999",
        "factorial(1000000)",
        "gamma(-1)",
        "tan(pi/2)",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    hunt("nasty", cases);
}

#[test]
fn the_hunt_actually_reaches_the_algebra() {
    // Guard against a vacuous hunt: most well-formed inputs must parse, so the operations really run.
    let mut rng = Lcg(12345);
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let total = 300;
    let parsed = (0..total)
        .filter(|_| parser.parse(&random_expr(&mut rng, 4)).is_ok())
        .count();
    assert!(
        parsed * 2 > total,
        "only {parsed} of {total} well-formed inputs parsed"
    );
}

#[test]
fn deeply_nested_input_is_refused_or_handled_not_a_stack_overflow() {
    // A stack overflow aborts the whole process, so this can only be tested by surviving it. Depths far beyond
    // what anyone types, on a thread with the default test stack; only the simplifier is run here (the derivative
    // of a deep power tower is legitimately astronomically large, which is a different matter).
    for depth in [60usize, 200, 2_000, 20_000] {
        let open = "(".repeat(depth);
        let close = ")".repeat(depth);
        let nested = format!("{open}x{close}");
        let powers = format!("{}x{}", "x^(".repeat(depth), ")".repeat(depth));
        let sums = vec!["x"; depth].join(" + ");
        let negs = format!("{}x", "-".repeat(depth));
        let funcs = format!("{}x{}", "sin(".repeat(depth), ")".repeat(depth));
        for input in [nested, powers, sums, negs, funcs].iter() {
            let graph = ExprGraph::new();
            if let Ok(id) = ExprParser::new(&graph).parse(input) {
                let _ = Simplifier::simplify(&graph, id);
            }
        }
    }
}

#[test]
fn the_depth_limit_refuses_deep_expressions_and_accepts_ordinary_ones() {
    use algebra_engine::simplify::{MAX_SIMPLIFY_DEPTH, expression_depth};
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let ok = parser.parse("sin(x^2 + 1) / (x + 3)").unwrap();
    assert!(expression_depth(&graph, ok) < 10);
    assert!(Simplifier::simplify(&graph, ok).is_ok());
    // A graph built directly (not through the parser) can be as deep as the caller likes; the simplifier refuses
    // what is deeper than its limit instead of overflowing the stack.
    let x = graph.symbol("x");
    let mut deep = x;
    for _ in 0..(MAX_SIMPLIFY_DEPTH + 10) {
        deep = graph.pow(deep, x);
    }
    assert!(expression_depth(&graph, deep) > MAX_SIMPLIFY_DEPTH);
    assert!(Simplifier::simplify(&graph, deep).is_err());
    // The parser refuses what would overflow before it gets that far.
    let brackets = format!("{}x{}", "(".repeat(500), ")".repeat(500));
    assert!(parser.parse(&brackets).is_err());
}

#[test]
fn the_nesting_guard_does_not_reject_long_flat_expressions() {
    // A 300-term polynomial with powers and several nested brackets is ordinary input and must still parse.
    let poly = (1..=300)
        .map(|k| format!("{k}*x^{k}"))
        .collect::<Vec<_>>()
        .join(" + ");
    let graph = ExprGraph::new();
    assert!(ExprParser::new(&graph).parse(&poly).is_ok());
    let nested = "((((sin(cos(exp(x^2)))))))";
    assert!(ExprParser::new(&graph).parse(nested).is_ok());
}

/// Whole commands through the session: every command word against random expressions. A stack overflow aborts the
/// process, so this test also fails by crashing; the case is printed before it runs (with `--nocapture`).
#[test]
fn random_commands_never_panic() {
    const COMMANDS: &[&str] = &[
        "simplify",
        "expand",
        "factor",
        "solve",
        "diff",
        "integrate",
        "limit",
        "series",
        "taylor",
        "partial_fractions",
        "apart",
        "det",
        "inv",
        "rank",
        "transpose",
        "is_prime",
        "prime_factors",
        "totient",
        "sigma",
        "divisors",
        "mobius",
        "gcd",
        "lcm",
        "ode",
        "dsolve",
        "float",
        "eval",
        "latex",
        "collect",
        "together",
        "cancel",
        "horner",
    ];
    let mut rng = Lcg(31337);
    let mut cases = Vec::new();
    for _ in 0..400 {
        let command = COMMANDS[rng.below(COMMANDS.len())];
        let body = random_expr(&mut rng, 3);
        cases.push(if rng.below(2) == 0 {
            format!("{command} {body}")
        } else {
            format!("{command}({body})")
        });
    }
    let verbose = std::env::var("HUNT_VERBOSE").is_ok();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut panics = Vec::new();
    let mut slow: Vec<String> = Vec::new();
    for case in &cases {
        if verbose {
            eprintln!("CASE {case}");
        }
        let started = std::time::Instant::now();
        let case_owned = case.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut session = algebra_engine::session::UraeSession::new();
            let _ = session.execute_line(&case_owned);
        }));
        if started.elapsed().as_secs_f64() > 20.0 {
            slow.push(format!("{:.0}s {case}", started.elapsed().as_secs_f64()));
        }
        if result.is_err() {
            panics.push(case.clone());
        }
    }
    std::panic::set_hook(previous);
    assert!(
        panics.is_empty(),
        "{} of {} commands panicked; first few: {:?}",
        panics.len(),
        cases.len(),
        &panics[..panics.len().min(10)]
    );
}
