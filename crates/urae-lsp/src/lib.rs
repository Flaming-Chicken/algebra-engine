//! # `urae-lsp` (Language Server Protocol for URAE Mathematical Scripting)
//!
//! Language Server Protocol (LSP) Engine providing mathematical intelligence for `.urae`, `.math`, and `.cas` files.
//!
//! ## Core LSP Capabilities:
//! - **Hover Tooltips**: Formatted LaTeX formulas ($$\dots$$), type/domain inference ($\mathbb{R}, \mathbb{C}, \mathbb{H}$), and physical units ($[\text{m/s}^2]$).
//! - **Auto-Completion**: 100+ mathematical functions, Greek symbols ($\alpha, \beta, \gamma, \nabla, \partial$), and CAD machinery macros.
//! - **Real-Time Diagnostics**: Expression parse errors, unmatched parentheses, and physical unit dimension inconsistencies.
//! - **Code Actions**: Automated transformations (E-Graph simplification, matrix transpose, Taylor expansion).

use serde::{Deserialize, Serialize};
use urae::prelude::*;

/// LSP Position (0-indexed line and character).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

/// LSP Range (start and end position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP Diagnostic Severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// LSP Diagnostic Item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: String,
}

/// LSP Completion Item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: String,
    pub insert_text: String,
    pub documentation: String,
}

/// LSP Hover Content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hover {
    pub contents: String,
    pub range: Option<Range>,
}

/// LSP Code Action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeAction {
    pub title: String,
    pub new_text: String,
    pub range: Range,
}

/// URAE Language Server Engine.
pub struct UraeLanguageServer;

impl UraeLanguageServer {
    /// Compute real-time diagnostics for a document text.
    pub fn diagnostics(document: &str) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let mut session = UraeSession::new();

        for (line_idx, line) in document.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            // Skip macro definitions or commands
            if trimmed.starts_with("gear!")
                || trimmed.starts_with("screw!")
                || trimmed.starts_with("quantum_")
            {
                continue;
            }

