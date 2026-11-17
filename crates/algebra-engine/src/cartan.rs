//! # `algebra_engine::cartan`
//!
//! Cartan Exterior Differential Calculus, Differential Forms $\Omega^k(M)$,
//! Exterior Derivative $\mathrm{d}$, Interior Contraction $\iota_X$,
//! Lie Derivative $\mathcal{L}_X$, and Hodge Star Dual $\star$.
//!
//! ## Mathematical Foundations
//! - **$k$-Forms**: $\omega = \sum_{1 \le i_1 < \dots < i_k \le n} \omega_{i_1 \dots i_k} \mathrm{d}x^{i_1} \wedge \dots \wedge \mathrm{d}x^{i_k} \in \Omega^k(M)$.
//! - **Wedge Product**: $\alpha \wedge \beta = (-1)^{\deg(\alpha) \deg(\beta)} \beta \wedge \alpha$.
//! - **Exterior Derivative**: $\mathrm{d}: \Omega^k \to \Omega^{k+1}$, satisfying $\mathrm{d}^2 = 0$ and the graded Leibniz rule:
//!   $$\mathrm{d}(\alpha \wedge \beta) = \mathrm{d}\alpha \wedge \beta + (-1)^{\deg(\alpha)} \alpha \wedge \mathrm{d}\beta$$
//! - **Interior Product (Contraction)**: $\iota_X: \Omega^k \to \Omega^{k-1}$ along vector field $X$.
//! - **Cartan's Magic Formula**:
//!   $$\mathcal{L}_X \omega = (\mathrm{d}\iota_X + \iota_X\mathrm{d})\omega$$
//! - **Hodge Star Dual**: $\star: \Omega^k \to \Omega^{n-k}$ satisfying $\star \star \omega = (-1)^{k(n-k)} \operatorname{sgn}(g) \omega$.
//! - **Laplace-de Rham Operator**: $\Delta = \mathrm{d}\delta + \delta\mathrm{d}$ where $\delta = (-1)^{n(k-1)+1} \star \mathrm{d} \star$.

use algebra_core::error::{AlgebraError, AlgebraResult};
use std::collections::BTreeMap;

/// Multi-index basis blade representing ordered wedge product: $\mathrm{d}x^{i_1} \wedge \dots \wedge \mathrm{d}x^{i_k}$.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FormBasis {
    /// Strictly sorted coordinate indices (0-indexed).
    pub indices: Vec<usize>,
}

impl FormBasis {
    /// Create a scalar (0-form) basis blade $\mathbf{1}$.
    pub fn scalar() -> Self {
        Self {
            indices: Vec::new(),
        }
    }

    /// Create a 1-form basis blade $\mathrm{d}x^i$.
    pub fn one_form(i: usize) -> Self {
        Self { indices: vec![i] }
    }

    /// Create a $k$-form basis blade $\mathrm{d}x^{i_1} \wedge \dots \wedge \mathrm{d}x^{i_k}$.
    pub fn new(indices: Vec<usize>) -> (Self, i64) {
        let mut idx = indices;
        let mut sign = 1i64;
        let n = idx.len();

        // Bubble sort to determine permutation parity and check for duplicates (dx^i ∧ dx^i = 0)
        for i in 0..n {
            for j in 0..n.saturating_sub(i + 1) {
                if idx[j] == idx[j + 1] {
                    return (
                        Self {
                            indices: Vec::new(),
                        },
                        0,
                    );
                }
                if idx[j] > idx[j + 1] {
                    idx.swap(j, j + 1);
                    sign = -sign;
                }
            }
        }

        (Self { indices: idx }, sign)
    }

    /// Degree $k$ of the differential form.
    pub fn degree(&self) -> usize {
        self.indices.len()
    }
}

/// Differential Form $\omega \in \Omega^k(M)$ with floating-point / analytical polynomial coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct DifferentialForm {
    /// Dimension $n$ of the manifold manifold $M^n$.
    pub dim: usize,
    /// Degree $k$ of the form.
    pub degree: usize,
    /// Coordinate names (e.g. `["x", "y", "z"]`).
    pub coord_names: Vec<String>,
    /// Terms mapping basis blade to its symbolic / numerical polynomial coefficient.
    /// Representation: `Basis -> Coefficient`.
    pub terms: BTreeMap<FormBasis, f64>,
}

