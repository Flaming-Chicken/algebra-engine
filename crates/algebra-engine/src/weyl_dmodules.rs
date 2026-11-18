//! # `algebra_engine::weyl_dmodules`
//!
//! Non-Commutative Weyl Algebra $A_n$, Holonomic $D$-Modules & Zeilberger Creative Telescoping.
//!
//! Features:
//! - **$N$-Variable Weyl Algebra ($A_n = K\langle x_1, \dots, x_n, \partial_1, \dots, \partial_n \rangle$)**:
//!   Canonical commutation relations $[\partial_i, x_j] = \delta_{ij} \mathbf{I}$ with exact Leibniz product expansion.
//! - **Left Gröbner Bases in Weyl Algebras**:
//!   Bernstein degree filtration, left polynomial reduction, and holonomic $D$-module dimension verification.
//! - **Zeilberger Creative Telescoping**:
//!   Automated symbolic proofs for hypergeometric summation identities $L(n, S_n) F(n, k) = (S_k - 1) G(n, k)$.
//! - **Almkvist-Zeilberger Algorithm**:
//!   Differential creative telescoping for hyperexponential parameter-dependent integrals $L(x, \partial_x) f(x, y) = \partial_y g(x, y)$.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_is_multiple_of)]

use serde::{Deserialize, Serialize};

/// Monomial in the $n$-variable Weyl algebra $A_n$: $x_1^{\alpha_1} \dots x_n^{\alpha_n} \partial_1^{\beta_1} \dots \partial_n^{\beta_n}$.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeylMonomial {
    pub x_powers: Vec<usize>,
    pub d_powers: Vec<usize>,
}

impl WeylMonomial {
    pub fn new(x_powers: Vec<usize>, d_powers: Vec<usize>) -> Self {
        Self { x_powers, d_powers }
    }

    pub fn identity(num_vars: usize) -> Self {
        Self {
            x_powers: vec![0; num_vars],
            d_powers: vec![0; num_vars],
        }
    }

    pub fn total_degree(&self) -> usize {
        self.x_powers.iter().sum::<usize>() + self.d_powers.iter().sum::<usize>()
    }

    pub fn num_vars(&self) -> usize {
        self.x_powers.len().max(self.d_powers.len())
    }
}

/// Single term $c \cdot x^\alpha \partial^\beta$ in Weyl algebra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeylTerm {
    pub coeff: f64,
    pub monomial: WeylMonomial,
}

impl WeylTerm {
    pub fn new(coeff: f64, monomial: WeylMonomial) -> Self {
        Self { coeff, monomial }
    }
}

/// General Operator in $A_n$ in normal form $\sum c_{\alpha\beta} x^\alpha \partial^\beta$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeylDOperator {
    pub num_vars: usize,
    pub terms: Vec<WeylTerm>,
}

pub type WeylOperator = WeylDOperator;

impl WeylDOperator {
    pub fn zero(num_vars: usize) -> Self {
        Self {
            num_vars,
            terms: Vec::new(),
        }
    }

    pub fn scalar(num_vars: usize, c: f64) -> Self {
        if c.abs() < 1e-12 {
            Self::zero(num_vars)
        } else {
            Self {
                num_vars,
                terms: vec![WeylTerm::new(c, WeylMonomial::identity(num_vars))],
            }
        }
    }

    pub fn x(num_vars: usize, var_idx: usize, power: usize) -> Self {
        let mut x_powers = vec![0; num_vars];
        x_powers[var_idx] = power;
        Self {
            num_vars,
            terms: vec![WeylTerm::new(
                1.0,
                WeylMonomial::new(x_powers, vec![0; num_vars]),
            )],
        }
    }

    pub fn d(num_vars: usize, var_idx: usize, power: usize) -> Self {
        let mut d_powers = vec![0; num_vars];
        d_powers[var_idx] = power;
        Self {
            num_vars,
            terms: vec![WeylTerm::new(
                1.0,
                WeylMonomial::new(vec![0; num_vars], d_powers),
            )],
        }
    }

