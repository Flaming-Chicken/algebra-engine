//! # `algebra_engine::systems::polynomial`
//!
//! Multi-Variate Non-Linear Polynomial Systems Solvers.
//!
//! Provides:
//! - **Gröbner Lexicographic Elimination**: Reduces multi-variate non-linear polynomial systems $\{f_1(\mathbf{x}) = 0, \dots, f_m(\mathbf{x}) = 0\}$
//!   into a triangular univariate system.
//! - **Exact Algebraic Variety Solving**: Combines Gröbner basis triangularization with analytical and numerical root solvers
//!   (Cardano, Ferrari, Durand-Kerner) to compute all exact and numerical solution tuples $\mathbf{x} = (x_1, \dots, x_n)$.

use crate::poly::grobner::GrobnerBasis;
use crate::poly::multivariate::{MonomialOrder, MultiPoly};
use crate::poly::roots::ComplexRoot;
use algebra_core::error::{AlgebraError, AlgebraResult};
use algebra_core::{ExprGraph, ExprId};

/// Multi-Variate Polynomial System Solver.
pub struct PolynomialSystemSolver;

#[allow(clippy::needless_range_loop)]
impl PolynomialSystemSolver {
    /// Solves a multi-variate polynomial system $\{f_1(\mathbf{x}) = 0, \dots, f_m(\mathbf{x}) = 0\}$
    /// for variables $\mathbf{x} = (x_1, \dots, x_n)$.
    pub fn solve(
        graph: &ExprGraph,
        equations: &[ExprId],
        var_names: &[String],
    ) -> AlgebraResult<Vec<Vec<ComplexRoot>>> {
        let num_vars = var_names.len();
        if num_vars == 0 {
            return Ok(Vec::new());
        }

        // 1. Convert expressions to MultiPoly under Lexicographic ordering
        let mut polys = Vec::with_capacity(equations.len());
        for &eq in equations {
            let p = MultiPoly::from_expr(graph, eq, var_names, MonomialOrder::Lex)?;
            polys.push(p);
        }

        // 2. Compute Reduced Gröbner Basis under Lex order
        let gb = GrobnerBasis::buchberger(&polys);
        if gb.polynomials.is_empty() {
            return Ok(Vec::new());
        }

        // Check for inconsistency: if basis contains a non-zero constant (1 = 0)
        for p in &gb.polynomials {
            if p.terms.len() == 1 && p.terms[0].monomial.total_degree == 0 {
                return Err(AlgebraError::EvaluationError(
                    "Inconsistent polynomial system: no common algebraic variety exists".into(),
                ));
            }
        }

        // 3. Solve level by level, last variable first (lexicographic order makes the basis triangular in
        //    the elimination ideals): at each level take the basis polynomials that involve only the variables
        //    from this one on, substitute the values already found, solve the resulting univariate polynomial
        //    over the complex numbers and branch over its roots. A level with no equation left means a free
        //    variable, that is a positive-dimensional variety.
        let basis: Vec<&MultiPoly> = gb.polynomials.iter().filter(|p| !p.is_zero()).collect();
        let mut tuples: Vec<Vec<Cx>> = Vec::new();
        let mut current = vec![Cx::ZERO; num_vars];
        Self::solve_level(&basis, num_vars, num_vars - 1, &mut current, &mut tuples)?;
        Ok(tuples
            .into_iter()
            .map(|t| {
                t.into_iter()
                    .map(|z| ComplexRoot::complex(z.re, z.im))
                    .collect()
            })
            .collect())
    }

    /// Branch over the roots of variable `var` given the values already in `current[var + 1..]`.
    fn solve_level(
        basis: &[&MultiPoly],
        num_vars: usize,
        var: usize,
        current: &mut Vec<Cx>,
        out: &mut Vec<Vec<Cx>>,
    ) -> AlgebraResult<()> {
        // The univariate polynomials (ascending coefficients) in `var` from the elimination ideal.
        let mut candidates: Vec<Vec<Cx>> = Vec::new();
        for p in basis {
            if p.terms
                .iter()
                .any(|t| t.monomial.exponents[..var].iter().any(|&e| e != 0))
            {
                continue; // involves a variable that is solved later
            }
            let degree = p
                .terms
                .iter()
                .map(|t| t.monomial.exponents[var])
                .max()
                .unwrap_or(0);
            let mut coeffs = vec![Cx::ZERO; degree + 1];
            for t in &p.terms {
                let mut value = Cx::real(t.coeff);
                for k in (var + 1)..num_vars {
                    for _ in 0..t.monomial.exponents[k] {
                        value = value.mul(current[k]);
                    }
                }
                let d = t.monomial.exponents[var];
                coeffs[d] = coeffs[d].add(value);
            }
            let scale = coeffs.iter().map(|c| c.abs()).fold(0.0_f64, f64::max);
            if scale == 0.0 {
                continue;
            }
            while coeffs.len() > 1 && coeffs[coeffs.len() - 1].abs() <= 1e-10 * scale {
                coeffs.pop();
            }
            if coeffs.len() == 1 {
                if coeffs[0].abs() > 1e-8 * scale.max(1.0) {
                    return Ok(()); // a non-zero constant: this branch has no solution
                }
                continue; // identically zero here: no constraint
            }
            candidates.push(coeffs);
        }
        let Some(first) = candidates.iter().min_by_key(|c| c.len()) else {
            return Err(AlgebraError::EvaluationError(
                "Positive-dimensional variety or underdetermined polynomial system".into(),
            ));
        };
        let mut roots = complex_roots(first);
        // Keep only the roots at which every other candidate vanishes too.
        roots.retain(|&z| {
            candidates.iter().all(|c| {
                let scale: f64 = c
                    .iter()
                    .enumerate()
                    .map(|(k, a)| a.abs() * z.abs().powi(k as i32))
                    .sum();
                eval_poly(c, z).abs() <= 1e-7 * scale.max(1.0)
            })
        });
        for z in roots {
            current[var] = z;
            if var == 0 {
                out.push(current.clone());
            } else {
                Self::solve_level(basis, num_vars, var - 1, current, out)?;
            }
        }
        Ok(())
    }
}

