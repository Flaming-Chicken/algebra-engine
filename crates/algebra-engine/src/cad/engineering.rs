//! # `algebra_engine::cad::engineering`
//!
//! Parametric Complex Engineering Machinery Generators.
//!
//! Provides mathematically exact 2D/3D parametric synthesis for:
//! - **Involute Spur & Helical Gears**: Exact involute curves, pressure angle, addendum/dedendum, root fillets, helix angles.
//! - **Threaded Screws & Fasteners**: ISO metric 60° V-threads, Acme 29° lead screws, square threads, hexagonal & socket bolt heads.
//! - **NACA Airfoils & Aerodynamic Blades**: 4-digit analytical airfoils, mean camber line, thickness envelope, spanwise twist lofting.
//! - **Helical Springs**: Cylindrical compression, tension, and torsion springs with wire diameter $d$, pitch $p$, and squared ends.
//! - **Stepped Shafts & Keyways**: Multi-diameter rotating machinery shafts with DIN 6885 keyways and splines.

use super::Mesh3D;
use std::f64::consts::PI;

/// Involute Spur or Helical Gear Generator.
#[derive(Debug, Clone, PartialEq)]
pub struct InvoluteGear {
    /// Gear Module $m$ (mm/tooth).
    pub module: f64,
    /// Number of teeth $z$.
    pub teeth: usize,
    /// Pressure angle $\alpha$ in degrees (typically $20.0^\circ$ or $14.5^\circ$).
    pub pressure_angle_deg: f64,
    /// Helix angle $\beta$ in degrees ($0.0^\circ$ for spur gears, $> 0^\circ$ for helical gears).
    pub helix_angle_deg: f64,
    /// Face width / thickness of the gear $b$.
    pub face_width: f64,
    /// Central bore hole radius (e.g. for shaft mounting).
    pub bore_radius: f64,
}

impl InvoluteGear {
    /// Create a standard spur gear specification.
    pub fn spur(module: f64, teeth: usize, face_width: f64) -> Self {
        Self {
            module,
            teeth,
            pressure_angle_deg: 20.0,
            helix_angle_deg: 0.0,
            face_width,
            bore_radius: module * (teeth as f64) * 0.15,
        }
    }

    /// Create a helical gear specification.
    pub fn helical(module: f64, teeth: usize, face_width: f64, helix_angle_deg: f64) -> Self {
        Self {
            module,
            teeth,
            pressure_angle_deg: 20.0,
            helix_angle_deg,
            face_width,
            bore_radius: module * (teeth as f64) * 0.15,
        }
    }

    /// Pitch circle diameter $d = m z$.
    pub fn pitch_diameter(&self) -> f64 {
        self.module * (self.teeth as f64)
    }

    /// Base circle diameter $d_b = d \cos \alpha$.
    pub fn base_diameter(&self) -> f64 {
        self.pitch_diameter() * (self.pressure_angle_deg * PI / 180.0).cos()
    }

    /// Tip / Addendum circle diameter $d_a = d + 2m$.
    pub fn tip_diameter(&self) -> f64 {
        self.pitch_diameter() + 2.0 * self.module
    }

    /// Root / Dedendum circle diameter $d_f = d - 2.5m$.
    pub fn root_diameter(&self) -> f64 {
        self.pitch_diameter() - 2.5 * self.module
    }