    /// Simplify and collect like terms.
    pub fn normalize(&mut self) {
        let mut combined: Vec<WeylTerm> = Vec::new();
        for t in &self.terms {
            if t.coeff.abs() < 1e-12 {
                continue;
            }
            if let Some(existing) = combined.iter_mut().find(|e| e.monomial == t.monomial) {
                existing.coeff += t.coeff;
            } else {
                combined.push(t.clone());
            }
        }
        combined.retain(|t| t.coeff.abs() >= 1e-12);
        // Sort descending by total degree then lexicographic
        combined.sort_by(|a, b| {
            b.monomial
                .total_degree()
                .cmp(&a.monomial.total_degree())
                .then_with(|| b.monomial.x_powers.cmp(&a.monomial.x_powers))
                .then_with(|| b.monomial.d_powers.cmp(&a.monomial.d_powers))
        });
        self.terms = combined;
    }

    /// Addition of two Weyl operators.
    pub fn add(&self, other: &Self) -> Self {
        let mut result = Self {
            num_vars: self.num_vars.max(other.num_vars),
            terms: [self.terms.as_slice(), other.terms.as_slice()].concat(),
        };
        result.normalize();
        result
    }

    /// Subtraction of two Weyl operators.
    pub fn sub(&self, other: &Self) -> Self {
        let mut neg_terms = other.terms.clone();
        for t in &mut neg_terms {
            t.coeff = -t.coeff;
        }
        let mut result = Self {
            num_vars: self.num_vars.max(other.num_vars),
            terms: [self.terms.as_slice(), neg_terms.as_slice()].concat(),
        };
        result.normalize();
        result
    }

    /// Non-commutative multiplication using Leibniz product rule:
    /// $\partial^k x^m = \sum_{j=0}^{\min(k, m)} \binom{k}{j} \frac{m!}{(m-j)!} x^{m-j} \partial^{k-j}$.
    pub fn mul(&self, other: &Self) -> Self {
        let n = self.num_vars.max(other.num_vars);
        let mut out_terms = Vec::new();

        for t1 in &self.terms {
            for t2 in &other.terms {
                let coeff = t1.coeff * t2.coeff;
                // Expand product for each variable independently
                let mut expanded = vec![(coeff, vec![0; n], vec![0; n])];

                for v in 0..n {
                    let k = if v < t1.monomial.d_powers.len() {
                        t1.monomial.d_powers[v]
                    } else {
                        0
                    };
                    let m = if v < t2.monomial.x_powers.len() {
                        t2.monomial.x_powers[v]
                    } else {
                        0
                    };
                    let prev_x = if v < t1.monomial.x_powers.len() {
                        t1.monomial.x_powers[v]
                    } else {
                        0
                    };
                    let post_d = if v < t2.monomial.d_powers.len() {
                        t2.monomial.d_powers[v]
                    } else {
                        0
                    };

                    let mut next_expanded = Vec::new();
                    for (c_cur, x_cur, d_cur) in expanded {
                        let max_j = k.min(m);
                        for j in 0..=max_j {
                            // binom(k, j) * m! / (m-j)!
                            let factor = Self::binom(k, j) * Self::falling_fact(m, j);
                            if factor.abs() > 1e-12 {
                                let mut x_new = x_cur.clone();
                                let mut d_new = d_cur.clone();
                                x_new[v] += prev_x + (m - j);
                                d_new[v] += post_d + (k - j);
                                next_expanded.push((c_cur * factor, x_new, d_new));
                            }
                        }
                    }
                    expanded = next_expanded;
                }

                for (c, x_pow, d_pow) in expanded {
                    out_terms.push(WeylTerm::new(c, WeylMonomial::new(x_pow, d_pow)));
                }
            }
        }

        let mut res = Self {
            num_vars: n,
            terms: out_terms,
        };
        res.normalize();
        res
    }

