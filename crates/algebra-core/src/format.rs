//! Formatting and pretty-printing engine for mathematical expressions (LaTeX, Unicode, MathML).

use crate::{Constant, ExprGraph, ExprId, ExprKind, Number, RelOp, SetOp};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Formatting error: {0}")]
    Custom(String),
}

pub type FormatResult<T> = Result<T, FormatError>;

/// Trait for formatting expressions into text representations.
pub trait Formatter {
    fn format(&self, graph: &ExprGraph, root: ExprId) -> FormatResult<String>;
}

fn resolve_symbol(graph: &ExprGraph, sym: crate::SymbolId) -> FormatResult<String> {
    graph
        .symbols
        .resolve(sym)
        .ok_or_else(|| FormatError::Custom(format!("Unresolved symbol id {:?}", sym)))
}

/// How tightly an expression's outermost operator binds, lowest first.
///
/// A child is parenthesised when its precedence is lower than its position
/// requires: `(a + b) · c`, `(x^a)^b`, `a - (b + c)`, `x^(1/2)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prec {
    /// Relations and set operations.
    Relation,
    /// Sums, differences, and operators printed with trailing text (∫, Σ, d/dx).
    Sum,
    /// A leading minus sign: unary negation or a negative literal.
    Negative,
    /// Products, quotients and fractions.
    Product,
    /// Powers.
    Power,
    /// Symbols, non-negative integers, function calls, matrices.
    Atom,
}

/// Precedence of the outermost operator of `id`, independent of output syntax.
fn precedence(graph: &ExprGraph, id: ExprId) -> Prec {
    use num_bigint::Sign;
    match &graph.get(id).kind {
        ExprKind::Number(num) => match num {
            Number::Integer(i) if *i < 0 => Prec::Negative,
            Number::BigInteger(b) if b.sign() == Sign::Minus => Prec::Negative,
            Number::Rational(n, d) if (*n < 0) != (*d < 0) => Prec::Negative,
            Number::Rational(..) => Prec::Product,
            Number::BigRational(r) if r.numer().sign() == Sign::Minus => Prec::Negative,
            Number::BigRational(_) => Prec::Product,
            Number::Scientific { mantissa, .. } if *mantissa < 0 => Prec::Negative,
            Number::Scientific { exponent, .. } if *exponent != 0 => Prec::Product,
            Number::Float(bits) if f64::from_bits(*bits).is_sign_negative() => Prec::Negative,
            Number::Constant(Constant::NegInfinity) => Prec::Negative,
            _ => Prec::Atom,
        },
        ExprKind::Symbol(_) | ExprKind::Function { .. } | ExprKind::Matrix { .. } => Prec::Atom,
        ExprKind::Add(_) | ExprKind::Sub(..) => Prec::Sum,
        ExprKind::Neg(_) => Prec::Negative,
        ExprKind::Mul(_) | ExprKind::Div(..) => Prec::Product,
        ExprKind::Pow(..) => Prec::Power,
        ExprKind::Relational { .. } | ExprKind::SetOperation { .. } => Prec::Relation,
        _ => Prec::Sum,
    }
}

/// The strings one output syntax uses for grouping and infix operators.
struct Syntax {
    open: &'static str,
    close: &'static str,
    plus: &'static str,
    minus: &'static str,
    times: &'static str,
    /// Prefix of a formatted negative term (`-`), if the syntax has one that
    /// can be turned into a subtraction.
    neg_prefix: Option<&'static str>,
}

/// Format `id`, parenthesised if it binds more loosely than `min`.
fn child<F: Formatter + ?Sized>(
    f: &F,
    syn: &Syntax,
    graph: &ExprGraph,
    id: ExprId,
    min: Prec,
) -> FormatResult<String> {
    let text = f.format(graph, id)?;
    if precedence(graph, id) < min {
        Ok(format!("{}{}{}", syn.open, text, syn.close))
    } else {
        Ok(text)
    }
}

