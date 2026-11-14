//! Hexagonal Efficient Coordinate System (HECS)
//!
//! Salvaged from archived `hecs` crate into `algebra-engine`.
//!
//! Provides a generic `Hecs<T>` data structure representing coordinates in a hexagonal
//! lattice using `(a: bool, r: T, c: T)`:
//! - `a: bool` — alternate/staggered offset flag between rows and columns.
//! - `r: T` — primary row coordinate.
//! - `c: T` — primary column coordinate.
//!
//! Quick examples:
//!
//! ```
//! use algebra_engine::hecs::Hecs;
//!
//! let p = Hecs::new(false, 0i32, 0i32);
//! let q = Hecs::new(true, 1i32, 2i32);
//! let r = p + q;
//! assert_eq!(r, Hecs::new(true, 1, 2));
//! ```
//!
//! Neighbours and Cartesian conversions:
//!
//! ```
//! use algebra_engine::hecs::Hecs;
//!
//! let origin = Hecs::new(false, 0i32, 0i32);
//! let _r = origin.neighbour_r();
//! let _l = origin.neighbour_l();
//! let _ur = origin.neighbour_ur();
//! let _ul = origin.neighbour_ul();
//! let _dr = origin.neighbour_dr();
//! let _dl = origin.neighbour_dl();
//!
//! let h = Hecs::new(true, 1i32, 2i32);
//! let (x, y) = h.cartesian().unwrap();
//! assert!(x > 0.0 && y > 0.0);
//! ```
//!
//! Parallelism and scalar multiples:
//!
//! ```
//! use algebra_engine::hecs::Hecs;
//!
//! let a = Hecs::new(false, 0i32, 2i32);
//! let b = Hecs::new(false, 0i32, 1i32);
//! assert!(a.parallel(&b));
//! assert_eq!(a.k(&b).unwrap(), 2i32);
//! ```

use num::{
    integer::{sqrt, Roots},
    one,
    traits::Euclid,
    zero, Integer, NumCast, Signed, Zero,
};
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

const ROOT3: f64 = 1.732_050_807_568_877_2;

/// A coordinate in the Hexagonal Efficient Coordinate System (HECS).
///
/// `Hecs<T>` is a compact, integer-based representation of positions on a
/// hexagonal lattice.
///
/// # Examples
///
/// ```
/// use algebra_engine::hecs::Hecs;
///
/// let a = Hecs::new(false, 0i32, 1i32);
/// let b = Hecs::new(true, 1i32, 2i32);
/// let c = a + b;
/// assert_eq!(c, Hecs::new(true, 1, 3));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Hecs<T>
where
    T: Integer,
{
    a: bool,
    r: T,
    c: T,
}

impl<T> Hecs<T>
where
    T: Integer,
{
    /// Construct a new `Hecs` coordinate.
    ///
    /// # Examples
    ///
    /// ```
    /// use algebra_engine::hecs::Hecs;
    ///
    /// let h = Hecs::new(true, 2i32, 3i32);
    /// assert!(*h.a());
    /// assert_eq!(*h.r(), 2);
    /// assert_eq!(*h.c(), 3);
    /// ```
    pub fn new(a: bool, r: T, c: T) -> Self {
        Self { a, r, c }
    }

    /// Read-only access to the alternate (`a`) flag.
    pub fn a(&self) -> &bool {
        &self.a
    }

    /// Read-only access to the row coordinate.
    pub fn r(&self) -> &T {
        &self.r
    }

    /// Read-only access to the column coordinate.
    pub fn c(&self) -> &T {
        &self.c
    }

    /// Mutable access to the alternate flag.
    pub fn a_mut(&mut self) -> &mut bool {
        &mut self.a
    }

    /// Mutable access to the row coordinate.
    pub fn r_mut(&mut self) -> &mut T {
        &mut self.r
    }

    /// Mutable access to the column coordinate.
    pub fn c_mut(&mut self) -> &mut T {
        &mut self.c
    }

    /// Convert the `a` flag to the integer type `T` (`1` if `true`, `0` if `false`).
    pub fn a_as_integer(&self) -> T {
        if self.a {
            T::one()
        } else {
            T::zero()
        }
    }

    /// Convert the `a` flag to `f64` (`1.0` if `true`, `0.0` if `false`).
    pub fn a_as_f64(&self) -> f64 {
        if self.a {
            1.0
        } else {
            0.0
        }
    }
}

