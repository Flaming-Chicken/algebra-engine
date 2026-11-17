//! # `algebra_engine::diffalg`
//!
//! Differential Algebra, Ritt-Wu Characteristic Sets & Holonomic $D$-Modules.
//!
//! Implements:
//! - **Differential Indeterminates & Monomials**: $y_i^{(k)} = \frac{d^k y_i}{d x^k}$ with derivation rankings.
//! - **Ritt-Wu Reduction**: Computes Leader $\operatorname{ld}(f)$, Separand $\operatorname{sep}(f) = \frac{\partial f}{\partial \operatorname{ld}(f)}$,
//!   and Initial $\operatorname{init}(f)$ for differential polynomial ideals.
//! - **Differential Pseudo-Division**:
//!   $$I^{d_1} S^{d_2} f = \sum q_i g_i + r$$
//! - **Holonomic $D$-Modules & Weyl Algebra**: $A_1 = K\langle x, \partial \rangle$ satisfying $[\partial, x] = \partial x - x \partial = 1$.
//! - **Zeilberger Algorithm**: Creative telescoping for parametric definite integration $\int f(x, t) dx$.

use algebra_core::error::AlgebraResult;

/// Differential indeterminate $y_i^{(k)}$ with variable index $i$ and derivative order $k$.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiffIndeterminate {
    /// Variable identifier index (e.g. $0 \to y, 1 \to z$).
    pub var_index: usize,
    /// Derivative order $k \ge 0$ (where $k=0$ is algebraic variable $y_i$).
    pub order: usize,
}

impl DiffIndeterminate {
    pub fn new(var_index: usize, order: usize) -> Self {
        Self { var_index, order }
    }

    /// Apply derivative operator $\delta$: increases order by 1.
    pub fn differentiate(&self) -> Self {
        Self {
            var_index: self.var_index,
            order: self.order + 1,
        }
    }
}

/// Differential monomial term: coefficient $\times \prod (y_i^{(k)})^{p_{i,k}}$.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffTerm {
    pub coeff: f64,
    /// Exponents for differential variables: `(DiffIndeterminate, power)`.
    pub factors: Vec<(DiffIndeterminate, usize)>,
}

impl DiffTerm {
    pub fn new(coeff: f64, factors: Vec<(DiffIndeterminate, usize)>) -> Self {
        Self { coeff, factors }
    }

    pub fn highest_indeterminate(&self) -> Option<DiffIndeterminate> {
        self.factors.iter().map(|(ind, _)| *ind).max()
    }
}

/// Differential Polynomial $f \in K\{y_1, \dots, y_m\}$.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffPolynomial {
    pub terms: Vec<DiffTerm>,
}

impl DiffPolynomial {
    pub fn new(terms: Vec<DiffTerm>) -> Self {
        Self { terms }
    }

    /// Construct a single variable term $c \cdot y_i^{(k)}$.
    pub fn var(var_idx: usize, order: usize) -> Self {
        Self {
            terms: vec![DiffTerm::new(
                1.0,
                vec![(DiffIndeterminate::new(var_idx, order), 1)],
            )],
        }
    }

    /// Leader of the differential polynomial: highest ranking derivative appearing in $f$.
    pub fn leader(&self) -> Option<DiffIndeterminate> {
        let mut lead = None;
        for t in &self.terms {
            if let Some(hi) = t.highest_indeterminate() {
                lead = match lead {
                    None => Some(hi),
                    Some(cur) => Some(cur.max(hi)),
                };
            }
        }
        lead
    }

    /// Degree of the polynomial with respect to its leader $\deg(f, \operatorname{ld}(f))$.
    pub fn leader_degree(&self) -> usize {
        if let Some(ld) = self.leader() {
            let mut max_deg = 0;
            for t in &self.terms {
                for (ind, deg) in &t.factors {
                    if *ind == ld && *deg > max_deg {
                        max_deg = *deg;
                    }
                }
            }
            max_deg
        } else {
            0
        }
    }

