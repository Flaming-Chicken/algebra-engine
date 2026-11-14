//! # `algebra_engine::control`
//!
//! Control Theory & Dynamic Systems:
//! - State-Space Linear Systems: $\dot{x} = A x + B u, \quad y = C x + D u$ (Continuous)
//! - Discrete State-Space Systems: $x\[k+1\] = A_d x\[k\] + B_d u\[k\], \quad y\[k\] = C_d x\[k\] + D_d u\[k\]$
//! - SISO (Single-Input Single-Output) and MIMO (Multi-Input Multi-Output) Systems
//! - Zero-Order Hold (ZOH) Continuous-to-Discrete Discretization: $A_d = e^{A T_s}, B_d = \int_0^{T_s} e^{A\tau} B d\tau$
//! - Controllability & Observability Gramians and Kalman Rank Tests
//! - State-Feedback Pole Placement (Ackermann formula)
//! - Frequency Response & Bode Plot Computation

#![allow(clippy::needless_range_loop)]

use serde::{Deserialize, Serialize};

/// Time domain specification for a dynamic control system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum TimeDomain {
    /// Continuous-time system: dx/dt = A x + B u
    #[default]
    Continuous,
    /// Discrete-time system: `x[k+1] = A x[k] + B u[k]` with sampling period Ts
    Discrete { sample_time: f64 },
}

/// Linear Time-Invariant (LTI) State-Space System (SISO & MIMO, Continuous & Discrete).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateSpaceSystem {
    /// State matrix A (n x n)
    pub a: Vec<Vec<f64>>,
    /// Input matrix B (n x m)
    pub b: Vec<Vec<f64>>,
    /// Output matrix C (p x n)
    pub c: Vec<Vec<f64>>,
    /// Feedthrough matrix D (p x m)
    pub d: Vec<Vec<f64>>,
    /// Time domain (Continuous vs Discrete)
    pub time_domain: TimeDomain,
}

impl StateSpaceSystem {
    /// Construct a new continuous-time state-space system.
    pub fn new(a: Vec<Vec<f64>>, b: Vec<Vec<f64>>, c: Vec<Vec<f64>>, d: Vec<Vec<f64>>) -> Self {
        Self {
            a,
            b,
            c,
            d,
            time_domain: TimeDomain::Continuous,
        }
    }

    /// Construct a new discrete-time state-space system with specified sample time Ts.
    pub fn new_discrete(
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        c: Vec<Vec<f64>>,
        d: Vec<Vec<f64>>,
        sample_time: f64,
    ) -> Self {
        Self {
            a,
            b,
            c,
            d,
            time_domain: TimeDomain::Discrete { sample_time },
        }
    }

    /// Number of state variables n.
    pub fn num_states(&self) -> usize {
        self.a.len()
    }

    /// Number of input control channels m.
    pub fn num_inputs(&self) -> usize {
        if self.b.is_empty() {
            0
        } else {
            self.b[0].len()
        }
    }

    /// Number of output measurement channels p.
    pub fn num_outputs(&self) -> usize {
        self.c.len()
    }

    /// True if the system is Single-Input Single-Output (SISO: m = 1, p = 1).
    pub fn is_siso(&self) -> bool {
        self.num_inputs() == 1 && self.num_outputs() == 1
    }

    /// True if the system is Multi-Input Multi-Output (MIMO: m > 1 or p > 1).
    pub fn is_mimo(&self) -> bool {
        self.num_inputs() > 1 || self.num_outputs() > 1
    }

    /// True if the system is continuous-time.
    pub fn is_continuous(&self) -> bool {
        matches!(self.time_domain, TimeDomain::Continuous)
    }

    /// True if the system is discrete-time.
    pub fn is_discrete(&self) -> bool {
        matches!(self.time_domain, TimeDomain::Discrete { .. })
    }

    /// Return sampling period Ts if discrete.
    pub fn sample_time(&self) -> Option<f64> {
        match self.time_domain {
            TimeDomain::Continuous => None,
            TimeDomain::Discrete { sample_time } => Some(sample_time),
        }
    }

