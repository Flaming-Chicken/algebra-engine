//! Rigid 3D SE(3) pose (`Pose<T>`) with URScript scaled axis-angle conventions.
//!
//! Salvaged from archived `URScript` (`src/vars/pose.rs`) and implemented over generic
//! `num_traits::Float` + unit quaternion representation without requiring external linear
//! algebra crates.

use core::fmt;
use core::ops::{Div, DivAssign, Mul, MulAssign};

use num_traits::Float;

/// Rigid 3D isometry (translation + unit-quaternion orientation) using URScript
/// scaled axis-angle coordinates `p[x, y, z, ax, ay, az]`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Pose<T>
where
    T: Float,
{
    translation: [T; 3],
    /// Unit quaternion stored as `[w, x, y, z]`.
    rotation: [T; 4],
}

impl<T> Default for Pose<T>
where
    T: Float,
{
    fn default() -> Self {
        Self::identity()
    }
}

impl<T> Pose<T>
where
    T: Float,
{
    /// Returns the identity SE(3) pose with zero translation and identity rotation.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            translation: [T::zero(), T::zero(), T::zero()],
            rotation: [T::one(), T::zero(), T::zero(), T::zero()],
        }
    }

    /// Creates a `Pose` from translation `(x, y, z)` and scaled axis-angle `(ax, ay, az)`.
    #[must_use]
    pub fn new_pose(x: T, y: T, z: T, ax: T, ay: T, az: T) -> Self {
        Self {
            translation: [x, y, z],
            rotation: quat_from_scaled_axis(ax, ay, az),
        }
    }

    /// Creates a pure translation `Pose` with identity orientation.
    #[must_use]
    pub fn new_pos(x: T, y: T, z: T) -> Self {
        Self {
            translation: [x, y, z],
            rotation: [T::one(), T::zero(), T::zero(), T::zero()],
        }
    }

    /// Creates a pure rotation `Pose` with zero translation from scaled axis-angle `(ax, ay, az)`.
    #[must_use]
    pub fn new_rot(ax: T, ay: T, az: T) -> Self {
        Self {
            translation: [T::zero(), T::zero(), T::zero()],
            rotation: quat_from_scaled_axis(ax, ay, az),
        }
    }

    /// Returns the translation vector `[x, y, z]`.
    #[must_use]
    pub fn translation(&self) -> [T; 3] {
        self.translation
    }

    /// Returns the orientation as a unit quaternion `[w, x, y, z]`.
    #[must_use]
    pub fn rotation_quaternion(&self) -> [T; 4] {
        self.rotation
    }

    /// Returns the orientation in scaled axis-angle form `[ax, ay, az]` (radians).
    #[must_use]
    pub fn scaled_axis(&self) -> [T; 3] {
        scaled_axis_from_quat(self.rotation)
    }

    /// Returns the inverse rigid transformation `self^{-1}`.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let r_inv = [
            self.rotation[0],
            -self.rotation[1],
            -self.rotation[2],
            -self.rotation[3],
        ];
        let rot_t = rotate_vec(r_inv, self.translation);
        Self {
            translation: [-rot_t[0], -rot_t[1], -rot_t[2]],
            rotation: r_inv,
        }
    }
}

fn quat_from_scaled_axis<T: Float>(ax: T, ay: T, az: T) -> [T; 4] {
    let half = T::one() / (T::one() + T::one());
    let theta_sq = ax * ax + ay * ay + az * az;
    let theta = theta_sq.sqrt();
    if theta <= T::epsilon() {
        let forty_eight = T::from(48.0).unwrap_or_else(T::one);
        let eight = T::from(8.0).unwrap_or_else(T::one);
        let scale = half - theta_sq / forty_eight;
        let w = T::one() - theta_sq / eight;
        return normalize_quat([w, ax * scale, ay * scale, az * scale]);
    }
    let half_angle = theta * half;
    let scale = half_angle.sin() / theta;
    [half_angle.cos(), ax * scale, ay * scale, az * scale]
}