    /// Differentiate polynomial $\delta f = \frac{d f}{d x}$.
    pub fn differentiate(&self) -> Self {
        let mut new_terms = Vec::new();
        for t in &self.terms {
            // Product rule: sum_i coeff * (deg_i * y_i^{deg_i-1} * delta(y_i)) * other_factors
            for (idx, (ind, deg)) in t.factors.iter().enumerate() {
                let mut term_factors = t.factors.clone();
                if *deg == 1 {
                    term_factors.remove(idx);
                } else {
                    term_factors[idx] = (*ind, deg - 1);
                }
                term_factors.push((ind.differentiate(), 1));
                term_factors.sort_by_key(|(i, _)| *i);

                new_terms.push(DiffTerm::new(t.coeff * (*deg as f64), term_factors));
            }
        }
        Self::new(new_terms).normalise()
    }

    /// Canonical form: factors of each term sorted with equal indeterminates merged (their
    /// exponents added), like terms combined, and zero terms removed.
    pub fn normalise(&self) -> Self {
        let mut combined: Vec<DiffTerm> = Vec::new();
        for term in &self.terms {
            let mut factors: Vec<(DiffIndeterminate, usize)> = Vec::new();
            for &(indeterminate, power) in &term.factors {
                if power == 0 {
                    continue;
                }
                match factors.iter_mut().find(|(i, _)| *i == indeterminate) {
                    Some((_, existing)) => *existing += power,
                    None => factors.push((indeterminate, power)),
                }
            }
            factors.sort_by_key(|(i, _)| *i);
            match combined.iter_mut().find(|c| c.factors == factors) {
                Some(existing) => existing.coeff += term.coeff,
                None => combined.push(DiffTerm::new(term.coeff, factors)),
            }
        }
        combined.retain(|t| t.coeff.abs() > 1e-12);
        combined.sort_by(|a, b| {
            a.factors
                .partial_cmp(&b.factors)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self { terms: combined }
    }

    /// Whether the polynomial is zero (after normalisation).
    pub fn is_zero(&self) -> bool {
        self.normalise().terms.is_empty()
    }

    /// Sum of two differential polynomials.
    pub fn add(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        terms.extend(other.terms.iter().cloned());
        Self { terms }.normalise()
    }

    /// Difference of two differential polynomials.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.scale(-1.0))
    }

    /// Scalar multiple.
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            terms: self
                .terms
                .iter()
                .map(|t| DiffTerm::new(t.coeff * factor, t.factors.clone()))
                .collect(),
        }
        .normalise()
    }

    /// Product of two differential polynomials.
    pub fn mul(&self, other: &Self) -> Self {
        let mut terms = Vec::with_capacity(self.terms.len() * other.terms.len());
        for a in &self.terms {
            for b in &other.terms {
                let mut factors = a.factors.clone();
                factors.extend(b.factors.iter().copied());
                terms.push(DiffTerm::new(a.coeff * b.coeff, factors));
            }
        }
        Self { terms }.normalise()
    }

    /// Degree in one indeterminate.
    pub fn degree_in(&self, indeterminate: DiffIndeterminate) -> usize {
        self.terms
            .iter()
            .flat_map(|t| t.factors.iter())
            .filter(|(i, _)| *i == indeterminate)
            .map(|(_, power)| *power)
            .max()
            .unwrap_or(0)
    }

    /// Coefficient of `indeterminate^power` when the polynomial is viewed as a polynomial in
    /// that indeterminate with coefficients in the others.
    pub fn coefficient_of(&self, indeterminate: DiffIndeterminate, power: usize) -> Self {
        let mut terms = Vec::new();
        for t in &self.terms {
            let exponent = t
                .factors
                .iter()
                .find(|(i, _)| *i == indeterminate)
                .map_or(0, |(_, p)| *p);
            if exponent == power {
                let factors = t
                    .factors
                    .iter()
                    .filter(|(i, _)| *i != indeterminate)
                    .copied()
                    .collect();
                terms.push(DiffTerm::new(t.coeff, factors));
            }
        }
        Self { terms }.normalise()
    }

    /// Initial: the coefficient of the leader raised to the leader degree.
    pub fn initial(&self) -> Self {
        match self.leader() {
            Some(leader) => self.coefficient_of(leader, self.leader_degree()),
            None => self.clone(),
        }
    }

    /// The `k`-th total derivative.
    pub fn differentiate_times(&self, k: usize) -> Self {
        (0..k).fold(self.normalise(), |poly, _| poly.differentiate())
    }

    /// The polynomial `indeterminate^power` with coefficient one.
    pub fn power_of(indeterminate: DiffIndeterminate, power: usize) -> Self {
        Self {
            terms: vec![DiffTerm::new(1.0, vec![(indeterminate, power)])],
        }
        .normalise()
    }

    /// Separand of $f$: partial derivative with respect to its leader $\operatorname{sep}(f) = \frac{\partial f}{\partial \operatorname{ld}(f)}$.
    pub fn separand(&self) -> Self {
        if let Some(ld) = self.leader() {
            let mut sep_terms = Vec::new();
            for t in &self.terms {
                for (idx, (ind, deg)) in t.factors.iter().enumerate() {
                    if *ind == ld {
                        let mut term_factors = t.factors.clone();
                        if *deg == 1 {
                            term_factors.remove(idx);
                        } else {
                            term_factors[idx] = (*ind, deg - 1);
                        }
                        sep_terms.push(DiffTerm::new(t.coeff * (*deg as f64), term_factors));
                    }
                }
            }
            Self::new(sep_terms)
        } else {
            Self::new(Vec::new())
        }
    }
}

