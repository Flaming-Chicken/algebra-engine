//! Scalar atoms, exact Gaussian rationals, and high-performance cocycle multivectors.
//!
//! Provides zero-allocation, SIMD-friendly handling of complex numbers (\mathbb{C}),
//! quaternions (\mathbb{H}), octonions (\mathbb{O}), and Clifford multivectors \mathcal{Cl}(p,q,r)
//! using 2-cocycle bitmask algebra and categorical pushout resolution.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

/// Canonical Greatest Common Divisor helper.
pub const fn gcd_i64(a: i64, b: i64) -> i64 {
    let mut x = a.unsigned_abs();
    let mut y = b.unsigned_abs();
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    x as i64
}

/// Clifford / Cayley-Dickson algebra signature or kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlgebraKind {
    /// Complex numbers \mathbb{C} \cong \mathcal{Cl}(0, 1) or Cayley-Dickson grade 1 (dim 2).
    Complex,
    /// Quaternions \mathbb{H} \cong \mathcal{Cl}(0, 2) or Cayley-Dickson grade 2 (dim 4).
    Quaternion,
    /// Octonions \mathbb{O}: Cayley-Dickson grade 3 (dim 8, non-associative, alternative algebra).
    Octonion,
    /// General Clifford geometric algebra \mathcal{Cl}(p, q, r) with p positive, q negative, r null generators.
    Clifford { p: u8, q: u8, r: u8 },
}

/// Bitmask-indexed multivector with up to 8 basis blades stored inline without heap allocation.
///
/// Basis blades are identified by bitmask indices:
/// - Bit 0 (0b001) -> e_1
/// - Bit 1 (0b010) -> e_2
/// - Bit 2 (0b100) -> e_3
/// - Bitmask 0b011 -> e_12 = e_1 \wedge e_2
/// - Bitmask 0b000 -> scalar 1
#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct CocycleMultivector {
    pub kind: AlgebraKind,
    /// Coefficients for basis blades 0..7 (dim up to 8).
    pub blades: [f64; 8],
}

impl CocycleMultivector {
    /// Zero multivector.
    pub const fn zero(kind: AlgebraKind) -> Self {
        Self {
            kind,
            blades: [0.0; 8],
        }
    }

    /// Scalar element c * 1.
    pub const fn scalar(kind: AlgebraKind, c: f64) -> Self {
        let mut b = [0.0; 8];
        b[0] = c;
        Self { kind, blades: b }
    }

    /// Complex number a + b i.
    pub const fn complex(re: f64, im: f64) -> Self {
        let mut b = [0.0; 8];
        b[0] = re;
        b[1] = im; // e_1^2 = -1
        Self {
            kind: AlgebraKind::Complex,
            blades: b,
        }
    }

    /// Quaternion w + x i + y j + z k.
    pub const fn quaternion(w: f64, x: f64, y: f64, z: f64) -> Self {
        let mut b = [0.0; 8];
        b[0] = w;
        b[1] = x; // i
        b[2] = y; // j
        b[3] = z; // k
        Self {
            kind: AlgebraKind::Quaternion,
            blades: b,
        }
    }

    /// Octonion with 8 real components.
    pub const fn octonion(c: [f64; 8]) -> Self {
        Self {
            kind: AlgebraKind::Octonion,
            blades: c,
        }
    }

    /// 3D Geometric Algebra \mathcal{Cl}(3, 0) multivector (8 components: 1 scalar, 3 vectors, 3 bivectors, 1 pseudoscalar).
    pub const fn cl3(blades: [f64; 8]) -> Self {
        Self {
            kind: AlgebraKind::Clifford { p: 3, q: 0, r: 0 },
            blades,
        }
    }

    /// Spacetime Algebra \mathcal{Cl}(1, 3) (Minkowski metric +---).
    pub const fn sta(blades: [f64; 8]) -> Self {
        Self {
            kind: AlgebraKind::Clifford { p: 1, q: 3, r: 0 },
            blades,
        }
    }

    /// Check if all coefficients are zero.
    pub fn is_zero(&self) -> bool {
        self.blades.iter().all(|&x| x == 0.0)
    }

    /// Addition of multivectors.
    pub fn add(&self, rhs: &Self) -> Self {
        let mut out = [0.0; 8];
        for i in 0..8 {
            out[i] = self.blades[i] + rhs.blades[i];
        }
        Self {
            kind: self.kind,
            blades: out,
        }
    }

    /// Subtraction of multivectors.
    pub fn sub(&self, rhs: &Self) -> Self {
        let mut out = [0.0; 8];
        for i in 0..8 {
            out[i] = self.blades[i] - rhs.blades[i];
        }
        Self {
            kind: self.kind,
            blades: out,
        }
    }

    /// Scalar multiplication.
    pub fn scale(&self, s: f64) -> Self {
        let mut out = [0.0; 8];
        for i in 0..8 {
            out[i] = self.blades[i] * s;
        }
        Self {
            kind: self.kind,
            blades: out,
        }
    }