/// `a + b - c`: terms that print with a leading minus become subtractions.
fn format_sum<F: Formatter + ?Sized>(
    f: &F,
    syn: &Syntax,
    graph: &ExprGraph,
    terms: &[ExprId],
) -> FormatResult<String> {
    let mut out = String::new();
    for (i, &t) in terms.iter().enumerate() {
        let text = child(f, syn, graph, t, Prec::Sum)?;
        if i == 0 {
            out.push_str(&text);
            continue;
        }
        match syn.neg_prefix.and_then(|neg| text.strip_prefix(neg)) {
            Some(rest) => {
                out.push_str(syn.minus);
                out.push_str(rest);
            }
            None => {
                out.push_str(syn.plus);
                out.push_str(&text);
            }
        }
    }
    Ok(out)
}

/// A deterministic display key for a factor: numbers first, then symbols and powers of symbols by name, then function
/// calls by name, then everything else in its stored order.
fn factor_key(graph: &ExprGraph, id: ExprId) -> (u8, String) {
    match &graph.get(id).kind {
        ExprKind::Number(_) => (0, String::new()),
        ExprKind::Symbol(sym) => (1, graph.symbols.resolve(*sym).unwrap_or_default()),
        ExprKind::Pow(base, _) => factor_key(graph, *base),
        ExprKind::Function { name, .. } => (3, graph.symbols.resolve(*name).unwrap_or_default()),
        _ => (4, String::new()),
    }
}

/// The order in which the factors of a product are printed. The stored order follows the order in which symbols were
/// first typed in the session, so `C1 · cos(x)` and `cos(x) · C1` could both appear for the same expression; a
/// product of commutative scalar factors is printed in a fixed order instead. Anything that may not commute
/// (matrices, tensors, Clifford elements) keeps its stored order.
fn display_order(graph: &ExprGraph, factors: &[ExprId]) -> Vec<ExprId> {
    let commutative = factors.iter().all(|&f| {
        let node = graph.get(f);
        node.domain.is_commutative_mul()
            && matches!(
                node.kind,
                ExprKind::Number(_)
                    | ExprKind::Symbol(_)
                    | ExprKind::Pow(..)
                    | ExprKind::Function { .. }
            )
    });
    let mut out = factors.to_vec();
    if commutative {
        out.sort_by_key(|&f| factor_key(graph, f));
    }
    out
}

/// `a · (b + c)`: sums are grouped; only the first factor may carry a minus.
fn format_product<F: Formatter + ?Sized>(
    f: &F,
    syn: &Syntax,
    graph: &ExprGraph,
    factors: &[ExprId],
) -> FormatResult<String> {
    let ordered = display_order(graph, factors);
    let mut parts = Vec::with_capacity(factors.len());
    for (i, &factor) in ordered.iter().enumerate() {
        let min = if i == 0 {
            Prec::Negative
        } else {
            Prec::Product
        };
        parts.push(child(f, syn, graph, factor, min)?);
    }
    Ok(parts.join(syn.times))
}

/// `a - (b + c)` and `a - (-b)`.
fn format_difference<F: Formatter + ?Sized>(
    f: &F,
    syn: &Syntax,
    graph: &ExprGraph,
    lhs: ExprId,
    rhs: ExprId,
) -> FormatResult<String> {
    let l = child(f, syn, graph, lhs, Prec::Sum)?;
    let r = child(f, syn, graph, rhs, Prec::Product)?;
    Ok(format!("{}{}{}", l, syn.minus, r))
}

const LATEX: Syntax = Syntax {
    open: "\\left(",
    close: "\\right)",
    plus: " + ",
    minus: " - ",
    times: " \\cdot ",
    neg_prefix: Some("-"),
};

const UNICODE: Syntax = Syntax {
    open: "(",
    close: ")",
    plus: " + ",
    minus: " - ",
    times: " · ",
    neg_prefix: Some("-"),
};

const MATHML: Syntax = Syntax {
    open: "<mo>(</mo>",
    close: "<mo>)</mo>",
    plus: "<mo>+</mo>",
    minus: "<mo>-</mo>",
    times: "<mo>&middot;</mo>",
    neg_prefix: None,
};

/// Formatter generating LaTeX string output.
#[derive(Debug, Clone, Copy, Default)]
pub struct LatexFormatter;

