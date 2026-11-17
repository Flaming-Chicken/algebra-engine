//! # `algebra_engine::poly::exact`
//!
//! Exact Gröbner bases over $\mathbb{Q}$.
//!
//! [`MultiPoly`] stores `f64` coefficients and discards terms below `1e-12`, which is fine for
//! numerical work but not for deciding ideal membership or computing a reduced Gröbner basis:
//! rounding can create or destroy a leading term and change the answer. [`ExactPoly`] keeps every
//! coefficient as a [`BigRational`], so [`ExactGrobner::compute`] returns the unique reduced basis
//! and [`ExactGrobner::contains`] is a decision procedure.
//!
//! The algorithm is Buchberger's with the product criterion for skipping pairs, followed by
//! minimalisation and full reduction (Cox, Little, O'Shea, *Ideals, Varieties, and Algorithms*,
//! §2.7).

use super::multivariate::{Monomial, MonomialOrder, MultiPoly};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// A polynomial in `num_vars` variables with exact rational coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactPoly {
    /// Non-zero terms, keyed by exponent vector.
    pub terms: BTreeMap<Vec<usize>, BigRational>,
    /// Number of variables.
    pub num_vars: usize,
    /// Monomial ordering used for leading terms.
    pub order: MonomialOrder,
}

impl ExactPoly {
    /// The zero polynomial.
    pub fn zero(num_vars: usize, order: MonomialOrder) -> Self {
        Self {
            terms: BTreeMap::new(),
            num_vars,
            order,
        }
    }

    /// Build from `(exponents, numerator, denominator)` triples; equal monomials are summed.
    pub fn from_terms(
        terms: &[(Vec<usize>, i64, i64)],
        num_vars: usize,
        order: MonomialOrder,
    ) -> Self {
        let mut poly = Self::zero(num_vars, order);
        for (exponents, numerator, denominator) in terms {
            let mut exponents = exponents.clone();
            exponents.resize(num_vars, 0);
            poly.add_term(
                exponents,
                BigRational::new(BigInt::from(*numerator), BigInt::from(*denominator)),
            );
        }
        poly
    }

    /// Convert from the floating-point representation. Each coefficient is converted exactly
    /// (every finite `f64` is a rational); `None` if one is not finite.
    pub fn from_multi_poly(poly: &MultiPoly) -> Option<Self> {
        let mut out = Self::zero(poly.num_vars, poly.order);
        for term in &poly.terms {
            let mut exponents = term.monomial.exponents.clone();
            exponents.resize(poly.num_vars, 0);
            out.add_term(exponents, BigRational::from_float(term.coeff)?);
        }
        Some(out)
    }

    fn add_term(&mut self, exponents: Vec<usize>, coefficient: BigRational) {
        if coefficient.is_zero() {
            return;
        }
        let entry = self
            .terms
            .entry(exponents)
            .or_insert_with(BigRational::zero);
        *entry += coefficient;
        if entry.is_zero() {
            // Remove cancelled terms so `is_zero` stays exact.
            self.terms.retain(|_, c| !c.is_zero());
        }
    }

    /// Whether this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    fn compare(&self, a: &[usize], b: &[usize]) -> Ordering {
        Monomial::new(a.to_vec()).cmp_with_order(&Monomial::new(b.to_vec()), self.order)
    }

    /// Leading exponent vector and coefficient under the polynomial's monomial order.
    pub fn leading(&self) -> Option<(Vec<usize>, BigRational)> {
        self.terms
            .iter()
            .max_by(|a, b| self.compare(a.0, b.0))
            .map(|(e, c)| (e.clone(), c.clone()))
    }

    /// `self -= coefficient * x^shift * other`.
    fn subtract_scaled(&mut self, other: &Self, coefficient: &BigRational, shift: &[usize]) {
        for (exponents, c) in &other.terms {
            let moved: Vec<usize> = exponents.iter().zip(shift).map(|(a, b)| a + b).collect();
            self.add_term(moved, -(c * coefficient));
        }
    }

    /// The monic multiple of this polynomial (`None` for zero).
    pub fn monic(&self) -> Option<Self> {
        let (_, lead) = self.leading()?;
        let mut out = Self::zero(self.num_vars, self.order);
        for (e, c) in &self.terms {
            out.terms.insert(e.clone(), c / &lead);
        }
        Some(out)
    }

    /// Remainder of division by `divisors` (multivariate division algorithm).
    pub fn reduce(&self, divisors: &[ExactPoly]) -> ExactPoly {
        let mut working = self.clone();
        let mut remainder = Self::zero(self.num_vars, self.order);
        let leads: Vec<Option<(Vec<usize>, BigRational)>> =
            divisors.iter().map(ExactPoly::leading).collect();
        while let Some((exponents, coefficient)) = working.leading() {
            let divisor = leads.iter().position(|lead| {
                lead.as_ref()
                    .is_some_and(|(lm, _)| lm.iter().zip(&exponents).all(|(d, e)| d <= e))
            });
            match divisor {
                Some(i) => {
                    let (lm, lc) = leads[i].clone().unwrap_or_default();
                    let shift: Vec<usize> = exponents.iter().zip(&lm).map(|(e, d)| e - d).collect();
                    working.subtract_scaled(&divisors[i], &(&coefficient / &lc), &shift);
                }
                None => {
                    working.terms.remove(&exponents);
                    remainder.add_term(exponents, coefficient);
                }
            }
        }
        remainder
    }

    /// The S-polynomial of `f` and `g`.
    pub fn s_polynomial(f: &Self, g: &Self) -> Self {
        let (Some((lm_f, lc_f)), Some((lm_g, lc_g))) = (f.leading(), g.leading()) else {
            return Self::zero(f.num_vars, f.order);
        };
        let lcm: Vec<usize> = lm_f.iter().zip(&lm_g).map(|(a, b)| *a.max(b)).collect();
        let shift_f: Vec<usize> = lcm.iter().zip(&lm_f).map(|(l, m)| l - m).collect();
        let shift_g: Vec<usize> = lcm.iter().zip(&lm_g).map(|(l, m)| l - m).collect();
        let mut out = Self::zero(f.num_vars, f.order);
        out.subtract_scaled(f, &(-BigRational::one() / &lc_f), &shift_f);
        out.subtract_scaled(g, &(BigRational::one() / &lc_g), &shift_g);
        out
    }
}

