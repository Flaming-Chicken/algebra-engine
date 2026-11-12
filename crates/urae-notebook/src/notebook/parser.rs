//! # `urae_notebook::notebook::parser`
//!
//! Document Line Representation, Symbol Cards, and Line Reference Resolution.

use crate::notebook::session::SymbolRole;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Permissive natural language solve command parser supporting multiple notations:
/// - `solve g(x) = 3, x`
/// - `solve g(x) = 3 for x`
/// - `given g(x) = 3 find x`
/// - `find x where g(x) = 3`
/// - `find x: g(x) = 3`
/// - `solve for x: g(x) = 3`
pub fn parse_permissive_solve_command(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();

    // 1. `solve ...` or `solveset ...`
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
            return Some((lhs.trim().to_string(), rhs.trim().to_string()));
        }
        if let Some((var_part, eq_part)) = clean.split_once(':') {
            let var_clean = var_part.strip_prefix("for ").unwrap_or(var_part).trim();
            return Some((eq_part.trim().to_string(), var_clean.to_string()));
        }
        if let Some((eq_p, v_p)) = clean.split_once(',') {
            return Some((eq_p.trim().to_string(), v_p.trim().to_string()));
        }
        return Some((clean.to_string(), "x".to_string()));
    }

    // 2. `given g(x) = 3 find x`
    if let Some(rest) = trimmed.strip_prefix("given ") {
        if let Some((eq_part, find_part)) = rest.split_once(" find ") {
            return Some((eq_part.trim().to_string(), find_part.trim().to_string()));
        }
    }

    // 3. `find x where g(x) = 3` or `find x: g(x) = 3`
    if let Some(rest) = trimmed.strip_prefix("find ") {
        if let Some((var_part, eq_part)) = rest.split_once(" where ") {
            return Some((eq_part.trim().to_string(), var_part.trim().to_string()));
        }
        if let Some((var_part, eq_part)) = rest.split_once(':') {
            return Some((eq_part.trim().to_string(), var_part.trim().to_string()));
        }
    }

    None
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineKind {
    Markdown,
    Formula,
    SliderDef {
        name: String,
        val: f64,
        unit: Option<String>,
    },
    DomainRestriction {
        name: String,
        rule_summary: String,
    },
    RoleDeclaration {
        name: String,
        role: SymbolRole,
    },
    SetBuilder {
        var_name: String,
        domain_type: String,
        condition: String,
    },
    CadMesh {
        model_kind: String,
        summary: String,
    },
    Plot3D {
        expr_str: String,
        x_var: String,
        y_var: String,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParsedLine {
    pub line_idx: usize,
    pub raw_text: String,
    pub kind: LineKind,
    pub output_latex: String,
    pub output_unicode: String,
    pub simplified_unicode: Option<String>,
    pub substituted_latex: Option<String>,
    pub derivative_latex: Option<String>,
    pub domain_info: Option<String>,
    pub error_msg: Option<String>,
    pub suggested_symbols: Vec<String>,
    pub is_function: bool,
    pub is_surface_3d: bool,
    pub custom_mesh_preset: Option<String>,
    pub physical_unit: Option<String>,
    pub numerical_roots: Option<Vec<f64>>,
    pub linearized_estimate: Option<f64>,
    pub is_pending: bool,
    pub eval_time_ms: u128,
    pub scoped_context: urae::MathContext,
}

impl Default for ParsedLine {
    fn default() -> Self {
        Self {
            line_idx: 0,
            raw_text: String::new(),
            kind: LineKind::Formula,
            output_latex: String::new(),
            output_unicode: String::new(),
            simplified_unicode: None,
            substituted_latex: None,
            derivative_latex: None,
            domain_info: None,
            error_msg: None,
            suggested_symbols: Vec::new(),
            is_function: false,
            is_surface_3d: false,
            custom_mesh_preset: None,
            physical_unit: None,
            numerical_roots: None,
            linearized_estimate: None,
            is_pending: false,
            eval_time_ms: 0,
            scoped_context: urae::MathContext::default(),
        }
    }
}

/// Structural representation of an individual notebook cell block kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellBlockKind {
    Math,
    Markdown,
    SectionHeader { level: usize, collapsed: bool },
}

/// A structured cell block in the notebook document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellBlock {
    pub id: String,
    pub kind: CellBlockKind,
    pub content: String,
    pub line_idx: usize,
}

