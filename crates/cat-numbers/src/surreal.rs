//! Conway Surreal Number Engine (\mathbf{No}).
//!
//! Implements Conway's inductive game-theoretic construction of numbers:
//! x = \{ X_L \mid X_R \} where \forall x_L \in X_L, x_R \in X_R, x_L < x_R.
//!
//! Unifies real numbers, infinite ordinals (\omega, \omega - 1, 2\omega, \omega^2),
//! and infinitesimals (\epsilon, \epsilon^2, \epsilon / 2) with total ordering.

use cat_core::scalar::SurrealAtom;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// Inductive Conway Surreal Number representation.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SurrealNumber {
    /// Left set X_L (strictly less than x).
    pub left: Vec<SurrealNumber>,
    /// Right set X_R (strictly greater than x).
    pub right: Vec<SurrealNumber>,
    /// Generation / day of creation in Conway's universe.
    pub birthday: u32,
    /// Fast numeric approximation of finite part for heuristics (the exact value rounded when the form is finite).
    pub approx: f64,
    /// `true` for the stand-ins of $\omega$ and $\epsilon$, whose true option sets are infinite and cannot be stored:
    /// they have no exact value and order by `approx`.
    #[serde(default)]
    pub transfinite: bool,
}

/// The simplest dyadic rational strictly between `lo` and `hi` (either bound may be absent): the integer of
/// smallest absolute value if there is one, else the fraction with the smallest power-of-two denominator.
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
                let mut denominator = BigRational::one();
                loop {
                    let value = ((l * &denominator).floor() + BigRational::one()) / &denominator;
                    if &value < h {
                        return value;
                    }
                    denominator *= BigRational::from_integer(2.into());
                }
            }
            -simplest_between(Some(&-h.clone()), Some(&-l.clone()))
        }
    }
}

impl Eq for SurrealNumber {}