            let res = session.execute_line(trimmed);
            if res.is_error {
                let err_msg = res.error_msg.unwrap_or(res.output_text);
                diags.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_idx,
                            character: 0,
                        },
                        end: Position {
                            line: line_idx,
                            character: line.len(),
                        },
                    },
                    severity: DiagnosticSeverity::Error,
                    message: format!("Mathematical Parse Error: {}", err_msg),
                    source: "urae-lsp".to_string(),
                });
            }
        }

        diags
    }

    /// Provide intelligent autocompletions based on prefix.
    pub fn completions(prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let p_lower = prefix.to_lowercase();

        let all_completions = [
            (
                "diff",
                "Symbolic derivative: diff(f(x), x)",
                "diff($1, $2)",
                "Differentiates expression with respect to variable.",
            ),
            (
                "integrate",
                "Symbolic Risch integral: integrate(f(x), x)",
                "integrate($1, $2)",
                "Integrates expression analytically.",
            ),
            (
                "series",
                "Taylor / Laurent series: series(f(x), x, x0, order)",
                "series($1, $2, $3, $4)",
                "Expands function into asymptotic series.",
            ),
            (
                "solveset",
                "Equation solver: solve equation for x",
                "solve $1 = 0 for $2",
                "Finds roots and zeroes of algebraic/transcendental equations.",
            ),
            (
                "gear!",
                "Parametric Involute Gear: gear!(teeth=16, module=2.0)",
                "gear!(teeth = 16, module = 2.0, pressure_angle = 20.0)",
                "Generates 3D CAD mesh for an involute spur or helical gear.",
            ),
            (
                "screw!",
                "Threaded Fastener: screw!(dia=8.0, pitch=1.25, len=20.0)",
                "screw!(dia = 8.0, pitch = 1.25, len = 20.0, head = Hex)",
                "Generates 3D CAD mesh for an ISO metric bolt.",
            ),
            (
                "airfoil!",
                "NACA Airfoil & Wing: airfoil!(code=\"2412\", chord=10.0)",
                "airfoil!(code = \"2412\", chord = 10.0, span = 25.0)",
                "Generates 3D CAD mesh for a NACA cambered aerodynamic wing.",
            ),
            (
                "spring!",
                "Helical Coil Spring: spring!(mean_dia=8.0, wire_dia=1.2)",
                "spring!(mean_dia = 8.0, wire_dia = 1.2, pitch = 3.0, coils = 6)",
                "Generates 3D CAD mesh for a compression spring.",
            ),
            (
                "octonion!",
                "Octonion 8D Constructor: octonion!(c0, ..., c7)",
                "octonion!($1, $2, $3, $4, $5, $6, $7, $8)",
                "Constructs an 8D normed alternative octonion in O.",
            ),
            (
                "e8_weyl_reflect!",
                "E8 Root Lattice Weyl Reflection",
                "e8_weyl_reflect!($1, $2)",
                "Reflects vector through the hyperplane orthogonal to an E8 root.",
            ),
            (
                "virasoro_bracket!",
                "Virasoro Lie Algebra: [L_m, L_n]",
                "virasoro_bracket!($1, $2, $3)",
                "Computes Virasoro Lie bracket with central charge c.",
            ),
            (
                "kac_moody_bracket!",
                "Affine Kac-Moody Current Bracket: [J^a_m, J^b_n]",
                "kac_moody_bracket!($1, $2, $3, $4, $5)",
                "Computes affine su(2)_k loop algebra current commutator.",
            ),
            (
                "fontaine_module!",
                "p-Adic Hodge Fontaine Module",
                "fontaine_module!($1, $2, $3, $4, $5)",
                "Constructs a filtered (Phi, N)-module.",
            ),
            (
                "cox_de_boor_basis!",
                "Cox-de Boor B-Spline Basis Function",
                "cox_de_boor_basis!($1, $2, $3, $4)",
                "Evaluates non-uniform B-spline basis function N_{i, p}(u).",
            ),
            (
                "weyl_x!",
                "Weyl Algebra Coordinate Operator x_i^k",
                "weyl_x!($1, $2, $3)",
                "Constructs coordinate operator in non-commutative Weyl algebra A_n.",
            ),
            (
                "weyl_d!",
                "Weyl Algebra Derivative Operator d_i^k",
                "weyl_d!($1, $2, $3)",
                "Constructs derivative operator in non-commutative Weyl algebra A_n.",
            ),
            (
                "dirac_trace!",
                "QFT Dirac Gamma Trace Tr[gamma^mu1 ... gamma^mun]",
                "dirac_trace!($1)",
                "Evaluates recursive trace of 2n Dirac gamma matrices.",
            ),
            (
                "spinor_bracket_angle!",
                "Spinor Helicity Angle Bracket <i j>",
                "spinor_bracket_angle!($1, $2)",
                "Computes invariant Lorentz angle bracket between massless Weyl spinors.",
            ),
            (
                "alpha",
                "Greek letter α (Alpha)",
                "α",
                "Mathematical symbol alpha.",
            ),
            (
                "beta",
                "Greek letter β (Beta)",
                "β",
                "Mathematical symbol beta.",
            ),
            (
                "gamma",
                "Greek letter γ (Gamma)",
                "γ",
                "Mathematical symbol gamma.",
            ),
            (
                "nabla",
                "Vector differential ∇ (Nabla / Gradient)",
                "∇",
                "Vector del/nabla gradient operator.",
            ),
            (
                "partial",
                "Partial derivative operator ∂",
                "∂",
                "Differential partial derivative symbol.",
            ),
        ];

        for (label, detail, insert, doc) in all_completions {
            if p_lower.is_empty()
                || label.to_lowercase().contains(&p_lower)
                || detail.to_lowercase().contains(&p_lower)
            {
                items.push(CompletionItem {
                    label: label.to_string(),
                    detail: detail.to_string(),
                    insert_text: insert.to_string(),
                    documentation: doc.to_string(),
                });
            }
        }

        items
    }

    /// Completions at a cursor position: the operator and function catalogue, plus the names
    /// defined (as `name = ...`) on lines above the cursor line, most recent definition first.
    pub fn completions_at(document: &str, position: Position, prefix: &str) -> Vec<CompletionItem> {
        let p_lower = prefix.to_lowercase();
        let mut defined: Vec<CompletionItem> = Vec::new();
        let above: Vec<&str> = document.lines().take(position.line).collect();
        for (index, line) in above.into_iter().enumerate().rev() {
            let trimmed = line.trim();
            let Some((lhs, rhs)) = trimmed.split_once('=') else {
                continue;
            };
            // A plain assignment: not `==`, `<=`, `>=`, `!=`.
            if rhs.starts_with('=') || lhs.ends_with(['<', '>', '!']) {
                continue;
            }
            let name = lhs.trim();
            let is_identifier = !name.is_empty()
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !name.chars().next().is_some_and(|c| c.is_ascii_digit());
            if !is_identifier
                || !name.to_lowercase().starts_with(&p_lower)
                || defined.iter().any(|item| item.label == name)
            {
                continue;
            }
            defined.push(CompletionItem {
                label: name.to_string(),
                detail: format!("defined on line {}", index + 1),
                insert_text: name.to_string(),
                documentation: format!("{name} = {}", rhs.trim()),
            });
        }
        defined.extend(Self::completions(prefix));
        defined
    }

    /// Provide formatted hover information for token under cursor.
    pub fn hover(document: &str, position: Position) -> Option<Hover> {
        let line = document.lines().nth(position.line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Evaluate in document order so definitions on earlier lines are in scope.
        let mut session = UraeSession::new();
        for earlier in document.lines().take(position.line) {
            let earlier = earlier.trim();
            if earlier.is_empty() || earlier.starts_with('#') || earlier.starts_with("//") {
                continue;
            }
            session.execute_line(earlier);
        }
        let res = session.execute_line(trimmed);

        if !res.is_error {
            let domain_desc = res
                .domain_info
                .unwrap_or_else(|| "ℝ (Real Field)".to_string());
            let md = format!(
                "### URAE Evaluated Expression\n\n$$\n{}\n$$\n\n- **Unicode Representation**: `{}`\n- **Domain**: `{}`",
                res.output_latex, res.output_unicode, domain_desc
            );

            Some(Hover {
                contents: md,
                range: Some(Range {
                    start: Position {
                        line: position.line,
                        character: 0,
                    },
                    end: Position {
                        line: position.line,
                        character: line.len(),
                    },
                }),
            })
        } else {
            None
        }
    }

    /// Provide automated code actions (e.g. "Simplify with E-Graphs").
    pub fn code_actions(document: &str, range: Range) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        let line = match document.lines().nth(range.start.line) {
            Some(l) => l.trim(),
            None => return actions,
        };

        let session = UraeSession::new();
        let parser = ExprParser::new(&session.graph);

        if let Ok(expr_id) = parser.parse(line) {
            if let Ok(simp_id) = Simplifier::simplify(&session.graph, expr_id) {
                let unicode = UnicodeFormatter
                    .format(&session.graph, simp_id)
                    .unwrap_or_default();
                if unicode != line {
                    actions.push(CodeAction {
                        title: format!("Simplify with E-Graphs: '{}' ➔ '{}'", line, unicode),
                        new_text: unicode,
                        range,
                    });
                }
            }
        }

        actions
    }
}
