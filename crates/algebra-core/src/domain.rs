//! Mathematical domains and structure specifications for typed algebraic expressions.

use std::fmt;

/// Category of algebraic structure governing operation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DomainCategory {
    Set,
    Group,
    AbelianGroup,
    Ring,
    /// Addition and multiplication with no negatives (the natural numbers).
    CommutativeSemiring,
    CommutativeRing,
    Field,
    AlgebraicClosedField,
    VectorSpace,
    Module,
    TensorAlgebra,
    BooleanAlgebra,
}

/// Representation of mathematical domains for typed expressions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Domain {
    /// Natural numbers N (0, 1, 2, ...)
    Naturals,

    /// Integers Z (..., -1, 0, 1, ...)
    Integers,

    /// Rational numbers Q
    Rationals,

    /// Real numbers R
    Reals,

    /// Complex numbers C
    Complex,

    /// Surreal numbers NO (John Conway's hyper-real extension)
    Surreals,

    /// p-adic field Q_p for a given prime p
    PAdics { prime: u32 },

    /// Ring of Adeles A (topological ring combining R with all p-adic completions)
    Adeles,

    /// Modulo ring Z / nZ
    Modulo { modulus: u64 },

    /// Finite Galois field GF(p^n)
    GaloisField { prime: u64, power: u32 },

    /// Hypercomplex numbers (e.g. Quaternions H, Octonions O, Clifford algebras)
    Hypercomplex { dimension: u32 },

    /// Matrix domain M_{m x n}(D)
    Matrix {
        rows: usize,
        cols: usize,
        element_domain: Box<Domain>,
    },

    /// Tensor domain over a base domain with tensor rank
    Tensor { rank: u32, base_domain: Box<Domain> },

    /// ZF Set theory domain
    Set,

    /// Boolean logic domain {True, False}
    Boolean,

    /// Free / Generic symbolic domain
    Generic,
}

impl DomainCategory {
    /// Check if this category represents a mathematical field.
    #[inline]
    pub const fn is_field(&self) -> bool {
        matches!(self, Self::Field | Self::AlgebraicClosedField)
    }

    /// Check if this category represents an algebraic ring.
    #[inline]
    pub const fn is_ring(&self) -> bool {
        matches!(
            self,
            Self::Ring | Self::CommutativeRing | Self::Field | Self::AlgebraicClosedField
        )
    }

    /// Check if this category represents an abelian group under addition.
    #[inline]
    pub const fn is_abelian_group(&self) -> bool {
        matches!(
            self,
            Self::AbelianGroup
                | Self::Ring
                | Self::CommutativeRing
                | Self::Field
                | Self::AlgebraicClosedField
                | Self::VectorSpace
                | Self::Module
        )
    }
}

/// Trial-division primality test, usable in `const fn` (moduli are small in practice).
const fn is_prime_u64(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true;
    }
    if n.is_multiple_of(2) {
        return false;
    }
    let mut d = 3;
    while d <= n / d {
        if n.is_multiple_of(d) {
            return false;
        }
        d += 2;
    }
    true
}

impl Domain {
    /// Returns the core algebraic category of this domain.
    #[inline]
    pub const fn category(&self) -> DomainCategory {
        match self {
            Self::Boolean => DomainCategory::BooleanAlgebra,
            // N has no additive inverses: a commutative semiring, not a ring.
            Self::Naturals => DomainCategory::CommutativeSemiring,
            // Z/nZ is a field exactly when n is prime; otherwise it has zero divisors.
            Self::Modulo { modulus } => {
                if is_prime_u64(*modulus) {
                    DomainCategory::Field
                } else {
                    DomainCategory::CommutativeRing
                }
            }
            Self::Integers => DomainCategory::CommutativeRing,
            // The adele ring is a product of completions with zero divisors, not a field.
            Self::Adeles => DomainCategory::CommutativeRing,
            Self::Complex => DomainCategory::AlgebraicClosedField,
            Self::Rationals
            | Self::Reals
            | Self::Surreals
            | Self::PAdics { .. }
            | Self::GaloisField { .. } => DomainCategory::Field,
            Self::Matrix { .. } => DomainCategory::Ring,
            Self::Tensor { .. } => DomainCategory::TensorAlgebra,
            Self::Hypercomplex { dimension } => {
                if *dimension <= 2 {
                    DomainCategory::Field
                } else {
                    DomainCategory::Ring
                }
            }
            Self::Set | Self::Generic => DomainCategory::Set,
        }
    }

    /// Check if multiplication is commutative in this domain.
    #[inline]
    pub const fn is_commutative_mul(&self) -> bool {
        match self {
            Self::Matrix { rows, cols, .. } => *rows == 1 && *cols == 1,
            Self::Hypercomplex { dimension } => *dimension <= 2,
            Self::Tensor { .. } => false,
            _ => true,
        }
    }