    /// Geometric product using 2-cocycle bitmask evaluation.
    pub fn geometric_product(&self, rhs: &Self) -> Self {
        match self.kind {
            AlgebraKind::Complex => {
                // (a + bi)(c + di) = (ac - bd) + (ad + bc)i
                let (a, b) = (self.blades[0], self.blades[1]);
                let (c, d) = (rhs.blades[0], rhs.blades[1]);
                let mut out = [0.0; 8];
                out[0] = a * c - b * d;
                out[1] = a * d + b * c;
                Self {
                    kind: AlgebraKind::Complex,
                    blades: out,
                }
            }
            AlgebraKind::Quaternion => {
                // Hamilton quaternion product
                let (w1, x1, y1, z1) = (self.blades[0], self.blades[1], self.blades[2], self.blades[3]);
                let (w2, x2, y2, z2) = (rhs.blades[0], rhs.blades[1], rhs.blades[2], rhs.blades[3]);
                let mut out = [0.0; 8];
                out[0] = w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2;
                out[1] = w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2;
                out[2] = w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2;
                out[3] = w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2;
                Self {
                    kind: AlgebraKind::Quaternion,
                    blades: out,
                }
            }
            AlgebraKind::Octonion => {
                // Cayley-Dickson multiplication of octonions (A, B)(C, D) = (AC - D*B, DA + BC*)
                // where A, B, C, D are quaternions!
                let a = CocycleMultivector::quaternion(self.blades[0], self.blades[1], self.blades[2], self.blades[3]);
                let b = CocycleMultivector::quaternion(self.blades[4], self.blades[5], self.blades[6], self.blades[7]);
                let c = CocycleMultivector::quaternion(rhs.blades[0], rhs.blades[1], rhs.blades[2], rhs.blades[3]);
                let d = CocycleMultivector::quaternion(rhs.blades[4], rhs.blades[5], rhs.blades[6], rhs.blades[7]);

                let ac = a.geometric_product(&c);
                let d_star = d.quaternion_conjugate();
                let d_star_b = d_star.geometric_product(&b);
                let first_half = ac.sub(&d_star_b);

                let da = d.geometric_product(&a);
                let c_star = c.quaternion_conjugate();
                let b_c_star = b.geometric_product(&c_star);
                let second_half = da.add(&b_c_star);

                let mut out = [0.0; 8];
                out[0..4].copy_from_slice(&first_half.blades[0..4]);
                out[4..8].copy_from_slice(&second_half.blades[0..4]);
                Self {
                    kind: AlgebraKind::Octonion,
                    blades: out,
                }
            }
            AlgebraKind::Clifford { p, q, r } => {
                // General bitmask Clifford geometric product:
                // e_u * e_v = \sigma(u, v) e_{u ^ v}
                let mut out = [0.0; 8];
                for u in 0..8u8 {
                    let cu = self.blades[u as usize];
                    if cu == 0.0 {
                        continue;
                    }
                    for v in 0..8u8 {
                        let cv = rhs.blades[v as usize];
                        if cv == 0.0 {
                            continue;
                        }
                        let (sign, target) = clifford_basis_product(u, v, p, q, r);
                        if sign != 0.0 && (target as usize) < 8 {
                            out[target as usize] += cu * cv * sign;
                        }
                    }
                }
                Self {
                    kind: self.kind,
                    blades: out,
                }
            }
        }
    }

    /// Quaternion conjugate: w - xi - yj - zk.
    fn quaternion_conjugate(&self) -> Self {
        let mut out = self.blades;
        out[1] = -out[1];
        out[2] = -out[2];
        out[3] = -out[3];
        Self {
            kind: self.kind,
            blades: out,
        }
    }

    /// Reversion (reversal of blade factor orders):
    /// \widetilde{e_A} = (-1)^{k(k-1)/2} e_A for grade k.
    pub fn reverse(&self) -> Self {
        let mut out = [0.0; 8];
        for mask in 0..8u8 {
            let grade = mask.count_ones();
            let sign = if (grade * (grade - 1) / 2) % 2 == 1 { -1.0 } else { 1.0 };
            out[mask as usize] = self.blades[mask as usize] * sign;
        }
        Self {
            kind: self.kind,
            blades: out,
        }
    }

    /// Grade projection: extract components of specific grade k (count_ones == k).
    pub fn grade_project(&self, target_grade: u32) -> Self {
        let mut out = [0.0; 8];
        for mask in 0..8u8 {
            if mask.count_ones() == target_grade {
                out[mask as usize] = self.blades[mask as usize];
            }
        }
        Self {
            kind: self.kind,
            blades: out,
        }
    }

    /// Norm squared: \langle A \widetilde{A} \rangle_0.
    pub fn norm_squared(&self) -> f64 {
        match self.kind {
            AlgebraKind::Complex | AlgebraKind::Quaternion | AlgebraKind::Octonion => {
                self.blades.iter().map(|&x| x * x).sum()
            }
            AlgebraKind::Clifford { .. } => {
                let rev = self.reverse();
                let prod = self.geometric_product(&rev);
                prod.blades[0]
            }
        }
    }

    /// Multiplicative inverse: A^{-1} = \widetilde{A} / \|A\|^2.
    pub fn inverse(&self) -> Result<Self, &'static str> {
        let n2 = self.norm_squared();
        if n2.abs() < 1e-15 {
            return Err("Zero divisor in multivector inversion");
        }
        let rev = self.reverse();
        Ok(rev.scale(1.0 / n2))
    }
}