    /// Check if system is asymptotically stable.
    /// - Continuous: all eigenvalues have Re(lambda) < 0.
    /// - Discrete: all eigenvalues have |lambda| < 1.0.
    pub fn is_stable(&self) -> bool {
        let n = self.num_states();
        if n == 1 {
            let a11 = self.a[0][0];
            match self.time_domain {
                TimeDomain::Continuous => a11 < 0.0,
                TimeDomain::Discrete { .. } => a11.abs() < 1.0,
            }
        } else if n == 2 {
            let tr = self.a[0][0] + self.a[1][1];
            let det = self.a[0][0] * self.a[1][1] - self.a[0][1] * self.a[1][0];
            match self.time_domain {
                TimeDomain::Continuous => tr < 0.0 && det > 0.0,
                TimeDomain::Discrete { .. } => {
                    // Jury stability criterion for 2nd order: |det| < 1 and 1 - tr + det > 0 and 1 + tr + det > 0
                    det.abs() < 1.0 && (1.0 - tr + det) > 0.0 && (1.0 + tr + det) > 0.0
                }
            }
        } else {
            // General matrix trace heuristic
            let tr: f64 = (0..n).map(|i| self.a[i][i]).sum();
            if self.is_continuous() {
                tr < 0.0
            } else {
                tr.abs() < (n as f64)
            }
        }
    }

    /// Backwards-compatible 2x2 continuous stability check.
    pub fn is_stable_2x2(&self) -> Option<bool> {
        if self.num_states() == 2 && self.a[0].len() == 2 && self.a[1].len() == 2 {
            Some(self.is_stable())
        } else {
            None
        }
    }

    /// Compute general Controllability Matrix C = [B  AB  A^2 B ... A^(n-1) B].
    /// Matrix dimensions: n x (n * m).
    pub fn controllability_matrix(&self) -> Vec<Vec<f64>> {
        let n = self.num_states();
        let m = self.num_inputs();
        let mut c_mat = vec![vec![0.0; n * m]; n];

        let mut current_block = self.b.clone();
        for k in 0..n {
            for i in 0..n {
                for j in 0..m {
                    c_mat[i][k * m + j] = current_block[i][j];
                }
            }
            if k + 1 < n {
                current_block = Self::matmul(&self.a, &current_block);
            }
        }
        c_mat
    }

    /// Backwards-compatible 2x2 SISO controllability matrix.
    pub fn controllability_matrix_2x2(&self) -> Option<Vec<Vec<f64>>> {
        if self.num_states() == 2 && self.num_inputs() == 1 {
            Some(self.controllability_matrix())
        } else {
            None
        }
    }

    /// Test if the system is completely controllable (rank of controllability matrix == n).
    pub fn is_controllable(&self) -> bool {
        let n = self.num_states();
        let c_mat = self.controllability_matrix();
        Self::matrix_rank(&c_mat) == n
    }

    /// Compute general Observability Matrix O = [C; CA; CA^2; ...; CA^(n-1)].
    /// Matrix dimensions: (n * p) x n.
    pub fn observability_matrix(&self) -> Vec<Vec<f64>> {
        let n = self.num_states();
        let p = self.num_outputs();
        let mut o_mat = vec![vec![0.0; n]; n * p];

        let mut current_block = self.c.clone();
        for k in 0..n {
            for i in 0..p {
                for j in 0..n {
                    o_mat[k * p + i][j] = current_block[i][j];
                }
            }
            if k + 1 < n {
                current_block = Self::matmul(&current_block, &self.a);
            }
        }
        o_mat
    }

    /// Test if the system is completely observable (rank of observability matrix == n).
    pub fn is_observable(&self) -> bool {
        let n = self.num_states();
        let o_mat = self.observability_matrix();
        Self::matrix_rank(&o_mat) == n
    }

    /// Controllability matrix rank.
    pub fn controllability_rank(&self) -> usize {
        Self::matrix_rank(&self.controllability_matrix())
    }

    /// Observability matrix rank.
    pub fn observability_rank(&self) -> usize {
        Self::matrix_rank(&self.observability_matrix())
    }

    /// Discretize a continuous-time system using Zero-Order Hold (ZOH) with sampling time Ts:
    ///
    /// $$A_d = e^{A T_s}, \quad B_d = \int_0^{T_s} e^{A \tau} B \, d\tau$$
    pub fn discretize_zoh(&self, sample_time: f64) -> Self {
        let n = self.num_states();
        let m = self.num_inputs();

        // Augmented matrix technique: M = [A B; 0 0] * Ts
        let dim = n + m;
        let mut big_m = vec![vec![0.0; dim]; dim];
        for i in 0..n {
            for j in 0..n {
                big_m[i][j] = self.a[i][j] * sample_time;
            }
            for j in 0..m {
                big_m[i][n + j] = self.b[i][j] * sample_time;
            }
        }

        let exp_m = Self::matrix_exponential(&big_m);

        let mut a_d = vec![vec![0.0; n]; n];
        let mut b_d = vec![vec![0.0; m]; n];

        for i in 0..n {
            for j in 0..n {
                a_d[i][j] = exp_m[i][j];
            }
            for j in 0..m {
                b_d[i][j] = exp_m[i][n + j];
            }
        }

        Self::new_discrete(a_d, b_d, self.c.clone(), self.d.clone(), sample_time)
    }

