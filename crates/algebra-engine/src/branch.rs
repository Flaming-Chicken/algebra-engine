//! # `algebra_engine::branch`
//!
//! Universal Branch Cut, Riemann Surface Sheet Tracking & Multi-Valued Function Subsystem.
//!
//! Provides a unified, consistent mathematical interface for branch cuts, branch points,
//! and multi-valued functions in complex analysis:
//! - **Branch Cut Geometries**: Standard rays $(-\infty, 0]$, custom angle rays, segments $[-1, 1]$, and complement cuts.
//! - **Branch Selections**:
//!   - `Principal`: Standard canonical sheet ($k=0$).
//!   - `Sheet(k)`: Specific Riemann surface sheet $k \in \mathbb{Z}$.
//!   - `CustomCut`: User-specified branch cut angle and sheet index.
//!   - `MultiValued`: Explicit multi-valued evaluation returning the full fiber $\{f_k(z)\}$.
//!   - `Unspecified`: Symbolic analytic continuation tracking winding numbers.
//! - **Supported Functions**:
//!   - Complex Logarithm $\ln_k(z) = \ln|z| + i(\operatorname{Arg}(z) + 2\pi k)$
//!   - $n$-th Roots $\sqrt\[n\]{z}_k = |z|^{1/n} e^{i(\theta + 2\pi k)/n}$
//!   - Complex Powers $z_k^\alpha = \exp(\alpha \ln_k(z))$
//!   - Inverse Trigonometric ($\arcsin, \arccos, \arctan$)
//!   - Inverse Hyperbolic ($\operatorname{arcsinh}, \operatorname{arccosh}, \operatorname{arctanh}$)
//!   - Lambert $W$-function $W_k(z)$ ($k = 0, -1, \dots$).
//! - **Analytic Continuation**: Continuous path tracking $\gamma(t)$ across branch cuts with monodromy group actions.

use algebra_core::error::{AlgebraError, AlgebraResult};
use std::f64::consts::PI;

/// Geometric path/locus of a branch cut in the complex plane $\mathbb{C}$.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchCutGeometry {
    /// Negative real axis $(-\infty, 0]$ (standard for $\ln(z), \sqrt{z}, z^\alpha$).
    NegativeRealAxis,
    /// Positive real axis $[0, \infty)$.
    PositiveRealAxis,
    /// Ray starting from $(x_0, y_0)$ extending to infinity at angle $\theta$.
    Ray { origin: (f64, f64), angle_rad: f64 },
    /// Finite line segment connecting two branch points $(x_1, y_1)$ and $(x_2, y_2)$ (e.g. $[-1, 1]$).
    Segment { start: (f64, f64), end: (f64, f64) },
    /// Complementary cuts on the real axis $(-\infty, -1] \cup [1, \infty)$ (e.g. $\operatorname{arccsc}, \operatorname{arcsec}$).
    RealComplementIntervals { threshold: f64 },
}

/// Selection mode for evaluating a multi-valued complex function.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchSelection {
    /// Standard principal branch ($k = 0$, default cut).
    Principal,
    /// Specific Riemann sheet index $k \in \mathbb{Z}$.
    Sheet(i64),
    /// Custom branch cut geometry with sheet index $k$.
    CustomCut {
        geometry: BranchCutGeometry,
        sheet: i64,
    },
    /// Explicit multi-valued evaluation on a specified set of Riemann sheets.
    MultiValued { sheets: Vec<i64> },
    /// All algebraic branches for an $n$-th degree algebraic function ($k = 0, \dots, n-1$).
    AllSheets { max_branches: usize },
    /// Unspecified branch for symbolic and monodromy tracking.
    Unspecified,
}

/// Multi-Valued complex evaluation result: represents single or multiple values.
#[derive(Debug, Clone, PartialEq)]
pub enum MultiValuedResult {
    /// Single complex value $(u, v) = u + iv$.
    Single { re: f64, im: f64, sheet: i64 },
    /// Discrete fiber / set of values $\{ u_k + iv_k \}$ across selected Riemann sheets.
    Set(Vec<(f64, f64, i64)>),
}