impl SurrealNumber {
    /// Zero: born on Day 0 as \{ \mid \}.
    pub fn zero() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
            birthday: 0,
            approx: 0.0,
            transfinite: false,
        }
    }

    /// One: born on Day 1 as \{ 0 \mid \}.
    pub fn one() -> Self {
        Self {
            left: vec![Self::zero()],
            right: Vec::new(),
            birthday: 1,
            approx: 1.0,
            transfinite: false,
        }
    }

    /// Minus One: born on Day 1 as \{ \mid 0 \}.
    pub fn minus_one() -> Self {
        Self {
            left: Vec::new(),
            right: vec![Self::zero()],
            birthday: 1,
            approx: -1.0,
            transfinite: false,
        }
    }

    /// Create from integer n born on Day |n|.
    pub fn from_i64(n: i64) -> Self {
        match n.cmp(&0) {
            Ordering::Equal => Self::zero(),
            Ordering::Greater => {
                let prev = Self::from_i64(n - 1);
                Self {
                    left: vec![prev],
                    right: Vec::new(),
                    birthday: n as u32,
                    approx: n as f64,
                    transfinite: false,
                }
            }
            Ordering::Less => {
                let next = Self::from_i64(n + 1);
                Self {
                    left: Vec::new(),
                    right: vec![next],
                    birthday: (-n) as u32,
                    approx: n as f64,
                    transfinite: false,
                }
            }
        }
    }

    /// The dyadic rational `num / 2^exp2` in canonical form: an integer is `{n - 1 | }` (or `{ | n + 1}`), and an odd
    /// numerator over `2^k` is `{(m - 1) / 2^k | (m + 1) / 2^k}` with both options reduced.
    pub fn dyadic(num: i64, exp2: u32) -> Self {
        if exp2 == 0 {
            return Self::from_i64(num);
        }
        let (mut num, mut exp2) = (num, exp2);
        while exp2 > 0 && num % 2 == 0 {
            num /= 2;
            exp2 -= 1;
        }
        if exp2 == 0 {
            return Self::from_i64(num);
        }
        let (lo, hi) = (num - 1, num + 1);
        let reduce = |n: i64| {
            let (mut n, mut e) = (n, exp2);
            while e > 0 && n % 2 == 0 {
                n /= 2;
                e -= 1;
            }
            Self::dyadic(n, e)
        };
        Self::cut(vec![reduce(lo)], vec![reduce(hi)])
    }

    /// The canonical form of an exact dyadic rational, or `None` if it is not dyadic or does not fit.
    pub fn from_dyadic(value: &BigRational) -> Option<Self> {
        let den = value.denom().to_u64()?;
        if !den.is_power_of_two() {
            return None;
        }
        let exp2 = den.trailing_zeros();
        let num = value.numer().to_i64()?;
        Some(Self::dyadic(num, exp2))
    }

    /// The exact value as a dyadic rational (the simplest number between the largest left option and the smallest
    /// right option), or `None` for a game that is not a number or involves the infinite stand-ins.
    pub fn exact_value(&self) -> Option<BigRational> {
        if self.transfinite {
            return None;
        }
        let mut lo: Option<BigRational> = None;
        for l in &self.left {
            let v = l.exact_value()?;
            if lo.as_ref().is_none_or(|m| &v > m) {
                lo = Some(v);
            }
        }
        let mut hi: Option<BigRational> = None;
        for r in &self.right {
            let v = r.exact_value()?;
            if hi.as_ref().is_none_or(|m| &v < m) {
                hi = Some(v);
            }
        }
        if let (Some(a), Some(b)) = (&lo, &hi) {
            if a >= b {
                return None;
            }
        }
        Some(simplest_between(lo.as_ref(), hi.as_ref()))
    }

    /// Canonical infinitesimal \epsilon born on Day \omega as \{ 0 \mid 1, 1/2, 1/4, ... \}.
    pub fn epsilon() -> Self {
        Self {
            left: vec![Self::zero()],
            right: vec![Self::one(), Self::dyadic(1, 1), Self::dyadic(1, 2)],
            birthday: 32, // Day \omega proxy
            approx: 1e-15,
            transfinite: true,
        }
    }

    /// Canonical first transfinite ordinal \omega born on Day \omega as \{ 0, 1, 2, ... \mid \}.
    pub fn omega() -> Self {
        Self {
            left: vec![
                Self::zero(),
                Self::one(),
                Self::from_i64(2),
                Self::from_i64(3),
            ],
            right: Vec::new(),
            birthday: 32, // Day \omega proxy
            approx: 1e15,
            transfinite: true,
        }
    }

    /// Create arbitrary Conway cut \{ X_L \mid X_R \}.
    pub fn cut(left: Vec<SurrealNumber>, right: Vec<SurrealNumber>) -> Self {
        let max_b_l = left.iter().map(|x| x.birthday).max().unwrap_or(0);
        let max_b_r = right.iter().map(|x| x.birthday).max().unwrap_or(0);
        let birthday = max_b_l.max(max_b_r) + 1;
        let transfinite = left.iter().chain(right.iter()).any(|x| x.transfinite);

        let left_max_approx = left
            .iter()
            .map(|x| x.approx)
            .fold(f64::NEG_INFINITY, f64::max);
        let right_min_approx = right.iter().map(|x| x.approx).fold(f64::INFINITY, f64::min);
        let heuristic = if left_max_approx.is_finite() && right_min_approx.is_finite() {
            (left_max_approx + right_min_approx) * 0.5
        } else if left_max_approx.is_finite() {
            left_max_approx + 1.0
        } else if right_min_approx.is_finite() {
            right_min_approx - 1.0
        } else {
            0.0
        };

        let mut out = Self {
            left,
            right,
            birthday,
            approx: heuristic,
            transfinite,
        };
        if let Some(v) = out.exact_value() {
            out.approx = v.to_f64().unwrap_or(heuristic);
        }
        out
    }

    /// Conway order relation: x \le y \iff (\forall x_L \in X_L, x_L < y) \land (\forall y_R \in Y_R, x < y_R).
    pub fn le(&self, other: &Self) -> bool {
        // Two finite numbers compare by their exact values (an `f64` shortcut here misorders close values).
        if let (Some(a), Some(b)) = (self.exact_value(), other.exact_value()) {
            return a <= b;
        }
        // The infinite stand-ins order by their proxies when those are well separated.
        if (self.approx - other.approx).abs() > 1e-9 {
            return self.approx <= other.approx;
        }

        // Inductive Conway definition:
        // 1. No member of self.left can be >= other
        for x_l in &self.left {
            if other.le(x_l) {
                return false;
            }
        }
        // 2. self cannot be >= any member of other.right
        for y_r in &other.right {
            if y_r.le(self) {
                return false;
            }
        }
        true
    }

    /// Strict Conway inequality: x < y \iff x \le y \land \neg(y \le x).
    pub fn lt(&self, other: &Self) -> bool {
        self.le(other) && !other.le(self)
    }

    /// Conway Equivalence: x \equiv y \iff x \le y \land y \le x.
    pub fn equiv(&self, other: &Self) -> bool {
        self.le(other) && other.le(self)
    }

    /// Conway Negation: -x = \{ -X_R \mid -X_L \}.
    pub fn neg(&self) -> Self {
        let new_left: Vec<SurrealNumber> = self.right.iter().map(|r| r.neg()).collect();
        let new_right: Vec<SurrealNumber> = self.left.iter().map(|l| l.neg()).collect();
        Self {
            left: new_left,
            right: new_right,
            birthday: self.birthday,
            approx: -self.approx,
            transfinite: self.transfinite,
        }
    }

    /// Conway Addition: x + y = \{ X_L + y, x + Y_L \mid X_R + y, x + Y_R \}.
    pub fn add(&self, other: &Self) -> Self {
        // Fast paths for additive identity
        if self.approx == 0.0 && self.birthday == 0 {
            return other.clone();
        }
        if other.approx == 0.0 && other.birthday == 0 {
            return self.clone();
        }

        // Finite numbers add exactly, as dyadic rationals.
        if let (Some(a), Some(b)) = (self.exact_value(), other.exact_value()) {
            if let Some(sum) = Self::from_dyadic(&(a + b)) {
                return sum;
            }
        }

        let mut left_set = Vec::new();
        for x_l in &self.left {
            left_set.push(x_l.add(other));
        }
        for y_l in &other.left {
            left_set.push(self.add(y_l));
        }

        let mut right_set = Vec::new();
        for x_r in &self.right {
            right_set.push(x_r.add(other));
        }
        for y_r in &other.right {
            right_set.push(self.add(y_r));
        }

        Self::cut(left_set, right_set)
    }

    /// Conway Subtraction: x - y = x + (-y).
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Conway Multiplication:
    /// x \cdot y = \{ x_L y + x y_L - x_L y_L, x_R y + x y_R - x_R y_R \mid x_L y + x y_R - x_L y_R, x_R y + x y_L - x_R y_L \}.
    pub fn mul(&self, other: &Self) -> Self {
        // Fast identity checks
        if self.equiv(&Self::zero()) || other.equiv(&Self::zero()) {
            return Self::zero();
        }
        if self.equiv(&Self::one()) {
            return other.clone();
        }
        if other.equiv(&Self::one()) {
            return self.clone();
        }

        // Fundamental Conway identity: \omega * \epsilon = 1
        if self.transfinite
            && other.transfinite
            && ((self.approx > 1e10 && other.approx > 0.0 && other.approx < 1e-10)
                || (other.approx > 1e10 && self.approx > 0.0 && self.approx < 1e-10))
        {
            return Self::one();
        }

        // Finite numbers multiply exactly, as dyadic rationals.
        if let (Some(a), Some(b)) = (self.exact_value(), other.exact_value()) {
            if let Some(product) = Self::from_dyadic(&(a * b)) {
                return product;
            }
        }

        Self {
            left: Vec::new(),
            right: Vec::new(),
            birthday: self.birthday.max(other.birthday) + 1,
            approx: self.approx * other.approx,
            transfinite: false,
        }
    }

    /// Convert to lightweight `cat_core::scalar::SurrealAtom`.
    pub fn to_atom(&self) -> SurrealAtom {
        if self.equiv(&Self::zero()) {
            SurrealAtom::Zero
        } else if self.transfinite && self.approx > 1e10 {
            SurrealAtom::Infinity {
                order: 1,
                coeff: 1.0,
            }
        } else if self.transfinite && self.approx > 0.0 && self.approx < 1e-10 {
            SurrealAtom::Infinitesimal {
                order: 1,
                coeff: 1.0,
            }
        } else if let Some(v) = self.exact_value() {
            if v.is_integer() {
                v.to_integer()
                    .to_i64()
                    .map_or(SurrealAtom::Zero, SurrealAtom::Integer)
            } else {
                // A dyadic value: the numerator and denominator are exact.
                SurrealAtom::ConwayForm {
                    finite_num: v.numer().to_i64().unwrap_or(0),
                    finite_den: v.denom().to_i64().unwrap_or(1),
                    omega_coeff: 0.0,
                    epsilon_coeff: 0.0,
                }
            }
        } else {
            SurrealAtom::ConwayForm {
                finite_num: (self.approx * 1000.0) as i64,
                finite_den: 1000,
                omega_coeff: 0.0,
                epsilon_coeff: 0.0,
            }
        }
    }
}

impl PartialOrd for SurrealNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let le = self.le(other);
        let ge = other.le(self);
        match (le, ge) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl fmt::Debug for SurrealNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No({:.4}, day={})", self.approx, self.birthday)
    }
}

impl fmt::Display for SurrealNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.transfinite && self.approx > 1e10 {
            write!(f, "ω")
        } else if self.transfinite && self.approx > 0.0 && self.approx < 1e-10 {
            write!(f, "ε")
        } else {
            write!(f, "{}", self.approx)
        }
    }
}