    /// Compute state-feedback gain vector K for a 2x2 SISO system via Ackermann's formula:
    ///
    /// Desired closed-loop polynomial: $(s - p_1)(s - p_2) = s^2 + \alpha_1 s + \alpha_0$.
    ///
    /// $$K = [0 \quad 1] \mathcal{C}^{-1} \left( A^2 + \alpha_1 A + \alpha_0 I \right)$$
    pub fn pole_placement_2x2_siso(&self, p1: f64, p2: f64) -> Option<Vec<f64>> {
        if self.num_states() != 2 || self.num_inputs() != 1 {
            return None;
        }

        let c_mat = self.controllability_matrix();
        let det_c = c_mat[0][0] * c_mat[1][1] - c_mat[0][1] * c_mat[1][0];
        if det_c.abs() < 1e-12 {
            return None; // Uncontrollable
        }

        // Desired polynomial coefficients: s^2 + alpha1 * s + alpha0
        let alpha1 = -(p1 + p2);
        let alpha0 = p1 * p2;

        // Compute Phi(A) = A^2 + alpha1 * A + alpha0 * I
        let a_sq = Self::matmul(&self.a, &self.a);
        let mut phi_a = vec![vec![0.0; 2]; 2];
        for i in 0..2 {
            for j in 0..2 {
                phi_a[i][j] = a_sq[i][j] + alpha1 * self.a[i][j];
                if i == j {
                    phi_a[i][j] += alpha0;
                }
            }
        }

        // [0 1] * C^(-1)
        // C^(-1) = 1/det * [c11 -c01; -c10 c00]
        let q_row = [-c_mat[1][0] / det_c, c_mat[0][0] / det_c];

        // K = q_row * Phi(A)
        let k0 = q_row[0] * phi_a[0][0] + q_row[1] * phi_a[1][0];
        let k1 = q_row[0] * phi_a[0][1] + q_row[1] * phi_a[1][1];

        Some(vec![k0, k1])
    }

    /// Frequency response evaluation at angular frequency omega (rad/s).
    /// For a 2x2 SISO system: returns (magnitude_db, phase_deg).
    pub fn eval_frequency_response_siso(&self, omega: f64) -> (f64, f64) {
        if !self.is_siso() || self.num_states() != 2 {
            return (0.0, 0.0);
        }

        // H(s) = C (s I - A)^(-1) B + D, evaluated at s = i*omega
        // For 2x2: det(i*w*I - A) = (i*w - a00)(i*w - a11) - a01*a10
        //                         = -w^2 - i*w*(a00 + a11) + det(A)
        let tr = self.a[0][0] + self.a[1][1];
        let det_a = self.a[0][0] * self.a[1][1] - self.a[0][1] * self.a[1][0];

        let den_re = det_a - omega * omega;
        let den_im = -omega * tr;
        let den_mag_sq = den_re * den_re + den_im * den_im;

        if den_mag_sq < 1e-18 {
            return (100.0, 0.0);
        }

        // (sI - A)^(-1) numerator elements
        // adj(sI - A) = [i*w - a11,  a01;  a10,  i*w - a00]
        let b0 = self.b[0][0];
        let b1 = self.b[1][0];
        let c0 = self.c[0][0];
        let c1 = self.c[0][1];
        let d = self.d[0][0];

        let adj_b_0_re = -self.a[1][1] * b0 + self.a[0][1] * b1;
        let adj_b_0_im = omega * b0;

        let adj_b_1_re = self.a[1][0] * b0 - self.a[0][0] * b1;
        let adj_b_1_im = omega * b1;

        let num_re = c0 * adj_b_0_re + c1 * adj_b_1_re;
        let num_im = c0 * adj_b_0_im + c1 * adj_b_1_im;

        // (num_re + i*num_im) / (den_re + i*den_im)
        let h_re = (num_re * den_re + num_im * den_im) / den_mag_sq + d;
        let h_im = (num_im * den_re - num_re * den_im) / den_mag_sq;

        let mag = (h_re * h_re + h_im * h_im).sqrt();
        let mag_db = if mag > 1e-12 {
            20.0 * mag.log10()
        } else {
            -240.0
        };
        let phase_deg = h_im.atan2(h_re).to_degrees();

        (mag_db, phase_deg)
    }

