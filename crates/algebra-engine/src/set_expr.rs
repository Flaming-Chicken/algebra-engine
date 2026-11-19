//! Finite set expressions typed as plain input: `{1, 2, 3} union {3, 4}`, `{1,2,3} intersect {2,3,4}`,
//! `{1,2,3} minus {2}`, `{1,2} subset {1,2,3}`, `2 in {1,2}`, `size {1,2,2}`, `{1,2} cross {a,b}`.
//!
//! Elements are compared by their canonical text (numbers by value, so `2` and `4/2` are one element).

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::One;
use std::collections::BTreeSet;

type Set = Vec<String>;

fn canonical(item: &str) -> String {
    let item = item.trim();
    // A rational number in lowest terms; anything else keeps its text.
    if let Some((n, d)) = item.split_once('/')
        && let (Ok(n), Ok(d)) = (n.trim().parse::<BigInt>(), d.trim().parse::<BigInt>())
        && d != BigInt::from(0)
    {
        let q = BigRational::new(n, d);
        return if q.denom().is_one() {
            q.numer().to_string()
        } else {
            format!("{}/{}", q.numer(), q.denom())
        };
    }
    if let Ok(n) = item.parse::<BigInt>() {
        return n.to_string();
    }
    if let Ok(x) = item.parse::<f64>() {
        // 0.5 is 1/2 (finite decimals only).
        let text = format!("{x}");
        if let Some((a, b)) = text.split_once('.') {
            let scale = BigInt::from(10).pow(b.len() as u32);
            if let Ok(numerator) = format!("{a}{b}").parse::<BigInt>() {
                let q = BigRational::new(numerator, scale);
                return if q.denom().is_one() {
                    q.numer().to_string()
                } else {
                    format!("{}/{}", q.numer(), q.denom())
                };
            }
        }
    }
    item.to_string()
}

fn sort_key(item: &str) -> (u8, f64, String) {
    let value = match item.split_once('/') {
        Some((n, d)) => n
            .parse::<f64>()
            .ok()
            .zip(d.parse::<f64>().ok())
            .map(|(n, d)| n / d),
        None => item.parse::<f64>().ok(),
    };
    match value {
        Some(v) => (0, v, String::new()),
        None => (1, 0.0, item.to_string()),
    }
}

fn normalise(items: Vec<String>) -> Set {
    let unique: BTreeSet<String> = items.into_iter().map(|i| canonical(&i)).collect();
    let mut v: Vec<String> = unique.into_iter().collect();
    v.sort_by(|a, b| {
        let (ka, kb) = (sort_key(a), sort_key(b));
        ka.0.cmp(&kb.0)
            .then(ka.1.total_cmp(&kb.1))
            .then(ka.2.cmp(&kb.2))
    });
    v
}

fn show(set: &Set) -> String {
    format!("{{{}}}", set.join(", "))
}

/// Split `{a, b, c}` into its items (top-level commas only).
fn items(text: &str) -> Option<Vec<String>> {
    let inner = text.trim().strip_prefix('{')?.strip_suffix('}')?;
    let (mut depth, mut current, mut out) = (0i32, String::new(), Vec::new());
    for c in inner.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    Some(out.into_iter().map(|s| s.trim().to_string()).collect())
}