impl MultiValuedResult {
    /// Returns the principal or first value as `(re, im)`.
    pub fn primary_value(&self) -> (f64, f64) {
        match self {
            MultiValuedResult::Single { re, im, .. } => (*re, *im),
            MultiValuedResult::Set(vals) => {
                if vals.is_empty() {
                    (0.0, 0.0)
                } else {
                    (vals[0].0, vals[0].1)
                }
            }
        }
    }

    /// Returns all values as a vector of complex tuples `(re, im, sheet)`.
    pub fn all_values(&self) -> Vec<(f64, f64, i64)> {
        match self {
            MultiValuedResult::Single { re, im, sheet } => vec![(*re, *im, *sheet)],
            MultiValuedResult::Set(vals) => vals.clone(),
        }
    }
}

/// Enumeration of elementary and special multi-valued functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BranchCutFunctionKind {
    /// Natural logarithm $\ln(z)$ (branch points: $0, \infty$).
    Log,
    /// Square root $\sqrt{z}$ (branch points: $0, \infty$).
    Sqrt,
    /// $n$-th root $\sqrt\[n\]{z}$ (branch points: $0, \infty$).
    Root(u32),
    /// Complex power $z^\alpha$ with default exponent (branch points: $0, \infty$).
    Power,
    /// Complex power $z^{\alpha}$ with explicit complex exponent $\alpha = a + bi$ stored via IEEE-754 bit patterns.
    PowerWithExponent { exp_re_bits: u64, exp_im_bits: u64 },
    /// Inverse sine $\arcsin(z)$ (branch points: $\pm 1, \infty$).
    ArcSin,
    /// Inverse cosine $\arccos(z)$ (branch points: $\pm 1, \infty$).
    ArcCos,
    /// Inverse tangent $\arctan(z)$ (branch points: $\pm i$).
    ArcTan,
    /// Inverse hyperbolic sine $\operatorname{arcsinh}(z)$ (branch points: $\pm i$).
    ArcSinh,
    /// Inverse hyperbolic cosine $\operatorname{arccosh}(z)$ (branch points: $+1, -1$).
    ArcCosh,
    /// Inverse hyperbolic tangent $\operatorname{arctanh}(z)$ (branch points: $\pm 1$).
    ArcTanh,
    /// Lambert $W$-function $W_k(z)$ (branch points: $-1/e, 0$).
    LambertW,
}

/// Unified Branch Cut Evaluator and Riemann Surface Manager.
pub struct BranchCutManager;

impl BranchCutManager {
    /// Returns the standard default branch cut geometry for a given function kind.
    pub fn default_cut(kind: BranchCutFunctionKind) -> BranchCutGeometry {
        match kind {
            BranchCutFunctionKind::Log
            | BranchCutFunctionKind::Sqrt
            | BranchCutFunctionKind::Root(_)
            | BranchCutFunctionKind::Power
            | BranchCutFunctionKind::PowerWithExponent { .. } => {
                BranchCutGeometry::NegativeRealAxis
            }
            BranchCutFunctionKind::ArcSin | BranchCutFunctionKind::ArcCos => {
                BranchCutGeometry::RealComplementIntervals { threshold: 1.0 }
            }
            BranchCutFunctionKind::ArcTan => BranchCutGeometry::Ray {
                origin: (0.0, 1.0),
                angle_rad: PI / 2.0,
            },
            BranchCutFunctionKind::ArcSinh => BranchCutGeometry::Ray {
                origin: (0.0, 1.0),
                angle_rad: PI / 2.0,
            },
            BranchCutFunctionKind::ArcCosh => BranchCutGeometry::NegativeRealAxis,
            BranchCutFunctionKind::ArcTanh => {
                BranchCutGeometry::RealComplementIntervals { threshold: 1.0 }
            }
            BranchCutFunctionKind::LambertW => BranchCutGeometry::Ray {
                origin: (-1.0 / std::f64::consts::E, 0.0),
                angle_rad: PI,
            },
        }
    }

