//! Matrix arithmetic typed as plain input: `A + B`, `2*A`, `A * v`, `A^3`, `A^-1`, `transpose(A) * A`, `det(inverse(A))`,
//! `identity(3)`, `eigenvectors(A)`, `solve_linear(A, b)`.
//!
//! Matrices are `[[a, b], [c, d]]`; a flat list `[a, b]` is a column vector when it is the right operand of a matrix.
//! Entries may be symbolic.

use crate::matrix::SymbolicMatrix;
use crate::simplify::SymbolicSimplifier;
use algebra_core::format::{Formatter, UnicodeFormatter};
use algebra_core::parser::ExprParser;
use algebra_core::{ExprGraph, ExprId};
use num_traits::ToPrimitive;

#[derive(Clone)]
enum Value {
    Scalar(ExprId),
    Matrix(SymbolicMatrix, bool),
    Text(String),
}

const FUNCTIONS: [&str; 19] = [
    "transpose",
    "inverse",
    "inv",
    "det",
    "determinant",
    "trace",
    "identity",
    "eye",
    "zeros",
    "ones",
    "diag",
    "charpoly",
    "eigenvalues",
    "eigenvectors",
    "expm",
    "cholesky",
    "qr",
    "solve_linear",
    "norm",
];

fn depth_zero_positions(s: &str, targets: &[char]) -> Vec<usize> {
    let mut depth = 0i32;
    let mut out = Vec::new();
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            c if depth == 0 && targets.contains(&c) => out.push(i),
            _ => {}
        }
    }
    out
}

/// Split at depth-0 binary `+` and `-` (a sign after an operator or at the start is unary).
fn split_sum(s: &str) -> Vec<(char, String)> {
    let mut parts = Vec::new();
    let mut sign = '+';
    let mut current = String::new();
    let mut depth = 0i32;
    let mut previous: Option<char> = None;
    for c in s.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        let binary = depth == 0
            && (c == '+' || c == '-')
            && previous.is_some_and(|p| !matches!(p, '*' | '/' | '^' | '+' | '-' | '(' | ','))
            && !current.trim().is_empty();
        if binary {
            parts.push((sign, std::mem::take(&mut current)));
            sign = c;
        } else {
            current.push(c);
        }
        if !c.is_whitespace() {
            previous = Some(c);
        }
    }
    parts.push((sign, current));
    parts
}

struct Evaluator<'g> {
    graph: &'g ExprGraph,
    parser: ExprParser<'g>,
}

impl<'g> Evaluator<'g> {
    fn simplify(&self, id: ExprId) -> ExprId {
        self.graph.simplify(id)
    }

    fn matrix_literal(&self, s: &str) -> Result<Value, String> {
        let flat = !s.trim_start().starts_with("[[") && !s.contains(';');
        let m = SymbolicMatrix::from_expr_str(self.graph, s).map_err(|e| e.to_string())?;
        // A flat list is a vector: stored as a column.
        if flat && m.rows == 1 && m.cols > 1 {
            return Ok(Value::Matrix(
                SymbolicMatrix::new(m.cols, 1, m.elements).map_err(|e| e.to_string())?,
                true,
            ));
        }
        Ok(Value::Matrix(m, false))
    }

    fn sum(&self, s: &str) -> Result<Value, String> {
        let parts = split_sum(s);
        let mut total: Option<Value> = None;
        for (sign, text) in parts {
            let mut v = self.product(text.trim())?;
            if sign == '-' {
                v = self.scale(&v, self.graph.integer(-1))?;
            }
            total = Some(match total {
                None => v,
                Some(acc) => self.add(&acc, &v)?,
            });
        }
        total.ok_or_else(|| "empty expression".to_string())
    }

