//! Linear Programming (`linprog_minimize` via Simplex with Bland's anti-cycling rule)
//! and Yun's square-free polynomial decomposition over `Z[x]`.

/// Termination status of a linear programming solve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinprogStatus {
    Optimal,
    Unbounded,
    Infeasible,
    InvalidInput,
    MaxIterations,
}

/// Result of `linprog_minimize`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinprogSolution {
    pub status: LinprogStatus,
    pub x: Vec<f64>,
    pub objective: f64,
    pub iterations: usize,
}

/// Solves $\min c^T x$ subject to $A_{\text{ub}} x \le b_{\text{ub}}$, $x \ge 0$
/// via the revised/tableau Simplex method with Bland's rule (`b_ub >= 0`).
pub fn linprog_minimize(c: &[f64], a_ub: &[Vec<f64>], b_ub: &[f64]) -> LinprogSolution {
    let n = c.len();
    let m = b_ub.len();
    if n == 0 || m == 0 || a_ub.len() != m || a_ub.iter().any(|row| row.len() != n) {
        return LinprogSolution {
            status: LinprogStatus::InvalidInput,
            x: vec![],
            objective: f64::NAN,
            iterations: 0,
        };
    }
    if b_ub.iter().any(|&b| b < -1e-12) {
        return LinprogSolution {
            status: LinprogStatus::Infeasible,
            x: vec![0.0; n],
            objective: f64::NAN,
            iterations: 0,
        };
    }

    let cols = n + m + 1;
    let mut tab = vec![vec![0.0; cols]; m + 1];
    for i in 0..m {
        for j in 0..n {
            tab[i][j] = a_ub[i][j];
        }
        tab[i][n + i] = 1.0;
        tab[i][cols - 1] = b_ub[i].max(0.0);
    }
    tab[m][..n].copy_from_slice(&c[..n]);

    let mut basis: Vec<usize> = (n..(n + m)).collect();
    let max_iters = (m + n + 4) * 32;
    let mut iters = 0usize;

    while iters < max_iters {
        // Bland's rule: first column with negative reduced cost
        let enter_col = (0..(n + m)).find(|&j| tab[m][j] < -1e-10);
        let Some(col) = enter_col else {
            let mut x = vec![0.0; n];
            for (row_idx, &var) in basis.iter().enumerate() {
                if var < n {
                    x[var] = tab[row_idx][cols - 1];
                }
            }
            return LinprogSolution {
                status: LinprogStatus::Optimal,
                x,
                objective: -tab[m][cols - 1],
                iterations: iters,
            };
        };

        let mut leave_row = None;
        let mut best_ratio = f64::INFINITY;
        for (i, row) in tab.iter().enumerate().take(m) {
            if row[col] > 1e-10 {
                let ratio = row[cols - 1] / row[col];
                if ratio < best_ratio - 1e-12
                    || ((ratio - best_ratio).abs() <= 1e-12
                        && leave_row.is_none_or(|lr| basis[i] < basis[lr]))
                {
                    best_ratio = ratio;
                    leave_row = Some(i);
                }
            }
        }

        let Some(r) = leave_row else {
            return LinprogSolution {
                status: LinprogStatus::Unbounded,
                x: vec![0.0; n],
                objective: f64::NEG_INFINITY,
                iterations: iters,
            };
        };

        let pivot = tab[r][col];
        for j in 0..cols {
            tab[r][j] /= pivot;
        }
        for i in 0..=m {
            if i == r {
                continue;
            }
            let factor = tab[i][col];
            if factor.abs() > 1e-15 {
                for j in 0..cols {
                    tab[i][j] -= factor * tab[r][j];
                }
            }
        }
        basis[r] = col;
        iters += 1;
    }

    LinprogSolution {
        status: LinprogStatus::MaxIterations,
        x: vec![0.0; n],
        objective: f64::NAN,
        iterations: iters,
    }
}

fn strip_zeros(mut p: Vec<i64>) -> Vec<i64> {
    while p.last() == Some(&0) {
        p.pop();
    }
    p
}

fn trim_poly(p: Vec<i64>) -> Vec<i64> {
    let mut p = strip_zeros(p);
    if p.is_empty() {
        return vec![];
    }
    let mut g = 0i64;
    for &c in &p {
        g = gcd_i64(g, c.abs());
    }
    if g > 1 {
        for c in &mut p {
            *c /= g;
        }
    }
    if let Some(&lc) = p.last()
        && lc < 0
    {
        for c in &mut p {
            *c = -*c;
        }
    }
    p
}

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.abs()
}

fn poly_deriv_raw(p: &[i64]) -> Vec<i64> {
    if p.len() <= 1 {
        return vec![];
    }
    strip_zeros(
        p.iter()
            .enumerate()
            .skip(1)
            .map(|(i, &c)| c * i as i64)
            .collect(),
    )
}