/// Tangent vector field $X = \sum_{i=1}^n v^i(x) \partial_{x_i}$.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorField {
    pub dim: usize,
    pub coord_names: Vec<String>,
    pub components: Vec<f64>,
}

impl DifferentialForm {
    /// Parse a form with constant coefficients such as `dx`, `2 dx^dy`, `dx + 3 dy` or `5`.
    ///
    /// Terms are separated by `+` or `-`; a term is an optional numeric coefficient followed by a
    /// wedge (`^`, `∧` or `*`) of coordinate differentials named after `coord_names`. Every term
    /// must have the same degree. Anything else is an error.
    pub fn parse(input: &str, dim: usize, coord_names: &[String]) -> AlgebraResult<Self> {
        let invalid = |why: String| AlgebraError::EvaluationError(format!("form `{input}`: {why}"));
        let text: String = input.replace('∧', "^").replace(' ', "");
        if text.is_empty() {
            return Err(invalid("empty".into()));
        }
        // Split into signed terms.
        let mut start = 0;
        let mut sign = 1.0;
        let bytes: Vec<char> = text.chars().collect();
        let mut pieces: Vec<(f64, String)> = Vec::new();
        for (i, &ch) in bytes.iter().enumerate() {
            if (ch == '+' || ch == '-') && i > start {
                pieces.push((sign, bytes[start..i].iter().collect()));
                sign = if ch == '-' { -1.0 } else { 1.0 };
                start = i + 1;
            } else if (ch == '+' || ch == '-') && i == start {
                sign = if ch == '-' { -sign } else { sign };
                start = i + 1;
            }
        }
        pieces.push((sign, bytes[start..].iter().collect()));

        let mut form: Option<DifferentialForm> = None;
        for (sign, piece) in pieces {
            if piece.is_empty() {
                return Err(invalid("dangling operator".into()));
            }
            let mut coefficient = sign;
            let mut indices: Vec<usize> = Vec::new();
            for factor in piece.split(['^', '*']) {
                // A coefficient may be glued to the differential (`2dx`, spaces removed above).
                let (number, differential) = match factor.find('d') {
                    Some(at) if at > 0 => factor.split_at(at),
                    _ => ("", factor),
                };
                if !number.is_empty() {
                    coefficient *= number
                        .parse::<f64>()
                        .map_err(|_| invalid(format!("cannot read `{number}`")))?;
                }
                let factor = differential;
                if let Ok(v) = factor.parse::<f64>() {
                    coefficient *= v;
                } else if let Some(name) = factor.strip_prefix('d') {
                    let index = coord_names
                        .iter()
                        .position(|c| c == name)
                        .filter(|&i| i < dim)
                        .ok_or_else(|| invalid(format!("unknown coordinate `{name}`")))?;
                    indices.push(index);
                } else {
                    return Err(invalid(format!("cannot read `{factor}`")));
                }
            }
            let (basis, blade_sign) = FormBasis::new(indices.clone());
            let term = if blade_sign == 0 {
                DifferentialForm::zero(dim, indices.len(), coord_names.to_vec())
            } else {
                let mut map = BTreeMap::new();
                map.insert(basis, coefficient * blade_sign as f64);
                DifferentialForm {
                    dim,
                    degree: indices.len(),
                    coord_names: coord_names.to_vec(),
                    terms: map,
                }
            };
            form = Some(match form {
                None => term,
                Some(acc) => acc.add(&term)?,
            });
        }
        form.ok_or_else(|| invalid("no terms".into()))
    }

    /// Render as `2 dx∧dy + 3 dz`, or `0` for the zero form.
    pub fn to_text(&self) -> String {
        if self.terms.is_empty() {
            return "0".to_string();
        }
        let mut out = String::new();
        for (basis, &coefficient) in &self.terms {
            let blade = basis
                .indices
                .iter()
                .map(|&i| format!("d{}", self.coord_names.get(i).map_or("?", String::as_str)))
                .collect::<Vec<_>>()
                .join("∧");
            let magnitude = coefficient.abs();
            let body = match (blade.is_empty(), (magnitude - 1.0).abs() < 1e-12) {
                (true, _) => format!("{magnitude}"),
                (false, true) => blade,
                (false, false) => format!("{magnitude} {blade}"),
            };
            if out.is_empty() {
                if coefficient < 0.0 {
                    out.push('-');
                }
            } else {
                out.push_str(if coefficient < 0.0 { " - " } else { " + " });
            }
            out.push_str(&body);
        }
        out
    }
}

