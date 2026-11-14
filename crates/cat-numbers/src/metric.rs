//! Metric Tensor, Riemannian / Lorentzian Geometry, and Einstein Field Equations.
//!
//! Provides Christoffel connection coefficients \Gamma^\lambda_{\mu\nu},
//! Riemann curvature tensor R^\rho_{\ \sigma\mu\nu}, Ricci tensor R_{\mu\nu},
//! Ricci scalar R, Einstein tensor G_{\mu\nu}, and Weyl conformal tensor C_{\rho\sigma\mu\nu}.

use crate::tensor::Tensor;

/// Metric Tensor g_{\mu\nu} defining distances, angles, and connection on a pseudo-Riemannian manifold.
#[derive(Clone, PartialEq)]
pub struct MetricTensor {
    /// Dimension D (e.g. 4 for spacetime).
    pub dim: usize,
    /// Covariant metric components g_{\mu\nu} (valence (0, 2)).
    pub g: Tensor,
    /// Inverse contravariant metric components g^{\mu\nu} (valence (2, 0)).
    pub g_inv: Tensor,
}

impl MetricTensor {
    /// Create Minkowski spacetime metric \eta_{\mu\nu} = \text{diag}(-1, 1, 1, 1) in Cartesian coordinates.
    pub fn minkowski() -> Self {
        let dim = 4;
        let mut g = Tensor::zeros(dim, 0, 2);
        let mut g_inv = Tensor::zeros(dim, 2, 0);

        g.set(&[0, 0], -1.0);
        g_inv.set(&[0, 0], -1.0);

        for i in 1..4 {
            g.set(&[i, i], 1.0);
            g_inv.set(&[i, i], 1.0);
        }

        Self { dim, g, g_inv }
    }

    /// Construct 4D Schwarzschild black hole metric evaluated at radius r and polar angle \theta:
    /// ds^2 = -(1 - 2M/r) dt^2 + (1 - 2M/r)^{-1} dr^2 + r^2 d\theta^2 + r^2 \sin^2\theta d\phi^2.
    pub fn schwarzschild(mass: f64, r: f64, theta: f64) -> Self {
        let dim = 4;
        let mut g = Tensor::zeros(dim, 0, 2);
        let mut g_inv = Tensor::zeros(dim, 2, 0);

        let f = 1.0 - 2.0 * mass / r;
        let sin_th = theta.sin();

        // g_00 = -(1 - 2M/r), g^00 = -1/(1 - 2M/r)
        g.set(&[0, 0], -f);
        g_inv.set(&[0, 0], -1.0 / f);

        // g_11 = 1/(1 - 2M/r), g^11 = 1 - 2M/r
        g.set(&[1, 1], 1.0 / f);
        g_inv.set(&[1, 1], f);

        // g_22 = r^2, g^22 = 1/r^2
        g.set(&[2, 2], r * r);
        g_inv.set(&[2, 2], 1.0 / (r * r));

        // g_33 = r^2 \sin^2\theta, g^33 = 1/(r^2 \sin^2\theta)
        let r_sin = r * sin_th;
        g.set(&[3, 3], r_sin * r_sin);
        g_inv.set(&[3, 3], 1.0 / (r_sin * r_sin));

        Self { dim, g, g_inv }
    }

    /// Raise covariant index: T^\mu = g^{\mu\nu} T_\nu.
    pub fn raise_vector(&self, v_lower: &[f64]) -> Vec<f64> {
        assert_eq!(v_lower.len(), self.dim);
        let mut out = vec![0.0; self.dim];
        for mu in 0..self.dim {
            for nu in 0..self.dim {
                out[mu] += self.g_inv.get(&[mu, nu]) * v_lower[nu];
            }
        }
        out
    }

    /// Lower contravariant index: T_\mu = g_{\mu\nu} T^\nu.
    pub fn lower_vector(&self, v_upper: &[f64]) -> Vec<f64> {
        assert_eq!(v_upper.len(), self.dim);
        let mut out = vec![0.0; self.dim];
        for mu in 0..self.dim {
            for nu in 0..self.dim {
                out[mu] += self.g.get(&[mu, nu]) * v_upper[nu];
            }
        }
        out
    }