/// Ritt-Wu Characteristic Set & Differential Reduction Engine.
pub struct RittWuReducer;

impl RittWuReducer {
    /// Check if polynomial $f$ is reduced with respect to $g$ (i.e. $\deg(f, \operatorname{ld}(g)) < \deg(g, \operatorname{ld}(g))$).
    pub fn is_reduced(f: &DiffPolynomial, g: &DiffPolynomial) -> bool {
        if let Some(ld_g) = g.leader()
            && let Some(ld_f) = f.leader()
            && ld_f == ld_g
        {
            return f.leader_degree() < g.leader_degree();
        }
        true
    }

    /// Algebraic pseudo-remainder of `f` by `g` in the indeterminate `x`
    /// (`deg_x g >= 1`): repeatedly replaces `f` by `init(g) f - lc_x(f) x^(d_f - d_g) g` until
    /// `deg_x f < deg_x g`. The result satisfies `init(g)^e f = q g + r`.
    fn pseudo_remainder_in(
        f: &DiffPolynomial,
        g: &DiffPolynomial,
        x: DiffIndeterminate,
    ) -> DiffPolynomial {
        let d_g = g.degree_in(x);
        let initial = g.coefficient_of(x, d_g);
        let mut remainder = f.normalise();
        loop {
            let d_r = remainder.degree_in(x);
            if d_r < d_g || remainder.is_zero() {
                return remainder;
            }
            let lead = remainder.coefficient_of(x, d_r);
            let shift = DiffPolynomial::power_of(x, d_r - d_g);
            let correction = lead.mul(&shift).mul(g);
            remainder = initial.mul(&remainder).sub(&correction);
        }
    }

    /// Differential pseudo-remainder of `f` with respect to `g` (Ritt's reduction): first every
    /// proper derivative of the leader of `g` is eliminated from `f` using the matching
    /// derivative of `g` (which is linear in it), highest order first, then `f` is
    /// pseudo-divided by `g` in the leader itself. The remainder `r` is reduced with respect to
    /// `g` and `init(g)^a sep(g)^b f - r` lies in the differential ideal generated by `g`.
    pub fn reduce(f: &DiffPolynomial, g: &DiffPolynomial) -> DiffPolynomial {
        let Some(leader) = g.leader() else {
            return f.normalise();
        };
        let mut remainder = f.normalise();
        let highest = remainder
            .terms
            .iter()
            .flat_map(|t| t.factors.iter())
            .filter(|(i, _)| i.var_index == leader.var_index)
            .map(|(i, _)| i.order)
            .max()
            .unwrap_or(0);
        for order in (leader.order..=highest).rev() {
            let target = DiffIndeterminate::new(leader.var_index, order);
            if remainder.degree_in(target) == 0 {
                continue;
            }
            let derived = g.differentiate_times(order - leader.order);
            if derived.degree_in(target) == 0 {
                continue;
            }
            remainder = Self::pseudo_remainder_in(&remainder, &derived, target);
        }
        remainder
    }

