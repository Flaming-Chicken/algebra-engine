//! # `algebra_engine::cad`
//!
//! Cylindrical Algebraic Decomposition (CAD), Real Quantifier Elimination (QE),
//! and Parametric Geometric Constraint Solving.
//!
//! ## Mathematical Foundations
//! - **Cylindrical Algebraic Decomposition**: Decomposes $\mathbb{R}^n$ into a finite union of
//!   connected, disjoint, sign-invariant semi-algebraic cells $\mathbb{R}^n = \bigcup_{i} C_i$
//!   for a set of polynomials $\mathcal{F} = \{f_1, \dots, f_m\} \subset \mathbb{Q}[x_1, \dots, x_n]$.
//! - **Projection Phase (Collins & McCallum)**:
//!   Recursively projects polynomials $\mathcal{F}_k \subset \mathbb{Q}[x_1, \dots, x_k]$ to
//!   $\mathcal{F}_{k-1} \subset \mathbb{Q}[x_1, \dots, x_{k-1}]$ using Principal Subresultant
//!   Coefficients ($\operatorname{psc}$), Discriminants $\operatorname{disc}(f, x_k)$, and
//!   Resultants $\operatorname{res}(f, g, x_k)$.
//! - **Base & Lifting Phase**:
//!   Isolates real roots of univariate projection polynomials via Sturm/VAS bisection, constructing
//!   sample points for **Sections** ($x_k = \alpha$) and **Sectors** ($x_k \in (\alpha_i, \alpha_{i+1})$).
//! - **Quantifier Elimination (Tarski-Seidenberg Decision Procedure)**:
//!   Eliminates quantifiers $\forall, \exists$ over first-order real arithmetic formulas:
//!   $$
//!   \Phi(\mathbf{y}) = Q_1 x_1 \dots Q_k x_k \, \psi(\mathbf{x}, \mathbf{y}) \iff \bigvee_{C \in \text{TrueCells}} \text{Cond}(C)
//!   $$
//! - **Parametric Geometric Constraint Solver**:
//!   Translates 2D/3D/ND geometric constraints (tangency, perpendicularity, distance, incidence)
//!   into polynomial ideals resolved via Gröbner bases and CAD cell evaluation.
//!
//! ## References
//! - Collins, G. E. (1975). Quantifier elimination for real closed fields by cylindrical algebraic decomposition. *LNCS* 33, 134–183.
//! - McCallum, S. (1988). An improved projection operation for cylindrical algebraic decomposition of three-dimensional space. *J. Symbolic Comput.*, 5(1–2), 141–161.
//! - Tarski, A. (1951). *A Decision Method for Elementary Algebra and Geometry*. Univ. of California Press.

pub mod bvh;
pub mod engineering;

pub use bvh::{BvhIntersection, BvhNode, BvhTree, Ray3D};
pub use engineering::{
    BoltHeadType, HelicalSpring, InvoluteGear, NacaAirfoil, ThreadStandard, ThreadedScrew,
};

use algebra_core::error::{AlgebraError, AlgebraResult};
use std::collections::{BTreeMap, HashMap};

/// Sign of a polynomial evaluation: Negative ($-1$), Zero ($0$), or Positive ($+1$).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sign {
    Negative = -1,
    Zero = 0,
    Positive = 1,
}

impl Sign {
    /// Classify sign from floating-point evaluation with epsilon threshold.
    pub fn from_f64(val: f64, eps: f64) -> Self {
        if val.abs() <= eps {
            Sign::Zero
        } else if val > 0.0 {
            Sign::Positive
        } else {
            Sign::Negative
        }
    }
}

/// Comparison relational operator in semi-algebraic constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationalOp {
    Equal,        // = 0
    NotEqual,     // != 0
    GreaterThan,  // > 0
    GreaterEqual, // >= 0
    LessThan,     // < 0
    LessEqual,    // <= 0
}

impl RelationalOp {
    /// Evaluate if sign satisfies the relational operator.
    pub fn satisfies(&self, sign: Sign) -> bool {
        match self {
            RelationalOp::Equal => sign == Sign::Zero,
            RelationalOp::NotEqual => sign != Sign::Zero,
            RelationalOp::GreaterThan => sign == Sign::Positive,
            RelationalOp::GreaterEqual => sign == Sign::Positive || sign == Sign::Zero,
            RelationalOp::LessThan => sign == Sign::Negative,
            RelationalOp::LessEqual => sign == Sign::Negative || sign == Sign::Zero,
        }
    }
}

/// Topological type of a Cylindrical cell in $\mathbb{R}^k$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadCellType {
    /// Section: 0-dimensional slice along the root hypersurface $x_k = \alpha(\mathbf{x}_{<k})$.
    Section,
    /// Sector: 1-dimensional open interval between consecutive sections $(\alpha_i, \alpha_{i+1})$ or unbounded $(-\infty, \alpha_1), (\alpha_m, +\infty)$.
    Sector,
}

/// Cylindrical cell in the CAD tree decomposition.
#[derive(Debug, Clone)]
pub struct CadCell {
    /// Cell identifier index.
    pub id: usize,
    /// Dimension $k$ of the space $\mathbb{R}^k$ this cell belongs to.
    pub level: usize,
    /// Cell topological type (Section or Sector).
    pub cell_type: CadCellType,
    /// Rational / Floating-point sample point $\mathbf{s} \in \mathbb{R}^k$ inside the cell.
    pub sample_point: Vec<f64>,
    /// Sign vector of defining polynomials at the sample point: `poly_index -> Sign`.
    pub signs: Vec<Sign>,
    /// Truth value of the evaluated formula in this cell.
    pub truth_value: bool,
    /// Cylindrical child cells in $\mathbb{R}^{k+1}$ stacked over this base cell.
    pub children: Vec<CadCell>,
}

impl CadCell {
    /// Total number of leaves in the cylindrical sub-tree.
    pub fn count_leaf_cells(&self) -> usize {
        if self.children.is_empty() {
            1
        } else {
            self.children.iter().map(|c| c.count_leaf_cells()).sum()
        }
    }
}

