//! # `algebra_advanced::galois`
//!
//! Galois Theory & Algebraic Field Extensions:
//! - Field extensions $K / F$
//! - Minimal Polynomials
//! - Galois Group $\text{Gal}(K / \mathbb{Q})$ classification
//! - Solvability by Radicals (Abel-Ruffini theorem checks)

use algebra_engine::poly::UniPoly;
use algebra_engine::poly::factor::factor_integer_polynomial;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use serde::{Deserialize, Serialize};

/// Classification of Galois group structure for low-degree polynomials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GaloisGroupType {
    /// Trivial identity group
    Identity,
    /// Cyclic group $C_n$
    Cyclic(usize),
    /// Dihedral group $D_n$
    Dihedral(usize),
    /// Alternating group $A_n$
    Alternating(usize),
    /// Symmetric group $S_n$
    Symmetric(usize),
    /// Klein four-group $V_4 \cong C_2 \times C_2$
    KleinFour,
}

impl GaloisGroupType {
    /// Check if the Galois group is solvable (by radicals).
    /// All $S_n$ and $A_n$ for $n \ge 5$ are non-solvable.
    pub fn is_solvable(&self) -> bool {
        match self {
            GaloisGroupType::Identity => true,
            GaloisGroupType::Cyclic(_) => true,
            GaloisGroupType::Dihedral(_) => true,
            GaloisGroupType::KleinFour => true,
            GaloisGroupType::Alternating(n) => *n < 5,
            GaloisGroupType::Symmetric(n) => *n < 5,
        }
    }
}

/// Simple field extension $F(\alpha) / F$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldExtension {
    pub base_field: String,
    pub generator_symbol: String,
    pub degree: usize,
    pub is_galois: bool,
    pub galois_group: Option<GaloisGroupType>,
}

impl FieldExtension {
    /// Create the extension $\mathbb{Q}(\sqrt{d})$: quadratic with group $C_2$ unless `d` is a perfect square
    /// (including 0 and 1), when the extension is trivial.
    pub fn quadratic(d: i64) -> Self {
        let is_square = d >= 0 && {
            let r = (d as f64).sqrt().round() as i64;
            r * r == d
        };
        Self {
            base_field: "Q".to_string(),
            generator_symbol: format!("sqrt({})", d),
            degree: if is_square { 1 } else { 2 },
            is_galois: true,
            galois_group: Some(if is_square {
                GaloisGroupType::Identity
            } else {
                GaloisGroupType::Cyclic(2)
            }),
        }
    }

    /// Check if roots of this polynomial extension are expressible in radicals.
    pub fn is_solvable_by_radicals(&self) -> bool {
        self.galois_group
            .as_ref()
            .map(|g| g.is_solvable())
            .unwrap_or(self.degree < 5)
    }
}

/// Why a Galois group could not be computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GaloisError {
    /// The polynomial is zero or constant.
    NoRoots,
    /// The polynomial is outside what this classification handles (degree above 4, or a reducible polynomial whose
    /// factors are not all of degree at most 2).
    Unsupported(String),
}

impl std::fmt::Display for GaloisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRoots => write!(f, "the polynomial has no roots to permute"),
            Self::Unsupported(why) => write!(f, "unsupported: {why}"),
        }
    }
}

impl std::error::Error for GaloisError {}

fn is_square(n: &BigInt) -> bool {
    if n.is_negative() {
        return false;
    }
    let r = n.sqrt();
    &r * &r == *n
}

/// The squarefree part of a non-zero integer, with its sign: the class of `n` in $\mathbb{Q}^\times / (\mathbb{Q}^\times)^2$.
fn squarefree_class(n: &BigInt) -> BigInt {
    let sign = if n.is_negative() {
        -BigInt::one()
    } else {
        BigInt::one()
    };
    let mut rest = n.abs();
    let mut out = BigInt::one();
    let mut d = BigInt::from(2);
    while &d * &d <= rest {
        let mut e = 0;
        while (&rest % &d).is_zero() {
            rest /= &d;
            e += 1;
        }
        if e % 2 == 1 {
            out *= &d;
        }
        d += 1;
    }
    sign * out * rest
}