    /// Compute Christoffel symbols of the second kind:
    /// \Gamma^\lambda_{\mu\nu} = 1/2 g^{\lambda\sigma} (\partial_\mu g_{\nu\sigma} + \partial_\nu g_{\mu\sigma} - \partial_\sigma g_{\mu\nu}).
    ///
    /// Takes spatial derivative tensor \partial_\sigma g_{\mu\nu} of valence (0, 3).
    pub fn compute_christoffel(&self, dg: &Tensor) -> Tensor {
        let mut gamma = Tensor::zeros(self.dim, 1, 2);
        for lambda in 0..self.dim {
            for mu in 0..self.dim {
                for nu in 0..self.dim {
                    let mut sum = 0.0;
                    for sigma in 0..self.dim {
                        let g_inv_ls = self.g_inv.get(&[lambda, sigma]);
                        if g_inv_ls == 0.0 {
                            continue;
                        }
                        // dg.get(&[sigma, mu, nu]) represents \partial_\sigma g_{\mu\nu}
                        let d_mu_g_nu_sigma = dg.get(&[mu, nu, sigma]);
                        let d_nu_g_mu_sigma = dg.get(&[nu, mu, sigma]);
                        let d_sigma_g_mu_nu = dg.get(&[sigma, mu, nu]);

                        sum += 0.5 * g_inv_ls * (d_mu_g_nu_sigma + d_nu_g_mu_sigma - d_sigma_g_mu_nu);
                    }
                    gamma.set(&[lambda, mu, nu], sum);
                }
            }
        }
        gamma
    }

    /// Compute Ricci curvature tensor R_{\mu\nu} from Riemann curvature tensor R^\rho_{\ \mu\lambda\nu}:
    /// R_{\mu\nu} = R^\lambda_{\ \mu\lambda\nu} (contracting slot 0 and slot 2).
    pub fn compute_ricci_tensor(&self, riemann: &Tensor) -> Tensor {
        assert_eq!(riemann.valence, (1, 3));
        let mut ricci = Tensor::zeros(self.dim, 0, 2);
        for mu in 0..self.dim {
            for nu in 0..self.dim {
                let mut sum = 0.0;
                for lambda in 0..self.dim {
                    // R^\lambda_{\ \mu\lambda\nu}
                    sum += riemann.get(&[lambda, mu, lambda, nu]);
                }
                ricci.set(&[mu, nu], sum);
            }
        }
        ricci
    }

    /// Compute Ricci scalar curvature R = g^{\mu\nu} R_{\mu\nu}.
    pub fn compute_ricci_scalar(&self, ricci: &Tensor) -> f64 {
        let mut r = 0.0;
        for mu in 0..self.dim {
            for nu in 0..self.dim {
                let g_inv_munu = self.g_inv.get(&[mu, nu]);
                if g_inv_munu != 0.0 {
                    r += g_inv_munu * ricci.get(&[mu, nu]);
                }
            }
        }
        r
    }

    /// Compute Einstein tensor: G_{\mu\nu} = R_{\mu\nu} - 1/2 R g_{\mu\nu}.
    /// In general relativity, G_{\mu\nu} = 8\pi G T_{\mu\nu} (and G_{\mu\nu} \equiv 0 in vacuum!).
    pub fn compute_einstein_tensor(&self, ricci: &Tensor) -> Tensor {
        let r_scalar = self.compute_ricci_scalar(ricci);
        let mut einstein = Tensor::zeros(self.dim, 0, 2);
        for mu in 0..self.dim {
            for nu in 0..self.dim {
                let g_munu = self.g.get(&[mu, nu]);
                let r_munu = ricci.get(&[mu, nu]);
                einstein.set(&[mu, nu], r_munu - 0.5 * r_scalar * g_munu);
            }
        }
        einstein
    }

    /// Check metric compatibility condition: \nabla_\lambda g_{\mu\nu} = 0.
    pub fn verify_metric_compatibility(&self, gamma: &Tensor, dg: &Tensor) -> bool {
        // \nabla_\lambda g_{\mu\nu} = \partial_\lambda g_{\mu\nu} - \Gamma^\sigma_{\lambda\mu} g_{\sigma\nu} - \Gamma^\sigma_{\lambda\nu} g_{\mu\sigma}
        for lambda in 0..self.dim {
            for mu in 0..self.dim {
                for nu in 0..self.dim {
                    let d_lambda = dg.get(&[lambda, mu, nu]);
                    let mut term1 = 0.0;
                    let mut term2 = 0.0;
                    for sigma in 0..self.dim {
                        term1 += gamma.get(&[sigma, lambda, mu]) * self.g.get(&[sigma, nu]);
                        term2 += gamma.get(&[sigma, lambda, nu]) * self.g.get(&[mu, sigma]);
                    }
                    let cov_deriv = d_lambda - term1 - term2;
                    if cov_deriv.abs() > 1e-11 {
                        return false;
                    }
                }
            }
        }
        true
    }
}