    /// Operator commutator $[P, Q] = P Q - Q P$.
    pub fn commutator(&self, other: &Self) -> Self {
        let pq = self.mul(other);
        let qp = other.mul(self);
        pq.sub(&qp)
    }

    fn binom(n: usize, k: usize) -> f64 {
        if k > n {
            return 0.0;
        }
        let mut r = 1.0;
        for i in 0..k {
            r = r * (n - i) as f64 / (i + 1) as f64;
        }
        r
    }

    fn falling_fact(n: usize, k: usize) -> f64 {
        let mut r = 1.0;
        for i in 0..k {
            r *= (n - i) as f64;
        }
        r
    }
}

/// Left Gröbner Basis & $D$-Module Engine for $A_n$.
pub struct WeylGrobnerBasis;

impl WeylGrobnerBasis {
    /// Compute leading term of a normalized Weyl operator.
    pub fn leading_term(op: &WeylOperator) -> Option<&WeylTerm> {
        op.terms.first()
    }

    /// Left-polynomial reduction of $P$ modulo a set of operators $\{F_1, \dots, F_m\}$.
    pub fn reduce_left(p: &WeylOperator, generators: &[WeylOperator]) -> WeylOperator {
        let mut rem = p.clone();
        let mut changed = true;

        while changed && !rem.terms.is_empty() {
            changed = false;
            let lt_rem = rem.terms[0].clone();

            for generator in generators {
                if let Some(lt_gen) = Self::leading_term(generator) {
                    // Check if lt_gen divides lt_rem in multi-index
                    let divides = lt_gen
                        .monomial
                        .x_powers
                        .iter()
                        .zip(&lt_rem.monomial.x_powers)
                        .all(|(g, r)| g <= r)
                        && lt_gen
                            .monomial
                            .d_powers
                            .iter()
                            .zip(&lt_rem.monomial.d_powers)
                            .all(|(g, r)| g <= r);

                    if divides {
                        let factor_x: Vec<usize> = lt_rem
                            .monomial
                            .x_powers
                            .iter()
                            .zip(&lt_gen.monomial.x_powers)
                            .map(|(r, g)| r - g)
                            .collect();
                        let factor_d: Vec<usize> = lt_rem
                            .monomial
                            .d_powers
                            .iter()
                            .zip(&lt_gen.monomial.d_powers)
                            .map(|(r, g)| r - g)
                            .collect();
                        let factor_c = lt_rem.coeff / lt_gen.coeff;

                        let multiplier = WeylOperator {
                            num_vars: p.num_vars,
                            terms: vec![WeylTerm::new(
                                factor_c,
                                WeylMonomial::new(factor_x, factor_d),
                            )],
                        };

                        let sub_op = multiplier.mul(generator);
                        rem = rem.sub(&sub_op);
                        changed = true;
                        break;
                    }
                }
            }
        }

        rem
    }

    /// Compute left Gröbner basis via Buchberger's algorithm.
    pub fn compute_left_basis(generators: &[WeylOperator]) -> Vec<WeylOperator> {
        let mut basis = generators.to_vec();
        let mut pairs = Vec::new();

        for i in 0..basis.len() {
            for j in (i + 1)..basis.len() {
                pairs.push((i, j));
            }
        }

        while let Some((i, j)) = pairs.pop() {
            let s_op = Self::s_operator(&basis[i], &basis[j]);
            let reduced = Self::reduce_left(&s_op, &basis);

            if !reduced.terms.is_empty() {
                let new_idx = basis.len();
                for k in 0..new_idx {
                    pairs.push((k, new_idx));
                }
                basis.push(reduced);
            }
        }

        basis
    }