/// The discriminant of an integer polynomial as an integer (it is one when the polynomial is monic).
fn integer_discriminant(f: &[BigInt]) -> BigInt {
    let q = UniPoly::from_coeffs(
        f.iter()
            .map(|c| BigRational::from_integer(c.clone()))
            .collect(),
    );
    q.discriminant().map(|d| d.to_integer()).unwrap_or_default()
}

/// A monic integer polynomial with the same splitting field as `f`: $a_n^{n-1} f(x / a_n)$.
fn monic_integer_model(f: &[BigInt]) -> Vec<BigInt> {
    let n = f.len() - 1;
    let lead = f[n].clone();
    (0..=n)
        .map(|i| {
            // coefficient of x^i is a_i * a_n^(n - 1 - i) for i < n, and 1 for i = n
            if i == n {
                BigInt::one()
            } else {
                &f[i] * lead.pow((n - 1 - i) as u32)
            }
        })
        .collect()
}

/// The Galois group over $\mathbb{Q}$ of the splitting field of a polynomial, for degree at most 4 (and any product
/// of linear and quadratic factors).
///
/// Irreducible cubics are $C_3$ or $S_3$ according to whether the discriminant is a square. Irreducible quartics are
/// classified with the resolvent cubic $R(x) = x^3 - b x^2 + (ac - 4d) x - (a^2 d - 4 b d + c^2)$: $S_4$ if it is
/// irreducible with a non-square discriminant, $A_4$ if the discriminant is a square, $V_4$ if it splits completely,
/// and with exactly one rational root $r$ either $C_4$ or $D_4$, decided by whether $x^2 - r x + d$ and
/// $x^2 + a x + (b - r)$ both split over $\mathbb{Q}(\sqrt{\Delta})$ (Kappe–Warren).
pub fn galois_group_over_q(coeffs: &[BigInt]) -> Result<GaloisGroupType, GaloisError> {
    let mut f = coeffs.to_vec();
    while f.last().is_some_and(|c| c.is_zero()) {
        f.pop();
    }
    if f.len() < 2 {
        return Err(GaloisError::NoRoots);
    }
    let factorisation =
        factor_integer_polynomial(&f).map_err(|e| GaloisError::Unsupported(e.to_string()))?;
    let mut irreducibles: Vec<Vec<BigInt>> =
        factorisation.factors.into_iter().map(|(g, _)| g).collect();
    irreducibles.sort_by_key(|g| g.len());

    if irreducibles.len() > 1 || irreducibles[0].len() <= 3 {
        // A compositum of quadratic fields: $(\mathbb{Z}/2)^r$ with $r$ the rank of the discriminant classes.
        let mut classes: Vec<BigInt> = Vec::new();
        for g in &irreducibles {
            match g.len() - 1 {
                1 => {}
                2 => classes.push(squarefree_class(&integer_discriminant(g))),
                n => {
                    return Err(GaloisError::Unsupported(format!(
                        "a reducible polynomial with an irreducible factor of degree {n}"
                    )));
                }
            }
        }
        let rank = f2_rank_of_classes(&classes);
        return Ok(match rank {
            0 => GaloisGroupType::Identity,
            1 => GaloisGroupType::Cyclic(2),
            2 => GaloisGroupType::KleinFour,
            r => {
                return Err(GaloisError::Unsupported(format!(
                    "the compositum of {r} independent quadratic fields has group (Z/2)^{r}"
                )));
            }
        });
    }

    let g = &irreducibles[0];
    let n = g.len() - 1;
    let monic = monic_integer_model(g);
    let disc = integer_discriminant(&monic);
    let disc_is_square = is_square(&disc);
    match n {
        3 => Ok(if disc_is_square {
            GaloisGroupType::Cyclic(3)
        } else {
            GaloisGroupType::Symmetric(3)
        }),
        4 => {
            let (d, c, b, a) = (&monic[0], &monic[1], &monic[2], &monic[3]);
            let resolvent = vec![
                -(a * a * d - BigInt::from(4) * b * d + c * c),
                a * c - BigInt::from(4) * d,
                -b.clone(),
                BigInt::one(),
            ];
            let degrees: Vec<usize> = factor_integer_polynomial(&resolvent)
                .map_err(|e| GaloisError::Unsupported(e.to_string()))?
                .factors
                .iter()
                .flat_map(|(g, m)| std::iter::repeat_n(g.len() - 1, *m as usize))
                .collect();
            if degrees == [3] {
                return Ok(if disc_is_square {
                    GaloisGroupType::Alternating(4)
                } else {
                    GaloisGroupType::Symmetric(4)
                });
            }
            if degrees.iter().all(|&d| d == 1) {
                return Ok(GaloisGroupType::KleinFour);
            }
            // Exactly one rational root r of the resolvent.
            let root = rational_integer_root(&resolvent).ok_or_else(|| {
                GaloisError::Unsupported("the resolvent cubic has no integer root".into())
            })?;
            let splits_over_q_sqrt_disc = |p: &BigInt, q: &BigInt| -> bool {
                // x^2 + p x + q has discriminant p^2 - 4q; it splits over Q(sqrt(D)) iff that is a square or D times a square.
                let delta = p * p - BigInt::from(4) * q;
                if delta.is_zero() || is_square(&delta) {
                    return true;
                }
                if disc_is_square {
                    return false;
                }
                squarefree_class(&delta) == squarefree_class(&disc)
            };
            let first = splits_over_q_sqrt_disc(&-root.clone(), d);
            let second = splits_over_q_sqrt_disc(a, &(b - &root));
            Ok(if first && second {
                GaloisGroupType::Cyclic(4)
            } else {
                GaloisGroupType::Dihedral(4)
            })
        }
        _ => Err(GaloisError::Unsupported(format!(
            "irreducible polynomials of degree {n}"
        ))),
    }
}