impl<T> Hecs<T>
where
    T: Integer + Clone,
{
    /// Neighbor to the right (increasing `c`).
    pub fn neighbour_r(&self) -> Self {
        Self {
            a: *self.a(),
            r: self.r().clone(),
            c: self.c().clone() + one(),
        }
    }

    /// Upper-right neighbor.
    pub fn neighbour_ur(&self) -> Self {
        Self {
            a: !self.a(),
            r: self.r().clone() + self.a_as_integer(),
            c: self.c().clone() + self.a_as_integer(),
        }
    }

    /// Upper-left neighbor.
    pub fn neighbour_ul(&self) -> Self {
        Self {
            a: !self.a(),
            r: self.r().clone() + self.a_as_integer(),
            c: self.c().clone() + self.a_as_integer() - one(),
        }
    }

    /// Neighbor to the left (decreasing `c`).
    pub fn neighbour_l(&self) -> Self {
        Self {
            a: *self.a(),
            r: self.r().clone(),
            c: self.c().clone() - one(),
        }
    }

    /// Lower-left neighbor.
    pub fn neighbour_dl(&self) -> Self {
        Self {
            a: !self.a(),
            r: self.r().clone() + self.a_as_integer() - one(),
            c: self.c().clone() + self.a_as_integer() - one(),
        }
    }

    /// Lower-right neighbor.
    pub fn neighbour_dr(&self) -> Self {
        Self {
            a: !self.a(),
            r: self.r().clone() + self.a_as_integer() - one(),
            c: self.c().clone() + self.a_as_integer(),
        }
    }
}

impl<T> Hecs<T>
where
    T: Integer + NumCast + Clone,
{
    /// X coordinate in a 2D Cartesian plane as `f64`, if conversion is possible.
    pub fn x(&self) -> Option<f64> {
        let c: f64 = NumCast::from(self.c.clone())?;
        let a2: f64 = if self.a { 0.5 } else { 0.0 };
        Some(c + a2)
    }

    /// Y coordinate in a 2D Cartesian plane as `f64`, if conversion is possible.
    pub fn y(&self) -> Option<f64> {
        let r: f64 = NumCast::from(self.r.clone())?;
        let a2: f64 = if self.a { 0.5 } else { 0.0 };
        Some(ROOT3 * (r + a2))
    }

    /// Cartesian `(x, y)` coordinates as `f64`, if conversion is possible.
    pub fn cartesian(&self) -> Option<(f64, f64)> {
        let r: f64 = NumCast::from(self.r.clone())?;
        let c: f64 = NumCast::from(self.c.clone())?;
        let a2: f64 = if self.a { 0.5 } else { 0.0 };
        Some((c + a2, ROOT3 * (r + a2)))
    }

    /// Euclidean magnitude (distance from origin) as `f64`, if conversion is possible.
    pub fn magnitude(&self) -> Option<f64> {
        let r: f64 = NumCast::from(self.r.clone())?;
        let c: f64 = NumCast::from(self.c.clone())?;
        let m: f64 = self.a_as_f64() * (3.0 * r + c + 1.0);
        let n: f64 = 3.0 * r.powi(2) + c.powi(2);
        Some(f64::sqrt(m + n))
    }

    /// Angle (argument) in radians as `f64`, if conversion is possible.
    pub fn angle(&self) -> Option<f64> {
        let r: f64 = NumCast::from(self.r.clone())?;
        let c: f64 = NumCast::from(self.c.clone())?;
        let a: f64 = self.a_as_f64();
        Some(f64::atan2(ROOT3 * (2.0 * r + a), 2.0 * c + a))
    }

    /// Polar coordinates `(r, theta)` as `f64`, if conversion is possible.
    pub fn polar(&self) -> Option<(f64, f64)> {
        let r: f64 = NumCast::from(self.r.clone())?;
        let c: f64 = NumCast::from(self.c.clone())?;
        let a: f64 = self.a_as_f64();
        let m: f64 = if self.a { 3.0 * r + c + 1.0 } else { 0.0 };
        let n: f64 = 3.0 * r.powi(2) + c.powi(2);
        Some((
            f64::sqrt(m + n),
            f64::atan2(ROOT3 * (2.0 * r + a), 2.0 * c + a),
        ))
    }
}

