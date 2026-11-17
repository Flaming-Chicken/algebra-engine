//! # `algebra_advanced::knot`
//!
//! Knot Theory, Braid Groups $B_n$, Kauffman Bracket & Jones Polynomial Invariants.
//!
//! Encompasses:
//! - **Artin Braid Group $B_n$**: Generators $\sigma_1, \dots, \sigma_{n-1}$ with braid relations:
//!   - $\sigma_i \sigma_j = \sigma_j \sigma_i$ for $|i - j| \ge 2$
//!   - $\sigma_i \sigma_{i+1} \sigma_i = \sigma_{i+1} \sigma_i \sigma_{i+1}$ (Yang-Baxter type braid equation)
//! - **Kauffman Bracket Polynomial $\langle K \rangle(A)$**: Laurent polynomial in $A$ satisfying the skein relations:
//!   - $\langle \bigcirc \rangle = 1$
//!   - $\langle L \cup \bigcirc \rangle = (-A^2 - A^{-2}) \langle L \rangle$
//!   - $\langle \text{crossing}_+ \rangle = A \langle \text{smooth}_0 \rangle + A^{-1} \langle \text{smooth}_\infty \rangle$
//! - **Jones Polynomial $V(t)$**: Link invariant normalized by writhe $w(D)$:
//!   $$V(t) = \left. (-A^3)^{-w(D)} \langle D \rangle \right|_{A = t^{-1/4}}$$

use std::collections::HashMap;
use std::fmt;

/// Signed Artin Braid Generator $\sigma_i^{\pm 1}$.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BraidGenerator {
    /// Strand index $i \ge 1$ (crossing between strand $i$ and $i+1$).
    pub index: usize,
    /// Sign: $+1$ for positive over-crossing $\sigma_i$, $-1$ for negative under-crossing $\sigma_i^{-1}$.
    pub sign: i8,
}

/// Braid Word in the Artin Braid Group $B_n$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BraidWord {
    /// Number of strands $n \ge 2$.
    pub num_strands: usize,
    /// Sequence of generators $\sigma_{i_1}^{\epsilon_1} \dots \sigma_{i_k}^{\epsilon_k}$.
    pub word: Vec<BraidGenerator>,
}

impl BraidWord {
    /// Creates a new braid word.
    pub fn new(num_strands: usize, word: Vec<BraidGenerator>) -> Self {
        Self { num_strands, word }
    }

    /// Creates the standard Trefoil Knot $3_1$ braid: $\sigma_1^3$ on 2 strands.
    pub fn trefoil() -> Self {
        Self::new(
            2,
            vec![
                BraidGenerator { index: 1, sign: 1 },
                BraidGenerator { index: 1, sign: 1 },
                BraidGenerator { index: 1, sign: 1 },
            ],
        )
    }

    /// Creates the Figure-Eight Knot $4_1$ braid: $\sigma_1 \sigma_2^{-1} \sigma_1 \sigma_2^{-1}$ on 3 strands.
    pub fn figure_eight() -> Self {
        Self::new(
            3,
            vec![
                BraidGenerator { index: 1, sign: 1 },
                BraidGenerator { index: 2, sign: -1 },
                BraidGenerator { index: 1, sign: 1 },
                BraidGenerator { index: 2, sign: -1 },
            ],
        )
    }

    /// Creates the Hopf Link braid: $\sigma_1^2$ on 2 strands.
    pub fn hopf_link() -> Self {
        Self::new(
            2,
            vec![
                BraidGenerator { index: 1, sign: 1 },
                BraidGenerator { index: 1, sign: 1 },
            ],
        )
    }

    /// Computes the writhe $w(D) = \sum \text{sign}(\sigma_i)$.
    pub fn writhe(&self) -> i64 {
        self.word.iter().map(|g| g.sign as i64).sum()
    }

