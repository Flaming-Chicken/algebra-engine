//! Mathematical expression parser using `nom` with support for standard math expressions and raw LaTeX syntax.

use crate::{BranchOpKind, ExprGraph, ExprId, RelOp, parse_value_uncertainty_and_unit};
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    combinator::opt,
    multi::separated_list0,
    sequence::{delimited, tuple},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Parse failed: {0}")]
    InvalidSyntax(String),
}

pub type ParseResult<T> = Result<T, ParseError>;

/// Helper to extract content of a `{...}` block using balanced brace counting.
fn extract_braced_block(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }
    let mut depth = 0;
    for (i, c) in trimmed.char_indices() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                let content = &trimmed[1..i];
                let remainder = &trimmed[i + 1..];
                return Some((content, remainder));
            }
        }
    }
    None
}

/// Normalize raw LaTeX and Unicode math inputs into standard parseable expression strings.
pub fn normalize_latex_input(input: &str) -> String {
    let mut s = input.trim().to_string();

    if s.starts_with("$$") && s.ends_with("$$") {
        s = s[2..s.len() - 2].trim().to_string();
    } else if s.starts_with('$') && s.ends_with('$') && s.len() >= 2 {
        s = s[1..s.len() - 1].trim().to_string();
    }

    // Convert Unicode superscripts: x² -> x^2, x³ -> x^3, xⁿ -> x^n
    s = s
        .replace("**", "^")
        .replace('²', "^2")
        .replace('³', "^3")
        .replace('⁴', "^4")
        .replace('⁵', "^5")
        .replace('⁶', "^6")
        .replace('⁷', "^7")
        .replace('⁸', "^8")
        .replace('⁹', "^9")
        .replace('ⁿ', "^n")
        .replace(['·', '×'], "*")
        .replace('≠', "!=")
        .replace('≤', "<=")
        .replace('≥', ">=")
        .replace("\\pm", "±")
        .replace('∈', " in ")
        .replace('∪', " union ")
        .replace('∩', " intersect ")
        .replace('ℝ', "Reals")
        .replace('ℂ', "Complex")
        .replace('ℤ', "Integers")
        .replace('ℍ', "Quaternion");

    // Convert Unicode square root: √(arg) -> sqrt(arg), every occurrence.
    s = s.replace("√(", "sqrt(");
    // A bare √x (no parentheses) takes the identifier or number that follows.
    while let Some(idx) = s.find('√') {
        let after_sqrt = &s[idx + '√'.len_utf8()..];
        let symbol_end = after_sqrt
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(after_sqrt.len());
        let target_var = &after_sqrt[..symbol_end];
        if target_var.is_empty() {
            break;
        }
        let replacement = format!("sqrt({})", target_var);
        let full_term = &s[idx..idx + '√'.len_utf8() + symbol_end];
        s = s.replace(full_term, &replacement);
    }

    // Replace LaTeX fraction: \frac{a}{b} -> ((a) / (b)) using balanced brace parsing
    while let Some(idx) = s.find("\\frac") {
        let after_frac = &s[idx + 5..];
        if let Some((num_str, rest_after_num)) = extract_braced_block(after_frac)
            && let Some((den_str, rest_after_den)) = extract_braced_block(rest_after_num)
        {
            let consumed_len = 5 + (after_frac.len() - rest_after_den.len());
            let replacement = format!("(({}) / ({}))", num_str, den_str);
            let full_frac_str = &s[idx..idx + consumed_len];
            s = s.replace(full_frac_str, &replacement);
            continue;
        }
        break;
    }

    // Replace LaTeX square root: \sqrt{x} -> sqrt(x)
    while let Some(idx) = s.find("\\sqrt") {
        let after_sqrt = &s[idx + 5..];
        if let Some((arg_str, rest_after_arg)) = extract_braced_block(after_sqrt) {
            let consumed_len = 5 + (after_sqrt.len() - rest_after_arg.len());
            let replacement = format!("sqrt({})", arg_str);
            let full_sqrt_str = &s[idx..idx + consumed_len];
            s = s.replace(full_sqrt_str, &replacement);
            continue;
        }
        break;
    }

    s = s
        .replace("\\mathrm{d}", "d")
        .replace("\\cdot", "*")
        .replace("\\times", "*")
        .replace("\\leq", "<=")
        .replace("\\geq", ">=")
        .replace("\\neq", "!=")
        .replace("\\in", " in ")
        .replace("\\operatorname{", "")
        .replace("\\left(", "(")
        .replace("\\right)", ")")
        .replace("\\left\\{", "{")
        .replace("\\right\\}", "}")
        .replace("\\mathbb{R}", "Reals")
        .replace("\\mathbb{C}", "Complex")
        .replace("\\mathbb{Z}", "Integers")
        .replace("\\mathbb{H}", "Quaternion");

    // Clean up exponent braces x^{2} -> x^2
    while let Some(idx) = s.find("^{") {
        let after_exp = &s[idx + 2..];
        if let Some(close_exp) = after_exp.find('}') {
            let exp_val = &after_exp[..close_exp];
            let replacement = format!("^{}", exp_val);
            let full_exp = &s[idx..idx + 2 + close_exp + 1];
            s = s.replace(full_exp, &replacement);
            continue;
        }
        break;
    }

    // Clean up LaTeX function prefixes e.g. \sin -> sin, \cos -> cos
    for fn_name in [
        "sin", "cos", "tan", "exp", "ln", "log", "asin", "acos", "atan",
    ] {
        let latex_fn = format!("\\{}", fn_name);
        s = s.replace(&latex_fn, fn_name);
    }

    s
}

/// The deepest bracket nesting the recursive-descent parser accepts. Each level costs several stack frames, so an
/// unbounded depth is a stack overflow (which aborts the process); 64 is far beyond what is written by hand.
pub const MAX_NESTING_DEPTH: usize = 64;

/// An estimate of the recursion depth the text will need: the maximum bracket nesting (`(`, `[`, `{`) plus the
/// longest run of prefix signs plus the longest chain of right-associative `^` operators.
fn bracket_depth(text: &str) -> usize {
    let (mut depth, mut max) = (0usize, 0usize);
    let (mut signs, mut max_signs) = (0usize, 0usize);
    let (mut carets, mut max_carets) = (0usize, 0usize);
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                max = max.max(depth);
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if c == '-' || c == '+' {
            signs += 1;
            max_signs = max_signs.max(signs);
        } else if !c.is_whitespace() {
            signs = 0;
        }
        match c {
            '^' => {
                carets += 1;
                max_carets = max_carets.max(carets);
            }
            '+' | '-' | '*' | '/' | '=' | '<' | '>' | ',' | '(' | ')' | '[' | ']' | '{' | '}' => {
                carets = 0
            }
            _ => {}
        }
    }
    max + max_signs + max_carets
}

/// Parser converting mathematical expression and LaTeX strings into [`ExprGraph`] nodes.
pub struct ExprParser<'a> {
    graph: &'a ExprGraph,
}

impl<'a> ExprParser<'a> {
    pub fn new(graph: &'a ExprGraph) -> Self {
        Self { graph }
    }

    /// Parse input text or LaTeX string into an [`ExprId`] in the expression graph.
    pub fn parse(&self, input: &str) -> ParseResult<ExprId> {
        let normalized = normalize_latex_input(input);
        let depth = bracket_depth(&normalized);
        if depth > MAX_NESTING_DEPTH {
            return Err(ParseError::InvalidSyntax(format!(
                "the input is nested {depth} brackets deep; the limit is {MAX_NESTING_DEPTH}"
            )));
        }
        let (rem, expr) = self.parse_relation(normalized.trim()).map_err(|e| {
            ParseError::InvalidSyntax(format!("Failed to parse input '{input}': {e}"))
        })?;

        if !rem.trim().is_empty() {
            return Err(ParseError::InvalidSyntax(format!(
                "Unexpected trailing tokens: '{rem}'"
            )));
        }

        Ok(expr)
    }

    fn parse_relation<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (input, lhs) = self.parse_expr(input)?;
        let (input, rel_opt) = opt(tuple((
            delimited(
                multispace0,
                alt((
                    tag("=="),
                    tag("="),
                    tag("!="),
                    tag("<="),
                    tag("<"),
                    tag(">="),
                    tag(">"),
                )),
                multispace0,
            ),
            |i| self.parse_expr(i),
        )))(input)?;

        if let Some((op_str, rhs)) = rel_opt {
            let op = match op_str {
                "=" | "==" => RelOp::Equal,
                "!=" => RelOp::NotEqual,
                "<" => RelOp::LessThan,
                "<=" => RelOp::LessEqual,
                ">" => RelOp::GreaterThan,
                ">=" => RelOp::GreaterEqual,
                _ => RelOp::Equal,
            };
            Ok((input, self.graph.relational(op, lhs, rhs)))
        } else {
            Ok((input, lhs))
        }
    }

    fn parse_expr<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (input, lhs) = self.parse_term(input)?;
        let (input, ops) = nom::multi::many0(tuple((
            delimited(multispace0, alt((char('+'), char('-'))), multispace0),
            |i| self.parse_term(i),
        )))(input)?;

        let mut current = lhs;
        for (op, rhs) in ops {
            if op == '+' {
                current = self.graph.add([current, rhs]);
            } else {
                current = self.graph.sub(current, rhs);
            }
        }

        Ok((input, current))
    }

    fn parse_term<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (mut rem, mut current) = self.parse_uncertainty(input)?;
        loop {
            let (next_rem, op_opt) = opt(tuple((
                delimited(
                    multispace0,
                    alt((
                        tag(".*"),
                        tag("./"),
                        tag("*"),
                        tag("/"),
                        tag("%"),
                        tag("\\"),
                    )),
                    multispace0,
                ),
                |i| self.parse_uncertainty(i),
            )))(rem)?;

            if let Some((op, rhs)) = op_opt {
                current = match op {
                    "*" => self.graph.mul([current, rhs]),
                    "/" => self.graph.div(current, rhs),
                    "%" => self.graph.function("mod", [current, rhs]),
                    ".*" => self.graph.function("elem_mul", [current, rhs]),
                    "./" => self.graph.function("elem_div", [current, rhs]),
                    "\\" => self.graph.function("mldivide", [current, rhs]),
                    _ => current,
                };
                rem = next_rem;
                continue;
            }

            // Contiguous implicit multiplication (no whitespace so `[1 2; 3 4]` keeps space-separated columns):
            // handles `2x`, `2(x+1)`, `(x+1)(x-1)`, `(x+1)2`, `2sin(x)`, `sin(x)cos(x)`.
            if let Some(next_ch) = rem.chars().next()
                && (next_ch == '('
                    || next_ch.is_alphabetic()
                    || next_ch == '_'
                    || next_ch.is_ascii_digit())
                && let Ok((after_rhs, rhs)) = self.parse_uncertainty(rem)
            {
                current = self.graph.mul([current, rhs]);
                rem = after_rhs;
                continue;
            }

            break;
        }

        Ok((rem, current))
    }

    /// Parses uncertainty expressions `val ± err`, `val +/- err`, or `val +- err`
    /// tighter than `*`, `/`, `+`, `-` so `-5 ± 0.1` and `10 ± 0.2 + 5 ± 0.1` work naturally.
    fn parse_uncertainty<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (input, lhs) = self.parse_factor(input)?;
        let (input, err_opt) = opt(tuple((
            delimited(
                multispace0,
                alt((tag("±"), tag("+/-"), tag("+-"))),
                multispace0,
            ),
            |i| self.parse_factor(i),
        )))(input)?;
        if let Some((_, err)) = err_opt {
            Ok((input, self.graph.function("uncertain", [lhs, err])))
        } else {
            Ok((input, lhs))
        }
    }

    fn parse_factor<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (input, _) = multispace0(input)?;
        if let Some(rest) = input.strip_prefix('-') {
            let (next_input, inner) = self.parse_factor(rest)?;
            return Ok((next_input, self.graph.neg(inner)));
        }
        if let Some(rest) = input.strip_prefix('+') {
            return self.parse_factor(rest);
        }
        let (mut input, mut base) = self.parse_primary(input)?;
        loop {
            let after_ws = input.trim_start();
            if after_ws.starts_with('!') && !after_ws.starts_with("!=") {
                base = self.graph.function("factorial", [base]);
                input = &after_ws[1..];
            } else {
                break;
            }
        }
        let (input, exp_opt) = opt(tuple((
            delimited(multispace0, alt((tag(".^"), tag("^"))), multispace0),
            |i| self.parse_factor(i),
        )))(input)?;

        if let Some((op, exp)) = exp_opt {
            if op == ".^" {
                Ok((input, self.graph.function("elem_pow", [base, exp])))
            } else {
                Ok((input, self.graph.pow(base, exp)))
            }
        } else {
            Ok((input, base))
        }
    }

    fn parse_primary<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (input, _) = multispace0(input)?;
        alt((
            |i| self.parse_bracket_matrix(i),
            |i| self.parse_number(i),
            |i| self.parse_symbol_or_fn(i),
            delimited(
                tuple((char('('), multispace0)),
                |i| self.parse_expr(i),
                tuple((multispace0, char(')'))),
            ),
        ))(input)
    }

    fn parse_bracket_matrix<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (mut rest, _) = char('[')(input)?;
        let mut rows = Vec::new();
        loop {
            let (r, _) = multispace0(rest)?;
            rest = r;
            if let Some(after) = rest.strip_prefix(']') {
                rest = after;
                break;
            }
            let mut cols = Vec::new();
            loop {
                let (r, _) = multispace0(rest)?;
                rest = r;
                if rest.starts_with(';') || rest.starts_with(']') {
                    break;
                }
                let (r, elem) = self.parse_expr(rest)?;
                cols.push(elem);
                let (r, _) = multispace0(r)?;
                rest = r.strip_prefix(',').unwrap_or(r);
            }
            if cols.is_empty() {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::SeparatedList,
                )));
            }
            rows.push(cols);
            if let Some(after_semi) = rest.strip_prefix(';') {
                rest = after_semi;
            }
        }
        if rows.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::SeparatedList,
            )));
        }

        let row_exprs: Vec<ExprId> = rows
            .into_iter()
            .map(|cols| self.graph.function("row", cols))
            .collect();
        Ok((rest, self.graph.function("matrix", row_exprs)))
    }

    fn parse_number<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let mut chars = input.char_indices().peekable();
        let mut end_idx = 0;
        let mut has_digit = false;
        let mut has_dot = false;
        let mut has_exp = false;

        while let Some(&(i, c)) = chars.peek() {
            if c.is_ascii_digit() {
                has_digit = true;
                end_idx = i + c.len_utf8();
                chars.next();
            } else if c == '.' && !has_dot && !has_exp {
                has_dot = true;
                end_idx = i + c.len_utf8();
                chars.next();
            } else if (c == 'e' || c == 'E') && has_digit && !has_exp {
                let mut look = chars.clone();
                look.next();
                if let Some(&(_, sc)) = look.peek()
                    && (sc == '+' || sc == '-')
                {
                    look.next();
                }
                if let Some(&(_, dc)) = look.peek()
                    && dc.is_ascii_digit()
                {
                    has_exp = true;
                    end_idx = i + c.len_utf8();
                    chars.next();
                    if let Some(&(si, sc)) = chars.peek()
                        && (sc == '+' || sc == '-')
                    {
                        end_idx = si + sc.len_utf8();
                        chars.next();
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if !has_digit {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Digit,
            )));
        }

        let num_str = &input[..end_idx];
        let rem = &input[end_idx..];

        if let Some(num) = crate::Number::from_decimal_str(num_str) {
            return Ok((rem, self.graph.number(num)));
        }

        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )))
    }

    fn parse_symbol_or_fn<'i>(&self, input: &'i str) -> IResult<&'i str, ExprId> {
        let (input, raw_name) =
            take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.')(input)?;

        if raw_name.contains(".curvilinear") && input.starts_with('(') {
            let parts: Vec<&str> = raw_name.split('.').collect();
            let base_var = parts.first().copied().unwrap_or("x");

            let (input, sys_part) = delimited(
                tuple((multispace0, char('('), multispace0)),
                take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '"' || c == '\''),
                tuple((multispace0, char(')'))),
            )(input)?;

            let sys_name = sys_part.trim().trim_matches('"').trim_matches('\'');

            let (input, member_opt) = opt(tuple((
                char('.'),
                take_while1(|c: char| c.is_alphanumeric() || c == '_'),
            )))(input)?;

            let coord_comp = member_opt.map(|(_, m)| m).unwrap_or("r");

            let vx = self.graph.symbol(&format!("{}_x", base_var));
            let vy = self.graph.symbol(&format!("{}_y", base_var));
            let vz = self.graph.symbol(&format!("{}_z", base_var));

            let res_node = match (sys_name, coord_comp) {
                ("spherical" | "polar" | "cylindrical", "r") => {
                    let two = self.graph.integer(2);
                    let x_sq = self.graph.pow(vx, two);
                    let y_sq = self.graph.pow(vy, two);
                    let sum = if sys_name == "spherical" {
                        let z_sq = self.graph.pow(vz, two);
                        self.graph.add([x_sq, y_sq, z_sq])
                    } else {
                        self.graph.add([x_sq, y_sq])
                    };
                    let half = self.graph.rational(1, 2);
                    self.graph.pow(sum, half)
                }
                ("spherical" | "polar" | "cylindrical", "theta") => {
                    self.graph.function("atan2", [vy, vx])
                }
                ("spherical", "phi") => {
                    let two = self.graph.integer(2);
                    let x_sq = self.graph.pow(vx, two);
                    let y_sq = self.graph.pow(vy, two);
                    let z_sq = self.graph.pow(vz, two);
                    let sum = self.graph.add([x_sq, y_sq, z_sq]);
                    let half = self.graph.rational(1, 2);
                    let r = self.graph.pow(sum, half);
                    let z_r = self.graph.div(vz, r);
                    self.graph.function("acos", [z_r])
                }
                _ => self.graph.symbol(&format!("{}_{}", base_var, coord_comp)),
            };

            return Ok((input, res_node));
        }

        let name = if raw_name.contains('.') {
            raw_name.replace('.', "_")
        } else {
            raw_name.to_string()
        };

        let (input, args_opt) = opt(delimited(
            tuple((multispace0, char('('), multispace0)),
            separated_list0(delimited(multispace0, char(','), multispace0), |i| {
                self.parse_expr(i)
            }),
            tuple((multispace0, char(')'))),
        ))(input)?;

        if let Some(args) = args_opt {
            if (name == "diff" || name == "derivative" || name == "d") && args.len() >= 2 {
                let wrt_name = if let Some(sym_str) =
                    self.graph
                        .symbols
                        .resolve(match &self.graph.get(args[1]).kind {
                            crate::ExprKind::Symbol(s) => *s,
                            _ => self.graph.symbols.get_or_intern("x"),
                        }) {
                    sym_str
                } else {
                    "x".to_string()
                };
                Ok((input, self.graph.derivative(args[0], &wrt_name, 1)))
            } else if (name == "int" || name == "integral") && args.len() >= 2 {
                let wrt_name = if let Some(sym_str) =
                    self.graph
                        .symbols
                        .resolve(match &self.graph.get(args[1]).kind {
                            crate::ExprKind::Symbol(s) => *s,
                            _ => self.graph.symbols.get_or_intern("x"),
                        }) {
                    sym_str
                } else {
                    "x".to_string()
                };
                let lower = if args.len() >= 3 { Some(args[2]) } else { None };
                let upper = if args.len() >= 4 { Some(args[3]) } else { None };
                Ok((input, self.graph.integral(args[0], &wrt_name, lower, upper)))
            } else {
                Ok((input, self.graph.function(&name, args)))
            }
        } else {
            Ok((input, self.graph.symbol(&name)))
        }
    }
}