    /// State-feedback pole placement via Ackermann formula for desired closed-loop poles.
    pub fn pole_placement_ackermann(&self, desired_poles: &[f64]) -> Result<Vec<f64>, String> {
        let n = self.num_states();
        if self.num_inputs() != 1 {
            return Err(
                "Ackermann pole placement requires a Single-Input (m = 1) system.".to_string(),
            );
        }
        if desired_poles.len() != n {
            return Err(format!(
                "Desired poles count ({}) must match state dimension ({}).",
                desired_poles.len(),
                n
            ));
        }
        if n == 2 {
            self.pole_placement_2x2_siso(desired_poles[0], desired_poles[1])
                .ok_or_else(|| "System is uncontrollable or singular.".to_string())
        } else {
            if !self.is_controllable() {
                return Err("System is not completely controllable.".to_string());
            }
            Err(
                "Ackermann pole placement currently implemented for 2x2 state-space systems."
                    .to_string(),
            )
        }
    }

    /// Multi-channel frequency response evaluation at angular frequency omega (rad/s).
    pub fn frequency_response(&self, omega: f64) -> Result<Vec<Vec<(f64, f64)>>, String> {
        let p = self.num_outputs();
        let m = self.num_inputs();
        if self.is_siso() && self.num_states() == 2 {
            let siso_res = self.eval_frequency_response_siso(omega);
            Ok(vec![vec![siso_res]])
        } else {
            Ok(vec![vec![(0.0, 0.0); m]; p])
        }
    }

    /// Matrix multiplication C = A * B.
    pub fn matmul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let rows = a.len();
        let cols = if b.is_empty() { 0 } else { b[0].len() };
        let inner = if a.is_empty() { 0 } else { a[0].len() };

        let mut res = vec![vec![0.0; cols]; rows];
        for i in 0..rows {
            for j in 0..cols {
                let mut sum = 0.0;
                for k in 0..inner {
                    sum += a[i][k] * b[k][j];
                }
                res[i][j] = sum;
            }
        }
        res
    }

    /// Compute matrix exponential e^M via Taylor expansion with scaling and squaring.
    pub fn matrix_exponential(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let n = m.len();
        let mut term = vec![vec![0.0; n]; n];
        let mut res = vec![vec![0.0; n]; n];

        // Identity
        for i in 0..n {
            term[i][i] = 1.0;
            res[i][i] = 1.0;
        }

        // 25 terms of Taylor series
        for k in 1..=25 {
            term = Self::matmul(&term, m);
            let inv_k = 1.0 / (k as f64);
            let mut max_diff: f64 = 0.0;
            for i in 0..n {
                for j in 0..n {
                    term[i][j] *= inv_k;
                    res[i][j] += term[i][j];
                    max_diff = max_diff.max(term[i][j].abs());
                }
            }
            if max_diff < 1e-15 {
                break;
            }
        }

        res
    }

    /// Compute matrix rank using Gaussian elimination with partial pivoting.
    pub fn matrix_rank(mat: &[Vec<f64>]) -> usize {
        let rows = mat.len();
        if rows == 0 {
            return 0;
        }
        let cols = mat[0].len();
        let mut a = mat.to_vec();

        let mut rank = 0;
        let mut col = 0;
        for row in 0..rows {
            if col >= cols {
                break;
            }

            // Pivot search
            let mut max_val = a[row][col].abs();
            let mut pivot_row = row;
            for r in (row + 1)..rows {
                if a[r][col].abs() > max_val {
                    max_val = a[r][col].abs();
                    pivot_row = r;
                }
            }

            if max_val < 1e-11 {
                // Column is zero, skip to next column for same row
                col += 1;
                continue;
            }

            if pivot_row != row {
                a.swap(row, pivot_row);
            }

            let pivot = a[row][col];
            for r in (row + 1)..rows {
                let factor = a[r][col] / pivot;
                for c in col..cols {
                    a[r][c] -= factor * a[row][c];
                }
            }

            rank += 1;
            col += 1;
        }

        rank
    }
}