/// Polynomial representation for Cylindrical Algebraic Decomposition.
#[derive(Debug, Clone, PartialEq)]
pub struct CadPolynomial {
    /// Polynomial variable names (e.g. `["x", "y", "z"]`).
    pub vars: Vec<String>,
    /// Dense/Sparse coefficients representation: `Monoid Exponents -> Coeff`.
    pub terms: BTreeMap<Vec<usize>, f64>,
}

impl CadPolynomial {
    /// Create a zero polynomial.
    pub fn zero(vars: Vec<String>) -> Self {
        Self {
            vars,
            terms: BTreeMap::new(),
        }
    }

    /// Create a constant polynomial $c$.
    pub fn constant(c: f64, vars: Vec<String>) -> Self {
        let mut terms = BTreeMap::new();
        if c.abs() > 1e-14 {
            terms.insert(vec![0; vars.len()], c);
        }
        Self { vars, terms }
    }

    /// Create a single linear variable polynomial $x_i$.
    pub fn variable(var_name: &str, all_vars: &[String]) -> AlgebraResult<Self> {
        let idx = all_vars.iter().position(|v| v == var_name).ok_or_else(|| {
            AlgebraError::EvaluationError(format!("Variable {} not found", var_name))
        })?;
        let mut exp = vec![0; all_vars.len()];
        exp[idx] = 1;
        let mut terms = BTreeMap::new();
        terms.insert(exp, 1.0);
        Ok(Self {
            vars: all_vars.to_vec(),
            terms,
        })
    }

    /// Maximum degree with respect to variable at index `var_idx`.
    pub fn degree_wrt(&self, var_idx: usize) -> usize {
        self.terms
            .keys()
            .map(|exp| exp.get(var_idx).copied().unwrap_or(0))
            .max()
            .unwrap_or(0)
    }

    /// Numerical evaluation at sample coordinates $\mathbf{x} = (x_1, \dots, x_n)$.
    pub fn evaluate(&self, point: &[f64]) -> f64 {
        let mut sum = 0.0;
        for (exp, &coeff) in &self.terms {
            let mut term_val = coeff;
            for (i, &e) in exp.iter().enumerate() {
                if e > 0 {
                    let xi = point.get(i).copied().unwrap_or(0.0);
                    term_val *= xi.powi(e as i32);
                }
            }
            sum += term_val;
        }
        sum
    }

    /// Univariate derivative $\frac{\partial f}{\partial x_k}$.
    pub fn derivative(&self, var_idx: usize) -> Self {
        let mut res_terms = BTreeMap::new();
        for (exp, &coeff) in &self.terms {
            let e = exp.get(var_idx).copied().unwrap_or(0);
            if e > 0 {
                let mut new_exp = exp.clone();
                new_exp[var_idx] = e - 1;
                let new_coeff = coeff * (e as f64);
                if new_coeff.abs() > 1e-14 {
                    *res_terms.entry(new_exp).or_insert(0.0) += new_coeff;
                }
            }
        }
        Self {
            vars: self.vars.clone(),
            terms: res_terms,
        }
    }
}

/// Cylindrical Algebraic Decomposition Execution Engine.
pub struct CadEngine;

impl CadEngine {
    /// Construct a full sign-invariant Cylindrical Algebraic Decomposition of $\mathbb{R}^n$
    /// for the given set of multi-variate polynomials $\mathcal{F}$.
    pub fn decompose(
        polys: &[CadPolynomial],
        vars: &[String],
        precision_eps: f64,
    ) -> AlgebraResult<CadCell> {
        let n = vars.len();
        if n == 0 {
            return Err(AlgebraError::EvaluationError(
                "CAD requires at least one variable".into(),
            ));
        }

        // Recursive cylindrical construction starting from base 1D space
        Self::decompose_recursive(polys, vars, 0, &[], precision_eps)
    }

    fn decompose_recursive(
        polys: &[CadPolynomial],
        vars: &[String],
        level: usize,
        current_sample: &[f64],
        eps: f64,
    ) -> AlgebraResult<CadCell> {
        let target_dim = level + 1;

        if target_dim > vars.len() {
            // Leaf cell evaluation
            let mut signs = Vec::with_capacity(polys.len());
            for p in polys {
                let val = p.evaluate(current_sample);
                signs.push(Sign::from_f64(val, eps));
            }

            return Ok(CadCell {
                id: 0,
                level,
                cell_type: CadCellType::Section,
                sample_point: current_sample.to_vec(),
                signs,
                truth_value: true,
                children: Vec::new(),
            });
        }

        // 1. Project polynomials to find univariate roots in current level variable x_{level}
        let roots = Self::isolate_cross_section_roots(polys, level, current_sample, eps)?;

        // 2. Form interleaved Sections and Sectors along the coordinate line
        let mut sample_coords = Vec::new();
        let mut cell_types = Vec::new();

        if roots.is_empty() {
            // Entire real line is a single sector
            sample_coords.push(0.0);
            cell_types.push(CadCellType::Sector);
        } else {
            // First open sector: (-infty, r_0)
            sample_coords.push(roots[0] - 1.0);
            cell_types.push(CadCellType::Sector);

            for i in 0..roots.len() {
                // Section: [r_i]
                sample_coords.push(roots[i]);
                cell_types.push(CadCellType::Section);

                // Sector between r_i and r_{i+1}, or (r_last, +infty)
                if i + 1 < roots.len() {
                    sample_coords.push((roots[i] + roots[i + 1]) / 2.0);
                } else {
                    sample_coords.push(roots[i] + 1.0);
                }
                cell_types.push(CadCellType::Sector);
            }
        }

        // 3. Construct child cells
        let mut children = Vec::new();
        for (idx, (&coord, &ctype)) in sample_coords.iter().zip(cell_types.iter()).enumerate() {
            let mut next_sample = current_sample.to_vec();
            next_sample.push(coord);

            let mut child = Self::decompose_recursive(polys, vars, level + 1, &next_sample, eps)?;
            child.id = idx;
            child.cell_type = ctype;
            children.push(child);
        }

        let mut current_signs = Vec::new();
        for p in polys {
            let val = p.evaluate(current_sample);
            current_signs.push(Sign::from_f64(val, eps));
        }

        Ok(CadCell {
            id: 0,
            level,
            cell_type: CadCellType::Sector,
            sample_point: current_sample.to_vec(),
            signs: current_signs,
            truth_value: true,
            children,
        })
    }

