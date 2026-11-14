//! 8-Dimensional rational physical dimension vector and gauge systems.
//!
//! Represents physical dimensions in the 8-basis grading:
//! [Length (L), Mass (M), Time (T), Current (I), Temperature (Theta),
//!  Amount (N), LuminousIntensity (J), PlaneAngle (Angle)].
//!
//! Powers are encoded as exact signed 16-bit integers in `scaled_twelfths`
//! ($12 \times \text{power}$). This allows exact representation of fractional powers:
//! - $1 \to 12$
//! - $1/2 \to 6$
//! - $1/3 \to 4$
//! - $1/4 \to 3$
//! - $2/3 \to 8$

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// Index constants for the 8 fundamental physical dimensions.
pub const DIM_L: usize = 0;
pub const DIM_M: usize = 1;
pub const DIM_T: usize = 2;
pub const DIM_I: usize = 3;
pub const DIM_THETA: usize = 4;
pub const DIM_N: usize = 5;
pub const DIM_J: usize = 6;
pub const DIM_ANGLE: usize = 7;

/// 8-Dimensional physical dimension vector in `scaled_twelfths`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct DimVector {
    /// Powers multiplied by 12. Index mapping:
    /// [L, M, T, I, Theta, N, J, Angle]
    pub scaled_twelfths: [i16; 8],
}

impl DimVector {
    /// Dimensionless scalar (all powers 0).
    pub const DIMENSIONLESS: Self = Self {
        scaled_twelfths: [0; 8],
    };

    /// Create from raw twelfths array.
    pub const fn from_twelfths(scaled_twelfths: [i16; 8]) -> Self {
        Self { scaled_twelfths }
    }

    /// Create from integer powers: [L, M, T, I, Theta, N, J, Angle].
    pub const fn from_integers(powers: [i8; 8]) -> Self {
        Self {
            scaled_twelfths: [
                powers[0] as i16 * 12,
                powers[1] as i16 * 12,
                powers[2] as i16 * 12,
                powers[3] as i16 * 12,
                powers[4] as i16 * 12,
                powers[5] as i16 * 12,
                powers[6] as i16 * 12,
                powers[7] as i16 * 12,
            ],
        }
    }

    /// Check if this vector is completely dimensionless.
    #[inline]
    pub fn is_dimensionless(&self) -> bool {
        self.scaled_twelfths == [0; 8]
    }

    // --- Standard Physical Units ---

    /// Length `[L]`
    pub const fn length() -> Self {
        let mut v = [0; 8];
        v[DIM_L] = 12;
        Self { scaled_twelfths: v }
    }

    /// Mass `[M]`
    pub const fn mass() -> Self {
        let mut v = [0; 8];
        v[DIM_M] = 12;
        Self { scaled_twelfths: v }
    }

    /// Time `[T]`
    pub const fn time() -> Self {
        let mut v = [0; 8];
        v[DIM_T] = 12;
        Self { scaled_twelfths: v }
    }

    /// Electric Current `[I]`
    pub const fn current() -> Self {
        let mut v = [0; 8];
        v[DIM_I] = 12;
        Self { scaled_twelfths: v }
    }

    /// Thermodynamic Temperature `[Theta]`
    pub const fn temperature() -> Self {
        let mut v = [0; 8];
        v[DIM_THETA] = 12;
        Self { scaled_twelfths: v }
    }

    /// Amount of Substance `[N]`
    pub const fn amount() -> Self {
        let mut v = [0; 8];
        v[DIM_N] = 12;
        Self { scaled_twelfths: v }
    }

    /// Luminous Intensity `[J]`
    pub const fn luminous_intensity() -> Self {
        let mut v = [0; 8];
        v[DIM_J] = 12;
        Self { scaled_twelfths: v }
    }

    /// Plane Angle `[Angle]` (dimensionless in SI, but tracked in categorical operad)
    pub const fn angle() -> Self {
        let mut v = [0; 8];
        v[DIM_ANGLE] = 12;
        Self { scaled_twelfths: v }
    }

    /// Velocity [L T^-1]
    pub const fn velocity() -> Self {
        let mut v = [0; 8];
        v[DIM_L] = 12;
        v[DIM_T] = -12;
        Self { scaled_twelfths: v }
    }