/// Permissive intent types inferred from natural mathematical language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissiveIntent {
    /// Domain or variable type declaration: `x is a real number`, `p is a 3D clifford number with signature (1, 1, 1)`, `A is a 2 by 3 matrix`
    DomainDeclaration { name: String, domain_desc: String },
    /// Function or variable definition: `f(x, q) = cos(x) + sin(q)`, `f(x, q) equals cos(x) + sin(q)`, `f(x, q) is cos(x) + sin(q)`
    Define {
        name: String,
        args: Vec<String>,
        body: String,
    },
    /// Solve equation for target variable: `solve g(x) = 3, x`, `given g(x) = 3 find x`, `find x where g(x) = 3`
    Solve { equation: String, variable: String },
    /// Differentiate expression: `d f(x, q) / dx`, `derivative of f(x, q)`, `partial derivative of f(x, q) by x`, `total derivative of f(x, q)`
    Differentiate {
        expression: String,
        variable: Option<String>,
        is_total: bool,
    },
    /// Integrate expression: `suma f(x, q) dx`, `suma_{3}^{q} f(x, q) dx`, `integrate f(x, q) along (q, 7] by x`
    Integrate {
        expression: String,
        variable: Option<String>,
        lower: Option<String>,
        upper: Option<String>,
    },
    /// Simplify or factor expression: `simplify (x + 0) * 1`, `reduce (x^2 - 1)/(x - 1)`, `expand (x + 1)^2`, `factor x^2 - 1`
    Simplify { expression: String },
    /// Probability distribution assignment: `X ~ Normal(0, 1)`, `let X be Normal(0, 1)`
    Distribution {
        name: String,
        dist_type: String,
        params: Vec<String>,
    },
    /// Equivalence or domain comparison: `are f(x) and g(x) equal?`, `is Quaternion isomorphic to Matrix2x2Complex?`
    Compare {
        left: String,
        right: String,
        comparison_type: String,
    },
    /// Substitute or alter: `substitute x = 2 in f(x)`, `replace x with 2 in f(x)`, `evaluate f(x) at x = 2`
    Substitute {
        expression: String,
        variable: String,
        value: String,
    },
    /// Set definition or set builder: `{ x in Reals | -5 <= x <= 5 }`, `set S = { 1, 2, 3 }`
    SetDeclaration {
        name: String,
        domain_type: String,
        condition: String,
    },
    /// Symbolic limit: `limit of sin(x)/x as x -> 0`, `lim sin(x)/x as x -> 0`
    Limit {
        expression: String,
        variable: String,
        point: String,
        direction: Option<String>,
    },
    /// Symbolic summation: `sum of 1/n^2 from n=1 to infinity`, `sum_{n=1}^{10} n`
    Sum {
        expression: String,
        variable: String,
        lower: String,
        upper: String,
    },
    /// Prime decomposition with prime classification across number systems: `prime_factors 60`, `prime_factors 3 + 4i in GaussianIntegers`
    PrimeDecomposition {
        expression: String,
        domain: Option<String>,
    },
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in input.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(input[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim());
    parts
}

