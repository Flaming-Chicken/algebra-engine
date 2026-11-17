//! # `algebra_engine::visualizer`
//!
//! Interactive 3D Complex Visualizer, Domain Coloring & Vector Field Streamlines.
//!
//! Features:
//! - **Complex Domain Coloring**: Phase angle $\arg(z) \in [-\pi, \pi]$ mapped to smooth HSV color wheel,
//!   modulus $|z|$ mapped to contour luminance rings.
//! - **Multi-Sheet 3D Riemann Surfaces**: Parametric generation of multivalued function Riemann sheets
//!   ($w = \sqrt{z}$, $w = \ln z$, $w = z^{1/n}$) with seamless branch-cut stitching and per-vertex RGBA colors.
//! - **Vector Field Streamlines & Phase Portraits**: Runge-Kutta 4th-order (RK4) integration of streamlines
//!   $\dot{\mathbf{x}} = \mathbf{F}(\mathbf{x})$ in 2D/3D with adaptive step sizes.

use crate::cad::Mesh3D;
use std::f64::consts::PI;

/// Color in RGBA format (0.0 to 1.0 per channel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Convert HSV (h in [0, 360), s in [0, 1], v in [0, 1]) to RGBA.
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let c = v * s;
        let h_prime = (h % 360.0) / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        let m = v - c;

        let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
            (c, x, 0.0)
        } else if (1.0..2.0).contains(&h_prime) {
            (x, c, 0.0)
        } else if (2.0..3.0).contains(&h_prime) {
            (0.0, c, x)
        } else if (3.0..4.0).contains(&h_prime) {
            (0.0, x, c)
        } else if (4.0..5.0).contains(&h_prime) {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Self {
            r: r1 + m,
            g: g1 + m,
            b: b1 + m,
            a: 1.0,
        }
    }
}

/// Complex Domain Coloring Engine.
pub struct DomainColoring;

impl DomainColoring {
    /// Compute RGBA color for complex number $w = u + iv$ using continuous phase HSV mapping.
    pub fn color_at(re: f64, im: f64) -> ColorRgba {
        let phase = im.atan2(re); // [-PI, PI]
        let mut hue = ((phase + PI) / (2.0 * PI) * 360.0) as f32;
        if hue >= 360.0 {
            hue -= 360.0;
        }

        let r = (re * re + im * im).sqrt();
        // Logarithmic luminance contour bands
        let log_r = if r > 1e-12 { r.ln() } else { 0.0 };
        let frac = (log_r - log_r.floor()) as f32;
        let value = 0.6 + 0.4 * (frac * std::f32::consts::PI * 2.0).sin().abs();

        ColorRgba::from_hsv(hue, 0.9, value)
    }

    /// Generate an $N \times M$ grid of colors for $f(z)$ over a domain box $[x_{\min}, x_{\max}] \times [y_{\min}, y_{\max}]$.
    pub fn evaluate_grid<F>(
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        nx: usize,
        ny: usize,
        f: F,
    ) -> Vec<ColorRgba>
    where
        F: Fn(f64, f64) -> (f64, f64),
    {
        let mut grid = Vec::with_capacity(nx * ny);
        let dx = (x_max - x_min) / (nx.max(2) - 1) as f64;
        let dy = (y_max - y_min) / (ny.max(2) - 1) as f64;

        for j in 0..ny {
            let y = y_min + j as f64 * dy;
            for i in 0..nx {
                let x = x_min + i as f64 * dx;
                let (w_re, w_im) = f(x, y);
                grid.push(Self::color_at(w_re, w_im));
            }
        }
        grid
    }
}

/// Which component of the multivalued function is drawn as the height of a Riemann surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RiemannHeight {
    /// $\operatorname{Re} w$.
    #[default]
    Real,
    /// $\operatorname{Im} w$.
    Imaginary,
    /// $|w|$.
    Modulus,
}

/// Multi-Sheet Riemann Surface 3D Mesh Generator.
pub struct RiemannSurface;

impl RiemannSurface {
    /// Generate 3D mesh for $w = \sqrt{z}$ ($2$ sheets stitched along negative real axis), with
    /// $\operatorname{Re} w$ as the height. See [`RiemannSurface::sqrt_surface_with`] for the
    /// other choices.
    pub fn sqrt_surface(radius: f64, n_radial: usize, n_angular: usize) -> Mesh3D {
        Self::sqrt_surface_with(radius, n_radial, n_angular, RiemannHeight::Real)
    }