/// A complex number for the back-substitution (the solver's own, to keep `ComplexRoot` a plain record).
#[derive(Clone, Copy, Debug)]
struct Cx {
    re: f64,
    im: f64,
}

impl Cx {
    const ZERO: Cx = Cx { re: 0.0, im: 0.0 };
    fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }
    fn add(self, o: Cx) -> Cx {
        Cx {
            re: self.re + o.re,
            im: self.im + o.im,
        }
    }
    fn sub(self, o: Cx) -> Cx {
        Cx {
            re: self.re - o.re,
            im: self.im - o.im,
        }
    }
    fn mul(self, o: Cx) -> Cx {
        Cx {
            re: self.re * o.re - self.im * o.im,
            im: self.re * o.im + self.im * o.re,
        }
    }
    fn div(self, o: Cx) -> Cx {
        let d = o.re * o.re + o.im * o.im;
        Cx {
            re: (self.re * o.re + self.im * o.im) / d,
            im: (self.im * o.re - self.re * o.im) / d,
        }
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
}

/// Horner evaluation of ascending coefficients.
fn eval_poly(c: &[Cx], z: Cx) -> Cx {
    c.iter().rev().fold(Cx::ZERO, |acc, &a| acc.mul(z).add(a))
}

/// All complex roots of a polynomial with ascending complex coefficients (non-zero leading coefficient), by
/// Aberth–Ehrlich simultaneous iteration followed by Newton polishing; roots closer than a small tolerance are
/// reported once (a repeated root of the elimination polynomial is one solution value).
fn complex_roots(c: &[Cx]) -> Vec<Cx> {
    let degree = c.len() - 1;
    if degree == 0 {
        return Vec::new();
    }
    let lead = c[degree];
    if degree == 1 {
        return vec![Cx::ZERO.sub(c[0]).div(lead)];
    }
    let monic: Vec<Cx> = c.iter().map(|&a| a.div(lead)).collect();
    let derivative: Vec<Cx> = (1..=degree)
        .map(|k| monic[k].mul(Cx::real(k as f64)))
        .collect();
    let radius = 1.0
        + monic[..degree]
            .iter()
            .map(|a| a.abs())
            .fold(0.0_f64, f64::max);
    let mut z: Vec<Cx> = (0..degree)
        .map(|k| {
            let angle = 2.0 * std::f64::consts::PI * k as f64 / degree as f64 + 0.4;
            Cx {
                re: radius * 0.5 * angle.cos(),
                im: radius * 0.5 * angle.sin(),
            }
        })
        .collect();
    for _ in 0..500 {
        let mut moved = 0.0_f64;
        for k in 0..degree {
            let f = eval_poly(&monic, z[k]);
            let df = eval_poly(&derivative, z[k]);
            if f.abs() == 0.0 {
                continue;
            }
            let ratio = f.div(df);
            let mut repulsion = Cx::ZERO;
            for (j, zj) in z.iter().enumerate() {
                if j != k {
                    repulsion = repulsion.add(Cx::real(1.0).div(z[k].sub(*zj)));
                }
            }
            let step = ratio.div(Cx::real(1.0).sub(ratio.mul(repulsion)));
            z[k] = z[k].sub(step);
            moved = moved.max(step.abs() / (1.0 + z[k].abs()));
        }
        if moved < 1e-15 {
            break;
        }
    }
    // Newton polish on the original polynomial.
    for zk in z.iter_mut() {
        for _ in 0..3 {
            let df = eval_poly(&derivative, *zk);
            if df.abs() > 0.0 {
                *zk = zk.sub(eval_poly(&monic, *zk).div(df));
            }
        }
    }
    let mut distinct: Vec<Cx> = Vec::new();
    for zk in z {
        if !distinct
            .iter()
            .any(|d| d.sub(zk).abs() <= 1e-6 * (1.0 + zk.abs()))
        {
            distinct.push(zk);
        }
    }
    distinct
}