    fn add(&self, a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Scalar(x), Value::Scalar(y)) => Ok(Value::Scalar(self.graph.add([*x, *y]))),
            (Value::Matrix(x, vx), Value::Matrix(y, vy)) => Ok(Value::Matrix(
                x.add(self.graph, y).map_err(|e| e.to_string())?,
                *vx && *vy,
            )),
            _ => Err("cannot add a matrix and a scalar".into()),
        }
    }

    fn scale(&self, v: &Value, k: ExprId) -> Result<Value, String> {
        match v {
            Value::Scalar(x) => Ok(Value::Scalar(self.graph.mul([k, *x]))),
            Value::Matrix(m, flag) => {
                let elements = m.elements.iter().map(|&e| self.graph.mul([k, e])).collect();
                Ok(Value::Matrix(
                    SymbolicMatrix::new(m.rows, m.cols, elements).map_err(|e| e.to_string())?,
                    *flag,
                ))
            }
            Value::Text(_) => Err("cannot scale a text result".into()),
        }
    }

    fn product(&self, s: &str) -> Result<Value, String> {
        // Division by a scalar and multiplication, left to right.
        let mut pieces: Vec<(char, String)> = Vec::new();
        let (mut depth, mut current, mut op) = (0i32, String::new(), '*');
        for c in s.chars() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 && (c == '*' || c == '/') && !current.trim().is_empty() {
                pieces.push((op, std::mem::take(&mut current)));
                op = c;
            } else if c == '·' && depth == 0 {
                pieces.push((op, std::mem::take(&mut current)));
                op = '*';
            } else {
                current.push(c);
            }
        }
        pieces.push((op, current));
        let mut result: Option<Value> = None;
        for (op, text) in pieces {
            let v = self.factor(text.trim())?;
            result = Some(match (result, op) {
                (None, _) => v,
                (Some(acc), '*') => self.multiply(&acc, &v)?,
                (Some(acc), _) => match v {
                    Value::Scalar(d) => {
                        self.scale(&acc, self.graph.div(self.graph.integer(1), d))?
                    }
                    _ => return Err("division by a matrix is not defined; use inverse(A)".into()),
                },
            });
        }
        result.ok_or_else(|| "empty product".to_string())
    }

    fn multiply(&self, a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Scalar(x), Value::Scalar(y)) => Ok(Value::Scalar(self.graph.mul([*x, *y]))),
            (Value::Scalar(k), m @ Value::Matrix(..))
            | (m @ Value::Matrix(..), Value::Scalar(k)) => self.scale(m, *k),
            (Value::Matrix(x, vx), Value::Matrix(y, vy)) => {
                let mut right = y.clone();
                // A vector on the left of a matrix is a row vector.
                let mut left = x.clone();
                if *vx && !*vy && x.cols == 1 && x.rows == y.rows {
                    left = x.transpose(self.graph).map_err(|e| e.to_string())?;
                }
                if *vy && right.cols == 1 && left.cols != 1 && right.rows != left.cols {
                    right = y.clone();
                }
                let product = left
                    .multiply(self.graph, &right)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Matrix(product, *vx && *vy))
            }
            _ => Err("cannot multiply these values".into()),
        }
    }

    fn factor(&self, s: &str) -> Result<Value, String> {
        let s = s.trim();
        if let Some(rest) = s.strip_prefix('-') {
            return self.scale(&self.factor(rest)?, self.graph.integer(-1));
        }
        if let Some(rest) = s.strip_prefix('+') {
            return self.factor(rest);
        }
        let carets = depth_zero_positions(s, &['^']);
        if let Some(&position) = carets.first() {
            let base = self.atom(s[..position].trim())?;
            let exponent = s[position + 1..].trim();
            return match base {
                Value::Matrix(m, flag) => {
                    let n: i64 = exponent
                        .trim_matches(|c| c == '(' || c == ')')
                        .trim()
                        .parse()
                        .map_err(|_| "a matrix power needs an integer exponent".to_string())?;
                    Ok(Value::Matrix(self.matrix_power(&m, n)?, flag))
                }
                scalar => {
                    let Value::Scalar(b) = scalar else {
                        return Err("cannot raise this value to a power".into());
                    };
                    let e = self.parser.parse(exponent).map_err(|e| e.to_string())?;
                    Ok(Value::Scalar(self.graph.pow(b, e)))
                }
            };
        }
        self.atom(s)
    }

    fn matrix_power(&self, m: &SymbolicMatrix, n: i64) -> Result<SymbolicMatrix, String> {
        if m.rows != m.cols {
            return Err("only a square matrix has a power".into());
        }
        if n.abs() > 10_000 {
            return Err("the exponent is too large".into());
        }
        let simplify_all = |x: &SymbolicMatrix| -> Result<SymbolicMatrix, String> {
            let elements = x.elements.iter().map(|&e| self.simplify(e)).collect();
            SymbolicMatrix::new(x.rows, x.cols, elements).map_err(|e| e.to_string())
        };
        let mut base = if n < 0 {
            simplify_all(&m.inverse(self.graph).map_err(|e| e.to_string())?)?
        } else {
            m.clone()
        };
        let mut result = SymbolicMatrix::identity(self.graph, m.rows);
        let mut e = n.unsigned_abs();
        while e > 0 {
            if e & 1 == 1 {
                result = simplify_all(
                    &result
                        .multiply(self.graph, &base)
                        .map_err(|e| e.to_string())?,
                )?;
            }
            e >>= 1;
            if e > 0 {
                base = simplify_all(
                    &base
                        .multiply(self.graph, &base)
                        .map_err(|e| e.to_string())?,
                )?;
            }
        }
        Ok(result)
    }

    fn atom(&self, s: &str) -> Result<Value, String> {
        let s = s.trim();
        if s.starts_with('[') && s.ends_with(']') {
            return self.matrix_literal(s);
        }
        if s.starts_with('(') && s.ends_with(')') {
            // Is the outer pair one group?
            let mut depth = 0i32;
            let mut whole = true;
            for (i, c) in s.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 && i != s.len() - 1 {
                            whole = false;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if whole {
                return self.sum(&s[1..s.len() - 1]);
            }
        }
        if let Some(open) = s.find('(')
            && s.ends_with(')')
            && s[..open]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && FUNCTIONS.contains(&&s[..open])
        {
            let inner = &s[open + 1..s.len() - 1];
            let mut args = Vec::new();
            let mut last = 0;
            for p in depth_zero_positions(inner, &[',']) {
                args.push(inner[last..p].trim().to_string());
                last = p + 1;
            }
            args.push(inner[last..].trim().to_string());
            return self.function(&s[..open], &args);
        }
        let id = self.parser.parse(s).map_err(|e| e.to_string())?;
        Ok(Value::Scalar(id))
    }

    fn matrix_arg(&self, text: &str) -> Result<SymbolicMatrix, String> {
        match self.sum(text)? {
            Value::Matrix(m, _) => Ok(m),
            _ => Err(format!("{text} is not a matrix")),
        }
    }

    fn integer_arg(&self, text: &str) -> Result<usize, String> {
        let id = self.parser.parse(text).map_err(|e| e.to_string())?;
        match crate::numeric::try_eval_exact(self.graph, id, &std::collections::HashMap::new()) {
            Ok(Some(q))
                if q.is_integer() && q.to_integer().to_usize().is_some_and(|n| n <= 200) =>
            {
                Ok(q.to_integer().to_usize().unwrap_or(0))
            }
            _ => Err(format!("{text} is not a small positive integer")),
        }
    }

    fn function(&self, name: &str, args: &[String]) -> Result<Value, String> {
        let first = args.first().map(String::as_str).unwrap_or("");
        let err = |e: algebra_core::AlgebraError| e.to_string();
        match name {
            "identity" | "eye" => Ok(Value::Matrix(
                SymbolicMatrix::identity(self.graph, self.integer_arg(first)?),
                false,
            )),
            "zeros" | "ones" => {
                let r = self.integer_arg(first)?;
                let c = match args.get(1) {
                    Some(c) => self.integer_arg(c)?,
                    None => r,
                };
                let fill = self.graph.integer(if name == "ones" { 1 } else { 0 });
                Ok(Value::Matrix(
                    SymbolicMatrix::new(r, c, vec![fill; r * c]).map_err(err)?,
                    false,
                ))
            }
            "diag" => {
                let Value::Matrix(v, _) = self.matrix_literal(first)? else {
                    return Err("diag needs a list".into());
                };
                let n = v.elements.len();
                let mut elements = vec![self.graph.integer(0); n * n];
                for (i, e) in v.elements.iter().enumerate() {
                    elements[i * n + i] = *e;
                }
                Ok(Value::Matrix(
                    SymbolicMatrix::new(n, n, elements).map_err(err)?,
                    false,
                ))
            }
            "transpose" => Ok(Value::Matrix(
                self.matrix_arg(first)?.transpose(self.graph).map_err(err)?,
                false,
            )),
            "inverse" | "inv" => Ok(Value::Matrix(
                self.matrix_arg(first)?.inverse(self.graph).map_err(err)?,
                false,
            )),
            "det" | "determinant" => Ok(Value::Scalar(
                self.matrix_arg(first)?
                    .determinant(self.graph)
                    .map_err(err)?,
            )),
            "trace" => Ok(Value::Scalar(
                self.matrix_arg(first)?.trace(self.graph).map_err(err)?,
            )),
            "norm" => {
                let m = self.matrix_arg(first)?;
                let squares: Vec<ExprId> = m
                    .elements
                    .iter()
                    .map(|&e| self.graph.pow(e, self.graph.integer(2)))
                    .collect();
                Ok(Value::Scalar(
                    self.graph.function("sqrt", [self.graph.add(squares)]),
                ))
            }
            "charpoly" => {
                let var = args.get(1).map(String::as_str).unwrap_or("lambda");
                Ok(Value::Scalar(
                    self.matrix_arg(first)?
                        .charpoly(self.graph, var)
                        .map_err(err)?,
                ))
            }
            "eigenvalues" => {
                let values = self
                    .matrix_arg(first)?
                    .eigenvalues(self.graph)
                    .map_err(err)?;
                let texts: Vec<String> = values
                    .iter()
                    .map(|&v| {
                        crate::complex_exact::simplify_text(self.graph, v).unwrap_or_else(|| {
                            UnicodeFormatter
                                .format(self.graph, self.simplify(v))
                                .unwrap_or_default()
                        })
                    })
                    .collect();
                Ok(Value::Text(format!("[{}]", texts.join(", "))))
            }
            "eigenvectors" => {
                let pairs = self
                    .matrix_arg(first)?
                    .eigenvectors(self.graph)
                    .map_err(err)?;
                let fmt = |id: ExprId| {
                    UnicodeFormatter
                        .format(self.graph, self.simplify(id))
                        .unwrap_or_default()
                };
                let lines: Vec<String> = pairs
                    .iter()
                    .map(|(value, vector)| {
                        format!(
                            "λ = {}: [{}]",
                            fmt(*value),
                            vector
                                .iter()
                                .map(|&e| fmt(e))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .collect();
                Ok(Value::Text(lines.join("; ")))
            }
            "expm" => {
                let m = self.matrix_arg(first)?;
                let t = self.graph.symbols.get_or_intern("t");
                let exp = m.matrix_exp(self.graph, "t").map_err(err)?;
                let one = self.graph.integer(1);
                let elements = exp
                    .elements
                    .iter()
                    .map(|&e| self.graph.substitute(e, t, one))
                    .collect();
                Ok(Value::Matrix(
                    SymbolicMatrix::new(exp.rows, exp.cols, elements).map_err(err)?,
                    false,
                ))
            }
            "cholesky" => Ok(Value::Matrix(
                self.matrix_arg(first)?
                    .cholesky_decomposition(self.graph)
                    .map_err(err)?,
                false,
            )),
            "qr" => {
                let (q, r) = self
                    .matrix_arg(first)?
                    .qr_decomposition(self.graph)
                    .map_err(err)?;
                let show = |m: &SymbolicMatrix| -> String {
                    let elements = m.elements.iter().map(|&e| self.simplify(e)).collect();
                    SymbolicMatrix::new(m.rows, m.cols, elements)
                        .ok()
                        .and_then(|m| {
                            UnicodeFormatter
                                .format(self.graph, m.to_expr_id(self.graph))
                                .ok()
                        })
                        .unwrap_or_default()
                };
                Ok(Value::Text(format!("Q = {}, R = {}", show(&q), show(&r))))
            }
            "solve_linear" => {
                let a = self.matrix_arg(first)?;
                let b = self.matrix_arg(
                    args.get(1)
                        .ok_or("solve_linear(A, b) needs two arguments")?,
                )?;
                let b = if b.rows == 1 && b.cols == a.rows && a.rows != 1 {
                    b.transpose(self.graph).map_err(err)?
                } else {
                    b
                };
                let x = a
                    .inverse(self.graph)
                    .map_err(err)?
                    .multiply(self.graph, &b)
                    .map_err(err)?;
                Ok(Value::Matrix(x, true))
            }
            other => Err(format!("unknown matrix function {other}")),
        }
    }
}

fn looks_like_matrix_arithmetic(input: &str) -> bool {
    let constructor = ["identity(", "eye(", "zeros(", "ones("]
        .iter()
        .any(|f| input.trim().starts_with(f));
    if !input.contains('[') && !constructor {
        return false;
    }
    let trimmed = input.trim();
    // A lone literal is displayed by the executor.
    if crate::executor::is_single_bracket_group(trimmed) {
        return false;
    }
    let has_operator = !depth_zero_positions(trimmed, &['+', '-', '*', '/', '^', '·']).is_empty();
    let function_call = FUNCTIONS
        .iter()
        .any(|f| trimmed.starts_with(&format!("{f}(")) || trimmed.contains(&format!("({f}(")));
    has_operator || function_call
}

/// Evaluate matrix arithmetic; `None` if `input` is not matrix arithmetic (or uses something outside it).
pub fn evaluate(graph: &ExprGraph, input: &str) -> Option<Result<String, String>> {
    if !looks_like_matrix_arithmetic(input) {
        return None;
    }
    let evaluator = Evaluator {
        graph,
        parser: ExprParser::new(graph),
    };
    let value = match evaluator.sum(input) {
        Ok(v) => v,
        Err(e) => {
            // An input that merely contains brackets (a list in a command, say) is not ours; real arithmetic
            // mistakes (a size mismatch) are reported.
            return if e.contains("dimension") || e.contains("Dimension") || e.contains("cannot") {
                Some(Err(e))
            } else {
                None
            };
        }
    };
    Some(Ok(match value {
        Value::Text(t) => t,
        Value::Scalar(id) => UnicodeFormatter
            .format(graph, evaluator.simplify(id))
            .unwrap_or_default(),
        Value::Matrix(m, _) => {
            let elements: Vec<ExprId> = m.elements.iter().map(|&e| evaluator.simplify(e)).collect();
            let m = SymbolicMatrix::new(m.rows, m.cols, elements).ok()?;
            UnicodeFormatter
                .format(graph, m.to_expr_id(graph))
                .unwrap_or_default()
        }
    }))
}