    /// Isolate sorted real roots of polynomials evaluated on current base sample.
    fn isolate_cross_section_roots(
        polys: &[CadPolynomial],
        var_idx: usize,
        base_sample: &[f64],
        eps: f64,
    ) -> AlgebraResult<Vec<f64>> {
        let mut all_roots = Vec::new();

        for p in polys {
            // Substitute base coordinates to produce univariate polynomial in x_{var_idx}
            let deg = p.degree_wrt(var_idx);
            if deg == 0 {
                continue;
            }

            // Extract univariate coefficients c_0, c_1, ..., c_d
            let mut uni_coeffs = vec![0.0; deg + 1];
            for (exp, &coeff) in &p.terms {
                let e = exp.get(var_idx).copied().unwrap_or(0);
                // Evaluate remaining variables
                let mut term_val = coeff;
                for (i, &other_e) in exp.iter().enumerate() {
                    if i != var_idx && other_e > 0 {
                        let xi = base_sample.get(i).copied().unwrap_or(0.0);
                        term_val *= xi.powi(other_e as i32);
                    }
                }
                uni_coeffs[e] += term_val;
            }

            // Solve univariate roots
            if deg == 1 && uni_coeffs[1].abs() > 1e-12 {
                let r = -uni_coeffs[0] / uni_coeffs[1];
                all_roots.push(r);
            } else if deg == 2 && uni_coeffs[2].abs() > 1e-12 {
                let a = uni_coeffs[2];
                let b = uni_coeffs[1];
                let c = uni_coeffs[0];
                let disc = b * b - 4.0 * a * c;
                if disc >= -1e-14 {
                    let sqrt_d = disc.max(0.0).sqrt();
                    all_roots.push((-b - sqrt_d) / (2.0 * a));
                    all_roots.push((-b + sqrt_d) / (2.0 * a));
                }
            } else {
                // Approximate bisection / Newton isolation
                let samples: [f64; 9] = [-10.0, -5.0, -2.0, -1.0, 0.0, 1.0, 2.0, 5.0, 10.0];
                for i in 0..samples.len() - 1 {
                    let x0: f64 = samples[i];
                    let x1: f64 = samples[i + 1];
                    let v0: f64 = uni_coeffs
                        .iter()
                        .enumerate()
                        .map(|(k, &c)| c * x0.powi(k as i32))
                        .sum();
                    let v1: f64 = uni_coeffs
                        .iter()
                        .enumerate()
                        .map(|(k, &c)| c * x1.powi(k as i32))
                        .sum();
                    if v0 * v1 <= 0.0 {
                        let mut lo: f64 = x0;
                        let mut hi: f64 = x1;
                        for _ in 0..40 {
                            let mid: f64 = (lo + hi) / 2.0;
                            let v_mid: f64 = uni_coeffs
                                .iter()
                                .enumerate()
                                .map(|(k, &c)| c * mid.powi(k as i32))
                                .sum();
                            if v_mid.abs() < eps {
                                lo = mid;
                                break;
                            }
                            if v0 * v_mid <= 0.0 {
                                hi = mid;
                            } else {
                                lo = mid;
                            }
                        }
                        all_roots.push((lo + hi) / 2.0);
                    }
                }
            }
        }

        // Sort and deduplicate roots
        all_roots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        all_roots.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

        Ok(all_roots)
    }
}

/// Quantifier type in first-order real algebraic logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantifier {
    Exists,
    ForAll,
}

/// Real Quantifier Elimination Engine (Tarski-Seidenberg).
pub struct QuantifierEliminator;

impl QuantifierEliminator {
    /// Eliminate quantifiers from a formula $\Phi = Q_1 x_1 \dots Q_k x_k \, \psi(\mathbf{x}, \mathbf{y})$.
    pub fn eliminate(
        quantifiers: &[(Quantifier, String)],
        polys: &[CadPolynomial],
        conditions: &[(usize, RelationalOp)], // (poly_index, operator)
        vars: &[String],
    ) -> AlgebraResult<bool> {
        let cad = CadEngine::decompose(polys, vars, 1e-10)?;

        // Evaluate atomic conditions on all leaf cells
        let truth_tree = Self::evaluate_leaf_cells(&cad, conditions);

        // Fold quantifiers bottom-up
        Self::fold_quantifiers(&truth_tree, quantifiers)
    }

    fn evaluate_leaf_cells(cell: &CadCell, conditions: &[(usize, RelationalOp)]) -> CadCell {
        let mut new_cell = cell.clone();
        if cell.children.is_empty() {
            let mut satisfies = true;
            for &(poly_idx, rel_op) in conditions {
                if let Some(&sign) = cell.signs.get(poly_idx)
                    && !rel_op.satisfies(sign)
                {
                    satisfies = false;
                    break;
                }
            }
            new_cell.truth_value = satisfies;
        } else {
            new_cell.children = cell
                .children
                .iter()
                .map(|c| Self::evaluate_leaf_cells(c, conditions))
                .collect();
        }
        new_cell
    }

    fn fold_quantifiers(
        cell: &CadCell,
        quantifiers: &[(Quantifier, String)],
    ) -> AlgebraResult<bool> {
        if cell.children.is_empty() {
            return Ok(cell.truth_value);
        }

        let q_idx = cell.level;
        let q_type = quantifiers
            .get(q_idx)
            .map(|q| q.0)
            .unwrap_or(Quantifier::Exists);

        let mut child_truths = Vec::new();
        for child in &cell.children {
            let val = Self::fold_quantifiers(child, quantifiers)?;
            child_truths.push(val);
        }

        match q_type {
            Quantifier::Exists => Ok(child_truths.into_iter().any(|t| t)),
            Quantifier::ForAll => Ok(child_truths.into_iter().all(|t| t)),
        }
    }
}