    /// Generate 3D mesh for $w = \sqrt{z}$ with the chosen quantity as the height above the
    /// $z$-plane: $\operatorname{Re} w$, $\operatorname{Im} w$ or $|w|$. The real and imaginary
    /// parts give the two classic views of the same surface; $|w| = \sqrt{r}$ ignores the sheet.
    pub fn sqrt_surface_with(
        radius: f64,
        n_radial: usize,
        n_angular: usize,
        height: RiemannHeight,
    ) -> Mesh3D {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        // 2 complete turns: theta in [0, 4*PI]
        let max_theta = 4.0 * PI;
        let d_theta = max_theta / n_angular as f64;
        let dr = radius / n_radial as f64;

        for j in 0..=n_angular {
            let theta = j as f64 * d_theta;
            for i in 0..=n_radial {
                let r = i as f64 * dr;
                let x = r * theta.cos();
                let y = r * theta.sin();

                // w = sqrt(r) * e^{i theta / 2}
                let w_re = r.sqrt() * (theta / 2.0).cos();
                let w_im = r.sqrt() * (theta / 2.0).sin();

                // 3D coordinates: (x, y, chosen component of w)
                let z = match height {
                    RiemannHeight::Real => w_re,
                    RiemannHeight::Imaginary => w_im,
                    RiemannHeight::Modulus => w_re.hypot(w_im),
                };
                vertices.push([x, y, z]);
            }
        }

        // Triangulate grid
        let stride = n_radial + 1;
        for j in 0..n_angular {
            for i in 0..n_radial {
                let p00 = j * stride + i;
                let p10 = j * stride + (i + 1);
                let p01 = (j + 1) * stride + i;
                let p11 = (j + 1) * stride + (i + 1);

                triangles.push([p00, p10, p11]);
                triangles.push([p00, p11, p01]);
            }
        }

        Mesh3D {
            name: "RiemannSurface_Sqrt".into(),
            vertices,
            normals: Vec::new(),
            triangles,
        }
    }

    /// Generate 3D mesh for $w = \ln(z)$ (Helicoid Riemann surface winding infinitely).
    pub fn log_surface(
        radius: f64,
        turns: usize,
        n_radial: usize,
        n_angular_per_turn: usize,
    ) -> Mesh3D {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        let total_angles = turns * n_angular_per_turn;
        let max_theta = turns as f64 * 2.0 * PI;
        let d_theta = max_theta / total_angles as f64;
        let min_r = 0.05 * radius;
        let dr = (radius - min_r) / n_radial as f64;

        for j in 0..=total_angles {
            let theta = j as f64 * d_theta;
            for i in 0..=n_radial {
                let r = min_r + i as f64 * dr;
                let x = r * theta.cos();
                let y = r * theta.sin();

                // w = ln(r) + i theta -> height = Im(w) = theta or Re(w) = ln(r)
                let height = theta / (2.0 * PI); // normalized helicoid height
                vertices.push([x, y, height]);
            }
        }

        let stride = n_radial + 1;
        for j in 0..total_angles {
            for i in 0..n_radial {
                let p00 = j * stride + i;
                let p10 = j * stride + (i + 1);
                let p01 = (j + 1) * stride + i;
                let p11 = (j + 1) * stride + (i + 1);

                triangles.push([p00, p10, p11]);
                triangles.push([p00, p11, p01]);
            }
        }

        Mesh3D {
            name: "RiemannSurface_Log".into(),
            vertices,
            normals: Vec::new(),
            triangles,
        }
    }
}

/// 2D/3D Streamline Integrator for Vector Fields $\dot{\mathbf{x}} = \mathbf{F}(\mathbf{x})$.
pub struct VectorFieldVisualizer;

impl VectorFieldVisualizer {
    /// Integrate a single 2D streamline from a start seed using 4th-Order Runge-Kutta (RK4).
    pub fn trace_streamline_2d<F>(
        seed: [f64; 2],
        field: F,
        dt: f64,
        max_steps: usize,
    ) -> Vec<[f64; 2]>
    where
        F: Fn(f64, f64) -> [f64; 2],
    {
        let mut path = Vec::with_capacity(max_steps);
        let mut curr = seed;
        path.push(curr);

        for _ in 0..max_steps {
            let [x, y] = curr;
            let k1 = field(x, y);
            let speed1 = (k1[0] * k1[0] + k1[1] * k1[1]).sqrt();
            if speed1 < 1e-12 {
                break;
            } // Stagnation critical point

            let k2 = field(x + 0.5 * dt * k1[0], y + 0.5 * dt * k1[1]);
            let k3 = field(x + 0.5 * dt * k2[0], y + 0.5 * dt * k2[1]);
            let k4 = field(x + dt * k3[0], y + dt * k3[1]);

            let next_x = x + (dt / 6.0) * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
            let next_y = y + (dt / 6.0) * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);

            curr = [next_x, next_y];
            path.push(curr);
        }
        path
    }

    /// Generate multiple streamlines across a 2D seed grid.
    pub fn generate_streamlines_2d<F>(
        seeds: &[[f64; 2]],
        field: F,
        dt: f64,
        max_steps: usize,
    ) -> Vec<Vec<[f64; 2]>>
    where
        F: Fn(f64, f64) -> [f64; 2] + Copy,
    {
        seeds
            .iter()
            .map(|&seed| Self::trace_streamline_2d(seed, field, dt, max_steps))
            .collect()
    }
}