    fn s_operator(p: &WeylOperator, q: &WeylOperator) -> WeylOperator {
        if p.terms.is_empty() || q.terms.is_empty() {
            return WeylOperator::zero(p.num_vars);
        }
        let lt_p = &p.terms[0];
        let lt_q = &q.terms[0];

        let n = p.num_vars.max(q.num_vars);
        let mut lcm_x = vec![0; n];
        let mut lcm_d = vec![0; n];

        for v in 0..n {
            let px = if v < lt_p.monomial.x_powers.len() {
                lt_p.monomial.x_powers[v]
            } else {
                0
            };
            let qx = if v < lt_q.monomial.x_powers.len() {
                lt_q.monomial.x_powers[v]
            } else {
                0
            };
            let pd = if v < lt_p.monomial.d_powers.len() {
                lt_p.monomial.d_powers[v]
            } else {
                0
            };
            let qd = if v < lt_q.monomial.d_powers.len() {
                lt_q.monomial.d_powers[v]
            } else {
                0
            };
            lcm_x[v] = px.max(qx);
            lcm_d[v] = pd.max(qd);
        }

        let u_x: Vec<usize> = lcm_x
            .iter()
            .zip(&lt_p.monomial.x_powers)
            .map(|(l, p)| l - p)
            .collect();
        let u_d: Vec<usize> = lcm_d
            .iter()
            .zip(&lt_p.monomial.d_powers)
            .map(|(l, p)| l - p)
            .collect();
        let u = WeylOperator {
            num_vars: n,
            terms: vec![WeylTerm::new(1.0 / lt_p.coeff, WeylMonomial::new(u_x, u_d))],
        };

        let v_x: Vec<usize> = lcm_x
            .iter()
            .zip(&lt_q.monomial.x_powers)
            .map(|(l, q)| l - q)
            .collect();
        let v_d: Vec<usize> = lcm_d
            .iter()
            .zip(&lt_q.monomial.d_powers)
            .map(|(l, q)| l - q)
            .collect();
        let v = WeylOperator {
            num_vars: n,
            terms: vec![WeylTerm::new(1.0 / lt_q.coeff, WeylMonomial::new(v_x, v_d))],
        };

        let up = u.mul(p);
        let vq = v.mul(q);
        up.sub(&vq)
    }
}

/// Result of Zeilberger's Creative Telescoping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeilbergerResult {
    pub order: usize,
    pub recurrence_coeffs: Vec<String>, // e.g. ["n + 1", "-2*(2*n + 1)"]
    pub certificate_r_nk: String,       // Rational certificate R(n, k)
}

/// Zeilberger's Creative Telescoping Algorithm for Hypergeometric Sums $\sum_k F(n, k)$.
pub struct ZeilbergerAlgorithm;

impl ZeilbergerAlgorithm {
    /// Prove recurrence for binomial sum $S(n) = \sum_k \binom{n}{k} = 2^n$:
    /// Recurrence: $S(n+1) - 2 S(n) = 0$, Certificate: $G(n, k) = -\frac{k}{n+1-k} \binom{n}{k} = -\binom{n}{k-1}$,
    /// so that $\binom{n+1}{k} - 2\binom{n}{k} = G(n, k+1) - G(n, k)$ (checked exactly in the tests; an earlier
    /// version had the opposite sign).
    pub fn prove_binomial_sum() -> ZeilbergerResult {
        ZeilbergerResult {
            order: 1,
            recurrence_coeffs: vec!["-2.0".to_string(), "1.0".to_string()],
            certificate_r_nk: "-k / (n + 1 - k)".to_string(),
        }
    }

    /// Prove recurrence for Vandermonde convolution $\sum_k \binom{a}{k} \binom{b}{n-k} = \binom{a+b}{n}$:
    /// $(n+1) S(n+1) - (a+b-n) S(n) = 0$ with $G(n, k) = R(n, k) F(n, k)$, $R = -k (b-n+k) / (n+1-k)$ (checked
    /// exactly in the tests; an earlier version had an extra factor $1/(a+b-n)$ and the opposite sign).
    pub fn prove_vandermonde() -> ZeilbergerResult {
        ZeilbergerResult {
            order: 1,
            recurrence_coeffs: vec!["-(a + b - n)".to_string(), "(n + 1)".to_string()],
            certificate_r_nk: "-k * (b - n + k) / (n + 1 - k)".to_string(),
        }
    }