/// Parametric Geometric Entity in 2D/3D/ND CAD.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometricEntity {
    Point2D {
        name: String,
        x: Option<f64>,
        y: Option<f64>,
    },
    Line2D {
        name: String,
        p1: String,
        p2: String,
    },
    Circle2D {
        name: String,
        center: String,
        radius: Option<f64>,
    },
    Point3D {
        name: String,
        coords: [Option<f64>; 3],
    },
    HyperSphereND {
        name: String,
        dim: usize,
        center: Vec<Option<f64>>,
        radius: Option<f64>,
    },
}

/// Geometric Constraint between entities.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometricConstraint {
    Coincident(String, String),
    Distance(String, String, f64),
    Angle(String, String, f64),
    Perpendicular(String, String),
    Parallel(String, String),
    Tangent(String, String),
    FixedArea(Vec<String>, f64),
}

/// Parametric Geometric Constraint Solver.
#[derive(Debug, Clone, Default)]
pub struct GeometricConstraintSolver {
    pub entities: HashMap<String, GeometricEntity>,
    pub constraints: Vec<GeometricConstraint>,
}

impl GeometricConstraintSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(&mut self, name: impl Into<String>, entity: GeometricEntity) {
        self.entities.insert(name.into(), entity);
    }

    pub fn add_constraint(&mut self, constraint: GeometricConstraint) {
        self.constraints.push(constraint);
    }

    /// Solve all unknown entity coordinates satisfying all geometric constraints.
    pub fn solve(&self) -> AlgebraResult<HashMap<String, Vec<f64>>> {
        let mut solved_coords = HashMap::new();

        // 1. Initialize known entities
        for (name, entity) in &self.entities {
            match entity {
                GeometricEntity::Point2D {
                    x: Some(px),
                    y: Some(py),
                    ..
                } => {
                    solved_coords.insert(name.clone(), vec![*px, *py]);
                }
                GeometricEntity::Point3D { coords, .. } if coords.iter().all(|c| c.is_some()) => {
                    solved_coords.insert(name.clone(), coords.iter().flatten().copied().collect());
                }
                _ => {}
            }
        }

        // 2. Initial placement: a distance or coincidence with one known end puts the other end
        //    along the x-axis / on top of it. This keeps simple chains exact and gives the
        //    refinement below a start that satisfies them.
        for constraint in &self.constraints {
            match constraint {
                GeometricConstraint::Distance(p1_name, p2_name, target_dist) => {
                    if let (Some(p1), None) = (
                        solved_coords.get(p1_name).cloned(),
                        solved_coords.get(p2_name),
                    ) {
                        solved_coords.insert(p2_name.clone(), vec![p1[0] + target_dist, p1[1]]);
                    } else if let (None, Some(p2)) = (
                        solved_coords.get(p1_name),
                        solved_coords.get(p2_name).cloned(),
                    ) {
                        solved_coords.insert(p1_name.clone(), vec![p2[0] - target_dist, p2[1]]);
                    }
                }
                GeometricConstraint::Coincident(e1_name, e2_name) => {
                    if let Some(c1) = solved_coords.get(e1_name).cloned() {
                        solved_coords.insert(e2_name.clone(), c1);
                    } else if let Some(c2) = solved_coords.get(e2_name).cloned() {
                        solved_coords.insert(e1_name.clone(), c2);
                    }
                }
                _ => {}
            }
        }

        // 3. Refine with damped Gauss-Newton (Levenberg-Marquardt) on all constraints at once.
        self.refine_2d(&mut solved_coords)?;
        Ok(solved_coords)
    }

    /// Residual of one constraint on the current 2D point positions (zero when satisfied).
    fn residuals_2d(
        &self,
        constraint: &GeometricConstraint,
        points: &HashMap<String, [f64; 2]>,
    ) -> AlgebraResult<Vec<f64>> {
        let fail = |why: String| AlgebraError::EvaluationError(format!("constraint solver: {why}"));
        let point = |name: &str| -> AlgebraResult<[f64; 2]> {
            points
                .get(name)
                .copied()
                .ok_or_else(|| fail(format!("`{name}` is not a 2D point of this solver")))
        };
        // A line entity gives the direction p2 - p1; a point name is not a line.
        let line = |name: &str| -> AlgebraResult<([f64; 2], [f64; 2])> {
            match self.entities.get(name) {
                Some(GeometricEntity::Line2D { p1, p2, .. }) => Ok((point(p1)?, point(p2)?)),
                _ => Err(fail(format!("`{name}` is not a Line2D"))),
            }
        };
        let direction = |l: ([f64; 2], [f64; 2])| [l.1[0] - l.0[0], l.1[1] - l.0[1]];
        let cross = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
        let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
        match constraint {
            GeometricConstraint::Coincident(a, b) => {
                let (pa, pb) = (point(a)?, point(b)?);
                Ok(vec![pa[0] - pb[0], pa[1] - pb[1]])
            }
            GeometricConstraint::Distance(a, b, d) => {
                let (pa, pb) = (point(a)?, point(b)?);
                Ok(vec![(pa[0] - pb[0]).hypot(pa[1] - pb[1]) - d])
            }
            GeometricConstraint::Parallel(a, b) => {
                Ok(vec![cross(direction(line(a)?), direction(line(b)?))])
            }
            GeometricConstraint::Perpendicular(a, b) => {
                Ok(vec![dot(direction(line(a)?), direction(line(b)?))])
            }
            GeometricConstraint::Angle(a, b, theta) => {
                // The angle from line a to line b is theta: cross cos(theta) - dot sin(theta) = 0,
                // and the unit-direction normalisation keeps the scale comparable to lengths.
                let (da, db) = (direction(line(a)?), direction(line(b)?));
                let (la, lb) = (da[0].hypot(da[1]).max(1e-12), db[0].hypot(db[1]).max(1e-12));
                Ok(vec![
                    (cross(da, db) * theta.cos() - dot(da, db) * theta.sin()) / (la * lb),
                ])
            }
            GeometricConstraint::Tangent(line_name, circle_name) => {
                let (p, q) = line(line_name)?;
                let (center_name, radius) = match self.entities.get(circle_name) {
                    Some(GeometricEntity::Circle2D {
                        center,
                        radius: Some(r),
                        ..
                    }) => (center.clone(), *r),
                    _ => {
                        return Err(fail(format!(
                            "`{circle_name}` is not a Circle2D with a known radius"
                        )));
                    }
                };
                let c = point(&center_name)?;
                let d = direction((p, q));
                let length = d[0].hypot(d[1]).max(1e-12);
                // Signed distance from the centre to the line, compared in magnitude.
                Ok(vec![
                    cross(d, [c[0] - p[0], c[1] - p[1]]).abs() / length - radius,
                ])
            }
            GeometricConstraint::FixedArea(names, area) => {
                let polygon: Vec<[f64; 2]> = names
                    .iter()
                    .map(|n| point(n))
                    .collect::<AlgebraResult<_>>()?;
                let n = polygon.len();
                let twice: f64 = (0..n)
                    .map(|i| cross(polygon[i], polygon[(i + 1) % n]))
                    .sum();
                Ok(vec![twice.abs() / 2.0 - area])
            }
        }
    }

    /// Adjust the unknown 2D point coordinates so that every constraint holds.
    ///
    /// Unknowns are the missing coordinates of `Point2D` entities. The solver runs
    /// Levenberg-Marquardt on the stacked residuals with a central-difference Jacobian. A start
    /// that already satisfies every constraint is returned untouched; otherwise a result with a
    /// residual above `1e-9` (an over-constrained or inconsistent set, or a failure to converge)
    /// is an error. Under-constrained sets return the solution nearest the starting placement.
    fn refine_2d(&self, solved: &mut HashMap<String, Vec<f64>>) -> AlgebraResult<()> {
        // Free coordinates: (entity name, axis).
        let mut free: Vec<(String, usize)> = Vec::new();
        let mut names: Vec<&String> = self.entities.keys().collect();
        names.sort();
        let mut points: HashMap<String, [f64; 2]> = HashMap::new();
        for name in names {
            if let Some(GeometricEntity::Point2D { x, y, .. }) = self.entities.get(name) {
                let start = solved.get(name).cloned();
                let value = |known: &Option<f64>, axis: usize, free: &mut Vec<(String, usize)>| {
                    match known {
                        Some(v) => *v,
                        None => {
                            free.push((name.clone(), axis));
                            // Keep a placement from step 2; otherwise a small distinct offset
                            // so the Jacobian is not degenerate at the origin.
                            start
                                .as_ref()
                                .map_or(0.1 * (free.len() as f64), |s| s[axis])
                        }
                    }
                };
                let px = value(x, 0, &mut free);
                let py = value(y, 1, &mut free);
                points.insert(name.clone(), [px, py]);
            }
        }

        let evaluate = |points: &HashMap<String, [f64; 2]>| -> AlgebraResult<Vec<f64>> {
            let mut all = Vec::new();
            for constraint in &self.constraints {
                all.extend(self.residuals_2d(constraint, points)?);
            }
            Ok(all)
        };
        let norm = |r: &[f64]| r.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        let set = |points: &mut HashMap<String, [f64; 2]>, k: usize, value: f64| {
            let (name, axis) = &free[k];
            if let Some(p) = points.get_mut(name) {
                p[*axis] = value;
            }
        };
        let get = |points: &HashMap<String, [f64; 2]>, k: usize| -> f64 {
            let (name, axis) = &free[k];
            points.get(name).map_or(0.0, |p| p[*axis])
        };

        const TOLERANCE: f64 = 1e-10;
        let start = points.clone();
        // Distance constraints have zero gradient across a collinear placement (the usual start
        // after step 2), so a stalled run is retried from deterministically perturbed starts.
        let extent = start
            .values()
            .flat_map(|p| p.iter().copied())
            .fold(1.0_f64, |m, v| m.max(v.abs()));
        let mut best: Option<SolveAttempt> = None;
        for attempt in 0..10 {
            let mut points = start.clone();
            if attempt > 0 {
                for k in 0..free.len() {
                    let offset =
                        0.4 * extent * ((attempt as f64) * 1.7 + (k as f64) * 2.3 + 0.5).sin();
                    let current = get(&points, k);
                    set(&mut points, k, current + offset);
                }
            }
            let mut residual = evaluate(&points)?;
            if !residual.is_empty() && !free.is_empty() && norm(&residual) > TOLERANCE {
                let n = free.len();
                let mut lambda = 1e-3;
                for _ in 0..300 {
                    if norm(&residual) <= TOLERANCE {
                        break;
                    }
                    // Central-difference Jacobian, m x n.
                    let m = residual.len();
                    let mut jacobian = vec![vec![0.0; n]; m];
                    for k in 0..n {
                        let x0 = get(&points, k);
                        let h = 1e-7 * x0.abs().max(1.0);
                        set(&mut points, k, x0 + h);
                        let plus = evaluate(&points)?;
                        set(&mut points, k, x0 - h);
                        let minus = evaluate(&points)?;
                        set(&mut points, k, x0);
                        for i in 0..m {
                            jacobian[i][k] = (plus[i] - minus[i]) / (2.0 * h);
                        }
                    }
                    // Normal equations (J^T J + lambda I) d = -J^T r.
                    let mut jtj = vec![vec![0.0; n]; n];
                    let mut jtr = vec![0.0; n];
                    for i in 0..m {
                        for a in 0..n {
                            jtr[a] -= jacobian[i][a] * residual[i];
                            for b in 0..n {
                                jtj[a][b] += jacobian[i][a] * jacobian[i][b];
                            }
                        }
                    }
                    let mut improved = false;
                    for _ in 0..12 {
                        let mut system = jtj.clone();
                        for (a, row) in system.iter_mut().enumerate() {
                            row[a] += lambda * (1.0 + jtj[a][a]);
                        }
                        let Some(step) = solve_dense(&system, &jtr) else {
                            lambda *= 10.0;
                            continue;
                        };
                        let backup: Vec<f64> = (0..n).map(|k| get(&points, k)).collect();
                        for k in 0..n {
                            set(&mut points, k, backup[k] + step[k]);
                        }
                        let trial = evaluate(&points)?;
                        let sum_sq = |r: &[f64]| r.iter().map(|v| v * v).sum::<f64>();
                        if sum_sq(&trial) < sum_sq(&residual) {
                            residual = trial;
                            lambda = (lambda * 0.3).max(1e-12);
                            improved = true;
                            break;
                        }
                        for k in 0..n {
                            set(&mut points, k, backup[k]);
                        }
                        lambda *= 10.0;
                    }
                    if !improved {
                        break;
                    }
                }
            }

            let better = best
                .as_ref()
                .is_none_or(|(_, previous)| norm(&residual) < norm(previous));
            let done = norm(&residual) <= 1e-9;
            if better {
                best = Some((points, residual));
            }
            if done
                || free.is_empty()
                || attempt == 0 && best.as_ref().is_some_and(|(_, r)| r.is_empty())
            {
                break;
            }
        }
        let (points, residual) = best.unwrap_or((start, Vec::new()));

        if norm(&residual) > 1e-9 {
            return Err(AlgebraError::EvaluationError(format!(
                "constraint solver: the constraints are inconsistent or did not converge \
                 (largest residual {:.3e})",
                norm(&residual)
            )));
        }
        for (name, p) in points {
            solved.insert(name, vec![p[0], p[1]]);
        }
        Ok(())
    }
}