/// Comprehensive algebraic object and computation classification for the Symbol & Object Inspector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectKind {
    /// Scalar numerical value
    Scalar {
        type_name: String,
        value_str: String,
        approx_f64: Option<f64>,
    },
    /// Complex number a + bi
    Complex { real: f64, imag: f64 },
    /// Vector / Matrix / Tensor
    Matrix {
        rows: usize,
        cols: usize,
        is_square: bool,
        shape_desc: String,
    },
    /// Function / Mapping f(x_1, ..., x_n)
    Function {
        arity: usize,
        domain: String,
        codomain: String,
        range: Option<String>,
        is_linear: bool,
        roots: Vec<String>,
        is_roots_symbolic: bool,
        critical_points: Vec<String>,
        is_critical_points_symbolic: bool,
        parity: Option<String>,
        period: Option<f64>,
    },
    /// Physical Quantity with dimensional units
    PhysicalQuantity {
        dimension: String,
        unit: String,
        magnitude: f64,
    },
    /// Mathematical Set / Bounded Interval
    SetOrInterval {
        set_type: String,
        bounds_str: String,
    },
    /// Polynomial expression
    Polynomial {
        indeterminate: String,
        degree: usize,
    },
    /// Fundamental Mathematical Constant (pi, e, i, gamma, phi)
    Constant {
        symbol_name: String,
        exact_desc: String,
        approx_val: f64,
    },
    /// Generic Symbolic Expression
    Expression {
        free_variables: Vec<String>,
        structure: String,
    },
    /// Line evaluation result ($1, $2, ans)
    LineResult { line_idx: usize, summary: String },
    /// Linear Time-Invariant Transfer Function H(s) / G(s)
    TransferFunction {
        numerator_degree: usize,
        denominator_degree: usize,
        poles: Vec<String>,
        zeros: Vec<String>,
        is_stable: bool,
    },
    /// Stochastic Process (Itô SDE, Wiener Process, Diffusion)
    StochasticProcess {
        drift_term: String,
        diffusion_term: String,
        is_martingale: bool,
    },
    /// Finite Element Analysis / Isogeometric Analysis (CAD Patch, Mesh)
    FEAResult {
        nodes: usize,
        elements: usize,
        max_stress: f64,
        deformation_scale: f64,
        solution_type: String,
    },
    /// Thermodynamic System State & Thermal Cycles (EoS, Carnot, Van der Waals)
    ThermodynamicState {
        system_type: String,
        equation_of_state: String,
        properties: Vec<(String, f64)>,
    },
    /// Canonical Algebraic Form (Horner polynomial, Factored, Expanded, Tropical)
    AlgebraicForm {
        form_type: String,
        complexity_score: usize,
    },
    /// Formal Logic System (Classical Propositional, Kleene K3, Łukasiewicz Ł3, Gödel G3, Modal S5)
    LogicSystem {
        system_name: String,
        truth_values_count: usize,
        is_paraconsistent: bool,
        is_intuitionistic: bool,
        tautologies_summary: String,
    },
    /// Elliptic Curve or Cryptographic Structure over Finite Field
    CryptographicCurve {
        curve_name: String,
        field_order: String,
        equation: String,
    },
    /// Discrete Graph, Network & Spectral Laplacian Structure
    GraphStructure {
        vertices: usize,
        edges: usize,
        is_directed: bool,
        spectral_gap: Option<f64>,
    },
    /// Differential Manifold & Riemannian Metric Tensor (General Relativity Spacetime)
    DifferentialManifold {
        dimension: usize,
        metric_name: String,
        curvature_scalar: Option<f64>,
    },
    /// Probability Distribution & Statistical Random Variable
    ProbabilityDistribution {
        dist_type: String,
        mean: f64,
        variance: f64,
        is_discrete: bool,
    },
}

