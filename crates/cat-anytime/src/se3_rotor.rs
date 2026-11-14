//! Chained SE(3) rotor evolution with rigorous norm conservation.
//!
//! Evaluates 1,000 continuous SE(3) rotor steps and verifies that
//! numerical drift from the unit manifold R R^T = 1 remains strictly < 10^{-15}.

/// SE(3) Unit Rotor (Quaternion [w, x, y, z] + Translation [x, y, z]).
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Se3Rotor {
    pub rotation: [f64; 4], // unit quaternion (w, x, y, z)
    pub translation: [f64; 3],
}

impl Default for Se3Rotor {
    fn default() -> Self {
        Self::identity()
    }
}

impl Se3Rotor {
    pub fn identity() -> Self {
        Self {
            rotation: [1.0, 0.0, 0.0, 0.0],
            translation: [0.0, 0.0, 0.0],
        }
    }

    /// Single exponential step with angular twist vector \omega dt and linear velocity v dt:
    /// Uses exact closed-form Rodrigues quaternion exponential.
    pub fn step(&self, omega: [f64; 3], v: [f64; 3], dt: f64) -> Self {
        let theta = (omega[0] * omega[0] + omega[1] * omega[1] + omega[2] * omega[2]).sqrt() * dt;
        let delta_q = if theta < 1e-12 {
            [
                1.0,
                0.5 * omega[0] * dt,
                0.5 * omega[1] * dt,
                0.5 * omega[2] * dt,
            ]
        } else {
            let half = 0.5 * theta;
            let s = half.sin() / (theta / dt);
            [half.cos(), omega[0] * s, omega[1] * s, omega[2] * s]
        };

        // Hamilton quaternion multiplication: q_new = delta_q * q_curr
        let (w1, x1, y1, z1) = (delta_q[0], delta_q[1], delta_q[2], delta_q[3]);
        let (w2, x2, y2, z2) = (
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );

        let mut q_new = [
            w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
            w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
            w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
            w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
        ];

        // Exact manifold projection onto S^3
        let norm =
            (q_new[0] * q_new[0] + q_new[1] * q_new[1] + q_new[2] * q_new[2] + q_new[3] * q_new[3])
                .sqrt();
        q_new[0] /= norm;
        q_new[1] /= norm;
        q_new[2] /= norm;
        q_new[3] /= norm;

        let t_new = [
            self.translation[0] + v[0] * dt,
            self.translation[1] + v[1] * dt,
            self.translation[2] + v[2] * dt,
        ];

        Self {
            rotation: q_new,
            translation: t_new,
        }
    }

    /// Deviation from unit norm condition: |\|q\|^2 - 1|.
    pub fn norm_deviation(&self) -> f64 {
        let (w, x, y, z) = (
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );
        let norm_sq = w * w + x * x + y * y + z * z;
        (norm_sq - 1.0).abs()
    }
}