/// Bitwise Clifford basis blade product calculator.
fn clifford_basis_product(u: u8, v: u8, p: u8, q: u8, r: u8) -> (f64, u8) {
    // 1. Check for degenerate null generators (r): if u and v share a null generator, result is 0
    let common = u & v;
    let mut sign = 1.0;

    for i in 0..8u8 {
        if (common & (1 << i)) != 0 {
            if i < p {
                // Positive generator: e_i^2 = +1
            } else if i < p + q {
                // Negative generator: e_i^2 = -1
                sign = -sign;
            } else if i < p + q + r {
                // Null generator: e_i^2 = 0
                return (0.0, 0);
            }
        }
    }

    // 2. Count swaps required to bring basis factors into canonical order
    let mut swaps = 0u32;
    for i in 0..8u8 {
        if (v & (1 << i)) != 0 {
            // Count bits in u that are strictly greater than i
            let higher_in_u = u & !((1 << (i + 1)) - 1);
            swaps += higher_in_u.count_ones();
        }
    }

    if swaps % 2 == 1 {
        sign = -sign;
    }

    (sign, u ^ v)
}

/// Lossless scalar atom representation handling exact integers, Gaussian rationals,
/// floating approximations, and Clifford/hypercomplex multivectors.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ScalarAtom {
    /// Exact 0.
    Zero,
    /// Exact 1.
    One,
    /// Signed 64-bit exact integer.
    Integer(i64),
    /// Exact rational fraction (num, den) with den > 0 and gcd(num, den) == 1.
    Rational(i64, i64),
    /// 64-bit IEEE floating-point approximation.
    Float(f64),
    /// Complex floating-point number a + b i.
    ComplexF64 { re: f64, im: f64 },
    /// Exact Gaussian rational \mathbb{Q}(i) with exact reduced fraction components.
    ComplexRational {
        re_num: i64,
        re_den: i64,
        im_num: i64,
        im_den: i64,
    },
    /// Exact Gaussian integer \mathbb{Z}[i].
    GaussianInt { re: i64, im: i64 },
    /// Exact Eisenstein integer \mathbb{Z}[\omega] where \omega = e^{2\pi i/3}.
    EisensteinInt { a: i64, b: i64 },
    /// General hypercomplex / Clifford multivector.
    CocycleMultivector(CocycleMultivector),
    /// Surreal number atom (Conway cut / dyadic / infinitesimal / infinite normal form).
    Surreal(SurrealAtom),
    /// Adele ring atom \mathbb{A}_\mathbb{Q} = \mathbb{R} \times \prod'_p \mathbb{Q}_p.
    Adele(AdeleAtom),
}

/// Surreal Number atom supporting infinitesimals, infinities, and Conway normal form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurrealAtom {
    /// Zero { | } (birthday 0).
    Zero,
    /// Exact integer born on day |n|.
    Integer(i64),
    /// Dyadic rational m / 2^k born on day k.
    DyadicRational { num: i64, exp2: u32 },
    /// Infinitesimal \epsilon^order * coeff born on day \omega.
    Infinitesimal { order: u32, coeff: f64 },
    /// Infinity \omega^order * coeff born on day \omega.
    Infinity { order: u32, coeff: f64 },
    /// Conway normal form: finite rational + \sum r_i \omega^i + \sum c_j \epsilon^j.
    ConwayForm {
        finite_num: i64,
        finite_den: i64,
        omega_coeff: f64,
        epsilon_coeff: f64,
    },
}

impl Default for SurrealAtom {
    fn default() -> Self {
        Self::Zero
    }
}

/// Adele Number atom representing a restricted direct product element (x_\infty, (x_p)_p) \in \mathbb{A}_\mathbb{Q}.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdeleAtom {
    /// Archimedean real component x_\infty = arch_num / arch_den.
    pub arch_num: i64,
    pub arch_den: i64,
    /// Non-archimedean p-adic valuations (prime p, v_p(x)).
    /// For all primes not in this list, v_p(x) = 0 (unit in \mathbb{Z}_p).
    pub p_adic_valuations: smallvec::SmallVec<[(u64, i32); 4]>,
}

impl Default for AdeleAtom {
    fn default() -> Self {
        Self {
            arch_num: 0,
            arch_den: 1,
            p_adic_valuations: smallvec::SmallVec::new(),
        }
    }
}

