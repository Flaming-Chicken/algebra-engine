//! # `algebra_advanced::surreal`
//!
//! Conway Surreal Numbers $\mathbf{No}$.
//!
//! Encompasses:
//! - **Dedekind-Conway Games**: Inductive construction $x = \{ L_x \mid R_x \}$ where no $l \in L_x \ge r \in R_x$.
//! - **Canonical Surreal Forms**: Dyadic fractions $\frac{m}{2^n}$, integers $\mathbb{Z}$, and birthday ordering.
//! - **Surreal Arithmetic**:
//!   - Addition: $x + y = \{ L_x + y, x + L_y \mid R_x + y, x + R_y \}$
//!   - Negation: $-x = \{ -R_x \mid -L_x \}$
//!   - Multiplication: $x \cdot y = \{ x'y + xy' - x'y' \mid \dots \}$
//! - **Transfinite & Infinitesimal Constants**:
//!   - Zero $\mathbf{0} = \{ \emptyset \mid \emptyset \}$
//!   - One $\mathbf{1} = \{ 0 \mid \emptyset \}$
//!   - Half $1/2 = \{ 0 \mid 1 \}$
//!   - Infinitesimal $\epsilon = \{ 0 \mid 1, 1/2, 1/4, \dots \}$
//!   - Transfinite $\omega = \{ 0, 1, 2, 3, \dots \mid \emptyset \}$

use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::fmt;

/// Inductive Conway Surreal Number representation.
#[derive(Debug, Clone, PartialEq)]
pub struct SurrealNo {
    /// Left set of surreal numbers $\{ l \in L \}$.
    pub left: Vec<SurrealNo>,
    /// Right set of surreal numbers $\{ r \in R \}$.
    pub right: Vec<SurrealNo>,
    /// Birthday / generation index.
    pub birthday: usize,
    /// Cached floating-point value (the exact value rounded when [`SurrealNo::value`] is known).
    pub approx: f64,
    /// The exact value, a dyadic rational, when the form is a valid number (every left option below every right option).
    ///
    /// Every surreal built from finite sets is a dyadic rational, namely the simplest number between its
    /// largest left option and its smallest right option; infinite sets (a true $\omega$ or $\epsilon$) cannot
    /// be stored in a `Vec`, so `omega(n)` and `epsilon(n)` below are finite truncations with finite values.
    pub value: Option<BigRational>,
}

/// The simplest dyadic rational strictly between `lo` and `hi` (either bound may be absent): the integer of smallest
/// absolute value if there is one, else the fraction with the smallest power-of-two denominator.
fn simplest_between(lo: Option<&BigRational>, hi: Option<&BigRational>) -> BigRational {
    let zero = BigRational::zero();
    match (lo, hi) {
        (None, None) => zero,
        (Some(l), None) => {
            if l.is_negative() {
                zero
            } else {
                BigRational::from_integer(l.floor().to_integer() + 1)
            }
        }
        (None, Some(h)) => {
            if h.is_positive() {
                zero
            } else {
                BigRational::from_integer(h.ceil().to_integer() - 1)
            }
        }
        (Some(l), Some(h)) => {
            if l.is_negative() && h.is_positive() {
                return zero;
            }
            if !l.is_negative() {
                // 0 <= l < h: search upward, integers first and then halves, quarters, ...
                let mut denominator = BigRational::one();
                loop {
                    let candidate = (l * &denominator).floor() + BigRational::one();
                    let value = candidate / &denominator;
                    if &value < h {
                        return value;
                    }
                    denominator *= BigRational::from_integer(2.into());
                }
            }
            // l < h <= 0: mirror image.
            -simplest_between(Some(&-h.clone()), Some(&-l.clone()))
        }
    }
}