/// Rich hover card and inspector metadata for a mathematical symbol or computed object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolInfoCard {
    pub name: String,
    pub role: SymbolRole,
    pub cur_val: f64,
    pub domain_type: String,
    pub unit_str: Option<String>,
    pub dependent_lines: Vec<usize>,
    pub formula_references: Vec<String>,
    pub object_kind: Option<ObjectKind>,
    #[serde(default)]
    pub computation_time_ms: Option<f64>,
    #[serde(default)]
    pub compound_tags: Vec<String>,
    #[serde(default)]
    pub ast_depth: Option<usize>,
    #[serde(default)]
    pub proof_search_depth: Option<usize>,
    #[serde(default)]
    pub neural_embedding: Option<Vec<f32>>,
}

impl Default for SymbolInfoCard {
    fn default() -> Self {
        Self {
            name: String::new(),
            role: SymbolRole::Variable,
            cur_val: 0.0,
            domain_type: "Real".to_string(),
            unit_str: None,
            dependent_lines: Vec::new(),
            formula_references: Vec::new(),
            object_kind: None,
            computation_time_ms: None,
            compound_tags: Vec::new(),
            ast_depth: None,
            proof_search_depth: None,
            neural_embedding: None,
        }
    }
}

/// Resolve `ans`, `$N`, and range `$M..$N` or `sum($M..$N)` references within a line before parsing.
pub fn resolve_line_references(
    text: &str,
    last_val: Option<f64>,
    line_results: &HashMap<usize, f64>,
) -> String {
    let mut resolved = text.to_string();

    // 1. Replace `ans` (word-bounded) if last_val is present
    if let Some(lv) = last_val {
        let val_str = if lv.fract() == 0.0 && lv.abs() < 1e12 {
            format!("{}", lv as i64)
        } else {
            format!("{:.6}", lv)
        };
        let mut new_s = String::new();
        let chars: Vec<char> = resolved.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if i + 3 <= chars.len() && chars[i..i + 3] == ['a', 'n', 's'] {
                let prev_ok = i == 0 || !chars[i - 1].is_alphanumeric();
                let next_ok = i + 3 == chars.len() || !chars[i + 3].is_alphanumeric();
                if prev_ok && next_ok {
                    new_s.push_str(&val_str);
                    i += 3;
                    continue;
                }
            }
            new_s.push(chars[i]);
            i += 1;
        }
        resolved = new_s;
    }

    // 2. Range Mathematical Aggregations: sum($1..$4), mean($1..$4), avg($1..$4), prod($1..$4), min($1..$4), max($1..$4)
    for fn_name in &["sum", "mean", "avg", "prod", "min", "max"] {
        let pattern = format!("{}($", fn_name);
        while let Some(start_pos) = resolved.find(&pattern) {
            let after_paren = start_pos + pattern.len();
            if let Some(close_paren) = resolved[after_paren..].find(')') {
                let inner = &resolved[after_paren..after_paren + close_paren];
                if let Some((start_s, end_s)) = inner.split_once("..$") {
                    if let (Ok(m), Ok(n)) = (
                        start_s.trim().parse::<usize>(),
                        end_s.trim().parse::<usize>(),
                    ) {
                        if m > 0 && n >= m {
                            let mut vals = Vec::new();
                            for l_idx in (m - 1)..n {
                                if let Some(&val) = line_results.get(&l_idx) {
                                    vals.push(val);
                                }
                            }
                            if !vals.is_empty() {
                                let res = match *fn_name {
                                    "sum" => vals.iter().sum::<f64>(),
                                    "mean" | "avg" => {
                                        vals.iter().sum::<f64>() / (vals.len() as f64)
                                    }
                                    "prod" => vals.iter().product::<f64>(),
                                    "min" => vals.iter().copied().fold(f64::INFINITY, f64::min),
                                    "max" => vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                                    _ => 0.0,
                                };
                                let res_str = if res.fract() == 0.0 && res.abs() < 1e12 {
                                    format!("{}", res as i64)
                                } else {
                                    format!("{:.6}", res)
                                };
                                let full_match = format!("{}(${}..${})", fn_name, m, n);
                                resolved = resolved.replace(&full_match, &res_str);
                                continue;
                            }
                        }
                    }
                }
            }
            break;
        }
    }

    // 3. Raw range expansion `$M..$N` -> comma-separated numbers
    while let Some(range_pos) = resolved.find("..$") {
        if let Some(dollar_pos) = resolved[..range_pos].rfind('$') {
            let start_num_s = &resolved[dollar_pos + 1..range_pos];
            let after_range = range_pos + 3;
            let end_digits_len = resolved[after_range..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count();
            if end_digits_len > 0 {
                let end_num_s = &resolved[after_range..after_range + end_digits_len];
                if let (Ok(m), Ok(n)) = (start_num_s.parse::<usize>(), end_num_s.parse::<usize>()) {
                    if m > 0 && n >= m {
                        let mut vals_str = Vec::new();
                        for l_idx in (m - 1)..n {
                            if let Some(&val) = line_results.get(&l_idx) {
                                let v_s = if val.fract() == 0.0 && val.abs() < 1e12 {
                                    format!("{}", val as i64)
                                } else {
                                    format!("{:.6}", val)
                                };
                                vals_str.push(v_s);
                            }
                        }
                        let full_token = format!("${}..${}", m, n);
                        resolved = resolved.replace(&full_token, &vals_str.join(", "));
                        continue;
                    }
                }
            }
        }
        break;
    }

    // 4. Replace single `$N` (1-indexed line reference, e.g. $1 for line 0, $2 for line 1)
    let mut new_s = String::new();
    let chars: Vec<char> = resolved.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            let num_str: String = chars[i + 1..j].iter().collect();
            if let Ok(line_num) = num_str.parse::<usize>() {
                if line_num > 0 {
                    let target_line_idx = line_num - 1;
                    if let Some(&line_val) = line_results.get(&target_line_idx) {
                        let v_str = if line_val.fract() == 0.0 && line_val.abs() < 1e12 {
                            format!("{}", line_val as i64)
                        } else {
                            format!("{:.6}", line_val)
                        };
                        new_s.push_str(&v_str);
                        i = j;
                        continue;
                    }
                }
            }
        }
        new_s.push(chars[i]);
        i += 1;
    }
    resolved = new_s;
    resolved
}