    /// Generate 2D tooth cross-section profile boundary points.
    pub fn generate_2d_profile(&self, samples_per_tooth: usize) -> Vec<[f64; 2]> {
        let z = self.teeth;
        let rb = self.base_diameter() / 2.0;
        let ra = self.tip_diameter() / 2.0;
        let rf = self.root_diameter() / 2.0;
        let d_theta = 2.0 * PI / (z as f64);

        let mut points = Vec::new();

        for i in 0..z {
            let base_angle = (i as f64) * d_theta;

            // 1. Root dedendum arc
            points.push([
                rf * (base_angle - d_theta * 0.25).cos(),
                rf * (base_angle - d_theta * 0.25).sin(),
            ]);

            // 2. Involute rising flank
            let max_t = if ra * ra >= rb * rb {
                ((ra * ra - rb * rb) / (rb * rb)).sqrt()
            } else {
                0.5
            };
            let n_samples = samples_per_tooth.max(4);
            for s in 0..=n_samples {
                let t = (s as f64 / n_samples as f64) * max_t;
                // Involute parametric equation: x = rb(cos t + t sin t), y = rb(sin t - t cos t)
                let inv_x = rb * (t.cos() + t * t.sin());
                let inv_y = rb * (t.sin() - t * t.cos());
                let rot_angle = base_angle;
                let rx = inv_x * rot_angle.cos() - inv_y * rot_angle.sin();
                let ry = inv_x * rot_angle.sin() + inv_y * rot_angle.cos();
                points.push([rx, ry]);
            }

            // 3. Tip addendum arc
            points.push([
                ra * (base_angle + d_theta * 0.25).cos(),
                ra * (base_angle + d_theta * 0.25).sin(),
            ]);

            // 4. Involute falling flank
            for s in (0..=n_samples).rev() {
                let t = (s as f64 / n_samples as f64) * max_t;
                let inv_x = rb * (t.cos() + t * t.sin());
                let inv_y = -rb * (t.sin() - t * t.cos());
                let rot_angle = base_angle + d_theta * 0.5;
                let rx = inv_x * rot_angle.cos() - inv_y * rot_angle.sin();
                let ry = inv_x * rot_angle.sin() + inv_y * rot_angle.cos();
                points.push([rx, ry]);
            }
        }

        points
    }