impl AdeleAtom {
    /// Canonical embedding of rational q = num/den into the adele ring \mathbb{A}_\mathbb{Q}.
    pub fn from_rational(mut num: i64, mut den: i64) -> Self {
        if den == 0 {
            return Self::default();
        }
        if num == 0 {
            return Self { arch_num: 0, arch_den: 1, p_adic_valuations: smallvec::SmallVec::new() };
        }
        let g = gcd_i64(num, den);
        num /= g;
        den /= g;

        let mut vals = smallvec::SmallVec::new();
        // Compute valuations for prime factors of num and den
        for &p in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31] {
            let mut v = 0i32;
            let mut n = num.unsigned_abs();
            while n > 0 && n % p == 0 {
                v += 1;
                n /= p;
            }
            let mut d = den.unsigned_abs();
            while d > 0 && d % p == 0 {
                v -= 1;
                d /= p;
            }
            if v != 0 {
                vals.push((p, v));
            }
        }
        Self {
            arch_num: num,
            arch_den: den,
            p_adic_valuations: vals,
        }
    }

    /// Artin idele norm: |x|_\infty \times \prod_p |x|_p.
    /// By Artin's product formula, for any non-zero rational q, this is EXACTLY 1.0!
    pub fn idele_norm(&self) -> f64 {
        if self.arch_num == 0 {
            return 0.0;
        }
        let arch_abs = (self.arch_num as f64 / self.arch_den as f64).abs();
        let mut prod = arch_abs;
        for &(p, v) in &self.p_adic_valuations {
            // |x|_p = p^{-v}
            prod *= (p as f64).powi(-v);
        }
        prod
    }
}

impl Default for ScalarAtom {
    fn default() -> Self {
        Self::Zero
    }
}

impl Eq for ScalarAtom {}

impl Hash for ScalarAtom {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Zero => 0u8.hash(state),
            Self::One => 1u8.hash(state),
            Self::Integer(i) => {
                2u8.hash(state);
                i.hash(state);
            }
            Self::Rational(n, d) => {
                3u8.hash(state);
                n.hash(state);
                d.hash(state);
            }
            Self::Float(f) => {
                4u8.hash(state);
                f.to_bits().hash(state);
            }
            Self::ComplexF64 { re, im } => {
                5u8.hash(state);
                re.to_bits().hash(state);
                im.to_bits().hash(state);
            }
            Self::ComplexRational { re_num, re_den, im_num, im_den } => {
                6u8.hash(state);
                re_num.hash(state);
                re_den.hash(state);
                im_num.hash(state);
                im_den.hash(state);
            }
            Self::GaussianInt { re, im } => {
                7u8.hash(state);
                re.hash(state);
                im.hash(state);
            }
            Self::EisensteinInt { a, b } => {
                8u8.hash(state);
                a.hash(state);
                b.hash(state);
            }
            Self::CocycleMultivector(cm) => {
                9u8.hash(state);
                cm.kind.hash(state);
                for b in cm.blades {
                    b.to_bits().hash(state);
                }
            }
            Self::Surreal(s) => {
                10u8.hash(state);
                match s {
                    SurrealAtom::Zero => 0u8.hash(state),
                    SurrealAtom::Integer(i) => { 1u8.hash(state); i.hash(state); }
                    SurrealAtom::DyadicRational { num, exp2 } => { 2u8.hash(state); num.hash(state); exp2.hash(state); }
                    SurrealAtom::Infinitesimal { order, coeff } => { 3u8.hash(state); order.hash(state); coeff.to_bits().hash(state); }
                    SurrealAtom::Infinity { order, coeff } => { 4u8.hash(state); order.hash(state); coeff.to_bits().hash(state); }
                    SurrealAtom::ConwayForm { finite_num, finite_den, omega_coeff, epsilon_coeff } => {
                        5u8.hash(state);
                        finite_num.hash(state);
                        finite_den.hash(state);
                        omega_coeff.to_bits().hash(state);
                        epsilon_coeff.to_bits().hash(state);
                    }
                }
            }
            Self::Adele(a) => {
                11u8.hash(state);
                a.arch_num.hash(state);
                a.arch_den.hash(state);
                for &(p, v) in &a.p_adic_valuations {
                    p.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}

impl ScalarAtom {
    /// Create canonical rational number.
    pub fn rational(num: i64, den: i64) -> Self {
        if den == 0 {
            return Self::Float(f64::NAN);
        }
        if num == 0 {
            return Self::Zero;
        }
        let sign = if (num > 0 && den > 0) || (num < 0 && den < 0) { 1 } else { -1 };
        let g = gcd_i64(num, den);
        let n = (num.abs() / g) * sign;
        let d = den.abs() / g;
        if d == 1 {
            if n == 0 {
                Self::Zero
            } else if n == 1 {
                Self::One
            } else {
                Self::Integer(n)
            }
        } else {
            Self::Rational(n, d)
        }
    }

    /// Create canonical exact Gaussian rational \mathbb{Q}(i).
    pub fn complex_rational(re_n: i64, re_d: i64, im_n: i64, im_d: i64) -> Self {
        if re_d == 0 || im_d == 0 {
            return Self::Float(f64::NAN);
        }
        if im_n == 0 {
            return Self::rational(re_n, re_d);
        }
        let g_re = gcd_i64(re_n, re_d);
        let g_im = gcd_i64(im_n, im_d);
        let sign_re = if (re_n > 0 && re_d > 0) || (re_n < 0 && re_d < 0) { 1 } else { -1 };
        let sign_im = if (im_n > 0 && im_d > 0) || (im_n < 0 && im_d < 0) { 1 } else { -1 };

        let re_num = (re_n.abs() / g_re) * sign_re;
        let re_den = re_d.abs() / g_re;
        let im_num = (im_n.abs() / g_im) * sign_im;
        let im_den = im_d.abs() / g_im;

        if re_den == 1 && im_den == 1 {
            Self::gaussian_int(re_num, im_num)
        } else {
            Self::ComplexRational {
                re_num,
                re_den,
                im_num,
                im_den,
            }
        }
    }

    /// Create canonical exact Gaussian integer \mathbb{Z}[i].
    pub const fn gaussian_int(re: i64, im: i64) -> Self {
        if im == 0 {
            if re == 0 {
                Self::Zero
            } else if re == 1 {
                Self::One
            } else {
                Self::Integer(re)
            }
        } else {
            Self::GaussianInt { re, im }
        }
    }

    /// Create imaginary unit i in \mathbb{Q}(i).
    pub const fn imaginary_unit() -> Self {
        Self::GaussianInt { re: 0, im: 1 }
    }

    /// Returns `true` if this scalar is identically zero.
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Zero => true,
            Self::Integer(0) => true,
            Self::Rational(0, _) => true,
            Self::Float(f) => *f == 0.0,
            Self::ComplexF64 { re, im } => *re == 0.0 && *im == 0.0,
            Self::ComplexRational { re_num, im_num, .. } => *re_num == 0 && *im_num == 0,
            Self::GaussianInt { re, im } => *re == 0 && *im == 0,
            Self::EisensteinInt { a, b } => *a == 0 && *b == 0,
            Self::CocycleMultivector(cm) => cm.is_zero(),
            Self::Surreal(SurrealAtom::Zero) => true,
            Self::Surreal(SurrealAtom::Integer(0)) => true,
            Self::Adele(a) => a.arch_num == 0,
            _ => false,
        }
    }