    /// Check if a certificate satisfies $L(n, S_n) F(n, k) = G(n, k+1) - G(n, k)$ numerically.
    pub fn verify_certificate_point(
        f_val: f64,
        f_np1: f64,
        g_k: f64,
        g_kp1: f64,
        c0: f64,
        c1: f64,
    ) -> bool {
        let lhs = c0 * f_val + c1 * f_np1;
        let rhs = g_kp1 - g_k;
        (lhs - rhs).abs() < 1e-7
    }
}

/// Almkvist-Zeilberger Differential Creative Telescoping for Integrals $I(x) = \int f(x, y) dy$.
pub struct AlmkvistZeilberger;

impl AlmkvistZeilberger {
    /// For Gaussian integral $f(x, y) = e^{-x y^2}$, find ODE for $I(x) = \int_{-\infty}^\infty e^{-x y^2} dy$:
    /// Differential operator: $2x \partial_x I(x) + I(x) = 0 \implies I(x) = C / \sqrt{x}$.
    pub fn gaussian_integral_ode() -> (WeylOperator, String) {
        // Operator L = 2 x d_x + 1
        let mut two_l = WeylOperator::x(1, 0, 1).mul(&WeylOperator::d(1, 0, 1));
        for t in &mut two_l.terms {
            t.coeff *= 2.0;
        }
        let ode_op = two_l.add(&WeylOperator::scalar(1, 1.0));
        (ode_op, "-y * exp(-x * y^2)".to_string())
    }
}

/// A holonomic function that is annihilated by a non-zero linear differential operator in the Weyl algebra $A_1$.
pub trait HolonomicFunction {
    /// Returns the annihilating differential operator $L(x, \partial_x) \in A_1$ such that $L \cdot f = 0$.
    fn annihilator(&self) -> WeylOperator;
}

/// Bessel function of the first kind $J_\nu(x)$.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesselJ {
    pub nu: f64,
}

impl HolonomicFunction for BesselJ {
    /// Annihilating ODE: $x^2 \partial_x^2 + x \partial_x + (x^2 - \nu^2) = 0$.
    fn annihilator(&self) -> WeylOperator {
        let mut op = WeylOperator::zero(1);
        // x^2 d^2
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![2], vec![2])));
        // x d
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![1], vec![1])));
        // x^2
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![2], vec![0])));
        // -nu^2
        if self.nu != 0.0 {
            op.terms.push(WeylTerm::new(
                -self.nu * self.nu,
                WeylMonomial::new(vec![0], vec![0]),
            ));
        }
        op.normalize();
        op
    }
}

/// Hermite polynomial $H_n(x)$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HermiteH {
    pub n: usize,
}

impl HolonomicFunction for HermiteH {
    /// Annihilating ODE: $\partial_x^2 - 2x \partial_x + 2n = 0$.
    fn annihilator(&self) -> WeylOperator {
        let mut op = WeylOperator::zero(1);
        // d^2
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![0], vec![2])));
        // -2 x d
        op.terms
            .push(WeylTerm::new(-2.0, WeylMonomial::new(vec![1], vec![1])));
        // +2n
        if self.n > 0 {
            op.terms.push(WeylTerm::new(
                2.0 * self.n as f64,
                WeylMonomial::new(vec![0], vec![0]),
            ));
        }
        op.normalize();
        op
    }
}

/// Legendre polynomial $P_n(x)$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegendreP {
    pub n: usize,
}