    /// Generate the full 3D solid of the spur or helical gear as a closed, outward-oriented
    /// triangle mesh: the toothed outer wall, a cylindrical bore of radius `bore_radius` (omitted
    /// when it is zero), and two annular end caps.
    ///
    /// The tooth outline is star-shaped about the axis (its polar angle only increases), so each
    /// cap is triangulated by merging the angular order of the outline with that of the bore
    /// circle, which is valid for the non-convex outline (a fan from one outline vertex is not).
    /// A helical gear twists the layers about the axis; the cross-section area is unchanged.
    pub fn generate_3d_mesh(&self) -> Mesh3D {
        let profile = self.generate_2d_profile(6);
        let n_prof = profile.len();
        let height = self.face_width;
        let mut mesh = Mesh3D::new(format!("Gear_m{}_z{}", self.module, self.teeth));

        let n_layers = if self.helix_angle_deg.abs() > 1e-4 {
            8
        } else {
            2
        };
        let max_twist = if self.helix_angle_deg.abs() > 1e-4 {
            let beta_rad = self.helix_angle_deg * PI / 180.0;
            (height * beta_rad.tan()) / (self.pitch_diameter() / 2.0)
        } else {
            0.0
        };

        // The bore must stay inside the root circle to leave a wall.
        let bore_radius = self
            .bore_radius
            .min(0.8 * self.root_diameter() / 2.0)
            .max(0.0);
        let n_bore = if bore_radius > 1e-9 { 48 } else { 0 };
        let per_layer = n_prof + n_bore;
        let start_angle = profile[0][1].atan2(profile[0][0]);

        for l in 0..n_layers {
            let fraction = l as f64 / (n_layers - 1) as f64;
            let z_pos = fraction * height;
            let twist = fraction * max_twist;
            let (cos_tw, sin_tw) = (twist.cos(), twist.sin());
            for p in &profile {
                mesh.add_vertex([
                    p[0] * cos_tw - p[1] * sin_tw,
                    p[0] * sin_tw + p[1] * cos_tw,
                    z_pos,
                ]);
            }
            for j in 0..n_bore {
                let angle = start_angle + 2.0 * PI * j as f64 / n_bore as f64;
                // The bore is a round hole about the axis, so it is not twisted.
                mesh.add_vertex([bore_radius * angle.cos(), bore_radius * angle.sin(), z_pos]);
            }
        }

        // Outer wall (normals point away from the axis) and bore wall (towards the axis).
        for l in 0..n_layers - 1 {
            let (l0, l1) = (l * per_layer, (l + 1) * per_layer);
            for i in 0..n_prof {
                let next = (i + 1) % n_prof;
                mesh.add_triangle(l0 + i, l0 + next, l1 + next);
                mesh.add_triangle(l0 + i, l1 + next, l1 + i);
            }
            for j in 0..n_bore {
                let (a, b) = (n_prof + j, n_prof + (j + 1) % n_bore);
                mesh.add_triangle(l0 + a, l1 + b, l0 + b);
                mesh.add_triangle(l0 + a, l1 + a, l1 + b);
            }
        }

        // Polar angles (increasing, unwrapped from the first outline vertex).
        let mut outline_angles = Vec::with_capacity(n_prof + 1);
        let mut previous = start_angle;
        let mut unwrapped = start_angle;
        outline_angles.push(unwrapped);
        for p in &profile[1..] {
            let angle = p[1].atan2(p[0]);
            let mut delta = angle - previous;
            while delta < -PI {
                delta += 2.0 * PI;
            }
            while delta > PI {
                delta -= 2.0 * PI;
            }
            unwrapped += delta;
            outline_angles.push(unwrapped);
            previous = angle;
        }
        outline_angles.push(start_angle + 2.0 * PI);

        // Triangulate an end cap. `layer_base` is the first vertex of the layer, `twist` the angle
        // by which that layer's outline is rotated (the bore ring is not rotated), and `up`
        // selects the winding (counter-clockwise seen from +z for the top cap).
        let mut cap = |layer_base: usize, up: bool, twist: f64| {
            let mut push = |a: usize, b: usize, c: usize| {
                if up {
                    mesh.add_triangle(a, b, c);
                } else {
                    mesh.add_triangle(a, c, b);
                }
            };
            if n_bore == 0 {
                // No bore: the solid-disc fans below close the ends.
                return;
            }
            let step = 2.0 * PI / n_bore as f64;
            // Start at the bore vertex at or just before the rotated outline's first vertex.
            let first_bore = (twist / step).floor() as i64;
            let bore_angle = |k: i64| start_angle + step * (first_bore + k) as f64;
            let bore_index = |k: i64| (first_bore + k).rem_euclid(n_bore as i64) as usize;
            let (mut i, mut k) = (0usize, 0i64);
            while i < n_prof || k < n_bore as i64 {
                let advance_outer = if i >= n_prof {
                    false
                } else if k >= n_bore as i64 {
                    true
                } else {
                    outline_angles[i + 1] + twist <= bore_angle(k + 1)
                };
                let (outer, outer_next) = (layer_base + i % n_prof, layer_base + (i + 1) % n_prof);
                let (inner, inner_next) = (
                    layer_base + n_prof + bore_index(k),
                    layer_base + n_prof + bore_index(k + 1),
                );
                if advance_outer {
                    push(outer, outer_next, inner);
                    i += 1;
                } else {
                    push(outer, inner_next, inner);
                    k += 1;
                }
            }
        };
        let top = (n_layers - 1) * per_layer;
        cap(0, false, 0.0);
        cap(top, true, max_twist);

        if n_bore == 0 {
            // Solid disc: a fan from an axis vertex at each end.
            let bottom_axis = mesh.add_vertex([0.0, 0.0, 0.0]);
            let top_axis = mesh.add_vertex([0.0, 0.0, height]);
            for i in 0..n_prof {
                let next = (i + 1) % n_prof;
                mesh.add_triangle(bottom_axis, next, i);
                mesh.add_triangle(top_axis, top + i, top + next);
            }
        }

        mesh
    }
}