fn poly_sub_raw(a: &[i64], b: &[i64]) -> Vec<i64> {
    let len = a.len().max(b.len());
    let mut out = vec![0i64; len];
    for i in 0..len {
        out[i] = a.get(i).copied().unwrap_or(0) - b.get(i).copied().unwrap_or(0);
    }
    strip_zeros(out)
}

fn poly_pseudo_rem(a: &[i64], b: &[i64]) -> Vec<i64> {
    let mut r = trim_poly(a.to_vec());
    let b = trim_poly(b.to_vec());
    if b.is_empty() {
        return r;
    }
    let deg_b = b.len() - 1;
    let lc_b = b[deg_b];
    while r.len() >= b.len() {
        let deg_r = r.len() - 1;
        let shift = deg_r - deg_b;
        let lc_r = r[deg_r];
        for c in &mut r {
            *c *= lc_b;
        }
        for (i, &bi) in b.iter().enumerate() {
            r[i + shift] -= lc_r * bi;
        }
        r = trim_poly(r);
    }
    r
}

fn poly_exact_div_raw(a: &[i64], b: &[i64]) -> Vec<i64> {
    let a = strip_zeros(a.to_vec());
    let b = strip_zeros(b.to_vec());
    if b.is_empty() || a.len() < b.len() {
        return vec![];
    }
    let deg_b = b.len() - 1;
    let mut rem: Vec<f64> = a.iter().map(|&v| v as f64).collect();
    let mut quot = vec![0.0_f64; a.len() - b.len() + 1];
    let lc_b = b[deg_b] as f64;
    for k in (0..quot.len()).rev() {
        let coeff = rem[k + deg_b] / lc_b;
        quot[k] = coeff;
        for (i, &bi) in b.iter().enumerate() {
            rem[k + i] -= coeff * (bi as f64);
        }
    }
    strip_zeros(quot.into_iter().map(|q| q.round() as i64).collect())
}

fn poly_gcd_z(mut a: Vec<i64>, mut b: Vec<i64>) -> Vec<i64> {
    a = trim_poly(a);
    b = trim_poly(b);
    while !b.is_empty() {
        let r = poly_pseudo_rem(&a, &b);
        a = b;
        b = r;
    }
    if a.is_empty() { vec![1] } else { a }
}

/// Yun's square-free decomposition of a primitive univariate polynomial `coeffs` (`c_0 + c_1 x + ...`)
/// returning `(factor_coeffs, multiplicity)` pairs with `multiplicity >= 1`.
pub fn yun_square_free_z(coeffs: &[i64]) -> Vec<(Vec<i64>, usize)> {
    let f = trim_poly(coeffs.to_vec());
    if f.len() <= 1 {
        return vec![];
    }
    let df = poly_deriv_raw(&f);
    let g = poly_gcd_z(f.clone(), df.clone());
    let mut w = trim_poly(poly_exact_div_raw(&f, &g));
    let mut y = poly_exact_div_raw(&df, &g);
    let mut z = poly_sub_raw(&y, &poly_deriv_raw(&w));

    let mut factors = Vec::new();
    let max_mult = f.len();
    for i in 1..=max_mult {
        if w.len() <= 1 {
            break;
        }
        let a = poly_gcd_z(w.clone(), z.clone());
        if a.len() > 1 {
            factors.push((a.clone(), i));
        }
        w = trim_poly(poly_exact_div_raw(&w, &a));
        if w.len() <= 1 {
            break;
        }
        y = poly_exact_div_raw(&z, &a);
        z = poly_sub_raw(&y, &poly_deriv_raw(&w));
    }
    factors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linprog_minimize_feasible_and_unbounded() {
        // Minimize -3x - 2y s.t. x + y <= 4, x <= 2, y <= 3, x,y >= 0
        // Optimal at (2, 2) with objective = -10
        let sol = linprog_minimize(
            &[-3.0, -2.0],
            &[vec![1.0, 1.0], vec![1.0, 0.0], vec![0.0, 1.0]],
            &[4.0, 2.0, 3.0],
        );
        assert_eq!(sol.status, LinprogStatus::Optimal);
        assert!((sol.x[0] - 2.0).abs() < 1e-6);
        assert!((sol.x[1] - 2.0).abs() < 1e-6);
        assert!((sol.objective + 10.0).abs() < 1e-6);

        let unbounded = linprog_minimize(&[-1.0], &[vec![-1.0]], &[1.0]);
        assert_eq!(unbounded.status, LinprogStatus::Unbounded);
    }

    #[test]
    fn test_yun_square_free_decomposition() {
        // (x - 1)^2 * (x + 2) = (x^2 - 2x + 1)(x + 2) = x^3 - 3x + 2
        let coeffs = vec![2, -3, 0, 1];
        let sqf = yun_square_free_z(&coeffs);
        assert_eq!(sqf.len(), 2);
        assert_eq!(sqf[0], (vec![2, 1], 1)); // (x + 2)^1
        assert_eq!(sqf[1], (vec![-1, 1], 2)); // (x - 1)^2
    }
}