impl<T> Hecs<T>
where
    T: Integer + Clone + Roots,
{
    /// Magnitude computed and returned as the integer type `T` via integer `sqrt`.
    pub fn magnitude_as_integer(&self) -> T {
        let three: T = T::one() + T::one() + T::one();
        let m: T = if self.a {
            three.clone() * self.r.clone() + self.c.clone() + one()
        } else {
            zero()
        };
        let n: T = three * self.r.clone() * self.r.clone() + self.c.clone() * self.c.clone();
        sqrt(m + n)
    }
}

impl<T> Hecs<T>
where
    T: Integer + Clone + AddAssign,
{
    /// Check whether two vectors are parallel in the HECS lattice.
    pub fn parallel(&self, other: &Self) -> bool {
        if self.is_zero() || other.is_zero() || (self == other) {
            return true;
        } else if self.a || !other.a {
            let two: T = T::one() + T::one();
            let mut m: T = two.clone() * self.r.clone() * other.c.clone();
            let mut n: T = two * other.r.clone() * self.c.clone();
            if self.a {
                m += other.c.clone();
                n += other.r.clone();
            }
            if other.a {
                m += self.r.clone();
                n += self.c.clone();
            }
            return m == n;
        }
        false
    }

    /// Compute the scalar `k` such that `self = k * other`, if they are parallel.
    pub fn k(&self, other: &Self) -> Option<T> {
        if self == other {
            return Some(T::one());
        } else if self.is_zero() || other.is_zero() || !self.parallel(other) {
            return None;
        }
        let order: (&Self, &Self) = if self.a ^ other.a {
            if self.a {
                (self, other)
            } else {
                (other, self)
            }
        } else {
            let one: T = T::one();
            if (self.r.mod_floor(&one) < other.r.mod_floor(&one))
                || (self.c.mod_floor(&one) < other.c.mod_floor(&one))
            {
                (self, other)
            } else {
                (other, self)
            }
        };
        let two: T = T::one() + T::one();
        let a0: T = order.0.a_as_integer();
        let a1: T = order.1.a_as_integer();
        let k: (T, T) = if order.0.r.is_zero() {
            (
                two.clone() * order.1.c.clone() + a1,
                two * order.0.c.clone() + a0,
            )
        } else {
            (
                two.clone() * order.1.r.clone() + a1,
                two * order.0.r.clone() + a0,
            )
        };
        Some(k.0 / k.1)
    }
}

impl<T> Copy for Hecs<T> where T: Integer + Copy {}

impl<T> Display for Hecs<T>
where
    T: Integer + Display,
{
    /// Format the HECS value for human consumption.
    ///
    /// # Examples
    ///
    /// ```
    /// use algebra_engine::hecs::Hecs;
    ///
    /// let h = Hecs::new(true, 2i32, 3i32);
    /// let s = format!("{}", h);
    /// assert!(s.contains("Arr:") && s.contains("Row:") && s.contains("Col:"));
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Arr: {}\nRow: {}\nCol: {}", self.a, self.c, self.r)
    }
}

impl<T> Add<Self> for Hecs<T>
where
    T: Integer + Clone,
{
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let a: T = if self.a && other.a { one() } else { zero() };
        let r: T = self.r + other.r + a.clone();
        let c: T = self.c + other.c + a;
        Self {
            a: self.a ^ other.a,
            r,
            c,
        }
    }
}

impl<T> Add<&Self> for Hecs<T>
where
    T: Integer + Clone,
{
    type Output = Self;

    fn add(self, other: &Self) -> Self::Output {
        let a: T = if self.a && other.a { one() } else { zero() };
        let r: T = self.r + other.r.clone() + a.clone();
        let c: T = self.c + other.c.clone() + a;
        Self {
            a: self.a ^ other.a,
            r,
            c,
        }
    }
}

impl<T> AddAssign<Self> for Hecs<T>
where
    T: Integer + Clone,
{
    fn add_assign(&mut self, other: Self) {
        let a: T = if self.a && other.a { one() } else { zero() };
        self.a ^= other.a;
        self.r = self.r.clone() + other.r + a.clone();
        self.c = self.c.clone() + other.c + a;
    }
}

impl<T> AddAssign<&Self> for Hecs<T>
where
    T: Integer + Clone,
{
    fn add_assign(&mut self, other: &Self) {
        let a: T = if self.a && other.a { one() } else { zero() };
        self.a ^= other.a;
        self.r = self.r.clone() + other.r.clone() + a.clone();
        self.c = self.c.clone() + other.c.clone() + a;
    }
}