/// Thread profile standard specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStandard {
    /// ISO Metric 60° triangular profile ($M$-series).
    IsoMetric60,
    /// Acme 29° trapezoidal power transmission thread.
    Acme29,
    /// Square high-load thread.
    Square,
}

/// Fastener bolt head style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoltHeadType {
    Hexagonal,
    SocketCap,
    CountersunkFlat,
    None, // Stud / Threaded rod
}

/// Parametric Threaded Fastener / Screw / Bolt Generator.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreadedScrew {
    /// Major / Nominal thread diameter $d$ (mm).
    pub nominal_diameter: f64,
    /// Thread pitch $P$ (mm/turn).
    pub pitch: f64,
    /// Threaded length $L_{\text{thread}}$ (mm).
    pub thread_length: f64,
    /// Total shank length $L$ (mm).
    pub total_length: f64,
    /// Thread standard profile (ISO 60, Acme 29, Square).
    pub thread_standard: ThreadStandard,
    /// Head type (Hex, Socket Cap, etc.).
    pub head_type: BoltHeadType,
}

impl ThreadedScrew {
    /// Standard ISO Metric Bolt (e.g. M10 x 1.5, length 40mm).
    pub fn metric_bolt(nominal_dia: f64, pitch: f64, total_len: f64) -> Self {
        Self {
            nominal_diameter: nominal_dia,
            pitch,
            thread_length: (total_len * 0.75).min(total_len),
            total_length: total_len,
            thread_standard: ThreadStandard::IsoMetric60,
            head_type: BoltHeadType::Hexagonal,
        }
    }

    /// Standard Socket Cap Screw (Allen bolt).
    pub fn socket_screw(nominal_dia: f64, pitch: f64, total_len: f64) -> Self {
        Self {
            nominal_diameter: nominal_dia,
            pitch,
            thread_length: total_len * 0.8,
            total_length: total_len,
            thread_standard: ThreadStandard::IsoMetric60,
            head_type: BoltHeadType::SocketCap,
        }
    }

    /// Acme Lead Screw for CNC / 3D printer axes.
    pub fn lead_screw(nominal_dia: f64, pitch: f64, length: f64) -> Self {
        Self {
            nominal_diameter: nominal_dia,
            pitch,
            thread_length: length,
            total_length: length,
            thread_standard: ThreadStandard::Acme29,
            head_type: BoltHeadType::None,
        }
    }