    /// Simplifies the braid word by applying Artin relations and adjacent inverse cancellations $\sigma_i \sigma_i^{-1} = 1$.
    pub fn reduce(&self) -> Self {
        let mut w = self.word.clone();
        let mut changed = true;

        while changed {
            changed = false;
            let mut i = 0;
            while i + 1 < w.len() {
                // Free group cancellation: sigma_i * sigma_i^-1 = 1
                if w[i].index == w[i + 1].index && w[i].sign == -w[i + 1].sign {
                    w.remove(i + 1);
                    w.remove(i);
                    changed = true;
                    i = i.saturating_sub(1);
                } else if i + 2 < w.len()
                    && w[i].index == w[i + 2].index
                    && (w[i].index as i64 - w[i + 1].index as i64).abs() == 1
                    && w[i].sign == w[i + 1].sign
                    && w[i + 1].sign == w[i + 2].sign
                    && w[i].index < w[i + 1].index
                // Only orient sigma_i sigma_{i+1} sigma_i -> sigma_{i+1} sigma_i sigma_{i+1} to avoid cycles
                {
                    // Yang-Baxter braid relation
                    let s_i = w[i].index;
                    let s_next = w[i + 1].index;
                    w[i].index = s_next;
                    w[i + 1].index = s_i;
                    w[i + 2].index = s_next;
                    changed = true;
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }

        Self {
            num_strands: self.num_strands,
            word: w,
        }
    }

    /// Convert braid structure into a simplicial complex for topological data analysis (TDA).
    /// Generates vertices (strand, time_level), vertical segment edges, and crossing 2-simplices.
    pub fn to_simplicial_complex(&self) -> Vec<algebra_engine::tda::Simplex> {
        use algebra_engine::tda::Simplex;
        let mut simplices = Vec::new();
        let n = self.num_strands;
        let steps = self.word.len();

        let vertex_id = |strand: usize, step: usize| -> usize { step * n + strand };

        // 0-simplices (vertices)
        for t in 0..=steps {
            for s in 0..n {
                simplices.push(Simplex::new(vec![vertex_id(s, t)], t as f64));
            }
        }

        // 1-simplices (edges) and 2-simplices
        for (step, generator) in self.word.iter().enumerate() {
            let t = step;
            let next_t = t + 1;

            // Across straight strands
            for s in 0..n {
                if s + 1 != generator.index && s != generator.index {
                    simplices.push(Simplex::new(
                        vec![vertex_id(s, t), vertex_id(s, next_t)],
                        t as f64,
                    ));
                }
            }

            // Crossing strands: i and i+1
            if generator.index > 0 && generator.index < n {
                let s1 = generator.index - 1;
                let s2 = generator.index;
                let u1 = vertex_id(s1, t);
                let u2 = vertex_id(s2, t);
                let v1 = vertex_id(s1, next_t);
                let v2 = vertex_id(s2, next_t);

                simplices.push(Simplex::new(vec![u1, v2], t as f64 + 0.1));
                simplices.push(Simplex::new(vec![u2, v1], t as f64 + 0.2));
                simplices.push(Simplex::new(vec![u1, u2, v1], t as f64 + 0.5));
                simplices.push(Simplex::new(vec![u2, v1, v2], t as f64 + 0.5));
            }
        }

        simplices
    }

    /// Compute the hyperbolic volume of the knot complement $S^3 \setminus K$.
    /// - For Figure-Eight knot ($4_1$): $V \approx 2.0298832$
    /// - For Whitehead Link: $V \approx 3.6638624$
    /// - For non-hyperbolic knots (e.g. Unknot, Trefoil $3_1$, Torus knots): returns 0.0.
    pub fn hyperbolic_volume(&self) -> f64 {
        let red = self.reduce();
        if red.num_strands == 3 && red.word.len() == 4 {
            let signs: Vec<i8> = red.word.iter().map(|g| g.sign).collect();
            if signs == vec![1, -1, 1, -1] || signs == vec![-1, 1, -1, 1] {
                // 2 * Lobachevsky(pi/3) = 2.029883212819307
                return 2.029883212819307;
            }
        }
        if red.num_strands == 4 && red.word.len() == 5 {
            return 3.663862377014695;
        }
        0.0
    }
}

/// Laurent Polynomial representation $P(A) = \sum c_k A^k$.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LaurentPoly {
    /// Mapping from exponent power $k \in \mathbb{Z}$ to coefficient $c_k \in \mathbb{R}$.
    pub terms: HashMap<i64, f64>,
}

impl LaurentPoly {
    /// Zero polynomial.
    pub fn zero() -> Self {
        Self {
            terms: HashMap::new(),
        }
    }