impl<T> Neg for Hecs<T>
where
    T: Integer + Clone + Signed,
{
    type Output = Self;

    /// Negation for HECS coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use algebra_engine::hecs::Hecs;
    /// use num::Zero;
    ///
    /// let h = Hecs::new(true, 1i32, 2i32);
    /// let n = -h;
    /// let m = n + h;
    /// assert!(m.is_zero());
    /// ```
    fn neg(self) -> Self::Output {
        let a: T = self.a_as_integer();
        let r: T = -(self.r + a.clone());
        let c: T = -(self.c + a);
        Self { a: self.a, r, c }
    }
}

impl<T> Sub<Self> for Hecs<T>
where
    T: Integer + Clone,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        let a: T = if (!self.a) && other.a { one() } else { zero() };
        let r: T = self.r - other.r - a.clone();
        let c: T = self.c - other.c - a;
        Self {
            a: self.a ^ other.a,
            r,
            c,
        }
    }
}

impl<T> Sub<&Self> for Hecs<T>
where
    T: Integer + Clone,
{
    type Output = Self;

    fn sub(self, other: &Self) -> Self::Output {
        let a: T = if (!self.a) && other.a { one() } else { zero() };
        let r: T = self.r - other.r.clone() - a.clone();
        let c: T = self.c - other.c.clone() - a;
        Self {
            a: self.a ^ other.a,
            r,
            c,
        }
    }
}

impl<T> SubAssign<Self> for Hecs<T>
where
    T: Integer + Clone,
{
    fn sub_assign(&mut self, other: Self) {
        let a: T = if (!self.a) && other.a { one() } else { zero() };
        self.a ^= other.a;
        self.r = self.r.clone() - other.r - a.clone();
        self.c = self.c.clone() - other.c - a;
    }
}

impl<T> SubAssign<&Self> for Hecs<T>
where
    T: Integer + Clone,
{
    fn sub_assign(&mut self, other: &Self) {
        let a: T = if (!self.a) && other.a { one() } else { zero() };
        self.a ^= other.a;
        self.r = self.r.clone() - other.r.clone() - a.clone();
        self.c = self.c.clone() - other.c.clone() - a;
    }
}

impl<T> Mul<T> for Hecs<T>
where
    T: Integer + Clone + Euclid,
{
    type Output = Self;

    fn mul(self, other: T) -> Self::Output {
        let two: T = T::one() + T::one();
        let k2: T = if self.a {
            other.div_floor(&two)
        } else {
            zero()
        };
        let a: bool = (other.rem_euclid(&two) == one()) && self.a;
        let r: T = other.clone() * self.r + k2.clone();
        let c: T = other * self.c + k2;
        Self { a, r, c }
    }
}

impl<T> Mul<&T> for Hecs<T>
where
    T: Integer + Clone + Euclid,
{
    type Output = Self;

    fn mul(self, other: &T) -> Self::Output {
        let two: T = T::one() + T::one();
        let k2: T = if self.a {
            other.div_floor(&two)
        } else {
            zero()
        };
        let a: bool = (other.rem_euclid(&two) == one()) && self.a;
        let r: T = other.clone() * self.r + k2.clone();
        let c: T = other.clone() * self.c + k2;
        Self { a, r, c }
    }
}

impl<T> MulAssign<T> for Hecs<T>
where
    T: Integer + Clone + Euclid,
{
    fn mul_assign(&mut self, other: T) {
        let two: T = T::one() + T::one();
        let k2: T = if self.a {
            other.div_floor(&two)
        } else {
            zero()
        };
        self.a = (other.rem_euclid(&two) == one()) && self.a;
        self.r = other.clone() * self.r.clone() + k2.clone();
        self.c = other * self.c.clone() + k2;
    }
}

impl<T> MulAssign<&T> for Hecs<T>
where
    T: Integer + Clone + Euclid,
{
    fn mul_assign(&mut self, other: &T) {
        let two: T = T::one() + T::one();
        let k2: T = if self.a {
            other.div_floor(&two)
        } else {
            zero()
        };
        self.a = (other.rem_euclid(&two) == one()) && self.a;
        self.r = other.clone() * self.r.clone() + k2.clone();
        self.c = other.clone() * self.c.clone() + k2;
    }
}