impl SurrealNo {
    /// Creates a surreal number from left and right sets: $\{ L \mid R \}$.
    pub fn new(left: Vec<SurrealNo>, right: Vec<SurrealNo>) -> Self {
        let max_b_l = left.iter().map(|s| s.birthday).max().unwrap_or(0);
        let max_b_r = right.iter().map(|s| s.birthday).max().unwrap_or(0);
        let birthday = if left.is_empty() && right.is_empty() {
            0
        } else {
            max_b_l.max(max_b_r) + 1
        };

        let left_approx = left
            .iter()
            .map(|s| s.approx)
            .fold(f64::NEG_INFINITY, f64::max);
        let right_approx = right.iter().map(|s| s.approx).fold(f64::INFINITY, f64::min);

        // The exact value, from the children's cached values: the simplest number between the largest
        // left option and the smallest right option, provided every left option is below every right one.
        let value = {
            let lefts: Option<Vec<&BigRational>> = left.iter().map(|s| s.value.as_ref()).collect();
            let rights: Option<Vec<&BigRational>> =
                right.iter().map(|s| s.value.as_ref()).collect();
            match (lefts, rights) {
                (Some(l), Some(r)) => {
                    let max_l = l.into_iter().max();
                    let min_r = r.into_iter().min();
                    match (max_l, min_r) {
                        (Some(a), Some(b)) if a >= b => None,
                        (a, b) => Some(simplest_between(a, b)),
                    }
                }
                _ => None,
            }
        };

        let approx = if let Some(v) = &value {
            v.to_f64().unwrap_or(f64::NAN)
        } else if left.is_empty() && right.is_empty() {
            0.0
        } else if right.is_empty() {
            left_approx + 1.0
        } else if left.is_empty() {
            right_approx - 1.0
        } else {
            (left_approx + right_approx) / 2.0
        };

        Self {
            left,
            right,
            birthday,
            approx,
            value,
        }
    }

    /// The exact value of a valid number as a dyadic rational (`None` for a game that is not a number).
    pub fn value(&self) -> Option<&BigRational> {
        self.value.as_ref()
    }