/// Tokens: set literals and operator words.
fn tokens(input: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '{' {
            let start = i;
            let mut depth = 0;
            while i < chars.len() {
                match chars[i] {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if i >= chars.len() {
                return None;
            }
            out.push(chars[start..=i].iter().collect());
            i += 1;
        } else if matches!(c, '∪' | '∩' | '\\' | '⊆' | '∈' | '(' | ')' | '×' | 'Δ') {
            out.push(c.to_string());
            i += 1;
        } else {
            let start = i;
            while i < chars.len()
                && !chars[i].is_whitespace()
                && !matches!(chars[i], '{' | '(' | ')')
            {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        }
    }
    Some(out)
}

struct Evaluator {
    tokens: Vec<String>,
    pos: usize,
}

enum Value {
    Set(Set),
    Bool(bool),
    Number(usize),
    Element(String),
}

impl Evaluator {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(String::as_str)
    }

    fn operand(&mut self) -> Option<Value> {
        let token = self.tokens.get(self.pos)?.clone();
        self.pos += 1;
        if token == "(" {
            let v = self.expression()?;
            if self.peek()? != ")" {
                return None;
            }
            self.pos += 1;
            return Some(v);
        }
        if matches!(token.as_str(), "size" | "card" | "cardinality" | "len") {
            return match self.operand()? {
                Value::Set(s) => Some(Value::Number(s.len())),
                _ => None,
            };
        }
        if token.starts_with('{') {
            return Some(Value::Set(normalise(items(&token)?)));
        }
        Some(Value::Element(canonical(&token)))
    }

    fn expression(&mut self) -> Option<Value> {
        let mut left = self.operand()?;
        while let Some(op) = self.peek().map(str::to_ascii_lowercase) {
            if op == ")" {
                break;
            }
            self.pos += 1;
            let right = self.operand()?;
            left = match (left, op.as_str(), right) {
                (Value::Set(a), "union" | "∪" | "or", Value::Set(b)) => {
                    Value::Set(normalise(a.into_iter().chain(b).collect()))
                }
                (Value::Set(a), "intersect" | "intersection" | "∩" | "and", Value::Set(b)) => {
                    Value::Set(a.into_iter().filter(|x| b.contains(x)).collect())
                }
                (Value::Set(a), "minus" | "\\" | "without", Value::Set(b)) => {
                    Value::Set(a.into_iter().filter(|x| !b.contains(x)).collect())
                }
                (Value::Set(a), "symdiff" | "xor" | "Δ" | "δ", Value::Set(b)) => {
                    let mut out: Vec<String> =
                        a.iter().filter(|x| !b.contains(x)).cloned().collect();
                    out.extend(b.iter().filter(|x| !a.contains(x)).cloned());
                    Value::Set(normalise(out))
                }
                (Value::Set(a), "subset" | "⊆" | "issubset", Value::Set(b)) => {
                    Value::Bool(a.iter().all(|x| b.contains(x)))
                }
                (Value::Set(a), "superset" | "⊇", Value::Set(b)) => {
                    Value::Bool(b.iter().all(|x| a.contains(x)))
                }
                (Value::Element(x), "in" | "∈", Value::Set(b)) => Value::Bool(b.contains(&x)),
                (Value::Element(x), "notin" | "∉", Value::Set(b)) => Value::Bool(!b.contains(&x)),
                (Value::Set(a), "cross" | "×" | "product", Value::Set(b)) => {
                    let mut pairs = Vec::new();
                    for x in &a {
                        for y in &b {
                            pairs.push(format!("({x}, {y})"));
                        }
                    }
                    // Ordered pairs keep the order of construction.
                    Value::Set(pairs)
                }
                _ => return None,
            };
        }
        Some(left)
    }
}

/// Evaluate a set expression, or `None` if the input is not one.
pub fn evaluate(input: &str) -> Option<String> {
    // Set-builder notation (`{ x in Reals | x > 0 }`) is handled elsewhere.
    if input.contains('|') || input.contains(" in Reals") || input.contains(" in Integers") {
        return None;
    }
    if !input.contains('{') && !input.contains(" in ") {
        return None;
    }
    let tokens = tokens(input)?;
    if tokens.len() < 2 && !tokens.first().is_some_and(|t| t.starts_with('{')) {
        return None;
    }
    let mut evaluator = Evaluator { tokens, pos: 0 };
    let value = evaluator.expression()?;
    if evaluator.pos != evaluator.tokens.len() {
        return None;
    }
    Some(match value {
        Value::Set(s) => show(&s),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Element(e) => e,
    })
}