    /// Check if domain represents a scalar number system.
    #[inline]
    pub const fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Naturals
                | Self::Integers
                | Self::Rationals
                | Self::Reals
                | Self::Complex
                | Self::Surreals
                | Self::PAdics { .. }
                | Self::GaloisField { .. }
        )
    }

    /// Returns `true` if `self` is a subset/subdomain of `other`.
    pub fn is_subdomain_of(&self, other: &Domain) -> bool {
        if self == other {
            return true;
        }

        matches!(
            (self, other),
            (Self::Naturals, Self::Integers)
                | (Self::Naturals, Self::Rationals)
                | (Self::Naturals, Self::Reals)
                | (Self::Naturals, Self::Complex)
                | (Self::Integers, Self::Rationals)
                | (Self::Integers, Self::Reals)
                | (Self::Integers, Self::Complex)
                | (Self::Rationals, Self::Reals)
                | (Self::Rationals, Self::Complex)
                | (Self::Reals, Self::Complex)
                | (Self::Reals, Self::Surreals)
        )
    }

    /// Promote two mathematical domains to their common algebraic super-domain or action domain.
    pub fn promote(d1: &Domain, d2: &Domain) -> Domain {
        if d1 == d2 {
            return d1.clone();
        }
        if d1.is_subdomain_of(d2) {
            return d2.clone();
        }
        if d2.is_subdomain_of(d1) {
            return d1.clone();
        }

        match (d1, d2) {
            // Complex (2-valued) or Real scalar + Matrix (m x n, 4-valued) -> Matrix
            (
                Self::Complex | Self::Reals | Self::Rationals | Self::Integers,
                Self::Matrix {
                    rows,
                    cols,
                    element_domain,
                },
            ) => Self::Matrix {
                rows: *rows,
                cols: *cols,
                element_domain: Box::new(Self::promote(d1, element_domain)),
            },
            (
                Self::Matrix {
                    rows,
                    cols,
                    element_domain,
                },
                Self::Complex | Self::Reals | Self::Rationals | Self::Integers,
            ) => Self::Matrix {
                rows: *rows,
                cols: *cols,
                element_domain: Box::new(Self::promote(element_domain, d2)),
            },
            // Complex or Real scalar + Hypercomplex (Quaternions H dim=4 or Dual Quaternions) -> Hypercomplex(4)
            (
                Self::Complex | Self::Reals | Self::Rationals | Self::Integers,
                Self::Hypercomplex { dimension },
            ) => Self::Hypercomplex {
                dimension: *dimension,
            },
            (
                Self::Hypercomplex { dimension },
                Self::Complex | Self::Reals | Self::Rationals | Self::Integers,
            ) => Self::Hypercomplex {
                dimension: *dimension,
            },
            // Modulo / GaloisField + Integer/Natural scalar
            (Self::Modulo { modulus }, Self::Integers | Self::Naturals) => {
                Self::Modulo { modulus: *modulus }
            }
            (Self::Integers | Self::Naturals, Self::Modulo { modulus }) => {
                Self::Modulo { modulus: *modulus }
            }
            (Self::GaloisField { prime, power }, Self::Integers | Self::Naturals) => {
                Self::GaloisField {
                    prime: *prime,
                    power: *power,
                }
            }
            (Self::Integers | Self::Naturals, Self::GaloisField { prime, power }) => {
                Self::GaloisField {
                    prime: *prime,
                    power: *power,
                }
            }
            // Tensor + Scalar
            (s, Self::Tensor { rank, base_domain }) | (Self::Tensor { rank, base_domain }, s)
                if s.is_scalar() =>
            {
                Self::Tensor {
                    rank: *rank,
                    base_domain: Box::new(Self::promote(s, base_domain)),
                }
            }
            _ => Self::Generic,
        }
    }
}

/// Interval and category domain constraint bound on a named symbol.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DomainBound {
    pub name: String,
    pub domain_type: String, // "Reals", "Integers", "Positive", "NonNegative", "Complex", etc.
    pub min_val: Option<f64>,
    pub max_val: Option<f64>,
    pub inclusive_min: bool,
    pub inclusive_max: bool,
}

impl DomainBound {
    /// Check whether this domain represents a discrete mathematical number system.
    pub fn is_discrete(&self) -> bool {
        matches!(
            self.domain_type.as_str(),
            "Integers"
                | "Naturals"
                | "Modulo"
                | "ModuloUnits"
                | "GaloisField"
                | "Boolean"
                | "GaussianIntegers"
                | "EisensteinIntegers"
                | "BitVector"
                | "EvenIntegers"
                | "OddIntegers"
        )
    }