/// Determine whether a document line represents markdown, a header, or natural language prose.
pub fn is_markdown_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with('>')
        || trimmed.starts_with("- ")
        || (trimmed.len() >= 3
            && trimmed.as_bytes()[0].is_ascii_digit()
            && trimmed.as_bytes()[1] == b'.'
            && trimmed.as_bytes()[2] == b' ')
    {
        return true;
    }

    // Check if permissive intent parser or domain parser recognizes this as a math/domain statement
    if urae::parser::parse_permissive_intent(trimmed).is_some() {
        return false;
    }
    if urae::parser::parse_domain_declaration(trimmed).is_some() {
        return false;
    }

    // Check if the line is plain english prose/sentence without math operators/keywords
    if !trimmed.contains('=')
        && !trimmed.contains('+')
        && !trimmed.contains('-')
        && !trimmed.contains('*')
        && !trimmed.contains('/')
        && !trimmed.contains('^')
        && !trimmed.contains('{')
        && !trimmed.contains('}')
        && !trimmed.contains('|')
        && !trimmed.contains('∈')
        && !trimmed.contains('∀')
        && !trimmed.contains('∑')
        && !trimmed.contains('∫')
        && !trimmed.contains('∂')
        && !trimmed.contains('√')
        && !trimmed.contains('→')
        && !trimmed.contains(" in ")
        && !trimmed.contains(':')
    {
        let lower = trimmed.to_lowercase();
        let math_keywords = [
            "diff",
            "derivative",
            "integrate",
            "integral",
            "sum",
            "limit",
            "lim",
            "solve",
            "given",
            "find",
            "evaluate",
            "eval",
            "plot",
            "let",
            "where",
            "matrix",
            "vector",
            "quaternion",
            "clifford",
            "grassmann",
            "sin",
            "cos",
            "tan",
            "exp",
            "ln",
            "log",
            "sqrt",
        ];
        let contains_math_keyword = math_keywords.iter().any(|kw| lower.contains(kw));
        if !contains_math_keyword {
            let words_count = trimmed.split_whitespace().count();
            if words_count >= 3 || trimmed.ends_with('.') || trimmed.ends_with(':') {
                return true;
            }
        }
    }

    false
}