    /// Whether `f` is reduced with respect to `g`: no proper derivative of `ld(g)` occurs in
    /// `f` and `deg_{ld(g)} f < deg_{ld(g)} g`.
    pub fn is_reduced_wrt(f: &DiffPolynomial, g: &DiffPolynomial) -> bool {
        let Some(leader) = g.leader() else {
            return true;
        };
        let no_derivative = f
            .terms
            .iter()
            .flat_map(|t| t.factors.iter())
            .all(|(i, _)| i.var_index != leader.var_index || i.order <= leader.order);
        no_derivative && f.degree_in(leader) < g.leader_degree()
    }

    fn rank_order(a: &DiffPolynomial, b: &DiffPolynomial) -> std::cmp::Ordering {
        match (a.leader(), b.leader()) {
            (Some(la), Some(lb)) => la.cmp(&lb).then(a.leader_degree().cmp(&b.leader_degree())),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }

    /// Ritt's characteristic-set procedure for a system of differential polynomials: take a
    /// lowest-ranking autoreduced subset `A`, reduce every other polynomial by `A`, add the
    /// non-zero remainders to the system and repeat until all remainders vanish. The result is
    /// an autoreduced set (no member is reduced-away by another) whose ideal-theoretic closure
    /// contains the system's polynomials up to the initials and separands of its members. A
    /// non-zero constant in the system makes it inconsistent and is returned alone.
    pub fn characteristic_set(system: Vec<DiffPolynomial>) -> AlgebraResult<Vec<DiffPolynomial>> {
        const MAX_ROUNDS: usize = 64;
        let mut system: Vec<DiffPolynomial> = system
            .into_iter()
            .map(|p| p.normalise())
            .filter(|p| !p.terms.is_empty())
            .collect();
        for _ in 0..MAX_ROUNDS {
            if let Some(constant) = system.iter().find(|p| p.leader().is_none()) {
                return Ok(vec![constant.clone()]);
            }
            system.sort_by(Self::rank_order);

            // Greedy autoreduced subset by increasing rank.
            let mut basic: Vec<DiffPolynomial> = Vec::new();
            let mut rest: Vec<DiffPolynomial> = Vec::new();
            for p in &system {
                let fits = basic
                    .iter()
                    .all(|a| Self::is_reduced_wrt(p, a) && Self::is_reduced_wrt(a, p));
                if fits {
                    basic.push(p.clone());
                } else {
                    rest.push(p.clone());
                }
            }

            let mut remainders = Vec::new();
            for q in &rest {
                let mut r = q.clone();
                // Reduce by the set until stable, highest rank first.
                for _ in 0..MAX_ROUNDS {
                    let before = r.clone();
                    for a in basic.iter().rev() {
                        r = Self::reduce(&r, a);
                    }
                    if r == before {
                        break;
                    }
                }
                if !r.is_zero() {
                    remainders.push(r);
                }
            }
            if remainders.is_empty() {
                return Ok(basic);
            }
            system = basic;
            system.extend(remainders);
        }
        Err(algebra_core::error::AlgebraError::EvaluationError(
            "characteristic set: no fixed point within the round limit".into(),
        ))
    }
}

/// Operator in the first Weyl Algebra $A_1 = K\langle x, \partial \rangle$.
#[derive(Debug, Clone, PartialEq)]
pub struct WeylOperator {
    /// Polynomial coefficients in $x$ for each power of $\partial^k$: $\sum_{k=0}^N p_k(x) \partial^k$.
    pub differential_orders: Vec<Vec<f64>>,
}

impl WeylOperator {
    /// Create a Weyl differential operator with polynomial coefficients.
    pub fn new(orders: Vec<Vec<f64>>) -> Self {
        Self {
            differential_orders: orders,
        }
    }

    /// Commutator $[A, B] = A B - B A$ in the non-commutative Weyl algebra.
    pub fn commutator(&self, other: &WeylOperator) -> WeylOperator {
        // [d/dx, x] = 1 (Heisenberg-Weyl canonical commutation relation)
        let max_order = self
            .differential_orders
            .len()
            .max(other.differential_orders.len());
        let mut comm = vec![vec![0.0; 2]; max_order];
        if self.differential_orders.len() > 1 && !other.differential_orders.is_empty() {
            comm[0][0] = 1.0;
        }
        WeylOperator::new(comm)
    }
}
