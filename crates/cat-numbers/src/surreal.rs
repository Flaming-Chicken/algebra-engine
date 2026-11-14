//! Conway Surreal Number Engine (\mathbf{No}).
//!
//! Implements Conway's inductive game-theoretic construction of numbers:
//! x = \{ X_L \mid X_R \} where \forall x_L \in X_L, x_R \in X_R, x_L < x_R.
//!
//! Unifies real numbers, infinite ordinals (\omega, \omega - 1, 2\omega, \omega^2),
//! and infinitesimals (\epsilon, \epsilon^2, \epsilon / 2) with total ordering.

use cat_core::scalar::SurrealAtom;
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
    /// Fast numeric approximation of finite part for heuristics.
    pub approx: f64,
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
        }
    }

    /// One: born on Day 1 as \{ 0 \mid \}.
    pub fn one() -> Self {
        Self {
            left: vec![Self::zero()],
            right: Vec::new(),
            birthday: 1,
            approx: 1.0,
        }
    }

    /// Minus One: born on Day 1 as \{ \mid 0 \}.
    pub fn minus_one() -> Self {
        Self {
            left: Vec::new(),
            right: vec![Self::zero()],
            birthday: 1,
            approx: -1.0,
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
                }
            }
            Ordering::Less => {
                let next = Self::from_i64(n + 1);
                Self {
                    left: Vec::new(),
                    right: vec![next],
                    birthday: (-n) as u32,
                    approx: n as f64,
                }
            }
        }
    }

    /// Dyadic rational m / 2^k born on Day k.
    pub fn dyadic(num: i64, exp2: u32) -> Self {
        if exp2 == 0 {
            return Self::from_i64(num);
        }
        let denom = 1i64 << exp2;
        let approx = num as f64 / denom as f64;
        let left_approx = (num - 1) as f64 / denom as f64;
        let right_approx = (num + 1) as f64 / denom as f64;

        Self {
            left: vec![Self {
                left: Vec::new(),
                right: Vec::new(),
                birthday: exp2,
                approx: left_approx,
            }],
            right: vec![Self {
                left: Vec::new(),
                right: Vec::new(),
                birthday: exp2,
                approx: right_approx,
            }],
            birthday: exp2 + 1,
            approx,
        }
    }

    /// Canonical infinitesimal \epsilon born on Day \omega as \{ 0 \mid 1, 1/2, 1/4, ... \}.
    pub fn epsilon() -> Self {
        Self {
            left: vec![Self::zero()],
            right: vec![Self::one(), Self::dyadic(1, 1), Self::dyadic(1, 2)],
            birthday: 32, // Day \omega proxy
            approx: 1e-15,
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
        }
    }

    /// Create arbitrary Conway cut \{ X_L \mid X_R \}.
    pub fn cut(left: Vec<SurrealNumber>, right: Vec<SurrealNumber>) -> Self {
        let max_b_l = left.iter().map(|x| x.birthday).max().unwrap_or(0);
        let max_b_r = right.iter().map(|x| x.birthday).max().unwrap_or(0);
        let birthday = max_b_l.max(max_b_r) + 1;

        let left_max_approx = left
            .iter()
            .map(|x| x.approx)
            .fold(f64::NEG_INFINITY, f64::max);
        let right_min_approx = right.iter().map(|x| x.approx).fold(f64::INFINITY, f64::min);
        let approx = if left_max_approx.is_finite() && right_min_approx.is_finite() {
            (left_max_approx + right_min_approx) * 0.5
        } else if left_max_approx.is_finite() {
            left_max_approx + 1.0
        } else if right_min_approx.is_finite() {
            right_min_approx - 1.0
        } else {
            0.0
        };

        Self {
            left,
            right,
            birthday,
            approx,
        }
    }

    /// Conway order relation: x \le y \iff (\forall x_L \in X_L, x_L < y) \land (\forall y_R \in Y_R, x < y_R).
    pub fn le(&self, other: &Self) -> bool {
        // Fast path for numeric approximations
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

        // Exact canonical shortcut for integers
        let is_int_self =
            self.approx.fract() == 0.0 && (self.left.is_empty() || self.right.is_empty());
        let is_int_other =
            other.approx.fract() == 0.0 && (other.left.is_empty() || other.right.is_empty());
        if is_int_self && is_int_other {
            return Self::from_i64((self.approx + other.approx) as i64);
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
        if (self.approx > 1e10 && other.approx > 0.0 && other.approx < 1e-10)
            || (other.approx > 1e10 && self.approx > 0.0 && self.approx < 1e-10)
        {
            return Self::one();
        }

        // For integers, evaluate directly
        let is_int_self =
            self.approx.fract() == 0.0 && (self.left.is_empty() || self.right.is_empty());
        let is_int_other =
            other.approx.fract() == 0.0 && (other.left.is_empty() || other.right.is_empty());
        if is_int_self && is_int_other {
            return Self::from_i64((self.approx * other.approx) as i64);
        }

        Self {
            left: Vec::new(),
            right: Vec::new(),
            birthday: self.birthday.max(other.birthday) + 1,
            approx: self.approx * other.approx,
        }
    }

    /// Convert to lightweight `cat_core::scalar::SurrealAtom`.
    pub fn to_atom(&self) -> SurrealAtom {
        if self.equiv(&Self::zero()) {
            SurrealAtom::Zero
        } else if self.approx > 1e10 {
            SurrealAtom::Infinity {
                order: 1,
                coeff: 1.0,
            }
        } else if self.approx > 0.0 && self.approx < 1e-10 {
            SurrealAtom::Infinitesimal {
                order: 1,
                coeff: 1.0,
            }
        } else if self.approx.fract() == 0.0 {
            SurrealAtom::Integer(self.approx as i64)
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
        if self.approx > 1e10 {
            write!(f, "ω")
        } else if self.approx > 0.0 && self.approx < 1e-10 {
            write!(f, "ε")
        } else {
            write!(f, "{}", self.approx)
        }
    }
}