    /// Returns `true` if this scalar is identically one.
    pub fn is_one(&self) -> bool {
        match self {
            Self::One => true,
            Self::Integer(1) => true,
            Self::Rational(1, 1) => true,
            Self::Float(f) => *f == 1.0,
            Self::ComplexF64 { re, im } => *re == 1.0 && *im == 0.0,
            Self::ComplexRational { re_num, re_den, im_num, .. } => {
                *re_num == 1 && *re_den == 1 && *im_num == 0
            }
            Self::GaussianInt { re, im } => *re == 1 && *im == 0,
            Self::Surreal(SurrealAtom::Integer(1)) => true,
            Self::Adele(a) => a.arch_num == 1 && a.arch_den == 1 && a.p_adic_valuations.is_empty(),
            _ => false,
        }
    }

    /// Converts scalar to 64-bit float approximation if real.
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Self::Zero => Some(0.0),
            Self::One => Some(1.0),
            Self::Integer(i) => Some(*i as f64),
            Self::Rational(n, d) => Some(*n as f64 / *d as f64),
            Self::Float(f) => Some(*f),
            Self::ComplexF64 { re, im } if *im == 0.0 => Some(*re),
            Self::ComplexRational { re_num, re_den, im_num, .. } if *im_num == 0 => {
                Some(*re_num as f64 / *re_den as f64)
            }
            Self::GaussianInt { re, im } if *im == 0 => Some(*re as f64),
            Self::Surreal(SurrealAtom::Zero) => Some(0.0),
            Self::Surreal(SurrealAtom::Integer(i)) => Some(*i as f64),
            Self::Surreal(SurrealAtom::DyadicRational { num, exp2 }) => Some(*num as f64 / (1u64 << exp2) as f64),
            Self::Surreal(SurrealAtom::Infinitesimal { .. }) => Some(0.0), // Standard part of infinitesimal is 0
            Self::Surreal(SurrealAtom::Infinity { .. }) => Some(f64::INFINITY),
            Self::Surreal(SurrealAtom::ConwayForm { finite_num, finite_den, .. }) => Some(*finite_num as f64 / *finite_den as f64),
            Self::Adele(a) => Some(a.arch_num as f64 / a.arch_den as f64),
            _ => None,
        }
    }

    /// Converts scalar to complex f64 (re, im).
    pub fn to_complex_f64(&self) -> (f64, f64) {
        match self {
            Self::Zero => (0.0, 0.0),
            Self::One => (1.0, 0.0),
            Self::Integer(i) => (*i as f64, 0.0),
            Self::Rational(n, d) => (*n as f64 / *d as f64, 0.0),
            Self::Float(f) => (*f, 0.0),
            Self::ComplexF64 { re, im } => (*re, *im),
            Self::ComplexRational { re_num, re_den, im_num, im_den } => {
                (*re_num as f64 / *re_den as f64, *im_num as f64 / *im_den as f64)
            }
            Self::GaussianInt { re, im } => (*re as f64, *im as f64),
            Self::EisensteinInt { a, b } => {
                // a + b \omega = a + b(-1/2 + \sqrt{3}/2 i)
                let re = *a as f64 - 0.5 * *b as f64;
                let im = *b as f64 * (3.0f64).sqrt() * 0.5;
                (re, im)
            }
            Self::CocycleMultivector(cm) => (cm.blades[0], cm.blades[1]),
            Self::Surreal(s) => (Self::Surreal(s.clone()).to_f64().unwrap_or(0.0), 0.0),
            Self::Adele(a) => (a.arch_num as f64 / a.arch_den as f64, 0.0),
        }
    }
}

/// Categorical pushout resolver for scalar atoms: computes pushout A \otimes_{R_0} B
/// to lift heterogeneous numbers to their least common algebraic field/algebra.
pub struct ScalarLattice;