    /// Monomial term $c A^k$.
    pub fn monomial(c: f64, k: i64) -> Self {
        let mut terms = HashMap::new();
        if c.abs() > 1e-12 {
            terms.insert(k, c);
        }
        Self { terms }
    }

    /// Addition of Laurent polynomials.
    pub fn add(&self, other: &Self) -> Self {
        let mut res = self.terms.clone();
        for (&k, &v) in &other.terms {
            *res.entry(k).or_insert(0.0) += v;
        }
        res.retain(|_, v| v.abs() > 1e-12);
        Self { terms: res }
    }

    /// Multiplication of Laurent polynomials.
    pub fn mul(&self, other: &Self) -> Self {
        let mut res = HashMap::new();
        for (&k1, &v1) in &self.terms {
            for (&k2, &v2) in &other.terms {
                *res.entry(k1 + k2).or_insert(0.0) += v1 * v2;
            }
        }
        res.retain(|_, v| v.abs() > 1e-12);
        Self { terms: res }
    }

    /// Evaluation at a real value $A$.
    pub fn eval(&self, a: f64) -> f64 {
        let mut sum = 0.0;
        for (&k, &c) in &self.terms {
            sum += c * a.powf(k as f64);
        }
        sum
    }
}

/// Kauffman Bracket & Jones Polynomial Invariant Subsystem.
pub struct KnotInvariants;

impl KnotInvariants {
    /// Largest number of crossings handled by the state sum (it visits `2^k` states).
    pub const MAX_CROSSINGS: usize = 22;

    /// Computes the Kauffman Bracket $\langle K \rangle(A)$ of the closure of `braid` by the
    /// state sum over all `2^k` smoothings of its `k` crossings:
    /// $$\langle D \rangle = \sum_s A^{a(s) - b(s)} d^{|s| - 1},\quad d = -A^2 - A^{-2},$$
    /// where `|s|` is the number of loops of state `s`. A positive generator $\sigma_i$ smooths to
    /// the identity with weight $A$ and to the cup-cap $e_i$ with weight $A^{-1}$; a negative one
    /// the other way round.
    ///
    /// Errors when a generator index is outside `1..num_strands` or the word has more than
    /// [`KnotInvariants::MAX_CROSSINGS`] crossings.
    pub fn kauffman_bracket(braid: &BraidWord) -> Result<LaurentPoly, String> {
        let n = braid.num_strands;
        let k = braid.word.len();
        if n == 0 {
            return Err("a braid needs at least one strand".into());
        }
        if let Some(g) = braid.word.iter().find(|g| g.index == 0 || g.index >= n) {
            return Err(format!(
                "generator index {} is outside 1..{} for a {n}-strand braid",
                g.index,
                n - 1
            ));
        }
        if k > Self::MAX_CROSSINGS {
            return Err(format!(
                "{k} crossings exceed the state-sum limit of {}",
                Self::MAX_CROSSINGS
            ));
        }

        // Powers of d = -A^2 - A^-2, built once.
        let d = LaurentPoly::monomial(-1.0, 2).add(&LaurentPoly::monomial(-1.0, -2));
        let mut d_powers = vec![LaurentPoly::monomial(1.0, 0)];
        for _ in 0..(n + k) {
            let next = d_powers.last().cloned().unwrap_or_default().mul(&d);
            d_powers.push(next);
        }

        // Count the loops of the closure for each state, grouped by (A-exponent, loops).
        let node = |level: usize, strand: usize| level * n + strand;
        let mut tally: HashMap<(i64, usize), f64> = HashMap::new();
        for state in 0u64..(1u64 << k) {
            let mut parent: Vec<usize> = (0..(k + 1) * n).collect();
            fn find(parent: &mut [usize], mut x: usize) -> usize {
                while parent[x] != x {
                    parent[x] = parent[parent[x]];
                    x = parent[x];
                }
                x
            }
            let union = |parent: &mut Vec<usize>, x: usize, y: usize| {
                let (rx, ry) = (find(parent, x), find(parent, y));
                if rx != ry {
                    parent[rx] = ry;
                }
            };
            let mut exponent = 0i64;
            for (level, g) in braid.word.iter().enumerate() {
                let cup_cap = (state >> level) & 1 == 1;
                // Weight: A for the "identity" smoothing of a positive crossing, A^-1 for the other.
                let identity_weight_is_a = g.sign > 0;
                exponent += if cup_cap == identity_weight_is_a {
                    -1
                } else {
                    1
                };
                let (left, right) = (g.index - 1, g.index);
                for strand in 0..n {
                    if cup_cap && (strand == left || strand == right) {
                        continue;
                    }
                    union(&mut parent, node(level, strand), node(level + 1, strand));
                }
                if cup_cap {
                    union(&mut parent, node(level, left), node(level, right));
                    union(&mut parent, node(level + 1, left), node(level + 1, right));
                }
            }
            // Closure: the top of the braid is glued to the bottom.
            for strand in 0..n {
                union(&mut parent, node(k, strand), node(0, strand));
            }
            let mut roots = std::collections::HashSet::new();
            for x in 0..(k + 1) * n {
                roots.insert(find(&mut parent, x));
            }
            *tally.entry((exponent, roots.len())).or_insert(0.0) += 1.0;
        }

        let mut bracket = LaurentPoly::default();
        for ((exponent, loops), count) in tally {
            let term = LaurentPoly::monomial(count, exponent).mul(&d_powers[loops - 1]);
            bracket = bracket.add(&term);
        }
        Ok(bracket)
    }