    /// Check whether a given real value falls within this domain boundary.
    pub fn contains_f64(&self, val: f64) -> bool {
        if !val.is_finite() {
            return false;
        }
        if self.domain_type == "Positive" && val <= 0.0 {
            return false;
        }
        if self.domain_type == "NonNegative" && val < 0.0 {
            return false;
        }
        if self.is_discrete() && val.fract().abs() > 1e-9 {
            return false;
        }
        if (self.domain_type == "Naturals" || self.domain_type == "Modulo") && val < 0.0 {
            return false;
        }
        if let Some(min) = self.min_val {
            if self.inclusive_min {
                if val < min {
                    return false;
                }
            } else if val <= min {
                return false;
            }
        }
        if let Some(max) = self.max_val {
            if self.inclusive_max {
                if val > max {
                    return false;
                }
            } else if val >= max {
                return false;
            }
        }
        true
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Naturals => write!(f, "ℕ"),
            Self::Integers => write!(f, "ℤ"),
            Self::Rationals => write!(f, "ℚ"),
            Self::Reals => write!(f, "ℝ"),
            Self::Complex => write!(f, "ℂ"),
            Self::Surreals => write!(f, "ℕO"),
            Self::PAdics { prime } => write!(f, "ℚ_{}", prime),
            Self::Adeles => write!(f, "𝔸"),
            Self::Modulo { modulus } => write!(f, "ℤ/{}ℤ", modulus),
            Self::GaloisField { prime, power } => write!(f, "GF({}^{})", prime, power),
            Self::Hypercomplex { dimension } => write!(f, "Hypercomplex(dim={})", dimension),
            Self::Matrix {
                rows,
                cols,
                element_domain,
            } => {
                write!(f, "Mat_{}x{}({})", rows, cols, element_domain)
            }
            Self::Tensor { rank, base_domain } => {
                write!(f, "Tensor(rank={}, {})", rank, base_domain)
            }
            Self::Set => write!(f, "Set"),
            Self::Boolean => write!(f, "Bool"),
            Self::Generic => write!(f, "Generic"),
        }
    }
}

/// Unified Ambient Mathematical Context representing active algebra, calculus, logic, and notation systems.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MathContext {
    pub algebra_domain: Domain,
    pub calculus_system: String, // "Leibniz", "Caputo(0.5)", "Variational", "QuantumCommutator", "JacksonQ(q)", "StochasticIto"
    pub logic_system: String, // "Boolean", "KleeneK3", "LukasiewiczL3", "BelnapB4", "Quantum", "NValued"
    pub non_commutative_override: Option<bool>,
}

impl Default for MathContext {
    fn default() -> Self {
        Self {
            algebra_domain: Domain::Generic,
            calculus_system: "Leibniz".to_string(),
            logic_system: "Boolean".to_string(),
            non_commutative_override: None,
        }
    }
}

impl MathContext {
    /// Create a new MathContext with custom settings.
    pub fn new(
        algebra_domain: Domain,
        calculus_system: impl Into<String>,
        logic_system: impl Into<String>,
    ) -> Self {
        Self {
            algebra_domain,
            calculus_system: calculus_system.into(),
            logic_system: logic_system.into(),
            non_commutative_override: None,
        }
    }

    /// Check if multiplication is commutative in this context.
    pub fn is_commutative(&self) -> bool {
        if let Some(nc) = self.non_commutative_override {
            !nc
        } else {
            self.algebra_domain.is_commutative_mul()
        }
    }

    /// Returns concise string summary of active mathematical context.
    pub fn summary(&self) -> String {
        format!(
            "Algebra: {:?} | Calculus: {} | Logic: {}",
            self.algebra_domain, self.calculus_system, self.logic_system
        )
    }
}

/// Target representation goal for algebraic transformations and inter-domain routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GoalDomain {
    /// Conserve original domain of the input expression (Default)
    #[default]
    ConserveOriginal,
    /// Complex Exponential Field (Euler expansion: e^{iz})
    ComplexExponential,
    /// Rational Field in Weierstrass parameter (t = tan(x/2))
    RationalWeierstrass,
    /// Real Hyperbolic Functions (sinh, cosh, tanh)
    Hyperbolic,
    /// Operational Frequency Domain (Laplace s / Fourier \omega)
    FrequencyOperational,
    /// 2-Nilpotent Dual Algebra for Itô differentials
    NilpotentDual,
    /// Geometric Clifford Multivector Blades
    CliffordBlade,
    /// Bring Radical / Modular Form for Quintics
    BringModularQuintic,
    /// Conway Surreal Number Form
    SurrealForm,
    /// Adele Component Projection
    AdeleRepresentation,
}