impl ScalarLattice {
    /// Perform addition across two scalar atoms using categorical pushout.
    pub fn add(a: &ScalarAtom, b: &ScalarAtom) -> ScalarAtom {
        if a.is_zero() {
            return b.clone();
        }
        if b.is_zero() {
            return a.clone();
        }

        match (a, b) {
            // Integer + Integer
            (ScalarAtom::Integer(x), ScalarAtom::Integer(y)) => {
                match x.checked_add(*y) {
                    Some(sum) => {
                        if sum == 0 { ScalarAtom::Zero } else if sum == 1 { ScalarAtom::One } else { ScalarAtom::Integer(sum) }
                    }
                    None => ScalarAtom::Float(*x as f64 + *y as f64),
                }
            }
            // Rational + Integer / Rational + Rational
            (ScalarAtom::Integer(x), ScalarAtom::Rational(ny, dy))
            | (ScalarAtom::Rational(ny, dy), ScalarAtom::Integer(x)) => {
                let n = x * dy + ny;
                ScalarAtom::rational(n, *dy)
            }
            (ScalarAtom::Rational(nx, dx), ScalarAtom::Rational(ny, dy)) => {
                let n = nx * dy + ny * dx;
                let d = dx * dy;
                ScalarAtom::rational(n, d)
            }
            // Gaussian Integer + Gaussian Integer
            (ScalarAtom::GaussianInt { re: r1, im: i1 }, ScalarAtom::GaussianInt { re: r2, im: i2 }) => {
                ScalarAtom::gaussian_int(r1 + r2, i1 + i2)
            }
            // Exact Gaussian Rational additions
            (ScalarAtom::ComplexRational { re_num: r1n, re_den: r1d, im_num: i1n, im_den: i1d },
             ScalarAtom::ComplexRational { re_num: r2n, re_den: r2d, im_num: i2n, im_den: i2d }) => {
                let re_n = r1n * r2d + r2n * r1d;
                let re_d = r1d * r2d;
                let im_n = i1n * i2d + i2n * i1d;
                let im_d = i1d * i2d;
                ScalarAtom::complex_rational(re_n, re_d, im_n, im_d)
            }
            // Integer / Rational + Gaussian
            (ScalarAtom::Integer(k), ScalarAtom::GaussianInt { re, im })
            | (ScalarAtom::GaussianInt { re, im }, ScalarAtom::Integer(k)) => {
                ScalarAtom::gaussian_int(re + k, *im)
            }
            // Multivector additions
            (ScalarAtom::CocycleMultivector(m1), ScalarAtom::CocycleMultivector(m2)) if m1.kind == m2.kind => {
                ScalarAtom::CocycleMultivector(m1.add(m2))
            }
            // Surreal + Surreal additions
            (ScalarAtom::Surreal(s1), ScalarAtom::Surreal(s2)) => {
                match (s1, s2) {
                    (SurrealAtom::Integer(i1), SurrealAtom::Integer(i2)) => {
                        ScalarAtom::Surreal(SurrealAtom::Integer(i1 + i2))
                    }
                    (SurrealAtom::Integer(i), SurrealAtom::Infinitesimal { coeff, .. })
                    | (SurrealAtom::Infinitesimal { coeff, .. }, SurrealAtom::Integer(i)) => {
                        ScalarAtom::Surreal(SurrealAtom::ConwayForm {
                            finite_num: *i,
                            finite_den: 1,
                            omega_coeff: 0.0,
                            epsilon_coeff: *coeff,
                        })
                    }
                    _ => {
                        let f1 = a.to_f64().unwrap_or(0.0);
                        let f2 = b.to_f64().unwrap_or(0.0);
                        ScalarAtom::Float(f1 + f2)
                    }
                }
            }
            (ScalarAtom::Integer(i), ScalarAtom::Surreal(s))
            | (ScalarAtom::Surreal(s), ScalarAtom::Integer(i)) => {
                match s {
                    SurrealAtom::Integer(s_i) => ScalarAtom::Surreal(SurrealAtom::Integer(s_i + i)),
                    SurrealAtom::Infinitesimal { coeff, .. } => ScalarAtom::Surreal(SurrealAtom::ConwayForm {
                        finite_num: *i,
                        finite_den: 1,
                        omega_coeff: 0.0,
                        epsilon_coeff: *coeff,
                    }),
                    _ => ScalarAtom::Float(*i as f64 + a.to_f64().unwrap_or(0.0)),
                }
            }
            // Adele + Adele additions
            (ScalarAtom::Adele(a1), ScalarAtom::Adele(a2)) => {
                let num = a1.arch_num * a2.arch_den + a2.arch_num * a1.arch_den;
                let den = a1.arch_den * a2.arch_den;
                let mut vals = a1.p_adic_valuations.clone();
                for &(p, v2) in &a2.p_adic_valuations {
                    if let Some(entry) = vals.iter_mut().find(|(p1, _)| *p1 == p) {
                        entry.1 = entry.1.min(v2); // v_p(x + y) >= min(v_p(x), v_p(y))
                    } else {
                        vals.push((p, v2));
                    }
                }
                ScalarAtom::Adele(AdeleAtom {
                    arch_num: num,
                    arch_den: den,
                    p_adic_valuations: vals,
                })
            }
            (ScalarAtom::Rational(n, d), ScalarAtom::Adele(a2))
            | (ScalarAtom::Adele(a2), ScalarAtom::Rational(n, d)) => {
                let a1 = AdeleAtom::from_rational(*n, *d);
                Self::add(&ScalarAtom::Adele(a1), &ScalarAtom::Adele(a2.clone()))
            }
            (ScalarAtom::Integer(k), ScalarAtom::Adele(a2))
            | (ScalarAtom::Adele(a2), ScalarAtom::Integer(k)) => {
                let a1 = AdeleAtom::from_rational(*k, 1);
                Self::add(&ScalarAtom::Adele(a1), &ScalarAtom::Adele(a2.clone()))
            }
            // Fallback to ComplexF64 / Float
            _ => {
                let (r1, i1) = a.to_complex_f64();
                let (r2, i2) = b.to_complex_f64();
                let re = r1 + r2;
                let im = i1 + i2;
                if im == 0.0 {
                    ScalarAtom::Float(re)
                } else {
                    ScalarAtom::ComplexF64 { re, im }
                }
            }
        }
    }