    /// Evaluates a multi-valued function $f(z)$ under the chosen `BranchSelection`.
    pub fn eval(
        kind: BranchCutFunctionKind,
        re: f64,
        im: f64,
        selection: &BranchSelection,
    ) -> AlgebraResult<MultiValuedResult> {
        match selection {
            BranchSelection::Principal => {
                let (res_re, res_im) = Self::eval_single_sheet(kind, re, im, 0, None)?;
                Ok(MultiValuedResult::Single {
                    re: res_re,
                    im: res_im,
                    sheet: 0,
                })
            }
            BranchSelection::Sheet(k) => {
                let (res_re, res_im) = Self::eval_single_sheet(kind, re, im, *k, None)?;
                Ok(MultiValuedResult::Single {
                    re: res_re,
                    im: res_im,
                    sheet: *k,
                })
            }
            BranchSelection::CustomCut { geometry, sheet } => {
                let (res_re, res_im) =
                    Self::eval_single_sheet(kind, re, im, *sheet, Some(geometry))?;
                Ok(MultiValuedResult::Single {
                    re: res_re,
                    im: res_im,
                    sheet: *sheet,
                })
            }
            BranchSelection::MultiValued { sheets } => {
                let mut results = Vec::with_capacity(sheets.len());
                for &k in sheets {
                    let (res_re, res_im) = Self::eval_single_sheet(kind, re, im, k, None)?;
                    results.push((res_re, res_im, k));
                }
                Ok(MultiValuedResult::Set(results))
            }
            BranchSelection::AllSheets { max_branches } => {
                let n = match kind {
                    BranchCutFunctionKind::Sqrt => 2,
                    BranchCutFunctionKind::Root(deg) => deg as usize,
                    BranchCutFunctionKind::PowerWithExponent {
                        exp_re_bits,
                        exp_im_bits,
                    } => {
                        let exp_re = f64::from_bits(exp_re_bits);
                        let exp_im = f64::from_bits(exp_im_bits);
                        if exp_im.abs() < 1e-12 {
                            infer_rational_denom(exp_re, *max_branches)
                        } else {
                            *max_branches
                        }
                    }
                    _ => *max_branches,
                };
                let mut results = Vec::with_capacity(n);
                for k in 0..(n as i64) {
                    let (res_re, res_im) = Self::eval_single_sheet(kind, re, im, k, None)?;
                    results.push((res_re, res_im, k));
                }
                Ok(MultiValuedResult::Set(results))
            }
            BranchSelection::Unspecified => {
                // Default to principal with sheet index 0
                let (res_re, res_im) = Self::eval_single_sheet(kind, re, im, 0, None)?;
                Ok(MultiValuedResult::Single {
                    re: res_re,
                    im: res_im,
                    sheet: 0,
                })
            }
        }
    }

    /// Evaluates complex power $z^w = \exp(w \cdot \ln_k(z))$ under the given `BranchSelection`.
    pub fn eval_power(
        base_re: f64,
        base_im: f64,
        exp_re: f64,
        exp_im: f64,
        selection: &BranchSelection,
    ) -> AlgebraResult<MultiValuedResult> {
        let r = base_re.hypot(base_im);
        if r < 1e-15 {
            if exp_re.abs() < 1e-15 && exp_im.abs() < 1e-15 {
                return Ok(MultiValuedResult::Single {
                    re: 1.0,
                    im: 0.0,
                    sheet: 0,
                });
            }
            if exp_re > 0.0 && exp_im.abs() < 1e-15 {
                return Ok(MultiValuedResult::Single {
                    re: 0.0,
                    im: 0.0,
                    sheet: 0,
                });
            }
            return Err(AlgebraError::SingularityEncountered(
                "Zero base raised to non-positive or complex exponent".into(),
            ));
        }
        Self::eval(
            BranchCutFunctionKind::PowerWithExponent {
                exp_re_bits: exp_re.to_bits(),
                exp_im_bits: exp_im.to_bits(),
            },
            base_re,
            base_im,
            selection,
        )
    }