impl<T> PartialOrd for Hecs<T>
where
    T: Integer + Clone + PartialOrd + AddAssign + Roots,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.parallel(other) {
            T::partial_cmp(&self.magnitude_as_integer(), &other.magnitude_as_integer())
        } else {
            None
        }
    }
}

impl<T> Zero for Hecs<T>
where
    T: Integer + Clone + Zero,
{
    fn zero() -> Self {
        Self {
            a: false,
            r: T::zero(),
            c: T::zero(),
        }
    }

    fn set_zero(&mut self) {
        self.a = false;
        self.r.set_zero();
        self.c.set_zero();
    }

    fn is_zero(&self) -> bool {
        !self.a && self.r.is_zero() && self.c.is_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::Hecs;

    const EPS: f64 = 1e-12;

    #[test]
    fn test_new_and_accessors() {
        let h = Hecs::new(true, 1i32, 2i32);
        assert!(*h.a());
        assert_eq!(*h.r(), 1);
        assert_eq!(*h.c(), 2);
    }

    #[test]
    fn test_neighbors_from_origin() {
        let h = Hecs::new(false, 0i32, 0i32);

        assert_eq!(h.neighbour_r(), Hecs::new(false, 0, 1));
        assert_eq!(h.neighbour_l(), Hecs::new(false, 0, -1));
        assert_eq!(h.neighbour_ur(), Hecs::new(true, 0, 0));
        assert_eq!(h.neighbour_ul(), Hecs::new(true, 0, -1));
        assert_eq!(h.neighbour_dr(), Hecs::new(true, -1, 0));
        assert_eq!(h.neighbour_dl(), Hecs::new(true, -1, -1));
    }

    #[test]
    fn test_cartesian_and_polar() {
        let h = Hecs::new(true, 1i32, 2i32);

        let cart = h.cartesian().expect("conversion to f64 should succeed");
        let x_expected = 2.0 + 0.5;
        let y_expected = 3f64.sqrt() * (1.0 + 0.5);
        assert!(
            (cart.0 - x_expected).abs() < EPS,
            "x: got {}, want {}",
            cart.0,
            x_expected
        );
        assert!(
            (cart.1 - y_expected).abs() < EPS,
            "y: got {}, want {}",
            cart.1,
            y_expected
        );

        let polar = h.polar().expect("polar conversion");
        let mag = h.magnitude().expect("magnitude");
        let ang = h.angle().expect("angle");
        assert!((polar.0 - mag).abs() < EPS, "magnitude mismatch");
        assert!((polar.1 - ang).abs() < EPS, "angle mismatch");
    }

    #[test]
    fn test_add_zero_and_sub() {
        let zero = Hecs::new(false, 0i32, 0i32);
        let v = Hecs::new(true, 3i32, 4i32);
        assert_eq!(v + zero, v);
        assert_eq!(v - zero, v);

        let mut w = v;
        w += zero;
        assert_eq!(w, v);

        let u = Hecs::new(true, 1i32, 2i32);
        let t = Hecs::new(false, 2i32, 3i32);
        let s = u - t;
        assert_eq!(s + t, u);
    }

    #[test]
    fn test_scalar_mul() {
        let h = Hecs::new(true, 1i32, 2i32);
        let scaled = h * 2i32;
        assert_eq!(scaled, Hecs::new(false, 3, 5));

        let mut m = h;
        m *= 2i32;
        assert_eq!(m, Hecs::new(false, 3, 5));

        let ident = h * 1i32;
        assert_eq!(ident, h);
    }

    #[test]
    fn test_parallel_and_k() {
        let a = Hecs::new(false, 0i32, 2i32);
        let b = Hecs::new(false, 0i32, 1i32);
        assert!(a.parallel(&b));
        let k = a.k(&b).expect("k should exist");
        assert_eq!(k, 2i32);

        let zero = Hecs::new(false, 0i32, 0i32);
        assert!(a.parallel(&zero));
        assert!(a.k(&zero).is_none());
    }

    #[test]
    fn test_partial_ord_for_parallel() {
        let small = Hecs::new(false, 0i32, 1i32);
        let large = Hecs::new(false, 0i32, 2i32);
        assert!(small < large);
        assert!(large > small);

        let p = Hecs::new(true, 1i32, 0i32);
        let q = Hecs::new(false, 0i32, 1i32);
        assert!(p.partial_cmp(&q).is_none());
    }
}