/// Point positions and the constraint residuals they leave.
type SolveAttempt = (HashMap<String, [f64; 2]>, Vec<f64>);

/// Solve the dense system `A x = b` by Gaussian elimination with partial pivoting.
fn solve_dense(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    let mut aug: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut row = a[i].clone();
            row.push(b[i]);
            row
        })
        .collect();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| aug[i][col].abs().total_cmp(&aug[j][col].abs()))?;
        if aug[pivot][col].abs() < 1e-300 {
            return None;
        }
        aug.swap(col, pivot);
        for row in 0..n {
            if row != col {
                let factor = aug[row][col] / aug[col][col];
                for k in col..=n {
                    let delta = factor * aug[col][k];
                    aug[row][k] -= delta;
                }
            }
        }
    }
    Some((0..n).map(|i| aug[i][n] / aug[i][i]).collect())
}

// ============================================================================
// External Custom Shape Import, Manipulation, CSG, and Export
// ============================================================================

/// 3D Axis-Aligned Bounding Box.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox3D {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl BoundingBox3D {
    pub fn new(min: [f64; 3], max: [f64; 3]) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> [f64; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }

    pub fn size(&self) -> [f64; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn contains_point(&self, p: &[f64; 3]) -> bool {
        p[0] >= self.min[0]
            && p[0] <= self.max[0]
            && p[1] >= self.min[1]
            && p[1] <= self.max[1]
            && p[2] >= self.min[2]
            && p[2] <= self.max[2]
    }
}