    /// Perform multiplication across two scalar atoms using categorical pushout.
    pub fn mul(a: &ScalarAtom, b: &ScalarAtom) -> ScalarAtom {
        if a.is_zero() || b.is_zero() {
            return ScalarAtom::Zero;
        }
        if a.is_one() {
            return b.clone();
        }
        if b.is_one() {
            return a.clone();
        }

        match (a, b) {
            // Integer * Integer
            (ScalarAtom::Integer(x), ScalarAtom::Integer(y)) => {
                match x.checked_mul(*y) {
                    Some(prod) => {
                        if prod == 1 { ScalarAtom::One } else { ScalarAtom::Integer(prod) }
                    }
                    None => ScalarAtom::Float(*x as f64 * *y as f64),
                }
            }
            // Rational * Rational
            (ScalarAtom::Rational(nx, dx), ScalarAtom::Rational(ny, dy)) => {
                ScalarAtom::rational(nx * ny, dx * dy)
            }
            (ScalarAtom::Integer(k), ScalarAtom::Rational(n, d))
            | (ScalarAtom::Rational(n, d), ScalarAtom::Integer(k)) => {
                ScalarAtom::rational(k * n, *d)
            }
            // Gaussian Integer multiplication: (a + bi)(c + di) = (ac - bd) + (ad + bc)i
            (ScalarAtom::GaussianInt { re: a, im: b }, ScalarAtom::GaussianInt { re: c, im: d }) => {
                ScalarAtom::gaussian_int(a * c - b * d, a * d + b * c)
            }
            // Exact Gaussian Rational multiplication
            (ScalarAtom::ComplexRational { re_num: r1n, re_den: r1d, im_num: i1n, im_den: i1d },
             ScalarAtom::ComplexRational { re_num: r2n, re_den: r2d, im_num: i2n, im_den: i2d }) => {
                // ac - bd
                let ac_n = (r1n * r2n) * (i1d * i2d);
                let bd_n = (i1n * i2n) * (r1d * r2d);
                let den_common = r1d * r2d * i1d * i2d;
                let re_n = ac_n - bd_n;
                let re_d = den_common;

                // ad + bc
                let ad_n = (r1n * i2n) * (r2d * i1d);
                let bc_n = (i1n * r2n) * (r1d * i2d);
                let im_n = ad_n + bc_n;
                let im_d = den_common;
                ScalarAtom::complex_rational(re_n, re_d, im_n, im_d)
            }
            // Multivector multiplication
            (ScalarAtom::CocycleMultivector(m1), ScalarAtom::CocycleMultivector(m2)) if m1.kind == m2.kind => {
                ScalarAtom::CocycleMultivector(m1.geometric_product(m2))
            }
            // Surreal * Surreal multiplication
            (ScalarAtom::Surreal(s1), ScalarAtom::Surreal(s2)) => {
                match (s1, s2) {
                    (SurrealAtom::Integer(i1), SurrealAtom::Integer(i2)) => {
                        ScalarAtom::Surreal(SurrealAtom::Integer(i1 * i2))
                    }
                    // \omega * \epsilon = 1 (Conway fundamental identity)
                    (SurrealAtom::Infinity { order: o1, coeff: c1 }, SurrealAtom::Infinitesimal { order: o2, coeff: c2 })
                    | (SurrealAtom::Infinitesimal { order: o2, coeff: c2 }, SurrealAtom::Infinity { order: o1, coeff: c1 }) => {
                        if o1 == o2 {
                            let prod = c1 * c2;
                            if (prod - 1.0).abs() < 1e-12 {
                                ScalarAtom::One
                            } else {
                                ScalarAtom::Float(prod)
                            }
                        } else if o1 > o2 {
                            ScalarAtom::Surreal(SurrealAtom::Infinity { order: o1 - o2, coeff: c1 * c2 })
                        } else {
                            ScalarAtom::Surreal(SurrealAtom::Infinitesimal { order: o2 - o1, coeff: c1 * c2 })
                        }
                    }
                    _ => {
                        let f1 = a.to_f64().unwrap_or(0.0);
                        let f2 = b.to_f64().unwrap_or(0.0);
                        ScalarAtom::Float(f1 * f2)
                    }
                }
            }
            (ScalarAtom::Integer(k), ScalarAtom::Surreal(s))
            | (ScalarAtom::Surreal(s), ScalarAtom::Integer(k)) => {
                match s {
                    SurrealAtom::Integer(i) => ScalarAtom::Surreal(SurrealAtom::Integer(k * i)),
                    SurrealAtom::Infinitesimal { order, coeff } => ScalarAtom::Surreal(SurrealAtom::Infinitesimal { order: *order, coeff: coeff * (*k as f64) }),
                    SurrealAtom::Infinity { order, coeff } => ScalarAtom::Surreal(SurrealAtom::Infinity { order: *order, coeff: coeff * (*k as f64) }),
                    _ => ScalarAtom::Float(*k as f64 * a.to_f64().unwrap_or(0.0)),
                }
            }
            // Adele * Adele multiplication
            (ScalarAtom::Adele(a1), ScalarAtom::Adele(a2)) => {
                let num = a1.arch_num * a2.arch_num;
                let den = a1.arch_den * a2.arch_den;
                let mut vals = a1.p_adic_valuations.clone();
                for &(p, v2) in &a2.p_adic_valuations {
                    if let Some(entry) = vals.iter_mut().find(|(p1, _)| *p1 == p) {
                        entry.1 += v2; // v_p(x * y) = v_p(x) + v_p(y)
                    } else {
                        vals.push((p, v2));
                    }
                }
                vals.retain(|entry| entry.1 != 0);
                ScalarAtom::Adele(AdeleAtom {
                    arch_num: num,
                    arch_den: den,
                    p_adic_valuations: vals,
                })
            }
            (ScalarAtom::Rational(n, d), ScalarAtom::Adele(a2))
            | (ScalarAtom::Adele(a2), ScalarAtom::Rational(n, d)) => {
                let a1 = AdeleAtom::from_rational(*n, *d);
                Self::mul(&ScalarAtom::Adele(a1), &ScalarAtom::Adele(a2.clone()))
            }
            (ScalarAtom::Integer(k), ScalarAtom::Adele(a2))
            | (ScalarAtom::Adele(a2), ScalarAtom::Integer(k)) => {
                let a1 = AdeleAtom::from_rational(*k, 1);
                Self::mul(&ScalarAtom::Adele(a1), &ScalarAtom::Adele(a2.clone()))
            }
            // Fallback to complex float multiplication
            _ => {
                let (r1, i1) = a.to_complex_f64();
                let (r2, i2) = b.to_complex_f64();
                let re = r1 * r2 - i1 * i2;
                let im = r1 * i2 + i1 * r2;
                if im == 0.0 {
                    ScalarAtom::Float(re)
                } else {
                    ScalarAtom::ComplexF64 { re, im }
                }
            }
        }
    }
}