    /// Acceleration [L T^-2]
    pub const fn acceleration() -> Self {
        let mut v = [0; 8];
        v[DIM_L] = 12;
        v[DIM_T] = -24;
        Self { scaled_twelfths: v }
    }

    /// Force [M L T^-2]
    pub const fn force() -> Self {
        let mut v = [0; 8];
        v[DIM_M] = 12;
        v[DIM_L] = 12;
        v[DIM_T] = -24;
        Self { scaled_twelfths: v }
    }

    /// Energy / Work / Torque [M L^2 T^-2]
    pub const fn energy() -> Self {
        let mut v = [0; 8];
        v[DIM_M] = 12;
        v[DIM_L] = 24;
        v[DIM_T] = -24;
        Self { scaled_twelfths: v }
    }

    /// Momentum [M L T^-1]
    pub const fn momentum() -> Self {
        let mut v = [0; 8];
        v[DIM_M] = 12;
        v[DIM_L] = 12;
        v[DIM_T] = -12;
        Self { scaled_twelfths: v }
    }

    /// Action / Planck's constant \hbar [M L^2 T^-1]
    pub const fn action() -> Self {
        let mut v = [0; 8];
        v[DIM_M] = 12;
        v[DIM_L] = 24;
        v[DIM_T] = -12;
        Self { scaled_twelfths: v }
    }

    /// Speed of light c [L T^-1]
    pub const fn speed_of_light() -> Self {
        Self::velocity()
    }

    /// Gravitational Constant G [M^-1 L^3 T^-2]
    pub const fn gravitational_constant() -> Self {
        let mut v = [0; 8];
        v[DIM_M] = -12;
        v[DIM_L] = 36;
        v[DIM_T] = -24;
        Self { scaled_twelfths: v }
    }

    /// Elementary Charge e [I T]
    pub const fn charge() -> Self {
        let mut v = [0; 8];
        v[DIM_I] = 12;
        v[DIM_T] = 12;
        Self { scaled_twelfths: v }
    }

    /// se(3) twist block dimensions for Lie algebra \mathfrak{se}(3):
    /// Linear velocity [L T^-1] and angular velocity [Angle T^-1]
    pub const fn se3_linear() -> Self {
        Self::velocity()
    }

    pub const fn se3_angular() -> Self {
        let mut v = [0; 8];
        v[DIM_ANGLE] = 12;
        v[DIM_T] = -12;
        Self { scaled_twelfths: v }
    }

    /// Power by a rational fraction: (num / den).
    pub fn pow_rational(&self, num: i32, den: i32) -> Result<Self, &'static str> {
        if den == 0 {
            return Err("Zero denominator in rational dimension power");
        }
        let mut out = [0i16; 8];
        for i in 0..8 {
            let p = (self.scaled_twelfths[i] as i64 * num as i64) / den as i64;
            if p < i16::MIN as i64 || p > i16::MAX as i64 {
                return Err("Dimension exponent overflow");
            }
            out[i] = p as i16;
        }
        Ok(Self { scaled_twelfths: out })
    }

    /// Square root of dimension vector.
    pub fn sqrt(&self) -> Result<Self, &'static str> {
        self.pow_rational(1, 2)
    }
}

impl Add for DimVector {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let mut out = [0; 8];
        for i in 0..8 {
            out[i] = self.scaled_twelfths[i].saturating_add(rhs.scaled_twelfths[i]);
        }
        Self { scaled_twelfths: out }
    }
}

impl Sub for DimVector {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let mut out = [0; 8];
        for i in 0..8 {
            out[i] = self.scaled_twelfths[i].saturating_sub(rhs.scaled_twelfths[i]);
        }
        Self { scaled_twelfths: out }
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for DimVector {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        self + rhs
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for DimVector {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        self - rhs
    }
}

impl fmt::Debug for DimVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DimVector({})", self)
    }
}

impl fmt::Display for DimVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return write!(f, "1");
        }
        let names = ["L", "M", "T", "I", "Θ", "N", "J", "rad"];
        let mut first = true;
        for i in 0..8 {
            let tw = self.scaled_twelfths[i];
            if tw != 0 {
                if !first {
                    write!(f, "·")?;
                }
                first = false;
                if tw == 12 {
                    write!(f, "{}", names[i])?;
                } else if tw % 12 == 0 {
                    write!(f, "{}^{}", names[i], tw / 12)?;
                } else {
                    write!(f, "{}^({}/12)", names[i], tw)?;
                }
            }
        }
        Ok(())
    }
}