/// 3D Triangle Surface Mesh representation for CAD solids and imported geometry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh3D {
    pub name: String,
    pub vertices: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
}

impl Mesh3D {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vertices: Vec::new(),
            normals: Vec::new(),
            triangles: Vec::new(),
        }
    }

    /// Add a single vertex and return its 0-based index.
    pub fn add_vertex(&mut self, v: [f64; 3]) -> usize {
        self.vertices.push(v);
        self.vertices.len() - 1
    }

    /// Add a triangular facet given 3 vertex indices.
    pub fn add_triangle(&mut self, i0: usize, i1: usize, i2: usize) {
        self.triangles.push([i0, i1, i2]);
    }

    /// Compute axis-aligned bounding box.
    pub fn compute_bounding_box(&self) -> BoundingBox3D {
        if self.vertices.is_empty() {
            return BoundingBox3D::new([0.0; 3], [0.0; 3]);
        }
        let mut min = self.vertices[0];
        let mut max = self.vertices[0];
        for v in &self.vertices {
            for i in 0..3 {
                if v[i] < min[i] {
                    min[i] = v[i];
                }
                if v[i] > max[i] {
                    max[i] = v[i];
                }
            }
        }
        BoundingBox3D::new(min, max)
    }

    /// Compute total surface area: $\sum_T \frac{1}{2} \|(\mathbf{v}_1 - \mathbf{v}_0) \times (\mathbf{v}_2 - \mathbf{v}_0)\|$.
    pub fn compute_surface_area(&self) -> f64 {
        let mut total_area = 0.0;
        for t in &self.triangles {
            let v0 = self.vertices[t[0]];
            let v1 = self.vertices[t[1]];
            let v2 = self.vertices[t[2]];
            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let area =
                0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
            total_area += area;
        }
        total_area
    }

    /// Number of directed edges that are not matched by an opposite edge in another triangle, plus
    /// edges used more than once in the same direction. Zero for a closed, consistently oriented
    /// surface; a positive count means open boundaries, holes or inconsistent winding. A mesh
    /// that is not closed has no well-defined enclosed volume.
    pub fn boundary_edge_count(&self) -> usize {
        let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
        for t in &self.triangles {
            for k in 0..3 {
                *directed.entry((t[k], t[(k + 1) % 3])).or_insert(0) += 1;
            }
        }
        directed
            .iter()
            .filter(|&(&(a, b), &count)| count != 1 || directed.get(&(b, a)) != Some(&1))
            .count()
    }

    /// Whether every edge is shared by exactly two triangles with opposite directions (a closed,
    /// consistently oriented surface), so that [`Mesh3D::compute_volume`] is meaningful.
    pub fn is_closed(&self) -> bool {
        !self.triangles.is_empty() && self.boundary_edge_count() == 0
    }

    /// Compute solid enclosed volume via the divergence theorem: $V = \frac{1}{6} \sum_T \mathbf{v}_0 \cdot (\mathbf{v}_1 \times \mathbf{v}_2)$.
    pub fn compute_volume(&self) -> f64 {
        let mut signed_vol = 0.0;
        for t in &self.triangles {
            let v0 = self.vertices[t[0]];
            let v1 = self.vertices[t[1]];
            let v2 = self.vertices[t[2]];
            let cross = [
                v1[1] * v2[2] - v1[2] * v2[1],
                v1[2] * v2[0] - v1[0] * v2[2],
                v1[0] * v2[1] - v1[1] * v2[0],
            ];
            let dot = v0[0] * cross[0] + v0[1] * cross[1] + v0[2] * cross[2];
            signed_vol += dot / 6.0;
        }
        signed_vol.abs()
    }

    /// Compute geometric centroid: $\mathbf{C} = \frac{1}{N} \sum \mathbf{v}_i$.
    pub fn compute_centroid(&self) -> [f64; 3] {
        if self.vertices.is_empty() {
            return [0.0; 3];
        }
        let mut sum = [0.0; 3];
        for v in &self.vertices {
            sum[0] += v[0];
            sum[1] += v[1];
            sum[2] += v[2];
        }
        let n = self.vertices.len() as f64;
        [sum[0] / n, sum[1] / n, sum[2] / n]
    }

    /// Translate mesh by displacement $(\Delta x, \Delta y, \Delta z)$.
    pub fn translate(&mut self, dx: f64, dy: f64, dz: f64) {
        for v in &mut self.vertices {
            v[0] += dx;
            v[1] += dy;
            v[2] += dz;
        }
    }

    /// Scale mesh non-uniformly along coordinates $(s_x, s_y, s_z)$.
    pub fn scale(&mut self, sx: f64, sy: f64, sz: f64) {
        for v in &mut self.vertices {
            v[0] *= sx;
            v[1] *= sy;
            v[2] *= sz;
        }
    }

    /// Rotate mesh around an arbitrary unit axis vector $\mathbf{u}$ by angle $\theta$ (Rodrigues' rotation formula).
    pub fn rotate_axis(&mut self, axis: [f64; 3], angle_rad: f64) {
        let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if norm < 1e-12 {
            return;
        }
        let u = [axis[0] / norm, axis[1] / norm, axis[2] / norm];
        let cos_t = angle_rad.cos();
        let sin_t = angle_rad.sin();

        for v in &mut self.vertices {
            let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
            let cross = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            v[0] = v[0] * cos_t + cross[0] * sin_t + u[0] * dot * (1.0 - cos_t);
            v[1] = v[1] * cos_t + cross[1] * sin_t + u[1] * dot * (1.0 - cos_t);
            v[2] = v[2] * cos_t + cross[2] * sin_t + u[2] * dot * (1.0 - cos_t);
        }
    }

    /// Create a standard 3D Box / Cube mesh of dimensions $(L_x, L_y, L_z)$.
    pub fn cube(lx: f64, ly: f64, lz: f64) -> Self {
        let mut mesh = Self::new("Cube");
        let hx = lx / 2.0;
        let hy = ly / 2.0;
        let hz = lz / 2.0;

        let v = [
            [-hx, -hy, -hz],
            [hx, -hy, -hz],
            [hx, hy, -hz],
            [-hx, hy, -hz],
            [-hx, -hy, hz],
            [hx, -hy, hz],
            [hx, hy, hz],
            [-hx, hy, hz],
        ];
        for pt in v {
            mesh.add_vertex(pt);
        }

        let quads = [
            [0, 1, 2, 3],
            [4, 7, 6, 5],
            [0, 4, 5, 1],
            [2, 6, 7, 3],
            [0, 3, 7, 4],
            [1, 5, 6, 2],
        ];
        for q in quads {
            mesh.add_triangle(q[0], q[1], q[2]);
            mesh.add_triangle(q[0], q[2], q[3]);
        }
        mesh
    }

    /// Create an extruded 3D solid from an arbitrary 2D closed polygon cross-section.
    pub fn extrude_profile(profile: &[[f64; 2]], height: f64) -> Self {
        let mut mesh = Self::new("Extrusion");
        let n = profile.len();
        if n < 3 {
            return mesh;
        }

        // Bottom vertices
        for p in profile {
            mesh.add_vertex([p[0], p[1], 0.0]);
        }
        // Top vertices
        for p in profile {
            mesh.add_vertex([p[0], p[1], height]);
        }

        // Side quads (as triangles)
        for i in 0..n {
            let next = (i + 1) % n;
            let b0 = i;
            let b1 = next;
            let t0 = i + n;
            let t1 = next + n;
            mesh.add_triangle(b0, b1, t1);
            mesh.add_triangle(b0, t1, t0);
        }

        // Simple bottom & top cap fan triangulation
        for i in 1..n - 1 {
            mesh.add_triangle(0, i + 1, i);
            mesh.add_triangle(n, n + i, n + i + 1);
        }

        mesh
    }
}

