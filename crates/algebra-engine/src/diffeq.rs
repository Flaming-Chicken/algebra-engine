//! # `algebra_engine::diffeq`
//!
//! Unified Differential Equation Classification and Automatic Solver Dispatch.
//!
//! Inspects equations to automatically distinguish between:
//! - **ODEs** (Ordinary Differential Equations: 1 independent variable)
//! - **PDEs** (Partial Differential Equations: >= 2 independent variables)
//! - **Order**: Maximum differentiation order
//! - **Linearity**: Linear, Semilinear, Quasilinear, or Nonlinear
//! - **Solution Strategy Recommendation**: Closed-form analytical vs. numerical integrator

use algebra_core::{ExprGraph, ExprId, ExprKind};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Differential Equation Classification Kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffEqKind {
    /// Ordinary Differential Equation (single independent variable).
    Ode,
    /// Partial Differential Equation (two or more independent variables).
    Pde,
}

/// Linearity of a differential equation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffEqLinearity {
    /// Fully linear in dependent variable and all its derivatives.
    Linear,
    /// Semilinear: highest derivatives linear with coefficients depending only on independent variables.
    Semilinear,
    /// Quasilinear: highest derivatives linear with coefficients depending on lower order derivatives.
    Quasilinear,
    /// Fully nonlinear in derivatives or dependent variable.
    Nonlinear,
}

/// Comprehensive metadata and classification descriptor for a differential equation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEqDescriptor {
    /// ODE vs PDE
    pub kind: DiffEqKind,
    /// Highest derivative order
    pub order: usize,
    /// Linearity classification
    pub linearity: DiffEqLinearity,
    /// List of identified independent variables (e.g. ["x"] or ["x", "t"])
    pub independent_vars: Vec<String>,
    /// Primary identified dependent variable (e.g. "y" or "u")
    pub dependent_var: String,
    /// Human-readable recommended solving methodology
    pub recommended_method: String,
}

/// Differential Equation Classifier.
pub struct DiffEqClassifier;

impl DiffEqClassifier {
    /// Automatically inspect an AST expression or equation to classify it as ODE or PDE.
    /// If a `manual_override` is provided by the user, the manual preference is respected.
    pub fn classify(
        graph: &ExprGraph,
        eq_id: ExprId,
        manual_override: Option<DiffEqKind>,
    ) -> DiffEqDescriptor {
        let mut indep_vars = HashSet::new();
        let mut max_order = 1usize;
        let mut has_nonlinear_dep = false;
        let mut dep_var = "y".to_string();

        Self::traverse_diffeq(
            graph,
            eq_id,
            &mut indep_vars,
            &mut max_order,
            &mut has_nonlinear_dep,
            &mut dep_var,
        );

        let kind = if let Some(forced) = manual_override {
            forced
        } else if indep_vars.len() >= 2 {
            DiffEqKind::Pde
        } else {
            DiffEqKind::Ode
        };

        if kind == DiffEqKind::Pde && dep_var == "y" {
            dep_var = "u".to_string();
        }

        // The traversal above misses powers and products of derivatives; check term by term.
        let has_nonlinear_dep =
            has_nonlinear_dep || !Self::is_linear_in_unknown(graph, eq_id, &dep_var);
        let linearity = if !has_nonlinear_dep {
            DiffEqLinearity::Linear
        } else if kind == DiffEqKind::Pde {
            // PDE theory distinguishes where the nonlinearity sits.
            Self::refine_pde_linearity(graph, eq_id, &dep_var, max_order)
        } else {
            DiffEqLinearity::Nonlinear
        };

        let mut sorted_indep: Vec<String> = indep_vars.into_iter().collect();
        sorted_indep.sort();
        if sorted_indep.is_empty() {
            sorted_indep.push(if kind == DiffEqKind::Pde {
                "x".to_string()
            } else {
                "t".to_string()
            });
            if kind == DiffEqKind::Pde {
                sorted_indep.push("t".to_string());
            }
        }

        let recommended_method = match (kind, max_order, linearity) {
            (DiffEqKind::Ode, 1, DiffEqLinearity::Linear) => {
                "Integrating Factor / First-Order Linear Solver".to_string()
            }
            (DiffEqKind::Ode, 2, DiffEqLinearity::Linear) => {
                "Characteristic Polynomial / Cauchy-Euler Euler-Ansatz".to_string()
            }
            (DiffEqKind::Ode, _, DiffEqLinearity::Linear) => {
                "Linear Differential Operator Factorization / Companion Matrix".to_string()
            }
            (DiffEqKind::Ode, _, DiffEqLinearity::Nonlinear) => {
                "Nonlinear Phase-Portrait / Adaptive DOPRI5 or Stiff Radau IIA".to_string()
            }
            (DiffEqKind::Pde, 1, _) => {
                "Method of Characteristics (u(x, t) = f(x - ct))".to_string()
            }
            (DiffEqKind::Pde, 2, DiffEqLinearity::Linear) => {
                "Sturm-Liouville Separation of Variables & Fundamental Green Kernels".to_string()
            }
            (DiffEqKind::Pde, _, DiffEqLinearity::Nonlinear)
            | (DiffEqKind::Pde, _, DiffEqLinearity::Semilinear)
            | (DiffEqKind::Pde, _, DiffEqLinearity::Quasilinear) => {
                "Traveling Wave Reduction (xi = x - ct) & Method of Lines".to_string()
            }
            (DiffEqKind::Pde, _, _) => {
                "Method of Lines (Semidiscretization to ODE System)".to_string()
            }
            _ => "General Differential Equation Solver".to_string(),
        };

        DiffEqDescriptor {
            kind,
            order: max_order,
            linearity,
            independent_vars: sorted_indep,
            dependent_var: dep_var,
            recommended_method,
        }
    }