/// A reduced Gröbner basis over $\mathbb{Q}$.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactGrobner {
    /// The reduced basis: monic, with no leading term dividing another and every other term
    /// reduced. It is unique for the ideal and the monomial order.
    pub basis: Vec<ExactPoly>,
}

impl ExactGrobner {
    /// Compute the reduced Gröbner basis of the ideal generated by `generators`.
    pub fn compute(generators: &[ExactPoly]) -> Self {
        let mut g: Vec<ExactPoly> = generators.iter().filter_map(ExactPoly::monic).collect();
        let mut pairs: Vec<(usize, usize)> = (0..g.len())
            .flat_map(|i| ((i + 1)..g.len()).map(move |j| (i, j)))
            .collect();

        while let Some((i, j)) = pairs.pop() {
            let (Some((lm_i, _)), Some((lm_j, _))) = (g[i].leading(), g[j].leading()) else {
                continue;
            };
            // Product criterion: coprime leading monomials reduce to zero.
            if lm_i.iter().zip(&lm_j).all(|(a, b)| *a == 0 || *b == 0) {
                continue;
            }
            let s = ExactPoly::s_polynomial(&g[i], &g[j]).reduce(&g);
            if let Some(new) = s.monic() {
                let index = g.len();
                g.push(new);
                pairs.extend((0..index).map(|k| (k, index)));
            }
        }

        // Minimalise: drop elements whose leading monomial is divisible by another's (keep the
        // earlier one when two are equal).
        let leads: Vec<Vec<usize>> = g.iter().filter_map(|p| p.leading().map(|l| l.0)).collect();
        let divides = |a: &[usize], b: &[usize]| a.iter().zip(b).all(|(x, y)| x <= y);
        let keep: Vec<usize> = (0..g.len())
            .filter(|&i| {
                !(0..g.len()).any(|j| {
                    j != i && divides(&leads[j], &leads[i]) && (leads[j] != leads[i] || j < i)
                })
            })
            .collect();
        let minimal: Vec<ExactPoly> = keep.iter().map(|&i| g[i].clone()).collect();

        // Fully reduce each element by the others.
        let mut reduced = Vec::with_capacity(minimal.len());
        for (i, p) in minimal.iter().enumerate() {
            let others: Vec<ExactPoly> = minimal
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, q)| q.clone())
                .collect();
            if let Some(r) = p.reduce(&others).monic() {
                reduced.push(r);
            }
        }
        // Deterministic order: descending leading monomial.
        reduced.sort_by(|a, b| match (a.leading(), b.leading()) {
            (Some((x, _)), Some((y, _))) => a.compare(&y, &x),
            _ => Ordering::Equal,
        });
        Self { basis: reduced }
    }

    /// Exact ideal membership: `f` is in the ideal iff its remainder is zero.
    pub fn contains(&self, f: &ExactPoly) -> bool {
        f.reduce(&self.basis).is_zero()
    }

    /// Whether every S-polynomial of the basis reduces to zero (Buchberger's criterion).
    pub fn is_groebner_basis(&self) -> bool {
        (0..self.basis.len()).all(|i| {
            ((i + 1)..self.basis.len()).all(|j| {
                ExactPoly::s_polynomial(&self.basis[i], &self.basis[j])
                    .reduce(&self.basis)
                    .is_zero()
            })
        })
    }
}