impl Formatter for LatexFormatter {
    fn format(&self, graph: &ExprGraph, root: ExprId) -> FormatResult<String> {
        let node = graph.get(root);
        match &node.kind {
            ExprKind::Number(num) => Ok(match num {
                Number::Integer(i) => i.to_string(),
                Number::BigInteger(b) => b.to_string(),
                Number::Rational(n, d) => format!("\\frac{{{}}}{{{}}}", n, d),
                Number::BigRational(r) => format!("\\frac{{{}}}{{{}}}", r.numer(), r.denom()),
                Number::Scientific { mantissa, exponent } => {
                    if *exponent == 0 {
                        mantissa.to_string()
                    } else {
                        format!("{} \\times 10^{{{}}}", mantissa, exponent)
                    }
                }
                Number::Float(bits) => f64::from_bits(*bits).to_string(),
                Number::Constant(c) => match c {
                    Constant::Pi => "\\pi".to_string(),
                    Constant::E => "e".to_string(),
                    Constant::I => "i".to_string(),
                    Constant::EulerGamma => "\\gamma".to_string(),
                    Constant::GoldenRatio => "\\phi".to_string(),
                    Constant::Apery => "\\zeta(3)".to_string(),
                    Constant::Catalan => "G".to_string(),
                    Constant::Infinity => "\\infty".to_string(),
                    Constant::NegInfinity => "-\\infty".to_string(),
                    Constant::Undefined => "\\text{Undefined}".to_string(),
                },
            }),
            ExprKind::Symbol(sym) => resolve_symbol(graph, *sym),
            ExprKind::Add(terms) => format_sum(self, &LATEX, graph, terms),
            ExprKind::Mul(factors) => format_product(self, &LATEX, graph, factors),
            ExprKind::Pow(base, exp) => {
                // The exponent sits in braces, so only the base needs grouping.
                let b_str = child(self, &LATEX, graph, *base, Prec::Atom)?;
                let e_str = self.format(graph, *exp)?;
                Ok(format!("{{{}}}^{{{}}}", b_str, e_str))
            }
            ExprKind::Div(num, den) => {
                let n_str = self.format(graph, *num)?;
                let d_str = self.format(graph, *den)?;
                Ok(format!("\\frac{{{}}}{{{}}}", n_str, d_str))
            }
            ExprKind::Sub(lhs, rhs) => format_difference(self, &LATEX, graph, *lhs, *rhs),
            ExprKind::Neg(inner) => {
                let i_str = child(self, &LATEX, graph, *inner, Prec::Product)?;
                Ok(format!("-{}", i_str))
            }
            ExprKind::Function { name, args } => {
                let fn_name = resolve_symbol(graph, *name)?;
                let arg_strs: Result<Vec<_>, _> =
                    args.iter().map(|&a| self.format(graph, a)).collect();
                let args_vec = arg_strs?;
                if (fn_name == "BringRadical" || fn_name == "BR") && args_vec.len() == 1 {
                    Ok(format!(
                        "\\operatorname{{BR}}\\left({}\\right)",
                        args_vec[0]
                    ))
                } else if fn_name == "besselj" && args_vec.len() == 2 {
                    Ok(format!(
                        "J_{{{}}}\\left({}\\right)",
                        args_vec[0], args_vec[1]
                    ))
                } else if fn_name == "uncertain" && args_vec.len() == 2 {
                    Ok(format!("{} \\pm {}", args_vec[0], args_vec[1]))
                } else if fn_name == "interval" && args_vec.len() == 2 {
                    Ok(format!("\\left[{}, {}\\right]", args_vec[0], args_vec[1]))
                } else if (fn_name == "lambert_w" || fn_name == "LambertW" || fn_name == "W")
                    && args_vec.len() == 2
                {
                    Ok(format!(
                        "W_{{{}}}\\left({}\\right)",
                        args_vec[0], args_vec[1]
                    ))
                } else {
                    Ok(format!(
                        "\\operatorname{{{}}}\\left({}\\right)",
                        fn_name,
                        args_vec.join(", ")
                    ))
                }
            }
            ExprKind::Derivative { expr, wrt, order } => {
                let e_str = self.format(graph, *expr)?;
                let wrt_str = resolve_symbol(graph, *wrt)?;
                if *order == 1 {
                    Ok(format!(
                        "\\frac{{\\mathrm{{d}}}}{{\\mathrm{{d}}{}}} \\left({}\\right)",
                        wrt_str, e_str
                    ))
                } else {
                    Ok(format!(
                        "\\frac{{\\mathrm{{d}}^{{{}}}}}{{\\mathrm{{d}}{}^{{{}}}}} \\left({}\\right)",
                        order, wrt_str, order, e_str
                    ))
                }
            }
            ExprKind::Integral {
                expr,
                wrt,
                lower,
                upper,
            } => {
                let e_str = self.format(graph, *expr)?;
                let wrt_str = resolve_symbol(graph, *wrt)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "\\int_{{{}}}^{{{}}} {} \\,\\mathrm{{d}}{}",
                            l_str, u_str, e_str, wrt_str
                        ))
                    }
                    _ => Ok(format!("\\int {} \\,\\mathrm{{d}}{}", e_str, wrt_str)),
                }
            }
            ExprKind::Matrix {
                rows,
                cols,
                elements,
            } => {
                let mut rows_str = Vec::new();
                for r in 0..*rows {
                    let mut col_strs = Vec::new();
                    for c in 0..*cols {
                        let idx = r * cols + c;
                        col_strs.push(self.format(graph, elements[idx])?);
                    }
                    rows_str.push(col_strs.join(" & "));
                }
                Ok(format!(
                    "\\begin{{pmatrix}} {} \\end{{pmatrix}}",
                    rows_str.join(" \\\\ ")
                ))
            }
            ExprKind::Relational { op, lhs, rhs } => {
                let l_str = self.format(graph, *lhs)?;
                let r_str = self.format(graph, *rhs)?;
                let op_str = match op {
                    RelOp::Equal => "=",
                    RelOp::NotEqual => "\\neq",
                    RelOp::LessThan => "<",
                    RelOp::LessEqual => "\\leq",
                    RelOp::GreaterThan => ">",
                    RelOp::GreaterEqual => "\\geq",
                };
                Ok(format!("{} {} {}", l_str, op_str, r_str))
            }
            ExprKind::SetOperation { op, args } => {
                let arg_strs: Result<Vec<_>, _> =
                    args.iter().map(|&a| self.format(graph, a)).collect();
                let op_str = match op {
                    SetOp::Union => "\\cup",
                    SetOp::Intersection => "\\cap",
                    SetOp::Difference => "\\setminus",
                    SetOp::SymmetricDifference => "\\Delta",
                    SetOp::CartesianProduct => "\\times",
                    SetOp::In => "\\in",
                    SetOp::Subset => "\\subset",
                };
                Ok(arg_strs?.join(&format!(" {} ", op_str)))
            }
            ExprKind::Sum {
                body,
                var,
                lower,
                upper,
            } => {
                let body_str = self.format(graph, *body)?;
                let var_str = resolve_symbol(graph, *var)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "\\sum_{{{}={}}}^{{{}}} {}",
                            var_str, l_str, u_str, body_str
                        ))
                    }
                    _ => Ok(format!("\\sum_{{{}}} {}", var_str, body_str)),
                }
            }
            ExprKind::Product {
                body,
                var,
                lower,
                upper,
            } => {
                let body_str = self.format(graph, *body)?;
                let var_str = resolve_symbol(graph, *var)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "\\prod_{{{}={}}}^{{{}}} {}",
                            var_str, l_str, u_str, body_str
                        ))
                    }
                    _ => Ok(format!("\\prod_{{{}}} {}", var_str, body_str)),
                }
            }
            ExprKind::TensorContraction {
                tensor_a, tensor_b, ..
            } => {
                let a_str = self.format(graph, *tensor_a)?;
                let b_str = self.format(graph, *tensor_b)?;
                Ok(format!(
                    "\\operatorname{{Contract}}\\left({}, {}\\right)",
                    a_str, b_str
                ))
            }
        }
    }
}