    /// Order of the derivative of the dependent variable that `id` denotes: `0` for the variable
    /// itself, `k` for `Derivative` nodes and subscript symbols such as `u_xx`. `None` otherwise.
    fn derivative_order(graph: &ExprGraph, id: ExprId, dep: &str) -> Option<usize> {
        match &graph.get(id).kind {
            ExprKind::Symbol(s) => {
                let name = graph.symbols.resolve(*s)?;
                if name == dep {
                    Some(0)
                } else {
                    name.strip_prefix(&format!("{dep}_"))
                        .filter(|subscript| {
                            !subscript.is_empty() && subscript.chars().all(|c| c.is_alphabetic())
                        })
                        .map(|subscript| subscript.chars().count())
                }
            }
            ExprKind::Derivative { expr, order, .. } => {
                Self::derivative_order(graph, *expr, dep).map(|inner| inner + *order as usize)
            }
            _ => None,
        }
    }

    /// Highest derivative order of the dependent variable anywhere inside `id`.
    fn deepest_order(graph: &ExprGraph, id: ExprId, dep: &str) -> usize {
        if let Some(order) = Self::derivative_order(graph, id, dep) {
            return order;
        }
        match &graph.get(id).kind {
            ExprKind::Add(items) | ExprKind::Mul(items) => items
                .iter()
                .map(|&i| Self::deepest_order(graph, i, dep))
                .max()
                .unwrap_or(0),
            ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
                Self::deepest_order(graph, *a, dep).max(Self::deepest_order(graph, *b, dep))
            }
            ExprKind::Neg(inner) => Self::deepest_order(graph, *inner, dep),
            ExprKind::Function { args, .. } => args
                .iter()
                .map(|&a| Self::deepest_order(graph, a, dep))
                .max()
                .unwrap_or(0),
            _ => 0,
        }
    }

    fn collect_terms(graph: &ExprGraph, id: ExprId, out: &mut Vec<ExprId>) {
        match &graph.get(id).kind {
            ExprKind::Add(items) => items
                .iter()
                .for_each(|&i| Self::collect_terms(graph, i, out)),
            ExprKind::Sub(a, b) => {
                Self::collect_terms(graph, *a, out);
                Self::collect_terms(graph, *b, out);
            }
            ExprKind::Neg(inner) => Self::collect_terms(graph, *inner, out),
            ExprKind::Relational { lhs, rhs, .. } => {
                Self::collect_terms(graph, *lhs, out);
                Self::collect_terms(graph, *rhs, out);
            }
            _ => out.push(id),
        }
    }

    fn collect_factors(graph: &ExprGraph, id: ExprId, out: &mut Vec<ExprId>) {
        match &graph.get(id).kind {
            ExprKind::Mul(items) => items
                .iter()
                .for_each(|&i| Self::collect_factors(graph, i, out)),
            ExprKind::Neg(inner) => Self::collect_factors(graph, *inner, out),
            ExprKind::Div(num, den) => {
                Self::collect_factors(graph, *num, out);
                // A denominator is only a harmless coefficient if it is free of the unknown.
                out.push(*den);
            }
            _ => out.push(id),
        }
    }

    /// Whether every term of the equation contains the unknown (or one of its derivatives) at
    /// most once, to the first power, outside any function or denominator.
    fn is_linear_in_unknown(graph: &ExprGraph, eq_id: ExprId, dep: &str) -> bool {
        let mut terms = Vec::new();
        Self::collect_terms(graph, eq_id, &mut terms);
        terms.iter().all(|&term| {
            let mut factors = Vec::new();
            Self::collect_factors(graph, term, &mut factors);
            let mut involved = 0;
            for &factor in &factors {
                if Self::derivative_order(graph, factor, dep).is_some() {
                    involved += 1;
                } else if Self::deepest_order(graph, factor, dep) > 0
                    || Self::contains_symbol_name(graph, factor, dep)
                {
                    // Inside a function, power or denominator: not linear.
                    return false;
                }
            }
            involved <= 1
        })
    }

    /// Distinguish semilinear, quasilinear and fully nonlinear PDEs. The caller has already
    /// established that the equation is not linear.
    ///
    /// Let `m` be the highest derivative order. The equation is *fully nonlinear* if `m` occurs
    /// inside a nonlinear function or twice in one product; *quasilinear* if each `m`-th
    /// derivative enters linearly but its coefficient depends on the unknown or lower
    /// derivatives; *semilinear* if those coefficients depend on the independent variables only.
    fn refine_pde_linearity(
        graph: &ExprGraph,
        eq_id: ExprId,
        dep: &str,
        max_order: usize,
    ) -> DiffEqLinearity {
        let mut terms = Vec::new();
        Self::collect_terms(graph, eq_id, &mut terms);
        let mut m = 0;
        for &term in &terms {
            m = m.max(Self::deepest_order(graph, term, dep));
        }
        if m == 0 {
            m = max_order;
        }
        let mut quasilinear = false;
        for &term in &terms {
            let mut factors = Vec::new();
            Self::collect_factors(graph, term, &mut factors);
            let mut top_atoms = 0; // factors that are exactly an m-th derivative
            let mut others = 0; // any other factor that involves the unknown
            let mut top_inside_other = false;
            for &factor in &factors {
                match Self::derivative_order(graph, factor, dep) {
                    Some(k) if k == m && m > 0 => top_atoms += 1,
                    Some(_) => others += 1,
                    None => {
                        let depth = Self::deepest_order(graph, factor, dep);
                        let involves = depth > 0 || Self::contains_symbol_name(graph, factor, dep);
                        if involves {
                            others += 1;
                            if depth == m && m > 0 {
                                top_inside_other = true;
                            }
                        }
                    }
                }
            }
            if top_inside_other || top_atoms > 1 {
                return DiffEqLinearity::Nonlinear;
            }
            if top_atoms == 1 && others > 0 {
                quasilinear = true;
            }
        }
        if quasilinear {
            DiffEqLinearity::Quasilinear
        } else {
            DiffEqLinearity::Semilinear
        }
    }

    fn traverse_diffeq(
        graph: &ExprGraph,
        node_id: ExprId,
        indep: &mut HashSet<String>,
        max_order: &mut usize,
        has_nonlinear_dep: &mut bool,
        dep_var: &mut String,
    ) {
        let node = graph.get(node_id);
        match &node.kind {
            ExprKind::Derivative { expr, wrt, order } => {
                if let Some(wrt_str) = graph.symbols.resolve(*wrt) {
                    indep.insert(wrt_str);
                }
                if let ExprKind::Symbol(s) = graph.get(*expr).kind
                    && let Some(s_name) = graph.symbols.resolve(s)
                {
                    *dep_var = s_name;
                }
                *max_order = (*max_order).max(*order as usize);
                Self::traverse_diffeq(graph, *expr, indep, max_order, has_nonlinear_dep, dep_var);
            }
            ExprKind::Symbol(s) => {
                if let Some(s_name) = graph.symbols.resolve(*s) {
                    // Check for common subscript derivative notation e.g. u_x, u_t, u_xx, u_tt
                    if s_name.starts_with("u_") || s_name.starts_with("y_") {
                        let parts: Vec<&str> = s_name.split('_').collect();
                        if parts.len() == 2 {
                            *dep_var = parts[0].to_string();
                            for ch in parts[1].chars() {
                                indep.insert(ch.to_string());
                            }
                            *max_order = (*max_order).max(parts[1].len());
                        }
                    } else if s_name == "y_prime" || s_name == "y_dot" {
                        *dep_var = "y".to_string();
                        indep.insert("x".to_string());
                        *max_order = (*max_order).max(1);
                    } else if s_name == "y_prime_prime" || s_name == "y_ddot" {
                        *dep_var = "y".to_string();
                        indep.insert("x".to_string());
                        *max_order = (*max_order).max(2);
                    }
                }
            }
            ExprKind::Pow(base, exp) => {
                // If dependent variable is raised to power != 1
                if let ExprKind::Symbol(s) = graph.get(*base).kind
                    && let Some(s_name) = graph.symbols.resolve(s)
                    && (s_name == *dep_var || s_name == "y" || s_name == "u")
                {
                    if let ExprKind::Number(n) = graph.get(*exp).kind {
                        if n.as_f64() != Some(1.0) {
                            *has_nonlinear_dep = true;
                        }
                    } else {
                        *has_nonlinear_dep = true;
                    }
                }
                Self::traverse_diffeq(graph, *base, indep, max_order, has_nonlinear_dep, dep_var);
                Self::traverse_diffeq(graph, *exp, indep, max_order, has_nonlinear_dep, dep_var);
            }
            ExprKind::Function { name, args } => {
                if let Some(fn_name) = graph.symbols.resolve(*name)
                    && ["sin", "cos", "tan", "exp", "log", "sinh", "cosh"]
                        .contains(&fn_name.as_str())
                {
                    for &arg in args {
                        if Self::contains_non_constant_symbol(graph, arg) {
                            *has_nonlinear_dep = true;
                        }
                    }
                }
                for &arg in args {
                    Self::traverse_diffeq(graph, arg, indep, max_order, has_nonlinear_dep, dep_var);
                }
            }
            ExprKind::Mul(factors) => {
                let mut dep_count = 0;
                for &f in factors {
                    if Self::contains_symbol_name(graph, f, dep_var)
                        || Self::contains_symbol_name(graph, f, "y")
                        || Self::contains_symbol_name(graph, f, "u")
                    {
                        dep_count += 1;
                    }
                    Self::traverse_diffeq(graph, f, indep, max_order, has_nonlinear_dep, dep_var);
                }
                if dep_count >= 2 {
                    *has_nonlinear_dep = true;
                }
            }
            ExprKind::Add(terms) => {
                for &t in terms {
                    Self::traverse_diffeq(graph, t, indep, max_order, has_nonlinear_dep, dep_var);
                }
            }
            ExprKind::Sub(a, b) => {
                Self::traverse_diffeq(graph, *a, indep, max_order, has_nonlinear_dep, dep_var);
                Self::traverse_diffeq(graph, *b, indep, max_order, has_nonlinear_dep, dep_var);
            }
            ExprKind::Neg(inner) => {
                Self::traverse_diffeq(graph, *inner, indep, max_order, has_nonlinear_dep, dep_var);
            }
            ExprKind::Div(num, den) => {
                Self::traverse_diffeq(graph, *num, indep, max_order, has_nonlinear_dep, dep_var);
                Self::traverse_diffeq(graph, *den, indep, max_order, has_nonlinear_dep, dep_var);
            }
            ExprKind::Relational { lhs, rhs, .. } => {
                Self::traverse_diffeq(graph, *lhs, indep, max_order, has_nonlinear_dep, dep_var);
                Self::traverse_diffeq(graph, *rhs, indep, max_order, has_nonlinear_dep, dep_var);
            }
            _ => {}
        }
    }

    fn contains_non_constant_symbol(graph: &ExprGraph, node_id: ExprId) -> bool {
        let node = graph.get(node_id);
        match &node.kind {
            ExprKind::Symbol(s) => {
                if let Some(name) = graph.symbols.resolve(*s) {
                    name != "pi" && name != "e"
                } else {
                    false
                }
            }
            ExprKind::Derivative { .. } => true,
            ExprKind::Add(terms) | ExprKind::Mul(terms) => terms
                .iter()
                .any(|&t| Self::contains_non_constant_symbol(graph, t)),
            ExprKind::Sub(a, b) | ExprKind::Div(a, b) => {
                Self::contains_non_constant_symbol(graph, *a)
                    || Self::contains_non_constant_symbol(graph, *b)
            }
            ExprKind::Neg(inner) => Self::contains_non_constant_symbol(graph, *inner),
            ExprKind::Pow(b, e) => {
                Self::contains_non_constant_symbol(graph, *b)
                    || Self::contains_non_constant_symbol(graph, *e)
            }
            ExprKind::Function { args, .. } => args
                .iter()
                .any(|&a| Self::contains_non_constant_symbol(graph, a)),
            _ => false,
        }
    }

    fn contains_symbol_name(graph: &ExprGraph, node_id: ExprId, target: &str) -> bool {
        let node = graph.get(node_id);
        match &node.kind {
            ExprKind::Symbol(s) => graph
                .symbols
                .resolve(*s)
                .map(|name| name == target)
                .unwrap_or(false),
            ExprKind::Derivative { expr, .. } => Self::contains_symbol_name(graph, *expr, target),
            ExprKind::Add(terms) | ExprKind::Mul(terms) => terms
                .iter()
                .any(|&t| Self::contains_symbol_name(graph, t, target)),
            ExprKind::Sub(a, b) | ExprKind::Div(a, b) => {
                Self::contains_symbol_name(graph, *a, target)
                    || Self::contains_symbol_name(graph, *b, target)
            }
            ExprKind::Neg(inner) => Self::contains_symbol_name(graph, *inner, target),
            ExprKind::Pow(b, e) => {
                Self::contains_symbol_name(graph, *b, target)
                    || Self::contains_symbol_name(graph, *e, target)
            }
            ExprKind::Function { args, .. } => args
                .iter()
                .any(|&a| Self::contains_symbol_name(graph, a, target)),
            _ => false,
        }
    }
}