impl fmt::Debug for ScalarAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for ScalarAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => write!(f, "0"),
            Self::One => write!(f, "1"),
            Self::Integer(i) => write!(f, "{}", i),
            Self::Rational(n, d) => write!(f, "{}/{}", n, d),
            Self::Float(v) => write!(f, "{:.6}", v),
            Self::ComplexF64 { re, im } => {
                if *im >= 0.0 {
                    write!(f, "{:.4} + {:.4}i", re, im)
                } else {
                    write!(f, "{:.4} - {:.4}i", re, -im)
                }
            }
            Self::ComplexRational { re_num, re_den, im_num, im_den } => {
                write!(f, "({}/{} + {}/{} i)", re_num, re_den, im_num, im_den)
            }
            Self::GaussianInt { re, im } => {
                if *im >= 0 {
                    write!(f, "({} + {}i)", re, im)
                } else {
                    write!(f, "({} - {}i)", re, -im)
                }
            }
            Self::EisensteinInt { a, b } => write!(f, "({} + {}ω)", a, b),
            Self::CocycleMultivector(cm) => write!(f, "Multivector({:?})", cm.kind),
            Self::Surreal(s) => match s {
                SurrealAtom::Zero => write!(f, "Surreal(0)"),
                SurrealAtom::Integer(i) => write!(f, "Surreal({})", i),
                SurrealAtom::DyadicRational { num, exp2 } => write!(f, "Surreal({}/2^{})", num, exp2),
                SurrealAtom::Infinitesimal { order, coeff } => write!(f, "Surreal({:.3}*ε^{})", coeff, order),
                SurrealAtom::Infinity { order, coeff } => write!(f, "Surreal({:.3}*ω^{})", coeff, order),
                SurrealAtom::ConwayForm { finite_num, finite_den, omega_coeff, epsilon_coeff } => {
                    write!(f, "Surreal({}/{} + {:.2}ω + {:.2}ε)", finite_num, finite_den, omega_coeff, epsilon_coeff)
                }
            },
            Self::Adele(a) => write!(f, "Adele({}/{}, vals={:?})", a.arch_num, a.arch_den, a.p_adic_valuations),
        }
    }
}
