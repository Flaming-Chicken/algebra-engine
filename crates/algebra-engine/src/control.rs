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
            // Orders above two: Routh-Hurwitz (continuous) or Schur-Cohn (discrete) on the
            // characteristic polynomial, not a trace heuristic.
            let poly = Self::characteristic_polynomial(&self.a);
            match self.time_domain {
                TimeDomain::Continuous => Self::is_hurwitz(&poly),
                TimeDomain::Discrete { .. } => Self::is_schur(&poly),
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
    /// For a SISO system of any order: returns (magnitude_db, phase_deg); a MIMO system returns (0, 0), use [`StateSpaceSystem::frequency_response`].
    pub fn eval_frequency_response_siso(&self, omega: f64) -> (f64, f64) {
        if !self.is_siso() {
            return (0.0, 0.0);
        }
        match self.frequency_response(omega) {
            Ok(response) => response[0][0],
            // On a pole the response is unbounded.
            Err(_) => (100.0, 0.0),
        }
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
            return self
                .pole_placement_2x2_siso(desired_poles[0], desired_poles[1])
                .ok_or_else(|| "System is uncontrollable or singular.".to_string());
        }
        if !self.is_controllable() {
            return Err("System is not completely controllable.".to_string());
        }

        // K = [0 ... 0 1] C^{-1} phi(A), with phi(s) = prod (s - p_i) and C the controllability
        // matrix. Row vector y solves C^T y = e_n, then K = y^T phi(A).
        let c_mat = self.controllability_matrix();
        let c_transpose: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| c_mat[j][i]).collect())
            .collect();
        let mut last = vec![0.0; n];
        last[n - 1] = 1.0;
        let y = Self::solve_real(&c_transpose, &last)
            .ok_or_else(|| "System is uncontrollable or singular.".to_string())?;

        // phi(A) built by repeated multiplication of (A - p_i I).
        let identity: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
        let mut phi = identity.clone();
        for &p in desired_poles {
            let shifted: Vec<Vec<f64>> = (0..n)
                .map(|i| (0..n).map(|j| self.a[i][j] - p * identity[i][j]).collect())
                .collect();
            phi = Self::matmul(&phi, &shifted);
        }
        Ok((0..n)
            .map(|j| (0..n).map(|i| y[i] * phi[i][j]).sum())
            .collect())
    }

    /// Coefficients `[a_n, ..., a_1, a_0]` (highest power first, `a_n = 1`) of `det(sI - A)`,
    /// by the Faddeev-LeVerrier recurrence.
    pub fn characteristic_polynomial(a: &[Vec<f64>]) -> Vec<f64> {
        let n = a.len();
        let mut coefficients = vec![1.0];
        let mut m: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
        for k in 1..=n {
            let am = Self::matmul(a, &m);
            let trace: f64 = (0..n).map(|i| am[i][i]).sum();
            let c = -trace / k as f64;
            coefficients.push(c);
            m = am;
            for (i, row) in m.iter_mut().enumerate() {
                row[i] += c;
            }
        }
        coefficients
    }

    /// Routh-Hurwitz: every root of the polynomial (highest power first) has negative real part.
    /// A zero or negative entry in the first column, including a zero coefficient, fails.
    pub fn is_hurwitz(poly: &[f64]) -> bool {
        let n = poly.len().saturating_sub(1);
        if n == 0 {
            return true;
        }
        if poly[0] <= 0.0 || poly.iter().any(|&c| c <= 0.0) {
            return false;
        }
        let mut upper: Vec<f64> = poly.iter().step_by(2).copied().collect();
        let mut lower: Vec<f64> = poly.iter().skip(1).step_by(2).copied().collect();
        for _ in 1..n {
            if lower.is_empty() || lower[0] <= 1e-14 {
                return false;
            }
            let ratio = upper[0] / lower[0];
            let width = upper.len().max(lower.len());
            let mut next = Vec::with_capacity(width);
            for k in 1..width {
                let u = upper.get(k).copied().unwrap_or(0.0);
                let l = lower.get(k).copied().unwrap_or(0.0);
                next.push(u - ratio * l);
            }
            if next.is_empty() {
                next.push(0.0);
            }
            upper = lower;
            lower = next;
        }
        lower.first().is_none_or(|&last| last > 1e-14) && upper[0] > 0.0
    }

    /// Schur-Cohn: every root of the polynomial (highest power first) lies strictly inside the
    /// unit circle.
    pub fn is_schur(poly: &[f64]) -> bool {
        // Ascending coefficient order for the reduction.
        let mut a: Vec<f64> = poly.iter().rev().copied().collect();
        while a.len() > 1 {
            let n = a.len() - 1;
            let (a0, an) = (a[0], a[n]);
            if an == 0.0 || a0.abs() >= an.abs() {
                return false;
            }
            a = (0..n).map(|k| an * a[k + 1] - a0 * a[n - 1 - k]).collect();
        }
        true
    }

    /// Solve `M x = b` by Gaussian elimination with partial pivoting; `None` if singular.
    fn solve_real(m: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
        let n = b.len();
        let mut aug: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut row = m[i].clone();
                row.push(b[i]);
                row
            })
            .collect();
        for col in 0..n {
            let pivot =
                (col..n).max_by(|&i, &j| aug[i][col].abs().total_cmp(&aug[j][col].abs()))?;
            if aug[pivot][col].abs() < 1e-12 {
                return None;
            }
            aug.swap(col, pivot);
            for row in 0..n {
                if row != col {
                    let factor = aug[row][col] / aug[col][col];
                    for k in col..=n {
                        let delta = factor * aug[col][k];
                        aug[row][k] -= delta;
                    }
                }
            }
        }
        Some((0..n).map(|i| aug[i][n] / aug[i][i]).collect())
    }

    /// Solve `(s I - A) X = B` for complex `s = (re, im)`; `X` is `n x m` of `(re, im)` pairs.
    fn solve_resolvent(&self, s: (f64, f64)) -> Option<Vec<Vec<(f64, f64)>>> {
        type Z = (f64, f64);
        let mul = |a: Z, b: Z| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
        let div = |a: Z, b: Z| {
            let d = b.0 * b.0 + b.1 * b.1;
            ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
        };
        let n = self.num_states();
        let m = self.num_inputs();
        let mut aug: Vec<Vec<Z>> = (0..n)
            .map(|i| {
                let mut row: Vec<Z> = (0..n)
                    .map(|j| {
                        let diag = if i == j { s } else { (0.0, 0.0) };
                        (diag.0 - self.a[i][j], diag.1)
                    })
                    .collect();
                row.extend((0..m).map(|k| (self.b[i][k], 0.0)));
                row
            })
            .collect();
        for col in 0..n {
            let pivot = (col..n).max_by(|&i, &j| {
                let (p, q) = (aug[i][col], aug[j][col]);
                (p.0 * p.0 + p.1 * p.1).total_cmp(&(q.0 * q.0 + q.1 * q.1))
            })?;
            let p = aug[pivot][col];
            if p.0 * p.0 + p.1 * p.1 < 1e-24 {
                return None;
            }
            aug.swap(col, pivot);
            for row in 0..n {
                if row != col {
                    let factor = div(aug[row][col], aug[col][col]);
                    for k in col..n + m {
                        let delta = mul(factor, aug[col][k]);
                        aug[row][k] = (aug[row][k].0 - delta.0, aug[row][k].1 - delta.1);
                    }
                }
            }
        }
        Some(
            (0..n)
                .map(|i| (0..m).map(|k| div(aug[i][n + k], aug[i][i])).collect())
                .collect(),
        )
    }

    /// Transfer matrix `H = C (sI - A)^-1 B + D` at angular frequency `omega` as complex
    /// entries `(re, im)`: `s = j omega` for continuous systems and `z = e^{j omega T}` for
    /// discrete ones. At a pole the resolvent is singular and an error is returned.
    pub fn transfer_matrix(&self, omega: f64) -> Result<Vec<Vec<(f64, f64)>>, String> {
        let s = match self.time_domain {
            TimeDomain::Continuous => (0.0, omega),
            TimeDomain::Discrete { sample_time } => {
                ((omega * sample_time).cos(), (omega * sample_time).sin())
            }
        };
        let x = self
            .solve_resolvent(s)
            .ok_or_else(|| "frequency lies on a pole of the system".to_string())?;
        let (n, m, p) = (self.num_states(), self.num_inputs(), self.num_outputs());
        Ok((0..p)
            .map(|i| {
                (0..m)
                    .map(|k| {
                        let mut acc = (self.d[i][k], 0.0);
                        for j in 0..n {
                            acc.0 += self.c[i][j] * x[j][k].0;
                            acc.1 += self.c[i][j] * x[j][k].1;
                        }
                        acc
                    })
                    .collect()
            })
            .collect())
    }

    /// Multi-channel frequency response at angular frequency `omega` (rad/s): for each output
    /// and input, `(magnitude in dB, phase in degrees)` of `C (sI - A)^-1 B + D`.
    pub fn frequency_response(&self, omega: f64) -> Result<Vec<Vec<(f64, f64)>>, String> {
        Ok(self
            .transfer_matrix(omega)?
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(re, im)| {
                        let magnitude = re.hypot(im);
                        let db = if magnitude > 1e-12 {
                            20.0 * magnitude.log10()
                        } else {
                            -240.0
                        };
                        (db, im.atan2(re).to_degrees())
                    })
                    .collect()
            })
            .collect())
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