    /// Surreal Zero $\mathbf{0} = \{ \emptyset \mid \emptyset \}$.
    pub fn zero() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
            birthday: 0,
            approx: 0.0,
            value: Some(BigRational::zero()),
        }
    }

    /// Surreal One $\mathbf{1} = \{ 0 \mid \emptyset \}$.
    pub fn one() -> Self {
        Self::new(vec![Self::zero()], Vec::new())
    }

    /// Surreal Minus One $\mathbf{-1} = \{ \emptyset \mid 0 \}$.
    pub fn minus_one() -> Self {
        Self::new(Vec::new(), vec![Self::zero()])
    }

    /// Surreal Half $1/2 = \{ 0 \mid 1 \}$.
    pub fn half() -> Self {
        Self::new(vec![Self::zero()], vec![Self::one()])
    }

    /// Surreal Integer from $n \in \mathbb{Z}$.
    pub fn from_int(n: i64) -> Self {
        if n == 0 {
            Self::zero()
        } else if n > 0 {
            let mut cur = Self::zero();
            for _ in 0..n {
                cur = Self::new(vec![cur], Vec::new());
            }
            cur
        } else {
            let mut cur = Self::zero();
            for _ in 0..(-n) {
                cur = Self::new(Vec::new(), vec![cur]);
            }
            cur
        }
    }

    /// The finite truncation $\{ 0, 1, \dots, n - 1 \mid \emptyset \}$ of $\omega = \{ 0, 1, 2, \dots \mid \emptyset \}$,
    /// which equals the integer `depth` (a true $\omega$ needs an infinite left set).
    pub fn omega(depth: usize) -> Self {
        let mut left = Vec::with_capacity(depth);
        let mut cur = Self::zero();
        for _ in 0..depth {
            left.push(cur.clone());
            cur = Self::new(vec![cur], Vec::new());
        }
        Self::new(left, Vec::new())
    }

    /// The finite truncation $\{ 0 \mid 1, 1/2, \dots, 2^{1-n} \}$ of $\epsilon = \{ 0 \mid 1, 1/2, 1/4, \dots \}$,
    /// which equals $2^{-n}$ (a true $\epsilon$ needs an infinite right set).
    pub fn epsilon(depth: usize) -> Self {
        let mut right = Vec::with_capacity(depth);
        let mut cur = Self::one();
        for _ in 0..depth {
            right.push(cur.clone());
            cur = Self::new(vec![Self::zero()], vec![cur]);
        }
        Self::new(vec![Self::zero()], right)
    }

    /// Conway Partial Order: $x \le y \iff (\forall l \in L_x, l \not\ge y) \land (\forall r \in R_y, x \not\ge r)$.
    ///
    /// For two valid numbers this compares their exact dyadic values (an `f64` shortcut here gave wrong answers:
    /// $\{0 \mid 3\} = 1$ was compared as $1.5$); for games that are not numbers it is the recursive definition.
    pub fn leq(&self, other: &Self) -> bool {
        if let (Some(a), Some(b)) = (&self.value, &other.value) {
            return a <= b;
        }

        for l in &self.left {
            if other.leq(l) {
                return false;
            }
        }
        for r in &other.right {
            if r.leq(self) {
                return false;
            }
        }
        true
    }

    /// Surreal Equivalence $x \equiv y \iff x \le y \land y \le x$.
    pub fn equiv(&self, other: &Self) -> bool {
        self.leq(other) && other.leq(self)
    }

    /// Negation $-x = \{ -R_x \mid -L_x \}$.
    pub fn neg(&self) -> Self {
        let new_left: Vec<SurrealNo> = self.right.iter().map(|r| r.neg()).collect();
        let new_right: Vec<SurrealNo> = self.left.iter().map(|l| l.neg()).collect();
        Self::new(new_left, new_right)
    }

    /// Addition $x + y = \{ L_x + y, x + L_y \mid R_x + y, x + R_y \}$.
    pub fn add(&self, other: &Self) -> Self {
        let mut new_left = Vec::new();
        let mut new_right = Vec::new();

        for lx in &self.left {
            new_left.push(lx.add(other));
        }
        for ly in &other.left {
            new_left.push(self.add(ly));
        }

        for rx in &self.right {
            new_right.push(rx.add(other));
        }
        for ry in &other.right {
            new_right.push(self.add(ry));
        }

        Self::new(new_left, new_right)
    }

    /// Subtraction $x - y = x + (-y)$.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Multiplication $x \cdot y$.
    pub fn mul(&self, other: &Self) -> Self {
        // Base cases
        if self.equiv(&Self::zero()) || other.equiv(&Self::zero()) {
            return Self::zero();
        }
        if self.equiv(&Self::one()) {
            return other.clone();
        }
        if other.equiv(&Self::one()) {
            return self.clone();
        }

        let mut new_left = Vec::new();
        let mut new_right = Vec::new();

        // Left set: x'y + xy' - x'y' (for x' in Lx, y' in Ly and x'' in Rx, y'' in Ry)
        for lx in &self.left {
            for ly in &other.left {
                // lx * y + x * ly - lx * ly
                let term = lx.mul(other).add(&self.mul(ly)).sub(&lx.mul(ly));
                new_left.push(term);
            }
        }
        for rx in &self.right {
            for ry in &other.right {
                let term = rx.mul(other).add(&self.mul(ry)).sub(&rx.mul(ry));
                new_left.push(term);
            }
        }

        // Right set: x'y + xy'' - x'y''
        for lx in &self.left {
            for ry in &other.right {
                let term = lx.mul(other).add(&self.mul(ry)).sub(&lx.mul(ry));
                new_right.push(term);
            }
        }
        for rx in &self.right {
            for ly in &other.left {
                let term = rx.mul(other).add(&self.mul(ly)).sub(&rx.mul(ly));
                new_right.push(term);
            }
        }

        Self::new(new_left, new_right)
    }
}

impl PartialOrd for SurrealNo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let le = self.leq(other);
        let ge = other.leq(self);
        match (le, ge) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl fmt::Display for SurrealNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4} (b={})", self.approx, self.birthday)
    }
}