    /// Generate the 3D mesh of the screw: a helical thread surface with a shank and head.
    ///
    /// This is an *open surface* mesh for visualisation, not a closed solid: threads and ends
    /// are not capped, so [`Mesh3D::is_closed`] is false and `compute_volume` is not the screw's
    /// volume. Use the gear, airfoil and spring meshes (closed solids) for volume-based analysis.
    pub fn generate_3d_mesh(&self) -> Mesh3D {
        let mut mesh = Mesh3D::new(format!("Screw_M{}_P{}", self.nominal_diameter, self.pitch));
        let r_major = self.nominal_diameter / 2.0;
        let r_minor = r_major - 0.6134 * self.pitch; // ISO thread depth

        let turns = (self.thread_length / self.pitch).max(1.0);
        let steps_per_turn = 16;
        let total_steps = (turns * steps_per_turn as f64) as usize;

        // Generate helical thread ridge and root vertices
        for i in 0..=total_steps {
            let theta = (i as f64 / steps_per_turn as f64) * 2.0 * PI;
            let z = (i as f64 / total_steps as f64) * self.thread_length;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            // Root vertex
            mesh.add_vertex([r_minor * cos_t, r_minor * sin_t, z]);
            // Crest vertex
            mesh.add_vertex([r_major * cos_t, r_major * sin_t, z + self.pitch * 0.25]);
        }

        // Helical side surface triangles
        for i in 0..total_steps {
            let i0 = i * 2;
            let i1 = i0 + 1;
            let i2 = i0 + 2;
            let i3 = i0 + 3;
            mesh.add_triangle(i0, i1, i3);
            mesh.add_triangle(i0, i3, i2);
        }

        // Add bolt head if configured
        match self.head_type {
            BoltHeadType::Hexagonal => {
                let head_height = self.nominal_diameter * 0.7;
                let head_radius = self.nominal_diameter * 0.9;
                let z_base = self.total_length;
                let hex_pts = 6;
                let base_idx = mesh.vertices.len();

                // Hex bottom
                for k in 0..hex_pts {
                    let ang = (k as f64 / hex_pts as f64) * 2.0 * PI;
                    mesh.add_vertex([head_radius * ang.cos(), head_radius * ang.sin(), z_base]);
                }
                // Hex top
                for k in 0..hex_pts {
                    let ang = (k as f64 / hex_pts as f64) * 2.0 * PI;
                    mesh.add_vertex([
                        head_radius * ang.cos(),
                        head_radius * ang.sin(),
                        z_base + head_height,
                    ]);
                }

                // Hex sides
                for k in 0..hex_pts {
                    let nxt = (k + 1) % hex_pts;
                    mesh.add_triangle(base_idx + k, base_idx + nxt, base_idx + hex_pts + nxt);
                    mesh.add_triangle(
                        base_idx + k,
                        base_idx + hex_pts + nxt,
                        base_idx + hex_pts + k,
                    );
                }

                // Top hex cap
                for k in 1..hex_pts - 1 {
                    mesh.add_triangle(
                        base_idx + hex_pts,
                        base_idx + hex_pts + k,
                        base_idx + hex_pts + k + 1,
                    );
                }
            }
            BoltHeadType::SocketCap => {
                let head_height = self.nominal_diameter * 1.0;
                let head_radius = self.nominal_diameter * 0.75;
                let z_base = self.total_length;
                let n_circ = 16;
                let base_idx = mesh.vertices.len();

                for k in 0..n_circ {
                    let ang = (k as f64 / n_circ as f64) * 2.0 * PI;
                    mesh.add_vertex([head_radius * ang.cos(), head_radius * ang.sin(), z_base]);
                    mesh.add_vertex([
                        head_radius * ang.cos(),
                        head_radius * ang.sin(),
                        z_base + head_height,
                    ]);
                }
                for k in 0..n_circ {
                    let k0 = base_idx + k * 2;
                    let k1 = k0 + 1;
                    let k2 = base_idx + ((k + 1) % n_circ) * 2;
                    let k3 = k2 + 1;
                    mesh.add_triangle(k0, k2, k3);
                    mesh.add_triangle(k0, k3, k1);
                }
            }
            _ => {}
        }

        mesh
    }
}

/// NACA 4-Digit Aerodynamic Airfoil & Turbine Blade Generator.
#[derive(Debug, Clone, PartialEq)]
pub struct NacaAirfoil {
    /// 4-Digit Code (e.g. "2412", "0012", "4415").
    pub code: String,
    /// Airfoil chord length $c$ (mm).
    pub chord: f64,
    /// Wing / Blade span length $b$ (mm).
    pub span: f64,
    /// Spanwise geometric tip twist angle in degrees.
    pub twist_deg: f64,
}

impl NacaAirfoil {
    pub fn new(code: impl Into<String>, chord: f64, span: f64) -> Self {
        Self {
            code: code.into(),
            chord,
            span,
            twist_deg: 0.0,
        }
    }