fn strip_matched_call<'a>(input: &'a str, names: &[&str]) -> Option<&'a str> {
    for &name in names {
        if let Some(rest) = input.strip_prefix(name) {
            if let Some(after_space) = rest.strip_prefix(' ') {
                return Some(after_space.trim());
            }
            if let Some(after_paren) = rest.strip_prefix('(') {
                let mut depth = 1usize;
                for (idx, c) in after_paren.char_indices() {
                    match c {
                        '(' | '[' | '{' => depth += 1,
                        ')' | ']' | '}' => {
                            depth -= 1;
                            if depth == 0 {
                                if idx == after_paren.len() - 1 {
                                    return Some(after_paren[..idx].trim());
                                }
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    None
}

fn parse_bracketed_list(input: &str) -> Option<Vec<String>> {
    let trimmed = input.trim();
    let (open, close) = if trimmed.starts_with('[') && trimmed.ends_with(']') {
        ('[', ']')
    } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
        ('{', '}')
    } else {
        return None;
    };
    let mut depth = 0usize;
    for (idx, c) in trimmed.char_indices() {
        if c == open || c == '(' || c == '[' || c == '{' {
            depth += 1;
        } else if c == close || c == ')' || c == ']' || c == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 && idx != trimmed.len() - 1 {
                return None;
            }
        }
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let items: Vec<String> = split_top_level_commas(inner)
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    Some(items)
}

fn split_expr_and_var(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();

    for keyword in [" by ", " with respect to ", " wrt "] {
        if let Some((expr_p, var_p)) = trimmed.rsplit_once(keyword) {
            return (expr_p.trim().to_string(), Some(var_p.trim().to_string()));
        }
    }

    if let Some(d_idx) = trimmed.rfind(" d") {
        let var_part = trimmed[d_idx + 2..].trim();
        if !var_part.is_empty() && var_part.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let expr_part = trimmed[..d_idx].trim();
            return (expr_part.to_string(), Some(var_part.to_string()));
        }
    }

    let mut depth = 0;
    let mut last_comma = None;
    for (i, c) in trimmed.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth > 0 => {
                depth -= 1;
            }
            ',' if depth == 0 => last_comma = Some(i),
            _ => {}
        }
    }

    if let Some(i) = last_comma {
        let (expr_p, var_p) = trimmed.split_at(i);
        let var_clean = var_p[1..].trim();
        if !var_clean.is_empty() {
            return (expr_p.trim().to_string(), Some(var_clean.to_string()));
        }
    }

    (trimmed.to_string(), None)
}

fn split_limit_point_direction(raw_pt: &str) -> (String, Option<String>) {
    let p = raw_pt.trim();
    if p.len() >= 2 && (p.ends_with('+') || p.ends_with('-')) {
        let dir = p[p.len() - 1..].to_string();
        let base = p[..p.len() - 1].trim().to_string();
        if !base.is_empty() && base != "from_above" && base != "from_below" {
            return (base, Some(dir));
        }
    }
    (p.to_string(), None)
}

/// Parse permissive natural language mathematical phrasing into structured intent.
pub fn parse_permissive_intent(input: &str) -> Option<PermissiveIntent> {
    let trimmed = input.trim();

    let is_command_prefix = trimmed.starts_with("solve")
        || trimmed.starts_with("solveset")
        || trimmed.starts_with("prime_factors")
        || trimmed.starts_with("prime_decomposition")
        || trimmed.starts_with("prime_decompose")
        || trimmed.starts_with("prime_factorization")
        || trimmed.starts_with("factor_primes")
        || trimmed.starts_with("decompose_prime")
        || trimmed.starts_with("factor ")
        || trimmed.starts_with("substitute ")
        || trimmed.starts_with("replace ")
        || trimmed.starts_with("evaluate ")
        || trimmed.starts_with("eval ")
        || trimmed.starts_with("diff ")
        || trimmed.starts_with("derivative")
        || trimmed.starts_with("integrate ")
        || trimmed.starts_with("int ");

    // 1. Domain & Variable Declarations e.g. `x is a real number`, `x is in Reals`, `x ∈ Reals`
    if !is_command_prefix
        && (trimmed.contains(" is a ")
            || trimmed.contains(" is in the set of ")
            || trimmed.contains(" is in ")
            || trimmed.contains(" ∈ ")
            || (trimmed.contains(" in ") && !trimmed.contains('{') && !trimmed.contains('|')))
        && let Some((var_part, dom_part)) = trimmed
            .split_once(" is in the set of ")
            .or_else(|| trimmed.split_once(" is in "))
            .or_else(|| trimmed.split_once(" is a "))
            .or_else(|| trimmed.split_once(" ∈ "))
            .or_else(|| trimmed.split_once(" in "))
    {
        return Some(PermissiveIntent::DomainDeclaration {
            name: var_part.trim().to_string(),
            domain_desc: dom_part.trim().to_string(),
        });
    }

    // 2. Solve commands e.g. `solve g(x) = 3, x`, `given g(x) = 3 find x`, `find x where g(x) = 3`
    if trimmed.starts_with("solve ")
        || trimmed.starts_with("solveset ")
        || trimmed.starts_with("solve(")
        || trimmed.starts_with("solveset(")
    {
        let clean = trimmed
            .strip_prefix("solve")
            .or_else(|| trimmed.strip_prefix("solveset"))
            .unwrap_or(trimmed)
            .trim()
            .trim_matches(|c| c == '(' || c == ')')
            .trim();

        if let Some((lhs, rhs)) = clean.split_once(" for ") {
            return Some(PermissiveIntent::Solve {
                equation: lhs.trim().to_string(),
                variable: rhs.trim().to_string(),
            });
        }
        if let Some((var_part, eq_part)) = clean.split_once(':') {
            let var_clean = var_part.strip_prefix("for ").unwrap_or(var_part).trim();
            return Some(PermissiveIntent::Solve {
                equation: eq_part.trim().to_string(),
                variable: var_clean.to_string(),
            });
        }
        if let Some((eq_p, v_p)) = clean.split_once(',') {
            return Some(PermissiveIntent::Solve {
                equation: eq_p.trim().to_string(),
                variable: v_p.trim().to_string(),
            });
        }
        return Some(PermissiveIntent::Solve {
            equation: clean.to_string(),
            variable: "x".to_string(),
        });
    }

    if let Some(rest) = trimmed.strip_prefix("given ")
        && let Some((eq_part, find_part)) = rest.split_once(" find ")
    {
        return Some(PermissiveIntent::Solve {
            equation: eq_part.trim().to_string(),
            variable: find_part.trim().to_string(),
        });
    }

    if let Some(rest) = trimmed.strip_prefix("find ") {
        if let Some((var_part, eq_part)) = rest.split_once(" where ") {
            return Some(PermissiveIntent::Solve {
                equation: eq_part.trim().to_string(),
                variable: var_part.trim().to_string(),
            });
        }
        if let Some((var_part, eq_part)) = rest.split_once(':') {
            return Some(PermissiveIntent::Solve {
                equation: eq_part.trim().to_string(),
                variable: var_part.trim().to_string(),
            });
        }
    }

    // 10. Limits e.g. `limit of sin(x)/x as x -> 0`, `limit of sin(x)/x as x approaches 0`, `lim_{x->0+} expr`, `lim_(x->0) (expr)`
    if trimmed.starts_with("limit ")
        || trimmed.starts_with("lim ")
        || trimmed.contains("limit of ")
        || trimmed.starts_with("lim_")
    {
        let mut clean = trimmed;
        if let Some(r) = clean.strip_prefix("limit of ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("limit ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("lim ") {
            clean = r.trim();
        }

        if let Some(after_lim) = clean.strip_prefix("lim_{")
            && let Some(close_brace) = after_lim.find('}')
        {
            let bound_part = &after_lim[..close_brace];
            let expr_part = after_lim[close_brace + 1..].trim();
            if let Some((var_part, val_part)) = bound_part
                .split_once("->")
                .or_else(|| bound_part.split_once("\\to"))
                .or_else(|| bound_part.split_once('='))
            {
                let (pt, dir) = split_limit_point_direction(val_part);
                return Some(PermissiveIntent::Limit {
                    expression: expr_part.to_string(),
                    variable: var_part.trim().to_string(),
                    point: pt,
                    direction: dir,
                });
            }
        } else if let Some((expr_part, bound_part)) = clean.split_once(" as ") {
            let expr = expr_part.trim().to_string();
            let bound = bound_part.trim();
            if let Some((var_part, val_part)) = bound
                .split_once("->")
                .or_else(|| bound.split_once(" approaches "))
                .or_else(|| bound.split_once(" to "))
            {
                let (pt, dir) = split_limit_point_direction(val_part);
                return Some(PermissiveIntent::Limit {
                    expression: expr,
                    variable: var_part.trim().to_string(),
                    point: pt,
                    direction: dir,
                });
            }
        } else if clean.starts_with("lim_(") || clean.starts_with("_(") {
            // The branch condition guarantees a '(' is present.
            let inner_start = clean.find('(').map_or(0, |i| i + 1);
            if let Some(close_rel) = clean[inner_start..].find(')') {
                let bound_part = &clean[inner_start..inner_start + close_rel];
                let expr_part = &clean[inner_start + close_rel + 1..];
                if let Some((var_part, val_part)) = bound_part
                    .split_once("->")
                    .or_else(|| bound_part.split_once('='))
                {
                    let (pt, dir) = split_limit_point_direction(val_part);
                    return Some(PermissiveIntent::Limit {
                        expression: expr_part.trim().to_string(),
                        variable: var_part.trim().to_string(),
                        point: pt,
                        direction: dir,
                    });
                }
            }
        } else {
            let parts = split_top_level_commas(clean);
            if parts.len() >= 3
                && !parts[0].is_empty()
                && !parts[1].is_empty()
                && !parts[2].is_empty()
            {
                let (mut pt, mut dir) = split_limit_point_direction(parts[2]);
                if parts.len() >= 4 && !parts[3].is_empty() {
                    let explicit_dir = match parts[3].trim() {
                        "+" | "right" | "from_above" => Some("+".to_string()),
                        "-" | "left" | "from_below" => Some("-".to_string()),
                        _ => None,
                    };
                    if explicit_dir.is_some() {
                        dir = explicit_dir;
                        pt = parts[2].trim().to_string();
                    }
                }
                return Some(PermissiveIntent::Limit {
                    expression: parts[0].to_string(),
                    variable: parts[1].to_string(),
                    point: pt,
                    direction: dir,
                });
            }
        }
    }

    // 11. Summations e.g. `sum of 1/n^2 from n=1 to infinity`, `sum_{n=1}^{10} n`, `sum k^2, k, 1, 10`, `sum(k, (k, 1, 10))`
    if trimmed.starts_with("sum ")
        || trimmed.starts_with("sum(")
        || trimmed.contains("sum of ")
        || trimmed.starts_with("sum_")
    {
        if trimmed.starts_with("sum(") && trimmed.ends_with(')') {
            let inner = &trimmed[4..trimmed.len() - 1];
            let parts = split_top_level_commas(inner);
            if parts.len() == 2 && parts[1].starts_with('(') && parts[1].ends_with(')') {
                let range_parts = split_top_level_commas(&parts[1][1..parts[1].len() - 1]);
                if range_parts.len() == 3 {
                    return Some(PermissiveIntent::Sum {
                        expression: parts[0].to_string(),
                        variable: range_parts[0].to_string(),
                        lower: range_parts[1].to_string(),
                        upper: range_parts[2].to_string(),
                    });
                }
            } else if parts.len() >= 4 {
                return Some(PermissiveIntent::Sum {
                    expression: parts[0].to_string(),
                    variable: parts[1].to_string(),
                    lower: parts[2].to_string(),
                    upper: parts[3].to_string(),
                });
            }
        }

        let mut clean = trimmed;
        if let Some(r) = clean.strip_prefix("sum of ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("sum ") {
            clean = r.trim();
        }

        if let Some((expr_part, range_part)) = clean.split_once(" from ") {
            let expr = expr_part.trim().to_string();
            if let Some((start_part, upper_part)) = range_part.trim().split_once(" to ")
                && let Some((var_part, lower_part)) = start_part.trim().split_once('=')
            {
                return Some(PermissiveIntent::Sum {
                    expression: expr,
                    variable: var_part.trim().to_string(),
                    lower: lower_part.trim().to_string(),
                    upper: upper_part.trim().to_string(),
                });
            }
        } else if clean.contains("_{")
            && clean.contains("}^{")
            && let Some(lower_start) = clean.find("_{")
            && let Some(lower_end) = clean[lower_start..].find('}')
        {
            let l_str = &clean[lower_start + 2..lower_start + lower_end];
            let rest = &clean[lower_start + lower_end + 1..];
            if let Some(upper_start) = rest.find("^{")
                && let Some(upper_end) = rest[upper_start..].find('}')
            {
                let u_str = &rest[upper_start + 2..upper_start + upper_end];
                let expr_part = &rest[upper_start + upper_end + 1..];
                if let Some((var_part, lower_val)) = l_str.split_once('=') {
                    return Some(PermissiveIntent::Sum {
                        expression: expr_part.trim().to_string(),
                        variable: var_part.trim().to_string(),
                        lower: lower_val.trim().to_string(),
                        upper: u_str.trim().to_string(),
                    });
                }
            }
        } else {
            let parts = split_top_level_commas(clean);
            if parts.len() >= 4 {
                return Some(PermissiveIntent::Sum {
                    expression: parts[0].to_string(),
                    variable: parts[1].to_string(),
                    lower: parts[2].to_string(),
                    upper: parts[3].to_string(),
                });
            } else if parts.len() == 2
                && let Some((var_part, bounds)) = parts[1].split_once('=')
                && let Some((l_str, u_str)) = bounds.split_once("..")
            {
                return Some(PermissiveIntent::Sum {
                    expression: parts[0].to_string(),
                    variable: var_part.trim().to_string(),
                    lower: l_str.trim().to_string(),
                    upper: u_str.trim().to_string(),
                });
            }
        }
    }

    // 3. Differentiation commands e.g. `d f(x, q) / dx`, `derivative of f(x, q)`, `partial derivative of f(x, q) by x`
    if trimmed.starts_with("d ")
        && trimmed.contains('/')
        && let Some((top_part, bot_part)) = trimmed.split_once('/')
    {
        let expr_part = top_part.strip_prefix("d ").unwrap_or(top_part).trim();
        let var_part = bot_part.trim().strip_prefix('d').unwrap_or(bot_part).trim();
        return Some(PermissiveIntent::Differentiate {
            expression: expr_part.to_string(),
            variable: if var_part.is_empty() {
                None
            } else {
                Some(var_part.to_string())
            },
            is_total: false,
        });
    }

    if trimmed.contains("derivative")
        || trimmed.contains("derivatives")
        || trimmed.contains("differentiate")
        || trimmed.contains("derive")
        || trimmed.starts_with("diff ")
    {
        let is_total = trimmed.contains("total");
        let mut clean = trimmed;
        if let Some(r) = clean.strip_prefix("total ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("partial ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("derivatives of ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("derivative of ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("differentiate ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("derive ") {
            clean = r.trim();
        }
        if let Some(r) = clean.strip_prefix("diff ") {
            clean = r.trim();
        }

        let (expr_str, var_str) = split_expr_and_var(clean);

        return Some(PermissiveIntent::Differentiate {
            expression: expr_str,
            variable: var_str,
            is_total,
        });
    }

    // 4. Integration commands e.g. `suma f(x, q) dx`, `suma_{3}^{q} f(x, q) dx`, `integrate f(x, q) along (q, 7] by x`
    if trimmed.starts_with("suma")
        || trimmed.starts_with("sum ")
        || trimmed.starts_with("int ")
        || trimmed.contains("integral")
        || trimmed.contains("integrals")
        || trimmed.starts_with("integrate ")
    {
        let mut lower = None;
        let mut upper = None;
        let mut clean_body = trimmed.to_string();

        if let Some(idx) = clean_body.find("along ") {
            let interval_part = clean_body[idx + 6..].trim();
            let mut int_clean = interval_part
                .trim_matches(|c| c == '[' || c == ']' || c == '(' || c == ')')
                .trim();
            if let Some((rest_int, var_after)) = int_clean.split_once(" by ") {
                int_clean = rest_int
                    .trim_matches(|c| c == '[' || c == ']' || c == '(' || c == ')')
                    .trim();
                let _ = var_after;
            }
            if let Some((l, u)) = int_clean.split_once(',') {
                lower = Some(l.trim().to_string());
                upper = Some(u.trim().to_string());
            }
            clean_body = clean_body[..idx].trim().to_string();
        } else if !clean_body.starts_with("sum")
            && !clean_body.starts_with("suma")
            && let Some((body, bounds)) = clean_body.split_once(" from ")
            && let Some((l, u)) = bounds.split_once(" to ")
        {
            // `integrate f(x) from a to b`: definite integral. (Dropping the bounds, as this branch once did,
            // returned the antiderivative as if it were the answer.)
            lower = Some(l.trim().to_string());
            upper = Some(u.trim().to_string());
            clean_body = body.trim().to_string();
        } else if clean_body.contains("_{")
            && clean_body.contains("}^{")
            && let Some(lower_start) = clean_body.find("_{")
            && let Some(lower_end) = clean_body[lower_start..].find('}')
        {
            let l_str = &clean_body[lower_start + 2..lower_start + lower_end];
            lower = Some(l_str.trim().to_string());
            let rest = &clean_body[lower_start + lower_end + 1..];
            if let Some(upper_start) = rest.find("^{")
                && let Some(upper_end) = rest[upper_start..].find('}')
            {
                let u_str = &rest[upper_start + 2..upper_start + upper_end];
                upper = Some(u_str.trim().to_string());
                clean_body = format!(
                    "{} {}",
                    &clean_body[..lower_start],
                    &rest[upper_start + upper_end + 1..]
                );
            }
        }

        let mut stripped = clean_body.as_str().trim();
        if let Some(r) = stripped.strip_prefix("suma") {
            stripped = r.trim();
        }
        if let Some(r) = stripped.strip_prefix("integrals of ") {
            stripped = r.trim();
        }
        if let Some(r) = stripped.strip_prefix("integral of ") {
            stripped = r.trim();
        }
        if let Some(r) = stripped.strip_prefix("integrate ") {
            stripped = r.trim();
        }
        if let Some(r) = stripped.strip_prefix("int ") {
            stripped = r.trim();
        }

        let (expr_str, var_str) = split_expr_and_var(stripped);

        return Some(PermissiveIntent::Integrate {
            expression: expr_str,
            variable: var_str,
            lower,
            upper,
        });
    }

    // 4.5 Prime decomposition commands e.g. `prime_factors 60`, `prime_decomposition 3 + 4i in GaussianIntegers`
    if let Some(rest) = trimmed
        .strip_prefix("prime_factors ")
        .or_else(|| trimmed.strip_prefix("prime_decomposition "))
        .or_else(|| trimmed.strip_prefix("prime_decompose "))
        .or_else(|| trimmed.strip_prefix("factor_primes "))
        .or_else(|| trimmed.strip_prefix("decompose_prime "))
        .or_else(|| trimmed.strip_prefix("prime_factorization "))
    {
        let (expr, dom) = if let Some((e, d)) = rest.split_once(" in ") {
            (e.trim().to_string(), Some(d.trim().to_string()))
        } else {
            (rest.trim().to_string(), None)
        };
        return Some(PermissiveIntent::PrimeDecomposition {
            expression: expr,
            domain: dom,
        });
    }

    // 5. Simplification commands e.g. `simplify (x + 0) * 1`
    if let Some(rest) = trimmed
        .strip_prefix("simplify ")
        .or_else(|| trimmed.strip_prefix("reduce "))
        .or_else(|| trimmed.strip_prefix("expand "))
    {
        return Some(PermissiveIntent::Simplify {
            expression: rest.trim().to_string(),
        });
    }

    if let Some(rest) = trimmed.strip_prefix("factor ") {
        if let Some((e, d)) = rest.split_once(" in ") {
            return Some(PermissiveIntent::PrimeDecomposition {
                expression: e.trim().to_string(),
                domain: Some(d.trim().to_string()),
            });
        }
        let clean = rest.trim();
        let is_pure_number = !clean.is_empty()
            && clean.chars().all(|c| {
                c.is_ascii_digit()
                    || c == '/'
                    || c == '+'
                    || c == '-'
                    || c == '*'
                    || c == ' '
                    || c == 'i'
                    || c == 'w'
                    || c == 'ω'
            });
        if is_pure_number && !clean.contains("x") && !clean.contains("y") {
            return Some(PermissiveIntent::PrimeDecomposition {
                expression: clean.to_string(),
                domain: None,
            });
        }
        return Some(PermissiveIntent::Simplify {
            expression: clean.to_string(),
        });
    }

    // 6. Substitutions / Alterations e.g. `substitute x = 2 in f(x)`
    if let Some(rest) = trimmed
        .strip_prefix("substitute ")
        .or_else(|| trimmed.strip_prefix("replace "))
        && let Some((sub_part, expr_part)) = rest.split_once(" in ")
        && let Some((var_part, val_part)) = sub_part
            .split_once('=')
            .or_else(|| sub_part.split_once(" with "))
    {
        return Some(PermissiveIntent::Substitute {
            expression: expr_part.trim().to_string(),
            variable: var_part.trim().to_string(),
            value: val_part.trim().to_string(),
        });
    }

    if let Some(rest) = trimmed.strip_prefix("evaluate ")
        && let Some((expr_part, at_part)) = rest.split_once(" at ")
        && let Some((var_part, val_part)) = at_part.split_once('=')
    {
        return Some(PermissiveIntent::Substitute {
            expression: expr_part.trim().to_string(),
            variable: var_part.trim().to_string(),
            value: val_part.trim().to_string(),
        });
    }

    // 7. Distributions e.g. `X ~ Normal(0, 1)`
    if trimmed.contains('~')
        && let Some((var_part, dist_part)) = trimmed.split_once('~')
        && let Some(open_p) = dist_part.find('(')
        && let Some(close_p) = dist_part.find(')')
    {
        let d_type = dist_part[..open_p].trim().to_string();
        let params = dist_part[open_p + 1..close_p]
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        return Some(PermissiveIntent::Distribution {
            name: var_part.trim().to_string(),
            dist_type: d_type,
            params,
        });
    }

    // 8. Comparisons / Equivalence e.g. `are f(x) and g(x) equal?`
    if let Some(rest) = trimmed
        .strip_prefix("are ")
        .or_else(|| trimmed.strip_prefix("check if "))
        .or_else(|| trimmed.strip_prefix("compare "))
        .or_else(|| trimmed.strip_prefix("is "))
    {
        let clean = rest.trim_end_matches('?').trim();
        if let Some((left, right)) = clean
            .split_once(" and ")
            .or_else(|| clean.split_once(" equal to "))
            .or_else(|| clean.split_once(" isomorphic to "))
            .or_else(|| clean.split_once(" equals "))
        {
            let right_clean = right
                .trim()
                .trim_end_matches("equal")
                .trim_end_matches("isomorphic")
                .trim();
            let c_type = if clean.contains("isomorphic") {
                "isomorphic"
            } else {
                "equal"
            };
            return Some(PermissiveIntent::Compare {
                left: left.trim().to_string(),
                right: right_clean.to_string(),
                comparison_type: c_type.to_string(),
            });
        }
    }

    // 9. Definitions e.g. `f(x, q) = cos(x) + sin(q)`, `f(x, q) equals cos(x) + sin(q)`, `f(x, q) is cos(x) + sin(q)`
    let def_clean = trimmed
        .strip_prefix("let ")
        .or_else(|| trimmed.strip_prefix("define "))
        .unwrap_or(trimmed);
    if let Some((lhs, rhs)) = def_clean
        .split_once(":=")
        .or_else(|| def_clean.split_once(" equals "))
        .or_else(|| def_clean.split_once(" is "))
        .or_else(|| def_clean.split_once(" as "))
        .or_else(|| def_clean.split_once('='))
    {
        let lhs_trim = lhs.trim();
        if let Some(open_p) = lhs_trim.find('(')
            && let Some(close_p) = lhs_trim.find(')')
            && close_p > open_p
        {
            let fn_name = lhs_trim[..open_p].trim().to_string();
            let args = lhs_trim[open_p + 1..close_p]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !fn_name.is_empty() {
                return Some(PermissiveIntent::Define {
                    name: fn_name,
                    args,
                    body: rhs.trim().to_string(),
                });
            }
        }
    }

    None
}

use crate::domain::DomainBound;
use crate::operation::*;
use std::collections::HashMap;

/// Parse a natural set-theory domain declaration e.g. `a in Reals [0, 100]` or `x in Positive`.
pub fn parse_domain_declaration(line: &str) -> Option<DomainBound> {
    let trimmed = line.trim();
    if !trimmed.contains(" in ") && !trimmed.contains(" ∈ ") {
        return None;
    }

    let (raw_var_part, rest) = if let Some(p) = trimmed.split_once(" in ") {
        p
    } else {
        trimmed.split_once(" ∈ ")?
    };

    let var_name = if let Some((v, _)) = raw_var_part.split_once(':') {
        v.trim()
    } else {
        raw_var_part.trim()
    };

    if var_name.is_empty() || !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    let rest_trimmed = rest.trim();
    let mut domain_type = "Reals".to_string();
    let mut min_val = None;
    let mut max_val = None;
    let mut inclusive_min = true;
    let mut inclusive_max = true;

    // Detect all supported number systems
    if rest_trimmed.starts_with("Positive") {
        domain_type = "Positive".to_string();
        min_val = Some(0.001);
        max_val = Some(100.0);
        inclusive_min = false;
    } else if rest_trimmed.starts_with("NonNegative") {
        domain_type = "NonNegative".to_string();
        min_val = Some(0.0);
        max_val = Some(100.0);
    } else if rest_trimmed.starts_with("Naturals") || rest_trimmed.starts_with("ℕ") {
        domain_type = "Naturals".to_string();
        min_val = Some(0.0);
    } else if rest_trimmed.starts_with("Integers") || rest_trimmed.starts_with("ℤ") {
        domain_type = "Integers".to_string();
    } else if rest_trimmed.starts_with("Rationals") || rest_trimmed.starts_with("ℚ") {
        domain_type = "Rationals".to_string();
    } else if rest_trimmed.starts_with("Complex") || rest_trimmed.starts_with("ℂ") {
        domain_type = "Complex".to_string();
    } else if rest_trimmed.starts_with("Reals") || rest_trimmed.starts_with("ℝ") {
        domain_type = "Reals".to_string();
    } else if rest_trimmed.starts_with("CayleyDickson") {
        domain_type = "CayleyDickson".to_string();
    } else if rest_trimmed.starts_with("Quaternion") || rest_trimmed.starts_with("ℍ") {
        domain_type = "Quaternion".to_string();
    } else if rest_trimmed.starts_with("Octonion") || rest_trimmed.starts_with("𝕆") {
        domain_type = "Octonion".to_string();
    } else if rest_trimmed.starts_with("Sedenion") || rest_trimmed.starts_with("𝕊") {
        domain_type = "Sedenion".to_string();
    } else if rest_trimmed.starts_with("DualComplex") {
        domain_type = "DualComplex".to_string();
    } else if rest_trimmed.starts_with("DualQuaternion") {
        domain_type = "DualQuaternion".to_string();
    } else if rest_trimmed.starts_with("Dual") || rest_trimmed.starts_with("𝔻") {
        domain_type = "Dual".to_string();
    } else if rest_trimmed.starts_with("SplitComplex") {
        domain_type = "SplitComplex".to_string();
    } else if rest_trimmed.starts_with("Bicomplex") {
        domain_type = "Bicomplex".to_string();
    } else if rest_trimmed.starts_with("Clifford") {
        domain_type = "Clifford".to_string();
    } else if rest_trimmed.starts_with("PAdics")
        || rest_trimmed.starts_with("Padics")
        || rest_trimmed.starts_with("ℚ_p")
        || rest_trimmed.starts_with("Q_p")
    {
        domain_type = "PAdics".to_string();
    } else if rest_trimmed.starts_with("Adeles") || rest_trimmed.starts_with("𝔸") {
        domain_type = "Adeles".to_string();
    } else if rest_trimmed.starts_with("Surreals")
        || rest_trimmed.starts_with("Surreal")
        || rest_trimmed.starts_with("𝐍𝐨")
    {
        domain_type = "Surreals".to_string();
    } else if rest_trimmed.starts_with("ModuloUnits") || rest_trimmed.starts_with("(ℤ/") {
        domain_type = "ModuloUnits".to_string();
    } else if rest_trimmed.starts_with("Modulo") || rest_trimmed.starts_with("ℤ/") {
        domain_type = "Modulo".to_string();
    } else if rest_trimmed.starts_with("GaloisField")
        || rest_trimmed.starts_with("GF")
        || rest_trimmed.starts_with("𝔽")
    {
        domain_type = "GaloisField".to_string();
    } else if rest_trimmed.starts_with("GaussianIntegers") || rest_trimmed.starts_with("ℤ[i]") {
        domain_type = "GaussianIntegers".to_string();
    } else if rest_trimmed.starts_with("EisensteinIntegers") || rest_trimmed.starts_with("ℤ[ω]")
    {
        domain_type = "EisensteinIntegers".to_string();
    } else if rest_trimmed.starts_with("Boolean") || rest_trimmed.starts_with("𝔹") {
        domain_type = "Boolean".to_string();
    } else if rest_trimmed.starts_with("BitVector") {
        domain_type = "BitVector".to_string();
    } else if rest_trimmed.starts_with("EvenIntegers") || rest_trimmed.starts_with("2ℤ") {
        domain_type = "EvenIntegers".to_string();
    } else if rest_trimmed.starts_with("OddIntegers") || rest_trimmed.starts_with("2ℤ+1") {
        domain_type = "OddIntegers".to_string();
    } else if rest_trimmed.starts_with("Matrix") {
        domain_type = "Matrix".to_string();
    } else if rest_trimmed.starts_with("Tensor") {
        domain_type = "Tensor".to_string();
    }

    // Check for bracket interval `[min, max]` or `(min, max)`
    if let Some(bracket_start) = rest_trimmed.find(['[', '('])
        && let Some(bracket_end) = rest_trimmed.rfind([']', ')'])
        && bracket_end > bracket_start
    {
        inclusive_min = &rest_trimmed[bracket_start..=bracket_start] == "[";
        inclusive_max = &rest_trimmed[bracket_end..=bracket_end] == "]";
        let inner = &rest_trimmed[bracket_start + 1..bracket_end];
        if let Some((min_s, max_s)) = inner.split_once(',') {
            let parse_bound = |s: &str| -> Option<f64> {
                let s = s.trim();
                if s == "-infinity" || s == "-inf" {
                    Some(-1e6)
                } else if s == "infinity" || s == "inf" {
                    Some(1e6)
                } else if s == "2*pi" || s == "2pi" {
                    Some(std::f64::consts::TAU)
                } else if s == "pi" {
                    Some(std::f64::consts::PI)
                } else {
                    s.parse::<f64>().ok()
                }
            };
            min_val = parse_bound(min_s);
            max_val = parse_bound(max_s);
        }
    }

    Some(DomainBound {
        name: var_name.to_string(),
        domain_type,
        min_val,
        max_val,
        inclusive_min,
        inclusive_max,
    })
}

/// Parse raw text string into a strongly-typed `MathOperation`.
/// Rewrite the call spellings people reach for first (`series(f, x, 0, 5)`, `isprime(97)`, `factorint(360)`,
/// `inverse(M)`, `limit(f, x, 0)`, `17 mod 5`) into the command forms the parser knows. Without this they were
/// parsed as an expression and echoed back unevaluated, which looks like an answer but is not one.
/// The canonical name of an integer function, if `name` is one of its spellings.
fn arithmetic_function_name(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "totient" | "euler_phi" | "eulerphi" | "phi" => "totient",
        "nextprime" | "next_prime" => "next_prime",
        "prevprime" | "prev_prime" | "previousprime" => "prev_prime",
        "divisors" => "divisors",
        "sigma" | "divisor_sum" => "sigma",
        "tau" | "num_divisors" | "divisor_count" => "tau",
        "mobius" | "moebius" => "mobius",
        "modinv" | "mod_inverse" | "invmod" => "mod_inverse",
        "powmod" | "pow_mod" | "modpow" => "pow_mod",
        "primitive_root" | "primitiveroot" => "primitive_root",
        _ => return None,
    })
}

/// The canonical name of a numerical or statistical command, if `name` is one of its spellings.
fn numeric_command_name(name: &str) -> Option<&'static str> {
    Some(match name.to_ascii_lowercase().as_str() {
        "nsolve" | "findroot" | "find_root" => "nsolve",
        "nintegrate" | "nint" | "numeric_integrate" => "nintegrate",
        "minimize" | "argmin" => "minimize",
        "maximize" | "argmax" => "maximize",
        "interpolate" | "lagrange" | "interp" => "interpolate",
        "fit" | "polyfit" | "linfit" => "fit",
        "mean" | "average" => "mean",
        "median" => "median",
        "variance" => "variance",
        "pvariance" => "pvariance",
        "stddev" => "stddev",
        "pstddev" => "pstddev",
        "product" | "prod" => "product",
        "next_term" | "nth_term" | "find_pattern" | "sequence_rule" | "guess_sequence" => {
            "next_term"
        }
        "percentile" => "percentile",
        "quantile" => "quantile",
        "quartiles" => "quartiles",
        "iqr" => "iqr",
        "skewness" => "skewness",
        "kurtosis" => "kurtosis",
        "zscore" => "zscore",
        "dec2bin" => "dec2bin",
        "bin2dec" => "bin2dec",
        "dec2hex" | "hex" => "dec2hex",
        "hex2dec" => "hex2dec",
        "dec2oct" | "oct" => "dec2oct",
        "oct2dec" => "oct2dec",
        "subsets" | "powerset" | "power_set" => "subsets",
        "permutations_list" | "all_permutations" => "permutations_list",
        "cycle_type" => "cycle_type",
        "next_permutation" => "next_permutation",
        "pell" => "pell",
        "diophantine" => "diophantine",
        "mode" => "mode",
        "range" => "range",
        "covariance" | "cov" => "covariance",
        "correlation" | "corr" => "correlation",
        "gradient" | "grad" => "gradient",
        "hessian" => "hessian",
        "jacobian" => "jacobian",
        "divergence" => "divergence",
        "curl" => "curl",
        "laplacian" => "laplacian",
        "cross" | "cross_product" => "cross",
        "dot" | "dot_product" => "dot",
        "norm" => "norm",
        "trigsimp" | "trig_simplify" => "trigsimp",
        "expand_trig" | "trigexpand" => "expand_trig",
        "matpow" | "matrix_power" | "mpow" => "matpow",
        "lu" | "lu_decomposition" => "lu",
        "ztransform" | "z_transform" | "ztrans" => "ztransform",
        "divmod" => "divmod",
        "floordiv" => "floordiv",
        "digits" => "digits",
        "egcd" | "xgcd" => "egcd",
        "crt" | "chinese_remainder" => "crt",
        "continued_fraction" | "cfrac" | "contfrac" => "continued_fraction",
        "convergents" => "convergents",
        _ => return None,
    })
}

fn normalize_call_syntax(input: &str) -> Option<String> {
    let input = input.trim();
    // `mean [1, 2, 3]`, `nsolve cos(x) - x, 1`: the command spelling (not an assignment such as `mean = 5`).
    if let Some((name, args)) = input.split_once(' ')
        && let Some(canonical) = numeric_command_name(name)
        && !args.trim_start().starts_with('=')
        && !args.trim_start().starts_with('(')
    {
        return Some(format!("numcmd {canonical} {}", args.trim()));
    }
    // `totient 36`, `nextprime 100`: the command spelling of an integer function.
    if let Some((name, args)) = input.split_once(' ')
        && let Some(canonical) = arithmetic_function_name(name)
        && !args.trim_start().starts_with('(')
    {
        return Some(format!("numfn {canonical} {}", args.trim()));
    }
    // `[[1,2],[3,4]] * [[0,1],[1,0]]`: a product of two matrix literals is a matrix product.
    // `[[1,2],[3,4]]^3`: a matrix literal to an integer power.
    if input.starts_with("[[") {
        let mut depth = 0usize;
        for (i, c) in input.char_indices() {
            match c {
                '[' | '(' | '{' => depth += 1,
                ']' | ')' | '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let rest = input[i + 1..].trim_start();
                        if let Some(power) = rest.strip_prefix('^') {
                            return Some(format!(
                                "numcmd matpow {}, {}",
                                &input[..=i],
                                power.trim()
                            ));
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    if input.starts_with('[') && input.ends_with(']') {
        let mut depth = 0usize;
        for (i, c) in input.char_indices() {
            match c {
                '[' | '(' | '{' => depth += 1,
                ']' | ')' | '}' => depth = depth.saturating_sub(1),
                '*' if depth == 0 => {
                    let (a, b) = (input[..i].trim(), input[i + 1..].trim());
                    if a.starts_with("[[")
                        && a.ends_with(']')
                        && b.starts_with("[[")
                        && b.ends_with(']')
                    {
                        return Some(format!("matmul {a}, {b}"));
                    }
                    return None;
                }
                _ => {}
            }
        }
    }
    // `a // b`: floor division.
    if let Some((a, b)) = input.split_once(" // ")
        && !a.trim().is_empty()
        && !b.trim().is_empty()
    {
        return Some(format!("numcmd floordiv {}, {}", a.trim(), b.trim()));
    }
    // `a mod b`
    if let Some((a, b)) = input.split_once(" mod ")
        && !a.contains(char::is_whitespace)
        && !b.contains(char::is_whitespace)
        && !a.is_empty()
        && !b.is_empty()
    {
        return Some(format!("simplify mod({a}, {b})"));
    }
    let open = input.find('(')?;
    let name = &input[..open];
    if name.is_empty()
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !input.ends_with(')')
    {
        return None;
    }
    // The call must be a single call: its opening bracket closes at the very end.
    let mut depth = 0usize;
    for (i, c) in input[open..].char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 && open + i != input.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    let args = input[open + 1..input.len() - 1].trim();
    let canonical = match name.to_ascii_lowercase().as_str() {
        "series" | "taylor" | "maclaurin" => "series",
        "is_prime" | "isprime" => "is_prime",
        "factor" if !args.trim().is_empty() && !args.chars().any(|c| c.is_ascii_alphabetic()) => {
            "prime_factors"
        }
        "factorint"
        | "factor_integer"
        | "primefactors"
        | "prime_factors"
        | "prime_factorization" => "prime_factors",
        "partial_fractions" | "apart" | "partfrac" => "partial_fractions",
        "inv" | "inverse" | "matrix_inverse" => "inv",
        "det" | "determinant" => "det",
        "transpose" => "transpose",
        "rank" => "rank",
        "dsolve" => "dsolve",
        // Commands that take their arguments after a space; the call spelling `expand((x+1)^5)` means the same.
        "expand" | "cancel" | "collect" | "together" | "horner" | "discriminant" | "resultant"
        | "roots" | "poly_gcd" | "laplace" | "fourier" | "ilt" | "inv_laplace" | "poly_roots" => {
            return Some(format!("{} {args}", name.to_ascii_lowercase()));
        }
        // `factor(x^4 - 1)`: a polynomial (an integer was handled above); `gcd(x^2 - 1, x^3 - 1)`, `lcm(...)`.
        "factor" if args.chars().any(|c| c.is_ascii_alphabetic()) => {
            return Some(format!("factor {args}"));
        }
        "gcd" if args.chars().any(|c| c.is_ascii_alphabetic()) => {
            return Some(format!("poly_gcd {args}"));
        }
        other if numeric_command_name(other).is_some() => {
            let canonical = numeric_command_name(other)?;
            return Some(format!("numcmd {canonical} {args}"));
        }
        other if arithmetic_function_name(other).is_some() => {
            let canonical = arithmetic_function_name(other)?;
            return Some(format!("numfn {canonical} {args}"));
        }
        "limit" | "lim" => {
            // limit(f, x, a) -> `limit f as x -> a`
            let parts = split_top_level_commas(args);
            if parts.len() >= 3 {
                // An optional fourth argument names a one-sided limit: `limit(f, x, 0, +)`, `limit(f, x, 0, "left")`.
                let side = match parts
                    .get(3)
                    .map(|s| s.trim().trim_matches('"').to_ascii_lowercase())
                {
                    Some(s) if matches!(s.as_str(), "+" | "right" | "plus" | "from_right") => "+",
                    Some(s) if matches!(s.as_str(), "-" | "left" | "minus" | "from_left") => "-",
                    _ => "",
                };
                return Some(format!(
                    "limit {} as {} -> {}{}",
                    parts[0].trim(),
                    parts[1].trim(),
                    parts[2].trim(),
                    side
                ));
            }
            return None;
        }
        _ => return None,
    };
    Some(format!("{canonical} {args}"))
}

/// Parse one input line into the operation it asks for. Call spellings such as `series(f, x, 0, 5)` are accepted
/// next to the command spellings `series f, x, 0, 5`.
pub fn parse_operation(input: &str) -> MathOperation {
    match normalize_call_syntax(input) {
        Some(rewritten) => parse_operation_canonical(&rewritten),
        None => parse_operation_canonical(input),
    }
}

fn parse_operation_canonical(input: &str) -> MathOperation {
    let input = input.trim();
    if input.is_empty() {
        return MathOperation::Expression { raw: String::new() };
    }

    // 1. System Commands
    // 1a. Assumptions: `assume x > 0`, `assume x >= 0`, `assume x < 0`, `assume x: positive|real|integer|negative|nonzero|even|odd|prime`
    if let Some(rest) = input
        .strip_prefix("assume ")
        .or_else(|| input.strip_prefix("Assume "))
    {
        let body = rest.trim();
        if let Some(bound) = parse_domain_declaration(body) {
            return MathOperation::DomainRestriction(bound);
        }
        let make_bound = |name: &str,
                          domain_type: &str,
                          min_val: Option<f64>,
                          max_val: Option<f64>,
                          inc_min: bool,
                          inc_max: bool| {
            MathOperation::DomainRestriction(DomainBound {
                name: name.to_string(),
                domain_type: domain_type.to_string(),
                min_val,
                max_val,
                inclusive_min: inc_min,
                inclusive_max: inc_max,
            })
        };
        if let Some((v, rhs)) = body.split_once(">=")
            && rhs.trim() == "0"
        {
            return make_bound(v.trim(), "NonNegative", Some(0.0), None, true, false);
        }
        if let Some((v, rhs)) = body.split_once('>')
            && rhs.trim() == "0"
        {
            return make_bound(v.trim(), "Positive", Some(0.0), None, false, false);
        }
        if let Some((v, rhs)) = body.split_once("<=")
            && rhs.trim() == "0"
        {
            return make_bound(v.trim(), "NonPositive", None, Some(0.0), false, true);
        }
        if let Some((v, rhs)) = body.split_once('<')
            && rhs.trim() == "0"
        {
            return make_bound(v.trim(), "Negative", None, Some(0.0), false, false);
        }
        if let Some((v, rhs)) = body.split_once("!=")
            && rhs.trim() == "0"
        {
            return make_bound(v.trim(), "NonZero", None, None, false, false);
        }
        if let Some((v, prop)) = body.split_once(':').or_else(|| body.split_once(" is ")) {
            let p_low = prop.trim().to_ascii_lowercase();
            let dt = match p_low.as_str() {
                "positive" | "pos" | "r+" => "Positive",
                "nonnegative" | "non_negative" | "non-negative" => "NonNegative",
                "negative" | "neg" | "r-" => "Negative",
                "nonzero" | "non_zero" | "non-zero" => "NonZero",
                "real" | "reals" => "Reals",
                "integer" | "integers" | "int" => "Integers",
                "even" => "EvenIntegers",
                "odd" => "OddIntegers",
                "prime" => "Prime",
                "complex" => "Complex",
                _ => prop.trim(),
            };
            return make_bound(v.trim(), dt, None, None, false, false);
        }
    }
    if input.eq_ignore_ascii_case("help") || input.eq_ignore_ascii_case("?") {
        return MathOperation::System(SystemCommandKind::Help);
    }
    if input.eq_ignore_ascii_case("vars") {
        return MathOperation::System(SystemCommandKind::Vars);
    }
    if input.eq_ignore_ascii_case("params") {
        return MathOperation::System(SystemCommandKind::Params);
    }
    if input.eq_ignore_ascii_case("context") || input.eq_ignore_ascii_case("mathcontext") {
        return MathOperation::System(SystemCommandKind::Context);
    }
    if input.eq_ignore_ascii_case("clear") || input.eq_ignore_ascii_case("cls") {
        return MathOperation::System(SystemCommandKind::Clear);
    }
    if let Some(log_body) = input.strip_prefix("log ") {
        return MathOperation::System(SystemCommandKind::Log {
            level: log_body.trim().to_string(),
        });
    }
    if let Some(preset_body) = input.strip_prefix("preset ") {
        return MathOperation::System(SystemCommandKind::Preset {
            name: preset_body.trim().to_string(),
        });
    }
    if let Some(exp_body) = input.strip_prefix("export ") {
        let (fmt, expr) = exp_body
            .trim()
            .split_once(' ')
            .unwrap_or((exp_body.trim(), ""));
        return MathOperation::System(SystemCommandKind::Export {
            format: fmt.trim().to_string(),
            expr: expr.trim().to_string(),
        });
    }

    // 2. Set Context Key-Value
    if let Some(set_body) = input.strip_prefix("set ")
        && let Some((k, v)) = set_body.trim().split_once(' ')
    {
        return MathOperation::SetContext {
            key: k.trim().to_string(),
            value: v.trim().to_string(),
        };
    }

    // 3. AI Copilot Query
    if let Some(agent_body) = input
        .strip_prefix("agent ")
        .or_else(|| input.strip_prefix("ai "))
    {
        return MathOperation::AiQuery {
            prompt: agent_body.trim().to_string(),
        };
    }

    // 4. Formal Proofs
    if let Some(proof_body) = input
        .strip_prefix("proof ")
        .or_else(|| input.strip_prefix("prove "))
    {
        return MathOperation::Proof {
            expression: proof_body.trim().to_string(),
            target: ProofTarget::Both,
        };
    }
    if let Some(lean_body) = input.strip_prefix("lean ") {
        return MathOperation::Proof {
            expression: lean_body.trim().to_string(),
            target: ProofTarget::Lean4,
        };
    }
    if let Some(coq_body) = input.strip_prefix("coq ") {
        return MathOperation::Proof {
            expression: coq_body.trim().to_string(),
            target: ProofTarget::Coq,
        };
    }

    // 5. Esoteric Hyperoperations (Knuth Up-Arrow, Tetration, Pentation, Ackermann, SuperLog)
    if let Some(body) = input
        .strip_prefix("tetration ")
        .or_else(|| input.strip_prefix("tet "))
    {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(a), Ok(b)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
        {
            return MathOperation::HyperOp(HyperOpKind::Tetration { a, b });
        }
    }
    if let Some(body) = input
        .strip_prefix("pentation ")
        .or_else(|| input.strip_prefix("pent "))
    {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(a), Ok(b)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
        {
            return MathOperation::HyperOp(HyperOpKind::Pentation { a, b });
        }
    }
    if let Some(body) = input
        .strip_prefix("knuth ")
        .or_else(|| input.strip_prefix("uparrow "))
    {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 3
            && let (Ok(a), Ok(arrows), Ok(b)) = (
                parts[0].parse::<u64>(),
                parts[1].parse::<usize>(),
                parts[2].parse::<u64>(),
            )
        {
            return MathOperation::HyperOp(HyperOpKind::KnuthUpArrow { a, arrows, b });
        }
    }
    if let Some(body) = input
        .strip_prefix("ackermann ")
        .or_else(|| input.strip_prefix("ack "))
    {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(m), Ok(n)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
        {
            return MathOperation::HyperOp(HyperOpKind::Ackermann { m, n });
        }
    }
    if let Some(body) = input
        .strip_prefix("slog ")
        .or_else(|| input.strip_prefix("superlog "))
    {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(base), Ok(val)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
        {
            return MathOperation::HyperOp(HyperOpKind::SuperLog { base, val });
        }
    }
    if let Some(body) = input.strip_prefix("hyperop ") {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 3
            && let (Ok(n), Ok(a), Ok(b)) = (
                parts[0].parse::<usize>(),
                parts[1].parse::<u64>(),
                parts[2].parse::<u64>(),
            )
        {
            return MathOperation::HyperOp(HyperOpKind::General { n, a, b });
        }
    }

    // 6. Tropical Semirings
    if let Some(body) = input.strip_prefix("maxplus_add ") {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
        {
            return MathOperation::Tropical(TropicalOpKind::MaxPlusAdd { a, b });
        }
    }
    if let Some(body) = input.strip_prefix("maxplus_mul ") {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
        {
            return MathOperation::Tropical(TropicalOpKind::MaxPlusMul { a, b });
        }
    }
    if let Some(body) = input.strip_prefix("minplus_add ") {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
        {
            return MathOperation::Tropical(TropicalOpKind::MinPlusAdd { a, b });
        }
    }
    if let Some(body) = input.strip_prefix("minplus_mul ") {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
        {
            return MathOperation::Tropical(TropicalOpKind::MinPlusMul { a, b });
        }
    }
    if let Some(body) = input
        .strip_prefix("log_sum_exp ")
        .or_else(|| input.strip_prefix("lse "))
    {
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() >= 2
            && let (Ok(x), Ok(y)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
        {
            let eps = if parts.len() >= 3 {
                parts[2].parse::<f64>().unwrap_or(0.01)
            } else {
                0.01
            };
            return MathOperation::Tropical(TropicalOpKind::LogSumExp { x, y, epsilon: eps });
        }
    }

    // 7. Computational Number Theory
    if let Some(body) = input
        .strip_prefix("cf ")
        .or_else(|| input.strip_prefix("continued_fraction "))
    {
        let parts: Vec<&str> = body.split(',').collect();
        let val_str = parts[0].trim().to_string();
        let max_terms = if parts.len() >= 2 {
            parts[1].trim().parse::<usize>().unwrap_or(8)
        } else {
            8
        };
        return MathOperation::NumberTheory(NumberTheoryOpKind::ContinuedFraction {
            val_str,
            max_terms,
        });
    }
    if let Some(body) = input
        .strip_prefix("prime_factors ")
        .or_else(|| input.strip_prefix("prime_decomposition "))
        .or_else(|| input.strip_prefix("prime_decompose "))
        .or_else(|| input.strip_prefix("factor_primes "))
        .or_else(|| input.strip_prefix("prime_factorization "))
    {
        let (expr_str, domain_hint) = if let Some((e, d)) = body.split_once(" in ") {
            (e.trim().to_string(), Some(d.trim().to_string()))
        } else {
            (body.trim().to_string(), None)
        };
        return MathOperation::NumberTheory(NumberTheoryOpKind::PrimeDecomposition {
            expr_str,
            domain_hint,
        });
    }
    if let Some(body) = input
        .strip_prefix("is_prime ")
        .or_else(|| input.strip_prefix("prime "))
    {
        let trimmed = body.trim();
        if let Ok(n) = trimmed.parse::<u64>() {
            return MathOperation::NumberTheory(NumberTheoryOpKind::IsPrime { n });
        }
        return MathOperation::NumberTheory(NumberTheoryOpKind::IsPrimeExpr {
            expr_str: trimmed.to_string(),
        });
    }
    if let Some(body) = input
        .strip_prefix("gcd ")
        .or_else(|| input.strip_prefix("egcd "))
        .or_else(|| input.strip_prefix("extended_gcd "))
    {
        let parts: Vec<&str> = if body.contains(',') {
            split_top_level_commas(body)
        } else {
            body.split_whitespace().collect()
        };
        if parts.len() >= 2 {
            if let (Ok(a), Ok(b)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                return MathOperation::NumberTheory(NumberTheoryOpKind::ExtendedGcd { a, b });
            }
            return MathOperation::NumberTheory(NumberTheoryOpKind::ExtendedGcdExpr {
                a_str: parts[0].to_string(),
                b_str: parts[1].to_string(),
            });
        }
    }
    if let Some(body) = input.strip_prefix("lcm ") {
        let parts: Vec<&str> = if body.contains(',') {
            split_top_level_commas(body)
        } else {
            body.split_whitespace().collect()
        };
        if parts.len() >= 2 {
            return MathOperation::NumberTheory(NumberTheoryOpKind::LcmExpr {
                a_str: parts[0].to_string(),
                b_str: parts[1].to_string(),
            });
        }
    }
    if let Some(body) = input.strip_prefix("compare ") {
        let parts = split_top_level_commas(body);
        if parts.len() == 2 {
            return MathOperation::Compare {
                left: parts[0].trim().to_string(),
                right: parts[1].trim().to_string(),
                kind: ComparisonKind::Equivalent,
            };
        }
    }
    if let Some(body) = input
        .strip_prefix("solve_ode ")
        .or_else(|| input.strip_prefix("dsolve "))
        .or_else(|| input.strip_prefix("ode "))
    {
        let parts = split_top_level_commas(body);
        if !parts.is_empty() && !parts[0].trim().is_empty() {
            return MathOperation::DiffEq(DiffEqOpKind::ClassifyAndSolve {
                equation: parts[0].trim().to_string(),
                dependent_var: parts
                    .get(1)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                independent_vars: parts
                    .get(2)
                    .map(|s| vec![s.trim().to_string()])
                    .filter(|v| !v[0].is_empty()),
                override_kind: Some("ode".to_string()),
            });
        }
    }
    if let Some(body) = input.strip_prefix("legendre ") {
        let parts: Vec<&str> = if body.contains(',') {
            body.split(',').map(|s| s.trim()).collect()
        } else {
            body.split_whitespace().collect()
        };
        if parts.len() >= 2
            && let (Ok(a), Ok(p)) = (parts[0].parse::<i64>(), parts[1].parse::<u64>())
        {
            return MathOperation::NumberTheory(NumberTheoryOpKind::Legendre { a, p });
        }
    }

    // 8. Control & StatMech & Galois
    if let Some(body) = input.strip_prefix("stability 2x2 ") {
        let parts: Vec<f64> = body
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if parts.len() == 4 {
            return MathOperation::Control(ControlOpKind::Stability2x2 {
                a11: parts[0],
                a12: parts[1],
                a21: parts[2],
                a22: parts[3],
            });
        }
    }
    if let Some(body) = input.strip_prefix("fermi_dirac ") {
        let parts: Vec<f64> = body
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if parts.len() >= 3 {
            return MathOperation::StatMech(StatMechOpKind::FermiDirac {
                energy: parts[0],
                chemical_potential: parts[1],
                beta: parts[2],
            });
        }
    }
    if let Some(body) = input.strip_prefix("bose_einstein ") {
        let parts: Vec<f64> = body
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if parts.len() >= 3 {
            return MathOperation::StatMech(StatMechOpKind::BoseEinstein {
                energy: parts[0],
                chemical_potential: parts[1],
                beta: parts[2],
            });
        }
    }
    if let Some(body) = input.strip_prefix("galois ") {
        let trimmed = body.trim();
        if let Ok(d) = trimmed.parse::<i64>() {
            return MathOperation::Galois(GaloisOpKind::QuadraticExtension { d });
        }
        return MathOperation::Galois(GaloisOpKind::CheckSolvability {
            group_name: trimmed.to_string(),
        });
    }
    if let Some(body) = input.strip_prefix("galois_solvable ") {
        return MathOperation::Galois(GaloisOpKind::CheckSolvability {
            group_name: body.trim().to_string(),
        });
    }

    // 9. Algebraic Transformations
    if let Some(body) = input.strip_prefix("expand ") {
        return MathOperation::Simplify {
            expression: body.trim().to_string(),
            strategy: SimplifyStrategy::Expand,
        };
    }
    if let Some(body) = input.strip_prefix("factor ") {
        return MathOperation::Simplify {
            expression: body.trim().to_string(),
            strategy: SimplifyStrategy::Factor,
        };
    }
    if let Some(body) = input.strip_prefix("together ") {
        return MathOperation::Simplify {
            expression: body.trim().to_string(),
            strategy: SimplifyStrategy::Together,
        };
    }
    if let Some(body) = input.strip_prefix("cancel ") {
        return MathOperation::Simplify {
            expression: body.trim().to_string(),
            strategy: SimplifyStrategy::Cancel,
        };
    }
    if let Some(body) = input
        .strip_prefix("min_tree ")
        .or_else(|| input.strip_prefix("min_nodes "))
    {
        return MathOperation::Simplify {
            expression: body.trim().to_string(),
            strategy: SimplifyStrategy::MinTree,
        };
    }
    if let Some(body) = input.strip_prefix("min_ops ") {
        return MathOperation::Simplify {
            expression: body.trim().to_string(),
            strategy: SimplifyStrategy::MinOps,
        };
    }
    if let Some(body) = input.strip_prefix("min_leaf ") {
        return MathOperation::Simplify {
            expression: body.trim().to_string(),
            strategy: SimplifyStrategy::MinLeaf,
        };
    }
    if let Some(body) = input.strip_prefix("horner ") {
        let (expr, var) = body
            .split_once(',')
            .map(|(e, v)| (e.trim(), Some(v.trim().to_string())))
            .unwrap_or((body.trim(), None));
        return MathOperation::Simplify {
            expression: expr.to_string(),
            strategy: SimplifyStrategy::Horner { var },
        };
    }
    if let Some(body) = input.strip_prefix("partial_fractions ") {
        let (expr, var) = body
            .split_once(',')
            .map(|(e, v)| (e.trim(), Some(v.trim().to_string())))
            .unwrap_or((body.trim(), None));
        return MathOperation::Simplify {
            expression: expr.to_string(),
            strategy: SimplifyStrategy::PartialFractions { var },
        };
    }
    if let Some(body) = input.strip_prefix("collect ") {
        let (expr, var) = body
            .split_once(',')
            .map(|(e, v)| (e.trim(), v.trim()))
            .unwrap_or((body.trim(), "x"));
        return MathOperation::Simplify {
            expression: expr.to_string(),
            strategy: SimplifyStrategy::Collect {
                var: var.to_string(),
            },
        };
    }

    // 9b. Polynomial Roots Command
    if let Some(body) = input.strip_prefix("roots ") {
        return MathOperation::Polynomial(PolynomialOpKind::Roots {
            poly_str: body.trim().to_string(),
        });
    }

    // 9c. Partial Differential Equations (Wave, Heat, Laplace, Transport, Bessel)
    if let Some(body) = input.strip_prefix("pde ") {
        let trimmed_body = body.trim();
        if let Some(wave_args) = trimmed_body.strip_prefix("wave ") {
            let parts: Vec<&str> = if wave_args.contains(',') {
                wave_args.split(',').map(|s| s.trim()).collect()
            } else {
                wave_args.split_whitespace().collect()
            };
            let speed = parts.first().copied().unwrap_or("c").to_string();
            let x_var = parts.get(1).copied().unwrap_or("x").to_string();
            let t_var = parts.get(2).copied().unwrap_or("t").to_string();
            let initial_pos = parts.get(3).copied().map(|s| s.to_string());
            let initial_vel = parts.get(4).copied().map(|s| s.to_string());
            return MathOperation::Pde(PdeOpKind::Wave1D {
                speed,
                x_var,
                t_var,
                initial_pos,
                initial_vel,
            });
        }
        if let Some(heat_args) = trimmed_body.strip_prefix("heat ") {
            let parts: Vec<&str> = if heat_args.contains(',') {
                heat_args.split(',').map(|s| s.trim()).collect()
            } else {
                heat_args.split_whitespace().collect()
            };
            let alpha = parts.first().copied().unwrap_or("alpha").to_string();
            let x_var = parts.get(1).copied().unwrap_or("x").to_string();
            let t_var = parts.get(2).copied().unwrap_or("t").to_string();
            let length = parts.get(3).copied().map(|s| s.to_string());
            return MathOperation::Pde(PdeOpKind::Heat1D {
                alpha,
                x_var,
                t_var,
                length,
            });
        }
        if let Some(laplace_args) = trimmed_body.strip_prefix("laplace ") {
            let parts: Vec<&str> = if laplace_args.contains(',') {
                laplace_args.split(',').map(|s| s.trim()).collect()
            } else {
                laplace_args.split_whitespace().collect()
            };
            let x_var = parts.first().copied().unwrap_or("x").to_string();
            let y_var = parts.get(1).copied().unwrap_or("y").to_string();
            let a_bound = parts.get(2).copied().map(|s| s.to_string());
            let b_bound = parts.get(3).copied().map(|s| s.to_string());
            return MathOperation::Pde(PdeOpKind::Laplace2D {
                x_var,
                y_var,
                a_bound,
                b_bound,
            });
        }
        if let Some(trans_args) = trimmed_body
            .strip_prefix("transport ")
            .or_else(|| trimmed_body.strip_prefix("advection "))
        {
            let parts: Vec<&str> = if trans_args.contains(',') {
                trans_args.split(',').map(|s| s.trim()).collect()
            } else {
                trans_args.split_whitespace().collect()
            };
            let speed = parts.first().copied().unwrap_or("c").to_string();
            let x_var = parts.get(1).copied().unwrap_or("x").to_string();
            let t_var = parts.get(2).copied().unwrap_or("t").to_string();
            let initial_state = parts.get(3).copied().map(|s| s.to_string());
            return MathOperation::Pde(PdeOpKind::Transport1D {
                speed,
                x_var,
                t_var,
                initial_state,
            });
        }
        if let Some(bessel_args) = trimmed_body.strip_prefix("bessel ") {
            let parts: Vec<&str> = if bessel_args.contains(',') {
                bessel_args.split(',').map(|s| s.trim()).collect()
            } else {
                bessel_args.split_whitespace().collect()
            };
            let wave_num = parts.first().copied().unwrap_or("k").to_string();
            let r_var = parts.get(1).copied().unwrap_or("r").to_string();
            let order = parts
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            return MathOperation::Pde(PdeOpKind::RadialBessel {
                wave_num,
                r_var,
                order,
            });
        }
    }

    // 10. Evaluation: `eval` defaults to exact rational/big-integer first;
    // `evalf`, `N(...)`, `approx` request floating-point approximation.
    if let Some(body) = input.strip_prefix("eval ") {
        return MathOperation::Evaluate {
            expression: body.trim().to_string(),
            precision_digits: None,
            bindings: HashMap::new(),
        };
    }
    // The number of digits may follow after a top-level comma: `N(pi, 50)`, `evalf sqrt(2), 100`.
    let split_digits = |body: &str| -> (String, usize) {
        let parts = split_top_level_commas(body);
        if parts.len() >= 2
            && let Ok(d) = parts[parts.len() - 1].trim().parse::<usize>()
        {
            let expression = parts[..parts.len() - 1].join(",");
            return (expression.trim().to_string(), d);
        }
        (body.trim().to_string(), 15)
    };
    if let Some(body) = input
        .strip_prefix("evalf ")
        .or_else(|| input.strip_prefix("N "))
        .or_else(|| input.strip_prefix("approx "))
        .or_else(|| input.strip_prefix("float "))
    {
        let (expression, digits) = split_digits(body);
        return MathOperation::Evaluate {
            expression,
            precision_digits: Some(digits),
            bindings: HashMap::new(),
        };
    }
    if (input.starts_with("N(")
        || input.starts_with("evalf(")
        || input.starts_with("approx(")
        || input.starts_with("float("))
        && input.ends_with(')')
        && let Some(open) = input.find('(')
    {
        let inner = &input[open + 1..input.len() - 1];
        let (expression, digits) = split_digits(inner);
        return MathOperation::Evaluate {
            expression,
            precision_digits: Some(digits),
            bindings: HashMap::new(),
        };
    }

    // 11. Symbol Declarations (Notebook style e.g. `a: Parameter = 2.0 [m]`, `x: Variable`, `a = 5.0 [kg]`)
    if input.contains(':')
        && (input.contains("Variable") || input.contains("Parameter") || input.contains("Constant"))
        && let Some((name_part, rest)) = input.split_once(':')
    {
        let name = name_part.trim().to_string();
        let role = if rest.contains("Parameter") {
            SymbolRoleKind::Parameter
        } else if rest.contains("Constant") {
            SymbolRoleKind::Constant
        } else {
            SymbolRoleKind::Variable
        };

        let mut value = None;
        let mut uncertainty = None;
        let mut unit = None;
        if let Some((_, val_part)) = rest.split_once('=') {
            if let Some((v, err_opt, u_opt)) = parse_value_uncertainty_and_unit(val_part) {
                value = Some(v);
                uncertainty = err_opt;
                unit = u_opt;
            } else {
                let val_str = val_part.trim();
                let (num_str, unit_str) = if let Some(bracket_idx) = val_str.find('[') {
                    let u = val_str[bracket_idx..]
                        .trim_matches(|c| c == '[' || c == ']')
                        .trim();
                    (&val_str[..bracket_idx], Some(u.to_string()))
                } else {
                    (val_str, None)
                };
                value = num_str.trim().parse::<f64>().ok();
                unit = unit_str;
            }
        }
        if let (Some(v), Some(u)) = (value, uncertainty) {
            return MathOperation::UncertainDeclaration {
                name,
                role,
                value: v,
                uncertainty: u,
                unit,
            };
        }
        return MathOperation::Declaration {
            name,
            role,
            value,
            unit,
        };
    }

    // 11b. Bare uncertain parameter assignment: e.g. `a = 5.0 +/- 0.1 [m]`, `m = 2.5 ± 0.05`
    if !input.starts_with("solve ")
        && !input.starts_with("eval ")
        && (input.contains("+/-")
            || input.contains('±')
            || input.contains("+-")
            || input.contains("\\pm"))
        && let Some((lhs, rhs)) = input.split_once('=')
    {
        let sym_name = lhs.trim();
        let rhs_norm = rhs.replace("\\pm", "±");
        if !sym_name.is_empty()
            && !sym_name.contains(':')
            && sym_name.chars().all(|c| c.is_alphanumeric() || c == '_')
            && sym_name
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
            && let Some((v, Some(u), unit)) = parse_value_uncertainty_and_unit(&rhs_norm)
        {
            return MathOperation::UncertainDeclaration {
                name: sym_name.to_string(),
                role: SymbolRoleKind::Parameter,
                value: v,
                uncertainty: u,
                unit,
            };
        }
    }

    // 11c. Multi-valued Branch Cut & Riemann Sheet Commands
    // Supports: `branch_cut("sqrt", sheet = 1, cut = "(-infinity, 0]")`,
    // `branches <expr>`, `branch <expr> [, sheet=<k>] [, cut=<...>]`
    if let Some(inner) = input
        .strip_prefix("branch_cut(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts = split_top_level_commas(inner);
        let function = parts
            .first()
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .unwrap_or_else(|| "sqrt".to_string());
        let mut sheet: i64 = 0;
        let mut cut: Option<String> = None;
        let mut expr: Option<String> = None;
        for part in parts.iter().skip(1) {
            if let Some((k, v)) = part.split_once('=') {
                let key = k.trim().to_ascii_lowercase();
                let val = v.trim().trim_matches('"').trim_matches('\'');
                match key.as_str() {
                    "sheet" | "k" => {
                        if let Ok(parsed_k) = val.parse::<i64>() {
                            sheet = parsed_k;
                        }
                    }
                    "cut" => cut = Some(val.to_string()),
                    "expr" | "z" | "at" => expr = Some(val.to_string()),
                    _ => {}
                }
            } else if expr.is_none() {
                expr = Some(part.trim().to_string());
            }
        }
        return MathOperation::Branch(BranchOpKind::EvaluateSheet {
            function,
            expression: expr,
            sheet,
            cut,
        });
    }
    if let Some(body) = input.strip_prefix("branches ") {
        let trimmed = body.trim();
        let (expression, max_branches) = if let Some((e, n_str)) = trimmed.rsplit_once(',') {
            if let Ok(n) = n_str.trim().parse::<usize>() {
                (e.trim().to_string(), n.clamp(1, 32))
            } else {
                (trimmed.to_string(), 6)
            }
        } else {
            (trimmed.to_string(), 6)
        };
        return MathOperation::Branch(BranchOpKind::AllBranches {
            expression,
            max_branches,
        });
    }
    if let Some(body) = input.strip_prefix("branch ") {
        let parts = split_top_level_commas(body);
        let expr_str = parts.first().copied().unwrap_or("").trim().to_string();
        let mut sheet: i64 = 0;
        let mut cut: Option<String> = None;
        for part in parts.iter().skip(1) {
            if let Some((k, v)) = part.split_once('=') {
                let key = k.trim().to_ascii_lowercase();
                let val = v.trim().trim_matches('"').trim_matches('\'');
                if (key == "sheet" || key == "k")
                    && let Ok(parsed_k) = val.parse::<i64>()
                {
                    sheet = parsed_k;
                } else if key == "cut" {
                    cut = Some(val.to_string());
                }
            } else if let Ok(parsed_k) = part.trim().parse::<i64>() {
                sheet = parsed_k;
            }
        }
        return MathOperation::Branch(BranchOpKind::EvaluateSheet {
            function: "auto".to_string(),
            expression: Some(expr_str),
            sheet,
            cut,
        });
    }

    // 12. Set-Builder Domain Declarations e.g. `{ x in Reals | -5 <= x <= 5 }` or `x in Reals | x >= 0`
    if (input.starts_with('{') && input.ends_with('}') && input.contains('|'))
        || (input.contains(" in ")
            && input.contains('|')
            && !input.contains("diff ")
            && !input.contains("integrate "))
    {
        let inner = input.trim_matches(|c| c == '{' || c == '}').trim();
        if let Some((left, right)) = inner.split_once('|') {
            let var_and_dom = left.trim();
            let cond = right.trim().to_string();
            let (v_name, d_type) = if let Some((v, d)) = var_and_dom.split_once(" in ") {
                (v.trim().to_string(), d.trim().to_string())
            } else if let Some((v, d)) = var_and_dom.split_once(" ∈ ") {
                (v.trim().to_string(), d.trim().to_string())
            } else {
                ("x".to_string(), "Reals".to_string())
            };
            return MathOperation::SetBuilder {
                var_name: v_name,
                domain_type: d_type,
                condition: cond,
            };
        }
    }

    // 12b. Domain Bound Restrictions e.g. `a in Reals [0, 100]`, `x in Positive`, `a in [-5, 5]`
    if let Some(bound) = parse_domain_declaration(input) {
        return MathOperation::DomainRestriction(bound);
    }

    // 13. Linear Algebra & Matrix Commands
    if let Some(body) = strip_matched_call(input, &["det"]) {
        return MathOperation::Matrix(MatrixOpKind::Determinant {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = strip_matched_call(input, &["inv"]) {
        return MathOperation::Matrix(MatrixOpKind::Inverse {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = strip_matched_call(input, &["trace", "tr"]) {
        return MathOperation::Matrix(MatrixOpKind::Trace {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = strip_matched_call(input, &["transpose"]) {
        return MathOperation::Matrix(MatrixOpKind::Transpose {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = strip_matched_call(input, &["rref"]) {
        return MathOperation::Matrix(MatrixOpKind::Rref {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = strip_matched_call(input, &["rank"]) {
        return MathOperation::Matrix(MatrixOpKind::Rank {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = strip_matched_call(input, &["nullspace", "kernel"]) {
        return MathOperation::Matrix(MatrixOpKind::Nullspace {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = strip_matched_call(input, &["eigenvalues", "eigvals"]) {
        return MathOperation::Matrix(MatrixOpKind::Eigenvalues {
            matrix_str: body.to_string(),
        });
    }
    if let Some(body) = input.strip_prefix("charpoly ") {
        let parts = split_top_level_commas(body);
        let (m_str, var) = if parts.len() >= 2 && !parts[1].is_empty() {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (body.trim().to_string(), "lambda".to_string())
        };
        return MathOperation::Matrix(MatrixOpKind::CharPoly {
            matrix_str: m_str,
            var,
        });
    }
    if let Some(body) = input
        .strip_prefix("matmul ")
        .or_else(|| input.strip_prefix("matrix_mul "))
        .or_else(|| input.strip_prefix("mat_mul "))
    {
        let parts = split_top_level_commas(body);
        if parts.len() >= 2 {
            return MathOperation::Matrix(MatrixOpKind::Multiply {
                a_str: parts[0].to_string(),
                b_str: parts[1].to_string(),
            });
        }
    }

    // 4.4 Integer functions: `numfn totient 36`, `numfn pow_mod 2, 10, 1000`
    if let Some(body) = input.strip_prefix("numfn ")
        && let Some((function, args)) = body.trim().split_once(' ')
    {
        let parsed: Option<Vec<i64>> = args
            .split(',')
            .map(|a| a.trim().parse::<i64>().ok())
            .collect();
        if let Some(args) = parsed {
            return MathOperation::NumberTheory(NumberTheoryOpKind::Arithmetic {
                function: function.to_string(),
                args,
            });
        }
    }

    // 14b. Numerical and statistical commands: `numcmd nsolve cos(x) - x, 1`
    if let Some(body) = input.strip_prefix("numcmd ")
        && let Some((function, args)) = body.trim().split_once(' ')
    {
        return MathOperation::Numeric {
            function: function.to_string(),
            args: split_top_level_commas(args)
                .into_iter()
                .map(|a| a.trim().to_string())
                .collect(),
        };
    }

    // 15. Discrete Combinatorics Commands
    if let Some(body) = input.strip_prefix("factorial ")
        && let Ok(n) = body.trim().parse::<u64>()
    {
        return MathOperation::Combinatorics(CombinatoricsOpKind::Factorial { n });
    }
    if let Some(body) = input
        .strip_prefix("combinations ")
        .or_else(|| input.strip_prefix("choose "))
        .or_else(|| input.strip_prefix("binom "))
    {
        let parts: Vec<&str> = if body.contains(',') {
            body.split(',').map(|s| s.trim()).collect()
        } else {
            body.split_whitespace().collect()
        };
        if parts.len() >= 2
            && let (Ok(n), Ok(k)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
        {
            return MathOperation::Combinatorics(CombinatoricsOpKind::Combinations { n, k });
        }
    }
    if let Some(body) = input
        .strip_prefix("permutations ")
        .or_else(|| input.strip_prefix("npr "))
    {
        let parts: Vec<&str> = if body.contains(',') {
            body.split(',').map(|s| s.trim()).collect()
        } else {
            body.split_whitespace().collect()
        };
        if parts.len() >= 2
            && let (Ok(n), Ok(k)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
        {
            return MathOperation::Combinatorics(CombinatoricsOpKind::Permutations { n, k });
        }
    }
    if let Some(body) = input.strip_prefix("derangements ")
        && let Ok(n) = body.trim().parse::<u64>()
    {
        return MathOperation::Combinatorics(CombinatoricsOpKind::Derangements { n });
    }
    if let Some(body) = input.strip_prefix("catalan ")
        && let Ok(n) = body.trim().parse::<u64>()
    {
        return MathOperation::Combinatorics(CombinatoricsOpKind::Catalan { n });
    }
    if let Some(body) = input.strip_prefix("bell ")
        && let Ok(n) = body.trim().parse::<u64>()
    {
        return MathOperation::Combinatorics(CombinatoricsOpKind::Bell { n });
    }
    if let Some(body) = input.strip_prefix("stirling2 ") {
        let parts: Vec<&str> = if body.contains(',') {
            body.split(',').map(|s| s.trim()).collect()
        } else {
            body.split_whitespace().collect()
        };
        if parts.len() >= 2
            && let (Ok(n), Ok(k)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
        {
            return MathOperation::Combinatorics(CombinatoricsOpKind::Stirling2 { n, k });
        }
    }
    if let Some(body) = input.strip_prefix("partitions ")
        && let Ok(n) = body.trim().parse::<u64>()
    {
        return MathOperation::Combinatorics(CombinatoricsOpKind::Partitions { n });
    }

    // 16. Integral Transforms
    if let Some(body) = input.strip_prefix("laplace ") {
        let parts: Vec<&str> = body.split(',').map(|s| s.trim()).collect();
        let expr = parts[0].to_string();
        let t_var = if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            "t".to_string()
        };
        let s_var = if parts.len() >= 3 {
            parts[2].to_string()
        } else {
            "s".to_string()
        };
        return MathOperation::Transform(TransformOpKind::Laplace {
            expression: expr,
            time_var: t_var,
            freq_var: s_var,
        });
    }
    if let Some(body) = input
        .strip_prefix("inv_laplace ")
        .or_else(|| input.strip_prefix("ilt "))
    {
        let parts: Vec<&str> = body.split(',').map(|s| s.trim()).collect();
        let expr = parts[0].to_string();
        let s_var = if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            "s".to_string()
        };
        let t_var = if parts.len() >= 3 {
            parts[2].to_string()
        } else {
            "t".to_string()
        };
        return MathOperation::Transform(TransformOpKind::InverseLaplace {
            expression: expr,
            freq_var: s_var,
            time_var: t_var,
        });
    }
    if let Some(body) = input.strip_prefix("fourier ") {
        let parts: Vec<&str> = body.split(',').map(|s| s.trim()).collect();
        let expr = parts[0].to_string();
        let t_var = if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            "t".to_string()
        };
        let w_var = if parts.len() >= 3 {
            parts[2].to_string()
        } else {
            "w".to_string()
        };
        return MathOperation::Transform(TransformOpKind::Fourier {
            expression: expr,
            time_var: t_var,
            freq_var: w_var,
        });
    }

    // 17. Polynomial Operations
    if let Some(body) = input
        .strip_prefix("poly_roots ")
        .or_else(|| input.strip_prefix("roots "))
    {
        return MathOperation::Polynomial(PolynomialOpKind::Roots {
            poly_str: body.trim().to_string(),
        });
    }
    if let Some(body) = input.strip_prefix("discriminant ") {
        let (p_str, var) = body
            .split_once(',')
            .map(|(p, v)| (p.trim().to_string(), v.trim().to_string()))
            .unwrap_or((body.trim().to_string(), "x".to_string()));
        return MathOperation::Polynomial(PolynomialOpKind::Discriminant {
            poly_str: p_str,
            var,
        });
    }
    if let Some(body) = input.strip_prefix("resultant ") {
        let parts: Vec<&str> = body.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 2 {
            let var = if parts.len() >= 3 {
                parts[2].to_string()
            } else {
                "x".to_string()
            };
            return MathOperation::Polynomial(PolynomialOpKind::Resultant {
                poly1_str: parts[0].to_string(),
                poly2_str: parts[1].to_string(),
                var,
            });
        }
    }
    if let Some(body) = input.strip_prefix("poly_gcd ")
        && let Some((p1, p2)) = body.split_once(',')
    {
        return MathOperation::Polynomial(PolynomialOpKind::Gcd {
            poly1_str: p1.trim().to_string(),
            poly2_str: p2.trim().to_string(),
        });
    }

    // 18. Physics & Dimensions
    if let Some(body) = input.strip_prefix("convert ")
        && let Some((from_part, to_part)) = body.split_once(" to ")
    {
        let from_trimmed = from_part.trim();
        let to_unit = to_part
            .trim()
            .trim_matches(|c| c == '[' || c == ']')
            .to_string();
        let (num_str, from_unit) = if let Some(bracket_idx) = from_trimmed.find('[') {
            let u = from_trimmed[bracket_idx..]
                .trim_matches(|c| c == '[' || c == ']')
                .trim();
            (&from_trimmed[..bracket_idx], u.to_string())
        } else {
            let parts: Vec<&str> = from_trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                (parts[0], parts[1].to_string())
            } else {
                (from_trimmed, "m".to_string())
            }
        };
        if let Ok(val) = num_str.trim().parse::<f64>() {
            return MathOperation::Physics(PhysicsOpKind::Convert {
                val,
                from_unit,
                to_unit,
            });
        }
    }

    // 19. Logic & SAT
    if let Some(body) = input.strip_prefix("sat ") {
        return MathOperation::Logic(LogicOpKind::SatSolve {
            formula: body.trim().to_string(),
        });
    }
    if let Some(body) = input.strip_prefix("truth_table ") {
        return MathOperation::Logic(LogicOpKind::TruthTable {
            formula: body.trim().to_string(),
        });
    }

    // 20. Direct Calculus commands
    if let Some(diff_body) = input.strip_prefix("diff ").or_else(|| {
        if input.starts_with("diff(") && input.ends_with(')') {
            Some(&input[5..input.len() - 1])
        } else {
            None
        }
    }) {
        let parts: Vec<&str> = split_top_level_commas(diff_body)
            .into_iter()
            .map(|s| s.trim())
            .collect();
        let expr = parts[0].to_string();
        let var = if parts.len() >= 2 && !parts[1].is_empty() {
            Some(parts[1].to_string())
        } else {
            None
        };
        let order = if parts.len() >= 3 {
            parts[2].parse::<usize>().unwrap_or(1)
        } else {
            1
        };
        return MathOperation::Differentiate {
            expression: expr,
            variable: var,
            order,
            is_total: false,
        };
    }
    if let Some(int_body) = input.strip_prefix("integrate ").or_else(|| {
        if input.starts_with("integrate(") && input.ends_with(')') {
            Some(&input[10..input.len() - 1])
        } else {
            None
        }
    }) {
        if let Some((expr_part, bounds_part)) = int_body.split_once(" from ")
            && let Some((lower_p, upper_p)) = bounds_part.split_once(" to ")
        {
            let expr_parts = split_top_level_commas(expr_part);
            let (expr, mut var) = if expr_parts.len() >= 2 {
                (
                    expr_parts[0].trim().to_string(),
                    Some(expr_parts[1].trim().to_string()),
                )
            } else {
                // `integrate x^2 dx from 0 to 1`: the differential names the variable.
                split_expr_and_var(expr_part.trim())
            };
            let upper_parts = split_top_level_commas(upper_p);
            let upper = if var.is_none() && upper_parts.len() >= 2 {
                let v_tr = upper_parts[1].trim();
                if !v_tr.is_empty() {
                    var = Some(v_tr.to_string());
                }
                upper_parts[0].trim().to_string()
            } else {
                upper_p.trim().to_string()
            };
            return MathOperation::Integrate {
                expression: expr,
                variable: var,
                lower: Some(lower_p.trim().to_string()),
                upper: Some(upper),
            };
        }
        let parts: Vec<&str> = split_top_level_commas(int_body)
            .into_iter()
            .map(|s| s.trim())
            .collect();
        let expr = parts[0].to_string();
        let var = if parts.len() >= 2 && !parts[1].is_empty() {
            Some(parts[1].to_string())
        } else {
            None
        };
        let lower = if parts.len() >= 3 && !parts[2].is_empty() {
            Some(parts[2].to_string())
        } else {
            None
        };
        let upper = if parts.len() >= 4 && !parts[3].is_empty() {
            Some(parts[3].to_string())
        } else {
            None
        };
        return MathOperation::Integrate {
            expression: expr,
            variable: var,
            lower,
            upper,
        };
    }
    if let Some(solve_body) = strip_matched_call(input, &["solve", "solve_system", "linsolve"]) {
        let parts: Vec<&str> = split_top_level_commas(solve_body)
            .into_iter()
            .map(|s| s.trim())
            .collect();
        if let Some(first) = parts.first()
            && let Some(eq_list) = parse_bracketed_list(first)
        {
            let variables = if parts.len() >= 2 {
                if let Some(var_list) = parse_bracketed_list(parts[1]) {
                    var_list
                } else {
                    parts[1..]
                        .iter()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            } else {
                Vec::new()
            };
            return MathOperation::SolveSystem {
                equations: eq_list,
                variables,
            };
        }
        let (eq, var) = if parts.len() >= 2 && !parts[1].is_empty() {
            (parts[0], Some(parts[1].to_string()))
        } else {
            (solve_body.trim(), None)
        };
        return MathOperation::Solve {
            equation: eq.to_string(),
            variable: var,
        };
    }
    if let Some(simp_body) = input.strip_prefix("simplify ").or_else(|| {
        if input.starts_with("simplify(") && input.ends_with(')') {
            Some(&input[9..input.len() - 1])
        } else {
            None
        }
    }) {
        return MathOperation::Simplify {
            expression: simp_body.trim().to_string(),
            strategy: SimplifyStrategy::Standard,
        };
    }

    // 21. Series & Taylor commands
    if let Some(body) = input
        .strip_prefix("series ")
        .or_else(|| input.strip_prefix("taylor "))
    {
        let parts: Vec<&str> = split_top_level_commas(body)
            .into_iter()
            .map(|s| s.trim())
            .collect();
        let expr = parts[0].to_string();
        let var = if parts.len() >= 2 && !parts[1].is_empty() {
            parts[1].to_string()
        } else {
            "x".to_string()
        };
        let pt = if parts.len() >= 3 && !parts[2].is_empty() {
            parts[2].to_string()
        } else {
            "0".to_string()
        };
        let order = if parts.len() >= 4 {
            parts[3].parse::<usize>().unwrap_or(5)
        } else {
            5
        };
        return MathOperation::Series {
            expression: expr,
            variable: var,
            point: pt,
            order,
        };
    }

    // 22. Bridge PermissiveIntent Natural Language Phrasings
    if let Some(intent) = parse_permissive_intent(input) {
        match intent {
            PermissiveIntent::Differentiate {
                expression,
                variable,
                is_total,
            } => {
                return MathOperation::Differentiate {
                    expression,
                    variable,
                    order: 1,
                    is_total,
                };
            }
            PermissiveIntent::Integrate {
                expression,
                variable,
                lower,
                upper,
            } => {
                return MathOperation::Integrate {
                    expression,
                    variable,
                    lower,
                    upper,
                };
            }
            PermissiveIntent::Solve { equation, variable } => {
                return MathOperation::Solve {
                    equation,
                    variable: Some(variable),
                };
            }
            PermissiveIntent::Simplify { expression } => {
                return MathOperation::Simplify {
                    expression,
                    strategy: SimplifyStrategy::Standard,
                };
            }
            PermissiveIntent::Substitute {
                expression,
                variable,
                value,
            } => {
                return MathOperation::Substitute {
                    expression,
                    variable,
                    value,
                };
            }
            PermissiveIntent::Limit {
                expression,
                variable,
                point,
                direction,
            } => {
                return MathOperation::Limit {
                    expression,
                    variable,
                    point,
                    direction,
                };
            }
            PermissiveIntent::Sum {
                expression,
                variable,
                lower,
                upper,
            } => {
                return MathOperation::Sum {
                    expression,
                    variable,
                    lower,
                    upper,
                };
            }
            PermissiveIntent::Distribution {
                name,
                dist_type,
                params,
            } => {
                return MathOperation::Distribution {
                    name,
                    dist_type,
                    params,
                };
            }
            PermissiveIntent::Compare {
                left,
                right,
                comparison_type,
            } => {
                let kind = if comparison_type == "isomorphic" {
                    ComparisonKind::Isomorphic
                } else {
                    ComparisonKind::Equivalent
                };
                return MathOperation::Compare { left, right, kind };
            }
            _ => {}
        }
    }

    // Fallback: Default Expression/Equation
    MathOperation::Expression {
        raw: input.to_string(),
    }
}

#[cfg(test)]
mod bracket_matrix_syntax_tests {
    use super::*;
    use crate::ExprKind;

    #[test]
    fn test_semicolon_matrix_and_elementwise_syntax() {
        let graph = ExprGraph::new();
        let parser = ExprParser::new(&graph);

        let mat = parser.parse("[1 2; 3 4]").expect("parse matrix");
        assert!(matches!(graph.get(mat).kind, ExprKind::Function { .. }));

        let mldiv = parser.parse("A \\ b").expect("parse backslash");
        if let ExprKind::Function { name, args } = &graph.get(mldiv).kind {
            assert_eq!(graph.symbols.resolve(*name).as_deref(), Some("mldivide"));
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected mldivide");
        }

        let elem_mul = parser.parse("x .* y").expect("parse elem_mul");
        if let ExprKind::Function { name, .. } = &graph.get(elem_mul).kind {
            assert_eq!(graph.symbols.resolve(*name).as_deref(), Some("elem_mul"));
        } else {
            panic!("expected elem_mul");
        }

        let elem_pow = parser.parse("x .^ 2").expect("parse elem_pow");
        if let ExprKind::Function { name, .. } = &graph.get(elem_pow).kind {
            assert_eq!(graph.symbols.resolve(*name).as_deref(), Some("elem_pow"));
        } else {
            panic!("expected elem_pow");
        }

        let held = parser.parse("hold(x + 0)").expect("parse hold");
        assert!(matches!(graph.get(held).kind, ExprKind::Function { .. }));
    }
}