    /// Evaluates on a single specified sheet $k \in \mathbb{Z}$.
    fn eval_single_sheet(
        kind: BranchCutFunctionKind,
        re: f64,
        im: f64,
        k: i64,
        custom_cut: Option<&BranchCutGeometry>,
    ) -> AlgebraResult<(f64, f64)> {
        let r = re.hypot(im);
        if r < 1e-15 {
            match kind {
                BranchCutFunctionKind::Log | BranchCutFunctionKind::Power => {
                    return Err(AlgebraError::SingularityEncountered(
                        "Essential logarithmic branch point at z = 0".into(),
                    ));
                }
                BranchCutFunctionKind::PowerWithExponent {
                    exp_re_bits,
                    exp_im_bits,
                } => {
                    let ere = f64::from_bits(exp_re_bits);
                    let eim = f64::from_bits(exp_im_bits);
                    if ere.abs() < 1e-15 && eim.abs() < 1e-15 {
                        return Ok((1.0, 0.0));
                    } else if ere > 0.0 && eim.abs() < 1e-15 {
                        return Ok((0.0, 0.0));
                    } else {
                        return Err(AlgebraError::SingularityEncountered(
                            "Zero base raised to non-positive or complex exponent".into(),
                        ));
                    }
                }
                BranchCutFunctionKind::Sqrt | BranchCutFunctionKind::Root(_) => {
                    return Ok((0.0, 0.0));
                }
                BranchCutFunctionKind::LambertW => {
                    if k == 0 {
                        return Ok((0.0, 0.0));
                    }
                    return Err(AlgebraError::SingularityEncountered(
                        "Non-principal Lambert W_k(0) diverges to infinity".into(),
                    ));
                }
                _ => {}
            }
        }

        // Standard polar angle theta in (-pi, pi]
        let mut theta = im.atan2(re);

        // Adjust theta if custom cut ray angle is provided
        if let Some(BranchCutGeometry::Ray { angle_rad, .. }) = custom_cut {
            // Shift branch cut from pi to angle_rad
            let shift = angle_rad - PI;
            let mut shifted_theta = (theta - shift) % (2.0 * PI);
            if shifted_theta <= -PI {
                shifted_theta += 2.0 * PI;
            } else if shifted_theta > PI {
                shifted_theta -= 2.0 * PI;
            }
            theta = shifted_theta + shift;
        }

        match kind {
            BranchCutFunctionKind::Log => {
                // ln_k(z) = ln|z| + i(theta + 2*pi*k)
                let log_r = r.ln();
                let arg_k = theta + 2.0 * PI * (k as f64);
                Ok((log_r, arg_k))
            }
            BranchCutFunctionKind::Sqrt => {
                // sqrt_k(z) = |z|^(1/2) * e^(i(theta + 2*pi*k)/2)
                let sqrt_r = r.sqrt();
                let phi_k = (theta + 2.0 * PI * (k as f64)) / 2.0;
                Ok((sqrt_r * phi_k.cos(), sqrt_r * phi_k.sin()))
            }
            BranchCutFunctionKind::Root(n) => {
                if n == 0 {
                    return Err(AlgebraError::DivisionByZero {
                        domain: "Complex Root".into(),
                    });
                }
                let root_r = r.powf(1.0 / (n as f64));
                let phi_k = (theta + 2.0 * PI * (k as f64)) / (n as f64);
                Ok((root_r * phi_k.cos(), root_r * phi_k.sin()))
            }
            BranchCutFunctionKind::Power => {
                // Default z^alpha for alpha = 2.0: exp(2.0 * ln_k(z))
                let log_r = r.ln();
                let arg_k = theta + 2.0 * PI * (k as f64);
                let exp_re = (2.0 * log_r).exp() * (2.0 * arg_k).cos();
                let exp_im = (2.0 * log_r).exp() * (2.0 * arg_k).sin();
                Ok((exp_re, exp_im))
            }
            BranchCutFunctionKind::PowerWithExponent {
                exp_re_bits,
                exp_im_bits,
            } => {
                let alpha_re = f64::from_bits(exp_re_bits);
                let alpha_im = f64::from_bits(exp_im_bits);
                let log_r = r.ln();
                let arg_k = theta + 2.0 * PI * (k as f64);
                // w = alpha * ln_k(z) = (alpha_re*log_r - alpha_im*arg_k) + i*(alpha_re*arg_k + alpha_im*log_r)
                let w_re = alpha_re * log_r - alpha_im * arg_k;
                let w_im = alpha_re * arg_k + alpha_im * log_r;
                let mag = w_re.exp();
                Ok((mag * w_im.cos(), mag * w_im.sin()))
            }
            BranchCutFunctionKind::ArcSin => {
                // Principal arcsin_0(z) = -i * ln(i*z + sqrt(1 - z^2))
                // All Riemann sheets: w_k = (-1)^k * w_0 + pi * k
                let one_minus_z2_re = 1.0 - re * re + im * im;
                let one_minus_z2_im = -2.0 * re * im;
                let (sq_re, sq_im) = Self::eval_single_sheet(
                    BranchCutFunctionKind::Sqrt,
                    one_minus_z2_re,
                    one_minus_z2_im,
                    0,
                    None,
                )?;

                let arg_log_re = -im + sq_re;
                let arg_log_im = re + sq_im;
                let (log_re, log_im) = Self::eval_single_sheet(
                    BranchCutFunctionKind::Log,
                    arg_log_re,
                    arg_log_im,
                    0,
                    None,
                )?;
                let (w0_re, w0_im) = (log_im, -log_re);
                let sign = if k.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
                Ok((sign * w0_re + PI * (k as f64), sign * w0_im))
            }
            BranchCutFunctionKind::ArcCos => {
                // Principal arccos_0(z) = pi/2 - arcsin_0(z)
                // All Riemann sheets: even k -> w0 + 2*pi*(k/2), odd k -> -w0 + 2*pi*((k+1)/2)
                let (asin_re, asin_im) =
                    Self::eval_single_sheet(BranchCutFunctionKind::ArcSin, re, im, 0, custom_cut)?;
                let (w0_re, w0_im) = (PI / 2.0 - asin_re, -asin_im);
                if k.rem_euclid(2) == 0 {
                    let m = k.div_euclid(2);
                    Ok((w0_re + 2.0 * PI * (m as f64), w0_im))
                } else {
                    let m = (k + 1).div_euclid(2);
                    Ok((-w0_re + 2.0 * PI * (m as f64), -w0_im))
                }
            }
            BranchCutFunctionKind::ArcTan => {
                // arctan(z) = i/2 * (ln(1 - iz) - ln(1 + iz)) + pi*k
                // 1 - iz = (1 + im) - i*re
                // 1 + iz = (1 - im) + i*re
                let (log1_re, log1_im) =
                    Self::eval_single_sheet(BranchCutFunctionKind::Log, 1.0 + im, -re, 0, None)?;
                let (log2_re, log2_im) =
                    Self::eval_single_sheet(BranchCutFunctionKind::Log, 1.0 - im, re, 0, None)?;

                // i/2 * ( (log1_re - log2_re) + i(log1_im - log2_im) )
                let diff_re = log1_re - log2_re;
                let diff_im = log1_im - log2_im;
                let res_re = -0.5 * diff_im + PI * (k as f64);
                let res_im = 0.5 * diff_re;
                Ok((res_re, res_im))
            }
            BranchCutFunctionKind::ArcSinh => {
                // arcsinh(z) = ln(z + sqrt(1 + z^2)) + 2*pi*i*k
                let one_plus_z2_re = 1.0 + re * re - im * im;
                let one_plus_z2_im = 2.0 * re * im;
                let (sq_re, sq_im) = Self::eval_single_sheet(
                    BranchCutFunctionKind::Sqrt,
                    one_plus_z2_re,
                    one_plus_z2_im,
                    0,
                    None,
                )?;

                let (log_re, log_im) = Self::eval_single_sheet(
                    BranchCutFunctionKind::Log,
                    re + sq_re,
                    im + sq_im,
                    k,
                    None,
                )?;
                Ok((log_re, log_im))
            }
            BranchCutFunctionKind::ArcCosh => {
                // arccosh(z) = ln(z + sqrt(z - 1)*sqrt(z + 1))
                let (sq1_re, sq1_im) =
                    Self::eval_single_sheet(BranchCutFunctionKind::Sqrt, re - 1.0, im, 0, None)?;
                let (sq2_re, sq2_im) =
                    Self::eval_single_sheet(BranchCutFunctionKind::Sqrt, re + 1.0, im, 0, None)?;
                let prod_re = sq1_re * sq2_re - sq1_im * sq2_im;
                let prod_im = sq1_re * sq2_im + sq1_im * sq2_re;

                let (log_re, log_im) = Self::eval_single_sheet(
                    BranchCutFunctionKind::Log,
                    re + prod_re,
                    im + prod_im,
                    k,
                    None,
                )?;
                Ok((log_re, log_im))
            }
            BranchCutFunctionKind::ArcTanh => {
                // arctanh(z) = 1/2 * (ln(1 + z) - ln(1 - z)) + i*pi*k
                let (log1_re, log1_im) =
                    Self::eval_single_sheet(BranchCutFunctionKind::Log, 1.0 + re, im, 0, None)?;
                let (log2_re, log2_im) =
                    Self::eval_single_sheet(BranchCutFunctionKind::Log, 1.0 - re, -im, 0, None)?;
                let res_re = 0.5 * (log1_re - log2_re);
                let res_im = 0.5 * (log1_im - log2_im) + PI * (k as f64);
                Ok((res_re, res_im))
            }
            BranchCutFunctionKind::LambertW => {
                // High-precision Halley iteration for W_k(z) using Corless et al. (1996) initial approximations
                let inv_e = -(-1.0f64).exp(); // -1/e
                let (mut w_re, mut w_im) = if im.abs() < 1e-14 && re >= inv_e && re < 0.0 && k == 0
                {
                    // Principal real branch on [-1/e, 0)
                    let p = (2.0 * (std::f64::consts::E * re + 1.0)).max(0.0).sqrt();
                    let w0 = if re < -0.3 {
                        -1.0 + p - p * p / 3.0
                    } else {
                        re
                    };
                    (w0, 0.0)
                } else if im.abs() < 1e-14 && re >= inv_e && re < 0.0 && k == -1 {
                    // Secondary real branch W_{-1} on [-1/e, 0)
                    let l1 = (-re).ln();
                    let l2 = (-l1).max(1e-6).ln();
                    (l1 - l2, 0.0)
                } else if k == 0 && r < 1.0 {
                    (re, im)
                } else {
                    // General Corless asymptotic initial guess: L1 - L2 + L2/L1 where L1 = ln_k(z)
                    let l1_re = r.ln();
                    let l1_im = theta + 2.0 * PI * (k as f64);
                    let l1_mag = l1_re.hypot(l1_im).max(1e-12);
                    let l2_re = l1_mag.ln();
                    let l2_im = l1_im.atan2(l1_re);
                    let denom = l1_re * l1_re + l1_im * l1_im;
                    let ratio_re = (l2_re * l1_re + l2_im * l1_im) / denom;
                    let ratio_im = (l2_im * l1_re - l2_re * l1_im) / denom;
                    (l1_re - l2_re + ratio_re, l1_im - l2_im + ratio_im)
                };

                // Halley's method for f(w) = w e^w - z = 0:
                // w_{n+1} = w_n - f / (e^w (w+1) - (w+2)*f / (2*(w+1)))
                for _ in 0..50 {
                    let mag_e = w_re.exp();
                    let ew_re = mag_e * w_im.cos();
                    let ew_im = mag_e * w_im.sin();

                    let f_re = (w_re * ew_re - w_im * ew_im) - re;
                    let f_im = (w_re * ew_im + w_im * ew_re) - im;

                    let wp1_re = w_re + 1.0;
                    let wp1_im = w_im;
                    let fp_re = ew_re * wp1_re - ew_im * wp1_im;
                    let fp_im = ew_re * wp1_im + ew_im * wp1_re;

                    // f''(w) = e^w * (w + 2)
                    let wp2_re = w_re + 2.0;
                    let wp2_im = w_im;
                    let fpp_re = ew_re * wp2_re - ew_im * wp2_im;
                    let fpp_im = ew_re * wp2_im + ew_im * wp2_re;

                    // denom = fp - f * fpp / (2 * fp)
                    let fp2 = fp_re * fp_re + fp_im * fp_im;
                    if fp2 < 1e-30 {
                        break;
                    }
                    // q = f / fp
                    let q_re = (f_re * fp_re + f_im * fp_im) / fp2;
                    let q_im = (f_im * fp_re - f_re * fp_im) / fp2;
                    // fpp / (2 * fp)
                    let r_re = 0.5 * (fpp_re * fp_re + fpp_im * fp_im) / fp2;
                    let r_im = 0.5 * (fpp_im * fp_re - fpp_re * fp_im) / fp2;
                    // halley_den = 1 - q * r
                    let hden_re = 1.0 - (q_re * r_re - q_im * r_im);
                    let hden_im = -(q_re * r_im + q_im * r_re);
                    let hden2 = hden_re * hden_re + hden_im * hden_im;
                    if hden2 < 1e-30 {
                        w_re -= q_re;
                        w_im -= q_im;
                        continue;
                    }
                    let step_re = (q_re * hden_re + q_im * hden_im) / hden2;
                    let step_im = (q_im * hden_re - q_re * hden_im) / hden2;
                    w_re -= step_re;
                    w_im -= step_im;

                    if step_re.hypot(step_im) < 1e-14 {
                        break;
                    }
                }

                Ok((w_re, w_im))
            }
        }
    }