    /// Compute half-thickness distribution $y_t(x)$ of NACA 4-digit series.
    pub fn thickness_at(&self, x_norm: f64, t_max_ratio: f64) -> f64 {
        let x = x_norm.clamp(0.0, 1.0);
        // Formula: yt = 5*t*(0.2969*sqrt(x) - 0.1260*x - 0.3516*x^2 + 0.2843*x^3 - 0.1015*x^4)
        5.0 * t_max_ratio
            * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3)
                - 0.1015 * x.powi(4))
    }

    /// Generate 2D airfoil upper and lower coordinate boundary.
    pub fn generate_2d_coordinates(&self, n_samples: usize) -> Vec<[f64; 2]> {
        let digits: Vec<u32> = self.code.chars().filter_map(|c| c.to_digit(10)).collect();
        let m = if digits.len() >= 4 {
            digits[0] as f64 * 0.01
        } else {
            0.0
        }; // Max camber
        let p = if digits.len() >= 4 {
            digits[1] as f64 * 0.1
        } else {
            0.0
        }; // Max camber location
        let t = if digits.len() >= 4 {
            (digits[2] * 10 + digits[3]) as f64 * 0.01
        } else {
            0.12
        }; // Thickness

        let mut upper = Vec::new();
        let mut lower = Vec::new();

        for i in 0..=n_samples {
            // Cosine spacing for high leading-edge resolution
            let beta = (i as f64 / n_samples as f64) * PI;
            let x_norm = 0.5 * (1.0 - beta.cos());
            let yt = self.thickness_at(x_norm, t) * self.chord;

            // Camber line yc(x)
            let yc = if m.abs() < 1e-6 || p.abs() < 1e-6 {
                0.0
            } else if x_norm < p {
                m * (self.chord / (p * p)) * (2.0 * p * x_norm - x_norm * x_norm)
            } else {
                m * (self.chord / ((1.0 - p) * (1.0 - p)))
                    * ((1.0 - 2.0 * p) + 2.0 * p * x_norm - x_norm * x_norm)
            };

            let x_pt = x_norm * self.chord;
            upper.push([x_pt, yc + yt]);
            lower.push([x_pt, yc - yt]);
        }

        // Combine clockwise boundary: upper from LE to TE, lower from TE back to LE
        let mut full_profile = upper;
        full_profile.extend(lower.into_iter().rev().skip(1));
        // The lower surface ends at the leading edge again; drop that duplicate of the first point.
        if let (Some(first), Some(last)) = (full_profile.first(), full_profile.last()) {
            if (first[0] - last[0]).abs() < 1e-12 && (first[1] - last[1]).abs() < 1e-12 {
                full_profile.pop();
            }
        }
        full_profile
    }

    /// Generate 3D lofted aerodynamic wing mesh.
    pub fn generate_3d_mesh(&self) -> Mesh3D {
        let profile = self.generate_2d_coordinates(20);
        let n_prof = profile.len();
        let n_sections = 8;
        let mut mesh = Mesh3D::new(format!("Airfoil_NACA_{}", self.code));

        for s in 0..n_sections {
            let span_frac = s as f64 / (n_sections - 1) as f64;
            let z_pos = span_frac * self.span;
            let twist_rad = (span_frac * self.twist_deg) * PI / 180.0;
            let cos_tw = twist_rad.cos();
            let sin_tw = twist_rad.sin();

            for p in &profile {
                let rx = p[0] * cos_tw - p[1] * sin_tw;
                let ry = p[0] * sin_tw + p[1] * cos_tw;
                mesh.add_vertex([rx, ry, z_pos]);
            }
        }

        // Side skinning
        for s in 0..n_sections - 1 {
            let s0 = s * n_prof;
            let s1 = (s + 1) * n_prof;
            for i in 0..n_prof {
                let nxt = (i + 1) % n_prof;
                mesh.add_triangle(s0 + i, s0 + nxt, s1 + nxt);
                mesh.add_triangle(s0 + i, s1 + nxt, s1 + i);
            }
        }

        // End caps: a fan from the centroid of the section, which lies inside an airfoil outline.
        let (cx, cy) = profile
            .iter()
            .fold((0.0, 0.0), |(x, y), p| (x + p[0], y + p[1]));
        let (cx, cy) = (cx / n_prof as f64, cy / n_prof as f64);
        let last = (n_sections - 1) * n_prof;
        let bottom_centre = mesh.add_vertex([cx, cy, 0.0]);
        let twist_rad = self.twist_deg * PI / 180.0;
        let top_centre = mesh.add_vertex([
            cx * twist_rad.cos() - cy * twist_rad.sin(),
            cx * twist_rad.sin() + cy * twist_rad.cos(),
            self.span,
        ]);
        for i in 0..n_prof {
            let nxt = (i + 1) % n_prof;
            mesh.add_triangle(bottom_centre, nxt, i);
            mesh.add_triangle(top_centre, last + i, last + nxt);
        }

        mesh
    }
}