fn scaled_axis_from_quat<T: Float>(q: [T; 4]) -> [T; 3] {
    let [mut w, mut x, mut y, mut z] = normalize_quat(q);
    if w < T::zero() {
        w = -w;
        x = -x;
        y = -y;
        z = -z;
    }
    let two = T::one() + T::one();
    let v_norm = (x * x + y * y + z * z).sqrt();
    if v_norm <= T::epsilon() {
        return [x * two, y * two, z * two];
    }
    let angle = two * v_norm.atan2(w);
    let scale = angle / v_norm;
    [x * scale, y * scale, z * scale]
}

fn normalize_quat<T: Float>(q: [T; 4]) -> [T; 4] {
    let norm_sq = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
    if norm_sq <= T::zero() {
        return [T::one(), T::zero(), T::zero(), T::zero()];
    }
    let drift = (norm_sq - T::one()).abs();
    let tol = T::from(1e-6).unwrap_or_else(T::epsilon);
    if drift <= tol {
        return q;
    }
    let inv = T::one() / norm_sq.sqrt();
    [q[0] * inv, q[1] * inv, q[2] * inv, q[3] * inv]
}

fn mul_quat<T: Float>(a: [T; 4], b: [T; 4]) -> [T; 4] {
    normalize_quat([
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ])
}

fn rotate_vec<T: Float>(q: [T; 4], v: [T; 3]) -> [T; 3] {
    let two = T::one() + T::one();
    let u = [q[1], q[2], q[3]];
    let w = q[0];
    let c1 = cross(u, v);
    // Standard Rodrigues quaternion vector rotation: v + 2 * (u x (u x v + w * v))
    let inner = [
        c1[0] + w * v[0],
        c1[1] + w * v[1],
        c1[2] + w * v[2],
    ];
    let c2 = cross(u, inner);
    [
        v[0] + two * c2[0],
        v[1] + two * c2[1],
        v[2] + two * c2[2],
    ]
}

fn cross<T: Float>(a: [T; 3], b: [T; 3]) -> [T; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

impl<T> fmt::Display for Pose<T>
where
    T: Float,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = self.scaled_axis();
        let x = self.translation[0].to_f64().unwrap_or(f64::NAN);
        let y = self.translation[1].to_f64().unwrap_or(f64::NAN);
        let z = self.translation[2].to_f64().unwrap_or(f64::NAN);
        let ax = r[0].to_f64().unwrap_or(f64::NAN);
        let ay = r[1].to_f64().unwrap_or(f64::NAN);
        let az = r[2].to_f64().unwrap_or(f64::NAN);
        write!(f, "p[{x},{y},{z},{ax},{ay},{az}]")
    }
}

impl<T> Div for Pose<T>
where
    T: Float,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl<T> DivAssign for Pose<T>
where
    T: Float,
{
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<T> Mul for Pose<T>
where
    T: Float,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let rot_rhs_t = rotate_vec(self.rotation, rhs.translation);
        Self {
            translation: [
                self.translation[0] + rot_rhs_t[0],
                self.translation[1] + rot_rhs_t[1],
                self.translation[2] + rot_rhs_t[2],
            ],
            rotation: mul_quat(self.rotation, rhs.rotation),
        }
    }
}

impl<T> MulAssign for Pose<T>
where
    T: Float,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul() {
        use core::f32::consts::PI;

        let h1 = Pose::new_pose(0., 1., 2., 0., PI, 0.);
        let h2 = Pose::new_pos(0., 2., 4.);
        let h3 = Pose::new_pose(0., 3., 6., 0., PI, 0.);

        assert_eq!(h3, h2 * h1);
    }

    #[test]
    fn inverse_and_div() {
        let a = Pose::new_pose(1.0_f64, 2.0, -3.0, 0.2, -0.4, 0.6);
        let b = Pose::new_pose(-0.5_f64, 1.5, 0.25, 0.5, 0.1, -0.3);
        let composed = a * b;
        let recovered = composed / b;
        for (got, want) in recovered.translation().iter().zip(a.translation().iter()) {
            assert!((got - want).abs() < 1e-12);
        }
        let mut prod = a * b;
        prod /= b;
        for (got, want) in prod.scaled_axis().iter().zip(a.scaled_axis().iter()) {
            assert!((got - want).abs() < 1e-12);
        }
    }

    #[test]
    fn display_format() {
        let p = Pose::new_pos(1.5_f32, -2.0, 3.0);
        assert_eq!(format!("{p}"), "p[1.5,-2,3,0,0,0]");
    }
}