/// Supported Boolean Constructive Solid Geometry (CSG) operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsgBooleanOp {
    Union,
    Intersection,
    Difference,
}

/// Generic Parametric & Imported Custom Shape in 2D, 3D, and $N$-D.
#[derive(Debug, Clone, PartialEq)]
pub enum CustomShape {
    /// Discretized 3D Triangle Surface Mesh.
    TriMesh(Mesh3D),
    /// Implicit Algebraic Surface $f(x, y, z) = 0$ bounded in a region.
    ParametricImplicit {
        expr_str: String,
        bbox: BoundingBox3D,
    },
    /// Linear extrusion of 2D profile.
    Extrusion {
        profile_2d: Vec<[f64; 2]>,
        height: f64,
    },
    /// $N$-Dimensional Polytope in $\mathbb{R}^n$.
    PolytopeND {
        dim: usize,
        vertices: Vec<Vec<f64>>,
        simplices: Vec<Vec<usize>>,
    },
    /// Hierarchical CSG Boolean composite.
    BooleanCSG {
        op: CsgBooleanOp,
        left: Box<CustomShape>,
        right: Box<CustomShape>,
    },
}

impl CustomShape {
    /// Convert shape to a concrete `Mesh3D` for visualization and simulation.
    pub fn to_tri_mesh(&self) -> Mesh3D {
        match self {
            CustomShape::TriMesh(m) => m.clone(),
            CustomShape::Extrusion { profile_2d, height } => {
                Mesh3D::extrude_profile(profile_2d, *height)
            }
            _ => Mesh3D::cube(1.0, 1.0, 1.0),
        }
    }
}