/// Formatter generating clean, pretty 2D/1D Unicode string output.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnicodeFormatter;

impl UnicodeFormatter {
    #[allow(dead_code)]
    fn superscript(exp_str: &str) -> String {
        exp_str
            .chars()
            .map(|c| match c {
                '0' => '⁰',
                '1' => '¹',
                '2' => '²',
                '3' => '³',
                '4' => '⁴',
                '5' => '⁵',
                '6' => '⁶',
                '7' => '⁷',
                '8' => '⁸',
                '9' => '⁹',
                '+' => '⁺',
                '-' => '⁻',
                'n' => 'ⁿ',
                'x' => 'ˣ',
                _ => c,
            })
            .collect()
    }
}

impl Formatter for UnicodeFormatter {
    fn format(&self, graph: &ExprGraph, root: ExprId) -> FormatResult<String> {
        let node = graph.get(root);
        match &node.kind {
            ExprKind::Number(num) => Ok(match num {
                Number::Integer(i) => i.to_string(),
                Number::BigInteger(b) => b.to_string(),
                Number::Rational(n, d) => format!("{}/{}", n, d),
                Number::BigRational(r) => format!("{}/{}", r.numer(), r.denom()),
                Number::Scientific { mantissa, exponent } => {
                    if *exponent == 0 {
                        mantissa.to_string()
                    } else {
                        format!("{} × 10^{}", mantissa, exponent)
                    }
                }
                Number::Float(bits) => f64::from_bits(*bits).to_string(),
                Number::Constant(c) => match c {
                    Constant::Pi => "π".to_string(),
                    Constant::E => "e".to_string(),
                    Constant::I => "i".to_string(),
                    Constant::EulerGamma => "γ".to_string(),
                    Constant::GoldenRatio => "ϕ".to_string(),
                    Constant::Apery => "ζ(3)".to_string(),
                    Constant::Catalan => "G".to_string(),
                    Constant::Infinity => "∞".to_string(),
                    Constant::NegInfinity => "-∞".to_string(),
                    Constant::Undefined => "Undefined".to_string(),
                },
            }),
            ExprKind::Symbol(sym) => resolve_symbol(graph, *sym),
            ExprKind::Add(terms) => format_sum(self, &UNICODE, graph, terms),
            ExprKind::Mul(factors) => format_product(self, &UNICODE, graph, factors),
            ExprKind::Pow(base, exp) => {
                // `^` is right-associative, so a power in the exponent needs no
                // grouping, but one in the base does: (x^a)^b.
                let b_str = child(self, &UNICODE, graph, *base, Prec::Atom)?;
                let e_str = child(self, &UNICODE, graph, *exp, Prec::Power)?;
                Ok(format!("{}^{}", b_str, e_str))
            }
            ExprKind::Div(num, den) => {
                // a / (b · c): the denominator must bind tighter than a product.
                let n_str = child(self, &UNICODE, graph, *num, Prec::Negative)?;
                let d_str = child(self, &UNICODE, graph, *den, Prec::Power)?;
                Ok(format!("{} / {}", n_str, d_str))
            }
            ExprKind::Sub(lhs, rhs) => format_difference(self, &UNICODE, graph, *lhs, *rhs),
            ExprKind::Neg(inner) => {
                let i_str = child(self, &UNICODE, graph, *inner, Prec::Product)?;
                Ok(format!("-{}", i_str))
            }
            ExprKind::Function { name, args } => {
                let fn_name = resolve_symbol(graph, *name)?;
                let arg_strs: Result<Vec<_>, _> =
                    args.iter().map(|&a| self.format(graph, a)).collect();
                let args_vec = arg_strs?;
                if fn_name == "sqrt" && args_vec.len() == 1 {
                    Ok(format!("√({})", args_vec[0]))
                } else if (fn_name == "BringRadical" || fn_name == "BR") && args_vec.len() == 1 {
                    Ok(format!("BR({})", args_vec[0]))
                } else if fn_name == "besselj" && args_vec.len() == 2 {
                    Ok(format!("J_{}({})", args_vec[0], args_vec[1]))
                } else if fn_name == "uncertain" && args_vec.len() == 2 {
                    Ok(format!("{} ± {}", args_vec[0], args_vec[1]))
                } else if fn_name == "interval" && args_vec.len() == 2 {
                    Ok(format!("[{}, {}]", args_vec[0], args_vec[1]))
                } else if (fn_name == "lambert_w" || fn_name == "LambertW" || fn_name == "W")
                    && args_vec.len() == 2
                {
                    Ok(format!("W_{}({})", args_vec[0], args_vec[1]))
                } else {
                    Ok(format!("{}({})", fn_name, args_vec.join(", ")))
                }
            }
            ExprKind::Derivative { expr, wrt, order } => {
                let e_str = self.format(graph, *expr)?;
                let wrt_str = resolve_symbol(graph, *wrt)?;
                if *order == 1 {
                    Ok(format!("d/d{} [{}]", wrt_str, e_str))
                } else {
                    Ok(format!("d^{}/d{}^{} [{}]", order, wrt_str, order, e_str))
                }
            }
            ExprKind::Integral {
                expr,
                wrt,
                lower,
                upper,
            } => {
                let e_str = self.format(graph, *expr)?;
                let wrt_str = resolve_symbol(graph, *wrt)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "∫_{{{}}}^{{{}}} {} d{}",
                            l_str, u_str, e_str, wrt_str
                        ))
                    }
                    _ => Ok(format!("∫ {} d{}", e_str, wrt_str)),
                }
            }
            ExprKind::Matrix {
                rows,
                cols,
                elements,
            } => {
                let mut rows_str = Vec::new();
                for r in 0..*rows {
                    let mut col_strs = Vec::new();
                    for c in 0..*cols {
                        let idx = r * cols + c;
                        col_strs.push(self.format(graph, elements[idx])?);
                    }
                    rows_str.push(col_strs.join(", "));
                }
                Ok(format!("[ {} ]", rows_str.join(" ; ")))
            }
            ExprKind::Relational { op, lhs, rhs } => {
                let l_str = self.format(graph, *lhs)?;
                let r_str = self.format(graph, *rhs)?;
                let op_str = match op {
                    RelOp::Equal => "=",
                    RelOp::NotEqual => "≠",
                    RelOp::LessThan => "<",
                    RelOp::LessEqual => "≤",
                    RelOp::GreaterThan => ">",
                    RelOp::GreaterEqual => "≥",
                };
                Ok(format!("{} {} {}", l_str, op_str, r_str))
            }
            ExprKind::SetOperation { op, args } => {
                let arg_strs: Result<Vec<_>, _> =
                    args.iter().map(|&a| self.format(graph, a)).collect();
                let op_str = match op {
                    SetOp::Union => "∪",
                    SetOp::Intersection => "∩",
                    SetOp::Difference => "\\",
                    SetOp::SymmetricDifference => "Δ",
                    SetOp::CartesianProduct => "×",
                    SetOp::In => "∈",
                    SetOp::Subset => "⊂",
                };
                Ok(arg_strs?.join(&format!(" {} ", op_str)))
            }
            ExprKind::Sum {
                body,
                var,
                lower,
                upper,
            } => {
                let body_str = self.format(graph, *body)?;
                let var_str = resolve_symbol(graph, *var)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "∑_({}={})^({}) [{}]",
                            var_str, l_str, u_str, body_str
                        ))
                    }
                    _ => Ok(format!("∑_({}) [{}]", var_str, body_str)),
                }
            }
            ExprKind::Product {
                body,
                var,
                lower,
                upper,
            } => {
                let body_str = self.format(graph, *body)?;
                let var_str = resolve_symbol(graph, *var)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "∏_({}={})^({}) [{}]",
                            var_str, l_str, u_str, body_str
                        ))
                    }
                    _ => Ok(format!("∏_({}) [{}]", var_str, body_str)),
                }
            }
            ExprKind::TensorContraction {
                tensor_a, tensor_b, ..
            } => {
                let a_str = self.format(graph, *tensor_a)?;
                let b_str = self.format(graph, *tensor_b)?;
                Ok(format!("Contract({}, {})", a_str, b_str))
            }
        }
    }
}