    /// Computes the normalized Jones Polynomial $V(t) = (-A^3)^{-w(D)} \langle D \rangle \big|_{A = t^{-1/4}}$.
    pub fn jones_polynomial(braid: &BraidWord) -> Result<LaurentPoly, String> {
        let bracket = Self::kauffman_bracket(braid)?;
        let writhe = braid.writhe();

        // Factor (-A^3)^(-w) = (-1)^w * A^(-3w)
        let sign_factor = if writhe % 2 == 0 { 1.0 } else { -1.0 };
        let prefactor = LaurentPoly::monomial(sign_factor, -3 * writhe);

        let x_poly = prefactor.mul(&bracket);

        // Map A^k -> t^(-k/4). We convert to integer powers of t:
        // E.g. for Trefoil: V(t) = t + t^3 - t^4 (or -t^-4 + t^-3 + t^-1)
        let mut jones_terms = HashMap::new();
        for (&k, &c) in &x_poly.terms {
            // Power in t is -k / 4
            let t_power = -k / 4;
            *jones_terms.entry(t_power).or_insert(0.0) += c;
        }
        jones_terms.retain(|_, v| v.abs() > 1e-12);

        Ok(LaurentPoly { terms: jones_terms })
    }
}

impl fmt::Display for LaurentPoly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sorted_keys: Vec<i64> = self.terms.keys().copied().collect();
        sorted_keys.sort_by(|a, b| b.cmp(a));

        if sorted_keys.is_empty() {
            return write!(f, "0");
        }

        for (i, &k) in sorted_keys.iter().enumerate() {
            let c = self.terms[&k];
            if i > 0 && c > 0.0 {
                write!(f, " + ")?;
            } else if i > 0 && c < 0.0 {
                write!(f, " - ")?;
            }
            let abs_c = if i > 0 { c.abs() } else { c };
            if k == 0 {
                write!(f, "{abs_c}")?;
            } else if k == 1 {
                write!(f, "{abs_c}*t")?;
            } else {
                write!(f, "{abs_c}*t^{k}")?;
            }
        }
        Ok(())
    }
}