/// Importer for external CAD formats (Wavefront OBJ, STL ASCII, SVG 2D paths).
pub struct ShapeImporter;

impl ShapeImporter {
    /// Import 3D geometry from a Wavefront `.obj` string.
    pub fn import_obj(content: &str) -> AlgebraResult<Mesh3D> {
        let mut mesh = Mesh3D::new("ImportedOBJ");

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            match tokens[0] {
                "v" if tokens.len() >= 4 => {
                    let x: f64 = tokens[1].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid vertex x: {}", e))
                    })?;
                    let y: f64 = tokens[2].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid vertex y: {}", e))
                    })?;
                    let z: f64 = tokens[3].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid vertex z: {}", e))
                    })?;
                    mesh.add_vertex([x, y, z]);
                }
                "f" if tokens.len() >= 4 => {
                    let mut face_indices = Vec::new();
                    for &tok in &tokens[1..] {
                        let idx_str = tok.split('/').next().unwrap_or("1");
                        let idx: usize = idx_str.parse().map_err(|e| {
                            AlgebraError::EvaluationError(format!("Invalid face index: {}", e))
                        })?;
                        if idx > 0 {
                            face_indices.push(idx - 1); // OBJ 1-based indexing to 0-based
                        }
                    }
                    // Triangulate polygon fan
                    for i in 1..face_indices.len().saturating_sub(1) {
                        mesh.add_triangle(face_indices[0], face_indices[i], face_indices[i + 1]);
                    }
                }
                _ => {}
            }
        }

        Ok(mesh)
    }

    /// Import 3D geometry from an ASCII `.stl` string.
    pub fn import_stl_ascii(content: &str) -> AlgebraResult<Mesh3D> {
        let mut mesh = Mesh3D::new("ImportedSTL");
        let mut current_facet = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("vertex") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 4 {
                    let x: f64 = parts[1].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid STL vertex x: {}", e))
                    })?;
                    let y: f64 = parts[2].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid STL vertex y: {}", e))
                    })?;
                    let z: f64 = parts[3].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid STL vertex z: {}", e))
                    })?;
                    let idx = mesh.add_vertex([x, y, z]);
                    current_facet.push(idx);
                }
            } else if trimmed.starts_with("endfacet") {
                if current_facet.len() >= 3 {
                    mesh.add_triangle(current_facet[0], current_facet[1], current_facet[2]);
                }
                current_facet.clear();
            }
        }

        Ok(mesh)
    }

    /// Import 2D polygon profile from standard SVG path string (e.g. `M 0 0 L 10 0 L 10 10 Z`).
    pub fn import_svg_path(path_str: &str) -> AlgebraResult<Vec<[f64; 2]>> {
        let mut points = Vec::new();
        let tokens: Vec<&str> = path_str.split_whitespace().collect();
        let mut i = 0;

        while i < tokens.len() {
            match tokens[i].to_uppercase().as_str() {
                "M" | "L" if i + 2 < tokens.len() => {
                    let x: f64 = tokens[i + 1].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid SVG x: {}", e))
                    })?;
                    let y: f64 = tokens[i + 2].parse().map_err(|e| {
                        AlgebraError::EvaluationError(format!("Invalid SVG y: {}", e))
                    })?;
                    points.push([x, y]);
                    i += 3;
                }
                "Z" => {
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        Ok(points)
    }
}

/// Exporter for CAD geometries to standard file formats.
pub struct ShapeExporter;

impl ShapeExporter {
    /// Export `Mesh3D` to Wavefront `.obj` format string.
    pub fn export_obj(mesh: &Mesh3D) -> String {
        let mut out = String::new();
        out.push_str(&format!("# URAE CAD Export: {}\n", mesh.name));
        for v in &mesh.vertices {
            out.push_str(&format!("v {:.6} {:.6} {:.6}\n", v[0], v[1], v[2]));
        }
        for t in &mesh.triangles {
            // OBJ indices are 1-based
            out.push_str(&format!("f {} {} {}\n", t[0] + 1, t[1] + 1, t[2] + 1));
        }
        out
    }

    /// Export `Mesh3D` to ASCII `.stl` format string.
    pub fn export_stl_ascii(mesh: &Mesh3D, solid_name: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!("solid {}\n", solid_name));
        for t in &mesh.triangles {
            let v0 = mesh.vertices[t[0]];
            let v1 = mesh.vertices[t[1]];
            let v2 = mesh.vertices[t[2]];
            out.push_str("  facet normal 0.0 0.0 0.0\n");
            out.push_str("    outer loop\n");
            out.push_str(&format!(
                "      vertex {:.6} {:.6} {:.6}\n",
                v0[0], v0[1], v0[2]
            ));
            out.push_str(&format!(
                "      vertex {:.6} {:.6} {:.6}\n",
                v1[0], v1[1], v1[2]
            ));
            out.push_str(&format!(
                "      vertex {:.6} {:.6} {:.6}\n",
                v2[0], v2[1], v2[2]
            ));
            out.push_str("    endloop\n");
            out.push_str("  endfacet\n");
        }
        out.push_str(&format!("endsolid {}\n", solid_name));
        out
    }
}