/// An integer root of a monic integer polynomial (a divisor of the constant term), if one exists.
fn rational_integer_root(f: &[BigInt]) -> Option<BigInt> {
    let constant = f[0].abs();
    if constant.is_zero() {
        return Some(BigInt::zero());
    }
    let value_at = |x: &BigInt| f.iter().rev().fold(BigInt::zero(), |acc, c| acc * x + c);
    let mut d = BigInt::one();
    while d <= constant {
        if (&constant % &d).is_zero() {
            for candidate in [d.clone(), -d.clone()] {
                if value_at(&candidate).is_zero() {
                    return Some(candidate);
                }
            }
        }
        d += 1;
    }
    None
}

/// The rank over $\mathbb{F}_2$ of a set of squarefree classes: the dimension of the span of their prime-exponent
/// parity vectors (with the sign as one more coordinate).
fn f2_rank_of_classes(classes: &[BigInt]) -> usize {
    let mut primes: Vec<BigInt> = Vec::new();
    for c in classes {
        let mut rest = c.abs();
        let mut d = BigInt::from(2);
        while &d * &d <= rest {
            if (&rest % &d).is_zero() {
                if !primes.contains(&d) {
                    primes.push(d.clone());
                }
                while (&rest % &d).is_zero() {
                    rest /= &d;
                }
            }
            d += 1;
        }
        if rest > BigInt::one() && !primes.contains(&rest) {
            primes.push(rest);
        }
    }
    let mut rows: Vec<Vec<bool>> = classes
        .iter()
        .map(|c| {
            let mut row: Vec<bool> = primes.iter().map(|p| (&c.abs() % p).is_zero()).collect();
            row.push(c.is_negative());
            row
        })
        .collect();
    let width = primes.len() + 1;
    let mut rank = 0;
    for col in 0..width {
        let Some(pivot) = (rank..rows.len()).find(|&r| rows[r][col]) else {
            continue;
        };
        rows.swap(rank, pivot);
        for r in 0..rows.len() {
            if r != rank && rows[r][col] {
                let pivot_row = rows[rank].clone();
                for (x, y) in rows[r].iter_mut().zip(pivot_row) {
                    *x ^= y;
                }
            }
        }
        rank += 1;
    }
    rank
}