/// Formatter generating MathML XML string output.
#[derive(Debug, Clone, Copy, Default)]
pub struct MathMLFormatter;

impl Formatter for MathMLFormatter {
    fn format(&self, graph: &ExprGraph, root: ExprId) -> FormatResult<String> {
        let node = graph.get(root);
        match &node.kind {
            ExprKind::Number(num) => Ok(match num {
                Number::Integer(i) => format!("<mn>{}</mn>", i),
                Number::BigInteger(b) => format!("<mn>{}</mn>", b),
                Number::Rational(n, d) => format!("<mfrac><mn>{}</mn><mn>{}</mn></mfrac>", n, d),
                Number::BigRational(r) => format!(
                    "<mfrac><mn>{}</mn><mn>{}</mn></mfrac>",
                    r.numer(),
                    r.denom()
                ),
                Number::Scientific { mantissa, exponent } => {
                    if *exponent == 0 {
                        format!("<mn>{}</mn>", mantissa)
                    } else {
                        format!(
                            "<mn>{}</mn><mo>&times;</mo><msup><mn>10</mn><mn>{}</mn></msup>",
                            mantissa, exponent
                        )
                    }
                }
                Number::Float(bits) => format!("<mn>{}</mn>", f64::from_bits(*bits)),
                Number::Constant(c) => match c {
                    Constant::Pi => "<mi>&pi;</mi>".to_string(),
                    Constant::E => "<mi>e</mi>".to_string(),
                    Constant::I => "<mi>i</mi>".to_string(),
                    Constant::EulerGamma => "<mi>&gamma;</mi>".to_string(),
                    Constant::GoldenRatio => "<mi>&phi;</mi>".to_string(),
                    Constant::Apery => "<mi>&zeta;</mi><mo>(</mo><mn>3</mn><mo>)</mo>".to_string(),
                    Constant::Catalan => "<mi>G</mi>".to_string(),
                    Constant::Infinity => "<mi>&infin;</mi>".to_string(),
                    Constant::NegInfinity => "<mo>-</mo><mi>&infin;</mi>".to_string(),
                    Constant::Undefined => "<mtext>Undefined</mtext>".to_string(),
                },
            }),
            ExprKind::Symbol(sym) => Ok(format!("<mi>{}</mi>", resolve_symbol(graph, *sym)?)),
            ExprKind::Add(terms) => format_sum(self, &MATHML, graph, terms),
            ExprKind::Mul(factors) => format_product(self, &MATHML, graph, factors),
            ExprKind::Pow(base, exp) => {
                // <msup> takes exactly two children, so each side is one <mrow>.
                let b_str = child(self, &MATHML, graph, *base, Prec::Atom)?;
                let e_str = self.format(graph, *exp)?;
                Ok(format!(
                    "<msup><mrow>{}</mrow><mrow>{}</mrow></msup>",
                    b_str, e_str
                ))
            }
            ExprKind::Div(num, den) => {
                let n_str = self.format(graph, *num)?;
                let d_str = self.format(graph, *den)?;
                Ok(format!(
                    "<mfrac><mrow>{}</mrow><mrow>{}</mrow></mfrac>",
                    n_str, d_str
                ))
            }
            ExprKind::Sub(lhs, rhs) => format_difference(self, &MATHML, graph, *lhs, *rhs),
            ExprKind::Neg(inner) => {
                let i_str = child(self, &MATHML, graph, *inner, Prec::Product)?;
                Ok(format!("<mo>-</mo>{}", i_str))
            }
            ExprKind::Function { name, args } => {
                let fn_name = resolve_symbol(graph, *name)?;
                let arg_strs: Result<Vec<_>, _> =
                    args.iter().map(|&a| self.format(graph, a)).collect();
                let args_vec = arg_strs?;
                if fn_name == "sqrt" && args_vec.len() == 1 {
                    Ok(format!("<msqrt>{}</msqrt>", args_vec[0]))
                } else {
                    Ok(format!(
                        "<mi>{}</mi><mo>(</mo>{}<mo>)</mo>",
                        fn_name,
                        args_vec.join("<mo>,</mo>")
                    ))
                }
            }
            ExprKind::Derivative { expr, wrt, order } => {
                let e_str = self.format(graph, *expr)?;
                let wrt_str = resolve_symbol(graph, *wrt)?;
                if *order == 1 {
                    Ok(format!(
                        "<mfrac><mi>d</mi><mrow><mi>d</mi><mi>{}</mi></mrow></mfrac><mo>(</mo>{}<mo>)</mo>",
                        wrt_str, e_str
                    ))
                } else {
                    Ok(format!(
                        "<mfrac><msup><mi>d</mi><mn>{}</mn></msup><mrow><mi>d</mi><msup><mi>{}</mi><mn>{}</mn></msup></mrow></mfrac><mo>(</mo>{}<mo>)</mo>",
                        order, wrt_str, order, e_str
                    ))
                }
            }
            ExprKind::Integral {
                expr,
                wrt,
                lower,
                upper,
            } => {
                let e_str = self.format(graph, *expr)?;
                let wrt_str = resolve_symbol(graph, *wrt)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "<msubsup><mo>&int;</mo><mrow>{}</mrow><mrow>{}</mrow></msubsup><mrow>{}</mrow><mi>d</mi><mi>{}</mi>",
                            l_str, u_str, e_str, wrt_str
                        ))
                    }
                    _ => Ok(format!(
                        "<mo>&int;</mo><mrow>{}</mrow><mi>d</mi><mi>{}</mi>",
                        e_str, wrt_str
                    )),
                }
            }
            ExprKind::Matrix {
                rows,
                cols,
                elements,
            } => {
                let mut rows_xml = String::new();
                for r in 0..*rows {
                    rows_xml.push_str("<mtr>");
                    for c in 0..*cols {
                        let idx = r * cols + c;
                        let cell = self.format(graph, elements[idx])?;
                        rows_xml.push_str(&format!("<mtd>{}</mtd>", cell));
                    }
                    rows_xml.push_str("</mtr>");
                }
                Ok(format!("<mo>(</mo><mtable>{}</mtable><mo>)</mo>", rows_xml))
            }
            ExprKind::Relational { op, lhs, rhs } => {
                let l_str = self.format(graph, *lhs)?;
                let r_str = self.format(graph, *rhs)?;
                let op_str = match op {
                    RelOp::Equal => "=",
                    RelOp::NotEqual => "&ne;",
                    RelOp::LessThan => "&lt;",
                    RelOp::LessEqual => "&le;",
                    RelOp::GreaterThan => "&gt;",
                    RelOp::GreaterEqual => "&ge;",
                };
                Ok(format!(
                    "<mrow>{}<mo>{}</mo>{}</mrow>",
                    l_str, op_str, r_str
                ))
            }
            ExprKind::SetOperation { op, args } => {
                let arg_strs: Result<Vec<_>, _> =
                    args.iter().map(|&a| self.format(graph, a)).collect();
                let op_str = match op {
                    SetOp::Union => "&cup;",
                    SetOp::Intersection => "&cap;",
                    SetOp::Difference => "\\",
                    SetOp::SymmetricDifference => "&Delta;",
                    SetOp::CartesianProduct => "&times;",
                    SetOp::In => "&isin;",
                    SetOp::Subset => "&sub;",
                };
                Ok(arg_strs?.join(&format!("<mo>{}</mo>", op_str)))
            }
            ExprKind::Sum {
                body,
                var,
                lower,
                upper,
            } => {
                let body_str = self.format(graph, *body)?;
                let var_str = resolve_symbol(graph, *var)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "<munderover><mo>&sum;</mo><mrow><mi>{}</mi><mo>=</mo>{}</mrow><mrow>{}</mrow></munderover><mrow>{}</mrow>",
                            var_str, l_str, u_str, body_str
                        ))
                    }
                    _ => Ok(format!(
                        "<msub><mo>&sum;</mo><mi>{}</mi></msub><mrow>{}</mrow>",
                        var_str, body_str
                    )),
                }
            }
            ExprKind::Product {
                body,
                var,
                lower,
                upper,
            } => {
                let body_str = self.format(graph, *body)?;
                let var_str = resolve_symbol(graph, *var)?;
                match (lower, upper) {
                    (Some(l), Some(u)) => {
                        let l_str = self.format(graph, *l)?;
                        let u_str = self.format(graph, *u)?;
                        Ok(format!(
                            "<munderover><mo>&prod;</mo><mrow><mi>{}</mi><mo>=</mo>{}</mrow><mrow>{}</mrow></munderover><mrow>{}</mrow>",
                            var_str, l_str, u_str, body_str
                        ))
                    }
                    _ => Ok(format!(
                        "<msub><mo>&prod;</mo><mi>{}</mi></msub><mrow>{}</mrow>",
                        var_str, body_str
                    )),
                }
            }
            ExprKind::TensorContraction {
                tensor_a, tensor_b, ..
            } => {
                let a_str = self.format(graph, *tensor_a)?;
                let b_str = self.format(graph, *tensor_b)?;
                Ok(format!(
                    "<mi>Contract</mi><mo>(</mo>{}<mo>,</mo>{}<mo>)</mo>",
                    a_str, b_str
                ))
            }
        }
    }
}