impl DifferentialForm {
    /// Create a zero $k$-form in $\mathbb{R}^n$.
    pub fn zero(dim: usize, degree: usize, coord_names: Vec<String>) -> Self {
        Self {
            dim,
            degree,
            coord_names,
            terms: BTreeMap::new(),
        }
    }

    /// Create a constant 0-form (scalar).
    pub fn scalar(val: f64, dim: usize, coord_names: Vec<String>) -> Self {
        let mut terms = BTreeMap::new();
        if val.abs() > 1e-15 {
            terms.insert(FormBasis::scalar(), val);
        }
        Self {
            dim,
            degree: 0,
            coord_names,
            terms,
        }
    }

    /// Create an elementary 1-form $\mathrm{d}x^i$.
    pub fn dx(i: usize, dim: usize, coord_names: Vec<String>) -> AlgebraResult<Self> {
        if i >= dim {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Coordinate index {} exceeds manifold dimension {}",
                i, dim
            )));
        }
        let mut terms = BTreeMap::new();
        terms.insert(FormBasis::one_form(i), 1.0);
        Ok(Self {
            dim,
            degree: 1,
            coord_names,
            terms,
        })
    }

    /// Add another differential form of the same degree.
    pub fn add(&self, other: &DifferentialForm) -> AlgebraResult<DifferentialForm> {
        if self.dim != other.dim || self.degree != other.degree {
            return Err(AlgebraError::EvaluationError(
                "Cannot add differential forms of different degrees or manifold dimensions".into(),
            ));
        }

        let mut res_terms = self.terms.clone();
        for (basis, &coeff) in &other.terms {
            *res_terms.entry(basis.clone()).or_insert(0.0) += coeff;
        }

        // Clean near-zero coefficients
        res_terms.retain(|_, v| v.abs() > 1e-14);

        Ok(DifferentialForm {
            dim: self.dim,
            degree: self.degree,
            coord_names: self.coord_names.clone(),
            terms: res_terms,
        })
    }

    /// Scale differential form by a scalar $c \cdot \omega$.
    pub fn scale(&self, scalar: f64) -> DifferentialForm {
        let mut res_terms = BTreeMap::new();
        if scalar.abs() > 1e-15 {
            for (b, c) in &self.terms {
                let val = c * scalar;
                if val.abs() > 1e-14 {
                    res_terms.insert(b.clone(), val);
                }
            }
        }
        DifferentialForm {
            dim: self.dim,
            degree: self.degree,
            coord_names: self.coord_names.clone(),
            terms: res_terms,
        }
    }

    /// Wedge Product $\omega \wedge \eta \in \Omega^{k+m}(M)$.
    ///
    /// Satisfies graded commutativity: $\alpha \wedge \beta = (-1)^{\deg(\alpha) \deg(\beta)} \beta \wedge \alpha$.
    pub fn wedge(&self, other: &DifferentialForm) -> AlgebraResult<DifferentialForm> {
        if self.dim != other.dim {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Manifold dimension mismatch: {} vs {}",
                self.dim, other.dim
            )));
        }

        let res_degree = self.degree + other.degree;
        if res_degree > self.dim {
            // Wedge product vanishes identically above manifold top dimension n
            return Ok(DifferentialForm::zero(
                self.dim,
                res_degree,
                self.coord_names.clone(),
            ));
        }

        let mut res_terms = BTreeMap::new();

        for (b1, &c1) in &self.terms {
            for (b2, &c2) in &other.terms {
                let mut combined = b1.indices.clone();
                combined.extend_from_slice(&b2.indices);

                let (basis, sign) = FormBasis::new(combined);
                if sign != 0 {
                    let term_val = c1 * c2 * (sign as f64);
                    *res_terms.entry(basis).or_insert(0.0) += term_val;
                }
            }
        }

        res_terms.retain(|_, v| v.abs() > 1e-14);

        Ok(DifferentialForm {
            dim: self.dim,
            degree: res_degree,
            coord_names: self.coord_names.clone(),
            terms: res_terms,
        })
    }

    /// Interior Product (Contraction) $\iota_X \omega \in \Omega^{k-1}(M)$ by vector field $X$.
    pub fn interior_product(&self, x: &VectorField) -> AlgebraResult<DifferentialForm> {
        if self.dim != x.dim {
            return Err(AlgebraError::DimensionMismatch(format!(
                "Vector field dimension {} does not match form dimension {}",
                x.dim, self.dim
            )));
        }

        if self.degree == 0 {
            // Contraction of a 0-form (scalar) is zero
            return Ok(DifferentialForm::zero(
                self.dim,
                0,
                self.coord_names.clone(),
            ));
        }

        let res_degree = self.degree - 1;
        let mut res_terms = BTreeMap::new();

        for (basis, &coeff) in &self.terms {
            for (r, &coord_idx) in basis.indices.iter().enumerate() {
                let v_r = x.components.get(coord_idx).copied().unwrap_or(0.0);
                if v_r.abs() < 1e-15 {
                    continue;
                }

                // (-1)^r sign
                let sign = if r % 2 == 0 { 1.0 } else { -1.0 };
                let mut sub_indices = basis.indices.clone();
                sub_indices.remove(r);

                let (sub_basis, perm_sign) = FormBasis::new(sub_indices);
                if perm_sign != 0 {
                    let val = coeff * v_r * sign * (perm_sign as f64);
                    *res_terms.entry(sub_basis).or_insert(0.0) += val;
                }
            }
        }

        res_terms.retain(|_, v| v.abs() > 1e-14);

        Ok(DifferentialForm {
            dim: self.dim,
            degree: res_degree,
            coord_names: self.coord_names.clone(),
            terms: res_terms,
        })
    }

    /// Exterior Derivative $\mathrm{d}\omega \in \Omega^{k+1}(M)$.
    ///
    /// For constant linear forms, $\mathrm{d}(\text{const}) = 0$, $\mathrm{d}(\mathrm{d}x^i) = 0$, $\mathrm{d}^2 = 0$.
    pub fn exterior_derivative(&self) -> AlgebraResult<DifferentialForm> {
        let res_degree = self.degree + 1;
        if res_degree > self.dim {
            return Ok(DifferentialForm::zero(
                self.dim,
                res_degree,
                self.coord_names.clone(),
            ));
        }

        // For constant differential forms, the exterior derivative is identically zero
        Ok(DifferentialForm::zero(
            self.dim,
            res_degree,
            self.coord_names.clone(),
        ))
    }

    /// Lie Derivative $\mathcal{L}_X \omega$ via Cartan's Magic Formula:
    /// $$\mathcal{L}_X \omega = (\mathrm{d}\iota_X + \iota_X\mathrm{d})\omega$$
    pub fn lie_derivative(&self, x: &VectorField) -> AlgebraResult<DifferentialForm> {
        let i_x_omega = self.interior_product(x)?;
        let d_i_x = i_x_omega.exterior_derivative()?;

        let d_omega = self.exterior_derivative()?;
        let i_d = d_omega.interior_product(x)?;

        d_i_x.add(&i_d)
    }

    /// Hodge Star Dual Operator $\star: \Omega^k \to \Omega^{n-k}$ with flat Euclidean / Minkowski metric.
    pub fn hodge_star(&self, is_minkowski: bool) -> AlgebraResult<DifferentialForm> {
        let n = self.dim;
        let k = self.degree;
        let res_degree = n - k;
        let mut res_terms = BTreeMap::new();

        let all_indices: Vec<usize> = (0..n).collect();

        for (basis, &coeff) in &self.terms {
            // Find complementary indices
            let mut comp_indices = Vec::with_capacity(res_degree);
            for &idx in &all_indices {
                if !basis.indices.contains(&idx) {
                    comp_indices.push(idx);
                }
            }

            // Permutation parity of [basis.indices, comp_indices]
            let mut full = basis.indices.clone();
            full.extend_from_slice(&comp_indices);
            let (_, sign) = FormBasis::new(full);

            // Metric signature adjustment: Minkowski diag(-1, 1, 1, ...) vs Euclidean diag(1, 1, 1, ...)
            let mut metric_factor = 1.0;
            if is_minkowski && basis.indices.contains(&0) {
                metric_factor = -1.0;
            }

            let star_coeff = coeff * (sign as f64) * metric_factor;
            let (star_basis, _) = FormBasis::new(comp_indices);
            *res_terms.entry(star_basis).or_insert(0.0) += star_coeff;
        }

        res_terms.retain(|_, v| v.abs() > 1e-14);

        Ok(DifferentialForm {
            dim: self.dim,
            degree: res_degree,
            coord_names: self.coord_names.clone(),
            terms: res_terms,
        })
    }
}