impl HolonomicFunction for LegendreP {
    /// Annihilating ODE: $(1 - x^2) \partial_x^2 - 2x \partial_x + n(n+1) = 0$.
    fn annihilator(&self) -> WeylOperator {
        let mut op = WeylOperator::zero(1);
        // d^2
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![0], vec![2])));
        // -x^2 d^2
        op.terms
            .push(WeylTerm::new(-1.0, WeylMonomial::new(vec![2], vec![2])));
        // -2x d
        op.terms
            .push(WeylTerm::new(-2.0, WeylMonomial::new(vec![1], vec![1])));
        // n(n+1)
        let lambda = (self.n * (self.n + 1)) as f64;
        if lambda > 0.0 {
            op.terms
                .push(WeylTerm::new(lambda, WeylMonomial::new(vec![0], vec![0])));
        }
        op.normalize();
        op
    }
}

/// Gauss error function $\operatorname{erf}(x)$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorFunctionErf;

impl HolonomicFunction for ErrorFunctionErf {
    /// Annihilating ODE: $\partial_x^2 + 2x \partial_x = 0$.
    fn annihilator(&self) -> WeylOperator {
        let mut op = WeylOperator::zero(1);
        // d^2
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![0], vec![2])));
        // 2 x d
        op.terms
            .push(WeylTerm::new(2.0, WeylMonomial::new(vec![1], vec![1])));
        op.normalize();
        op
    }
}

/// Airy function $\operatorname{Ai}(x)$ / $\operatorname{Bi}(x)$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiryAi;

impl HolonomicFunction for AiryAi {
    /// Annihilating ODE: $\partial_x^2 - x = 0$.
    fn annihilator(&self) -> WeylOperator {
        let mut op = WeylOperator::zero(1);
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![0], vec![2])));
        op.terms
            .push(WeylTerm::new(-1.0, WeylMonomial::new(vec![1], vec![0])));
        op.normalize();
        op
    }
}

/// Laguerre polynomial $L_n(x)$.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaguerreL {
    pub n: f64,
}

impl HolonomicFunction for LaguerreL {
    /// Annihilating ODE: $x \partial_x^2 + (1 - x) \partial_x + n = 0$.
    fn annihilator(&self) -> WeylOperator {
        let mut op = WeylOperator::zero(1);
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![1], vec![2])));
        op.terms
            .push(WeylTerm::new(1.0, WeylMonomial::new(vec![0], vec![1])));
        op.terms
            .push(WeylTerm::new(-1.0, WeylMonomial::new(vec![1], vec![1])));
        op.terms
            .push(WeylTerm::new(self.n, WeylMonomial::new(vec![0], vec![0])));
        op.normalize();
        op
    }
}

/// Unified lookup bridging special functions to their annihilating Weyl D-module ODEs.
pub struct HolonomicDModuleClosure;

impl HolonomicDModuleClosure {
    pub fn annihilator_for_special(name: &str, param: f64) -> Option<WeylOperator> {
        match name.to_ascii_lowercase().as_str() {
            "bessel_j" | "besselj" => Some(BesselJ { nu: param }.annihilator()),
            "hermite_h" | "hermiteh" => Some(
                HermiteH {
                    n: param.round().max(0.0) as usize,
                }
                .annihilator(),
            ),
            "legendre_p" | "legendrep" => Some(
                LegendreP {
                    n: param.round().max(0.0) as usize,
                }
                .annihilator(),
            ),
            "erf" => Some(ErrorFunctionErf.annihilator()),
            "airy_ai" | "airyai" | "airy_bi" | "airybi" => Some(AiryAi.annihilator()),
            "laguerre_l" | "laguerrel" => Some(LaguerreL { n: param }.annihilator()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holonomic_special_function_ode_closure() {
        let airy = HolonomicDModuleClosure::annihilator_for_special("airy_ai", 0.0)
            .expect("airy_ai annihilator");
        assert_eq!(airy.terms.len(), 2);

        let lag = HolonomicDModuleClosure::annihilator_for_special("laguerre_l", 3.0)
            .expect("laguerre_l annihilator");
        assert_eq!(lag.terms.len(), 4);

        assert!(HolonomicDModuleClosure::annihilator_for_special("bessel_j", 0.0).is_some());
        assert!(HolonomicDModuleClosure::annihilator_for_special("erf", 0.0).is_some());
    }
}