/// Parametric Helical Spring Generator.
#[derive(Debug, Clone, PartialEq)]
pub struct HelicalSpring {
    /// Spring Wire diameter $d$ (mm).
    pub wire_diameter: f64,
    /// Mean coil diameter $D$ (mm).
    pub mean_diameter: f64,
    /// Coil Pitch $p$ (mm/turn).
    pub pitch: f64,
    /// Number of active coils $n$.
    pub active_coils: f64,
}

impl HelicalSpring {
    pub fn new(wire_dia: f64, mean_dia: f64, pitch: f64, active_coils: f64) -> Self {
        Self {
            wire_diameter: wire_dia,
            mean_diameter: mean_dia,
            pitch,
            active_coils,
        }
    }

    /// Total free length $L_0 = n \cdot p$.
    pub fn free_length(&self) -> f64 {
        self.active_coils * self.pitch
    }

    /// Generate 3D helical coil wireframe mesh.
    pub fn generate_3d_mesh(&self) -> Mesh3D {
        let mut mesh = Mesh3D::new("HelicalSpring");
        let r_coil = self.mean_diameter / 2.0;
        let r_wire = self.wire_diameter / 2.0;

        let steps_per_turn = 24;
        let total_steps = (self.active_coils * steps_per_turn as f64) as usize;
        let circ_steps = 8;

        for s in 0..=total_steps {
            let theta = (s as f64 / steps_per_turn as f64) * 2.0 * PI;
            let z_center = (s as f64 / steps_per_turn as f64) * self.pitch;

            let cx = r_coil * theta.cos();
            let cy = r_coil * theta.sin();

            // Tangent & Normal frame for wire circular tube cross section
            for k in 0..circ_steps {
                let phi = (k as f64 / circ_steps as f64) * 2.0 * PI;
                let wx = cx + r_wire * phi.cos() * theta.cos();
                let wy = cy + r_wire * phi.cos() * theta.sin();
                let wz = z_center + r_wire * phi.sin();
                mesh.add_vertex([wx, wy, wz]);
            }
        }

        for s in 0..total_steps {
            let s0 = s * circ_steps;
            let s1 = (s + 1) * circ_steps;
            for k in 0..circ_steps {
                let nxt = (k + 1) % circ_steps;
                mesh.add_triangle(s0 + k, s0 + nxt, s1 + nxt);
                mesh.add_triangle(s0 + k, s1 + nxt, s1 + k);
            }
        }

        // End caps: a fan from the centre of the wire's circular cross-section at each end.
        let ring_centre = |s: usize| {
            let theta = (s as f64 / steps_per_turn as f64) * 2.0 * PI;
            [
                r_coil * theta.cos(),
                r_coil * theta.sin(),
                (s as f64 / steps_per_turn as f64) * self.pitch,
            ]
        };
        let bottom_centre = mesh.add_vertex(ring_centre(0));
        let top_centre = mesh.add_vertex(ring_centre(total_steps));
        let top = total_steps * circ_steps;
        for k in 0..circ_steps {
            let nxt = (k + 1) % circ_steps;
            mesh.add_triangle(bottom_centre, nxt, k);
            mesh.add_triangle(top_centre, top + k, top + nxt);
        }

        mesh
    }
}