    /// Evaluates the jump discontinuity across a branch cut: $\Delta f(z) = \lim_{\epsilon \to 0^+} [f(z + i\epsilon) - f(z - i\epsilon)]$.
    pub fn eval_jump(
        kind: BranchCutFunctionKind,
        cut_re: f64,
        cut_im: f64,
    ) -> AlgebraResult<(f64, f64)> {
        let eps = 1e-8;
        // Upper side z + i*eps (normal to cut)
        let (val_pos_re, val_pos_im) =
            Self::eval_single_sheet(kind, cut_re, cut_im + eps, 0, None)?;
        // Lower side z - i*eps
        let (val_neg_re, val_neg_im) =
            Self::eval_single_sheet(kind, cut_re, cut_im - eps, 0, None)?;

        Ok((val_pos_re - val_neg_re, val_pos_im - val_neg_im))
    }

    /// Performs analytic continuation along a discretized path $\gamma(t) = (x(t), y(t))$,
    /// tracking winding numbers around branch points and returning the resulting value and final Riemann sheet index.
    pub fn analytic_continuation(
        kind: BranchCutFunctionKind,
        path: &[(f64, f64)],
        start_sheet: i64,
    ) -> AlgebraResult<(f64, f64, i64)> {
        if path.is_empty() {
            return Err(AlgebraError::EvaluationError("Path cannot be empty".into()));
        }

        let mut current_sheet = start_sheet;
        let mut prev_theta = path[0].1.atan2(path[0].0);

        for pt in path.iter().skip(1) {
            let cur_theta = pt.1.atan2(pt.0);
            let d_theta = cur_theta - prev_theta;

            // Detect crossing of negative real axis (-pi / +pi jump)
            if d_theta < -PI {
                // Counter-clockwise crossing from -pi to +pi -> sheet + 1
                current_sheet += 1;
            } else if d_theta > PI {
                // Clockwise crossing from +pi to -pi -> sheet - 1
                current_sheet -= 1;
            }
            prev_theta = cur_theta;
        }

        let last_pt = match path.last() {
            Some(pt) => pt,
            None => return Err(AlgebraError::EvaluationError("Path cannot be empty".into())),
        };
        let (final_re, final_im) =
            Self::eval_single_sheet(kind, last_pt.0, last_pt.1, current_sheet, None)?;
        Ok((final_re, final_im, current_sheet))
    }
}

/// Infer small denominator `q` if `x` is close to a rational `p/q`, capped by `max_branches`.
fn infer_rational_denom(x: f64, max_branches: usize) -> usize {
    let cap = max_branches.clamp(1, 64);
    for q in 1..=cap {
        let scaled = x * (q as f64);
        if (scaled - scaled.round()).abs() < 1e-9 {
            return q;
        }
    }
    cap
}