/// Unit gauge system defining which fundamental constants are set to 1.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum UnitGaugeSystem {
    /// International System of Units: 8 distinct dimensions.
    #[default]
    SI,
    /// High-energy natural units: \hbar = c = 1.
    /// Energy, mass, and momentum have dimension `[E]`, length and time have dimension `[E^-1]`.
    Natural,
    /// Planck units: \hbar = c = G = k_B = 1.
    Planck,
    /// Atomic units: e = \hbar = m_e = 4\pi\epsilon_0 = 1.
    Atomic,
    /// Relativistic / Geometrized units: G = c = 1.
    Geometrized,
}

impl UnitGaugeSystem {
    /// Reduce a dimension vector according to the gauge constraints.
    pub fn reduce(&self, dim: DimVector) -> DimVector {
        match self {
            Self::SI => dim,
            Self::Natural => {
                // c = 1 => [L] = [T]
                // \hbar = 1 => [M] [L]^2 [T]^-1 = 1 => [L] = [M]^-1, [T] = [M]^-1
                // Therefore, any mechanical dimension [M]^m [L]^l [T]^t becomes [M]^(m - l - t).
                let l = dim.scaled_twelfths[DIM_L] as i32;
                let m = dim.scaled_twelfths[DIM_M] as i32;
                let t = dim.scaled_twelfths[DIM_T] as i32;
                let net_energy_twelfths = m - l - t;
                let mut out = [0i16; 8];
                out[DIM_M] = net_energy_twelfths as i16;
                // Retain other non-mechanical dimensions
                out[DIM_I] = dim.scaled_twelfths[DIM_I];
                out[DIM_THETA] = dim.scaled_twelfths[DIM_THETA];
                out[DIM_N] = dim.scaled_twelfths[DIM_N];
                out[DIM_J] = dim.scaled_twelfths[DIM_J];
                out[DIM_ANGLE] = dim.scaled_twelfths[DIM_ANGLE];
                DimVector { scaled_twelfths: out }
            }
            Self::Planck => {
                // All mechanical dimensions become dimensionless (1)
                let mut out = [0i16; 8];
                out[DIM_N] = dim.scaled_twelfths[DIM_N];
                out[DIM_ANGLE] = dim.scaled_twelfths[DIM_ANGLE];
                DimVector { scaled_twelfths: out }
            }
            Self::Geometrized => {
                // G = c = 1 => [M] = [L] = [T]
                let l = dim.scaled_twelfths[DIM_L] as i32;
                let m = dim.scaled_twelfths[DIM_M] as i32;
                let t = dim.scaled_twelfths[DIM_T] as i32;
                let net_len = l + m + t;
                let mut out = [0i16; 8];
                out[DIM_L] = net_len as i16;
                out[DIM_I] = dim.scaled_twelfths[DIM_I];
                out[DIM_THETA] = dim.scaled_twelfths[DIM_THETA];
                out[DIM_N] = dim.scaled_twelfths[DIM_N];
                out[DIM_J] = dim.scaled_twelfths[DIM_J];
                out[DIM_ANGLE] = dim.scaled_twelfths[DIM_ANGLE];
                DimVector { scaled_twelfths: out }
            }
            Self::Atomic => {
                // e = \hbar = m_e = 1
                let mut out = dim.scaled_twelfths;
                out[DIM_M] = 0;
                out[DIM_I] = 0;
                DimVector { scaled_twelfths: out }
            }
        }
    }
}

/// Unit space geometric character: affine (origin-dependent) vs linear (vector translation).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum UnitSpaceKind {
    /// Standard vector space: values can be added and scaled freely (e.g. Kelvin, Meters, Joules).
    #[default]
    LinearVectorSpace,
    /// Affine space: has an arbitrary zero reference (e.g. Celsius, Fahrenheit, calendar timestamp, potential gauge).
    /// Adding two affine points is forbidden; subtracting two affine points produces a linear vector!
    AffineSpace,
    /// Projective space: rays up to scale (e.g. homogeneous projective coordinates).
    ProjectiveSpace,
}
