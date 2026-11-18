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
    /// Profile shift coefficient $x$ (rack displacement $x m$; positive thickens the tooth and
    /// lengthens the addendum, avoiding undercut on small pinions). $0$ is a standard gear.
    pub profile_shift: f64,
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
            profile_shift: 0.0,
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
            profile_shift: 0.0,
        }
    }

    /// The same gear with profile shift coefficient `x`.
    pub fn with_profile_shift(mut self, x: f64) -> Self {
        self.profile_shift = x;
        self
    }

    /// Pitch circle diameter $d = m z$.
    pub fn pitch_diameter(&self) -> f64 {
        self.module * (self.teeth as f64)
    }

    /// Base circle diameter $d_b = d \cos \alpha$.
    pub fn base_diameter(&self) -> f64 {
        self.pitch_diameter() * (self.pressure_angle_deg * PI / 180.0).cos()
    }

    /// Tip / Addendum circle diameter $d_a = d + 2(1 + x)m$.
    pub fn tip_diameter(&self) -> f64 {
        self.pitch_diameter() + 2.0 * (1.0 + self.profile_shift) * self.module
    }

    /// Root / Dedendum circle diameter $d_f = d - 2(1.25 - x)m$.
    pub fn root_diameter(&self) -> f64 {
        self.pitch_diameter() - 2.0 * (1.25 - self.profile_shift) * self.module
    }

    /// Half the angular width of a tooth on the circle of radius `r` (radians): the involute
    /// geometry psi + inv(alpha) - inv(alpha_r), where psi = (pi/2 + 2 x tan(alpha)) / z
    /// is the half-width at the pitch circle and cos(alpha_r) = r_b / r. Never negative (a
    /// tooth that would be thinner than a point is cut off there).
    pub fn tooth_half_angle(&self, r: f64) -> f64 {
        let alpha = self.pressure_angle_deg.to_radians();
        let rb = self.base_diameter() / 2.0;
        let inv = |a: f64| a.tan() - a;
        let psi = (PI / 2.0 + 2.0 * self.profile_shift * alpha.tan()) / self.teeth as f64;
        let alpha_r = (rb / r.max(rb)).clamp(-1.0, 1.0).acos();
        (psi + inv(alpha) - inv(alpha_r)).max(0.0)
    }

    /// Generate the 2D tooth outline, counter-clockwise, as boundary points.
    ///
    /// Each tooth is centred on `i 2pi/z`. Its flanks are true involutes of the base circle
    /// (`samples_per_tooth` points each) from the base circle (or the root circle, along a radial
    /// line, when the root lies inside the base circle) out to the tip circle, so the tooth
    /// thickness on the pitch circle is `pi m / 2 + 2 x m tan(alpha)`. The tip and root arcs are
    /// each represented by a midpoint. The outline is star-shaped about the axis, and an
    /// over-thick tooth that would overlap its neighbour is limited to the available space.
    pub fn generate_2d_profile(&self, samples_per_tooth: usize) -> Vec<[f64; 2]> {
        let z = self.teeth;
        let rb = self.base_diameter() / 2.0;
        let ra = self.tip_diameter() / 2.0;
        let rf = self.root_diameter() / 2.0;
        let d_theta = 2.0 * PI / (z as f64);
        let n_samples = samples_per_tooth.max(4);
        let polar = |r: f64, a: f64| [r * a.cos(), r * a.sin()];
        // Flank start radius: the base circle, or the root when the root is outside it.
        let r_start = rf.max(rb);
        let limit = 0.5 * d_theta * 0.98;
        let half = |r: f64| self.tooth_half_angle(r).min(limit);

        let mut points = Vec::new();
        for i in 0..z {
            let c = (i as f64) * d_theta;
            // Root arc midpoint, then the radial run-in below the base circle if there is one.
            points.push(polar(rf, c - 0.5 * d_theta));
            if rf < r_start {
                points.push(polar(rf, c - half(r_start)));
            }
            // Rising (leading) flank: the angular half-width shrinks as r grows.
            for s in 0..=n_samples {
                let r = r_start + (ra - r_start) * (s as f64 / n_samples as f64);
                points.push(polar(r, c - half(r)));
            }
            points.push(polar(ra, c));
            // Falling flank, mirrored.
            for s in (0..=n_samples).rev() {
                let r = r_start + (ra - r_start) * (s as f64 / n_samples as f64);
                points.push(polar(r, c + half(r)));
            }
            if rf < r_start {
                points.push(polar(rf, c + half(r_start)));
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
        let height = self.face_width;
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
        toothed_gear_solid(
            format!("Gear_m{}_z{}", self.module, self.teeth),
            &profile,
            bore_radius,
            height,
            n_layers,
            max_twist,
            false,
        )
    }
}

/// Extrude a toothed outline (counter-clockwise, star-shaped about the axis: its polar angle only
/// increases) into a closed, outward-oriented solid of the given `height`, with a cylindrical
/// bore of radius `bore_radius` (none when zero) and annular end caps. `n_layers` layers are
/// stacked and each is rotated about the axis so that the top is twisted by `max_twist` radians
/// (a helical gear); the cross-section area is unchanged by the twist.
///
/// With `inside_out` the circle of radius `bore_radius` is the *outer* wall and the outline the
/// inner one (an internal gear): the rings are merged by polar angle exactly as before, which
/// works whichever is larger, and every triangle is then reversed so that the normals point out
/// of the material.
fn toothed_gear_solid(
    name: String,
    profile: &[[f64; 2]],
    bore_radius: f64,
    height: f64,
    n_layers: usize,
    max_twist: f64,
    inside_out: bool,
) -> Mesh3D {
    let n_prof = profile.len();
    let mut mesh = Mesh3D::new(name);
    let n_bore = if bore_radius > 1e-9 { 48 } else { 0 };
    let per_layer = n_prof + n_bore;
    let start_angle = profile[0][1].atan2(profile[0][0]);

    for l in 0..n_layers {
        let fraction = l as f64 / (n_layers - 1) as f64;
        let z_pos = fraction * height;
        let twist = fraction * max_twist;
        let (cos_tw, sin_tw) = (twist.cos(), twist.sin());
        for p in profile {
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

    if inside_out {
        for tri in &mut mesh.triangles {
            tri.swap(1, 2);
        }
    }
    mesh
}

/// Cycloidal spur gear: the addendum flanks are epicycloids and the dedendum flanks hypocycloids
/// generated by a rolling circle on the pitch circle, as in clock and watch gearing.
///
/// Unlike an involute pair, a cycloidal gear meshes correctly only with a partner whose rolling
/// circles match (same module, and the addendum rolling circle of one equal to the dedendum one
/// of the other). The tooth is thick `pi m / 2` on the pitch circle. With a rolling radius of half
/// the pitch radius the hypocycloid degenerates to a radial line.
#[derive(Debug, Clone, PartialEq)]
pub struct CycloidalGear {
    /// Gear module $m$ (mm/tooth).
    pub module: f64,
    /// Number of teeth $z$.
    pub teeth: usize,
    /// Face width.
    pub face_width: f64,
    /// Central bore radius.
    pub bore_radius: f64,
    /// Rolling-circle radius as a fraction of the pitch radius, in `(0, 0.5]`.
    pub rolling_ratio: f64,
}

impl CycloidalGear {
    /// A cycloidal spur gear with the usual rolling circle of 0.3 pitch radii.
    pub fn new(module: f64, teeth: usize, face_width: f64) -> Self {
        Self {
            module,
            teeth,
            face_width,
            bore_radius: module * (teeth as f64) * 0.15,
            rolling_ratio: 0.3,
        }
    }

    /// Pitch radius `m z / 2`.
    pub fn pitch_radius(&self) -> f64 {
        self.module * self.teeth as f64 / 2.0
    }

    /// Tip radius `R + m` (reduced where the tooth would otherwise come to a point).
    pub fn tip_radius(&self) -> f64 {
        self.pitch_radius() + self.module
    }

    /// Root radius `R - 1.25 m`.
    pub fn root_radius(&self) -> f64 {
        self.pitch_radius() - 1.25 * self.module
    }

    /// Point at parameter `theta` of the epicycloid (`outside = true`) or hypocycloid of the
    /// rolling circle of radius `rho` on the pitch circle of radius `r`; both start at `(r, 0)`.
    fn cycloid(r: f64, rho: f64, theta: f64, outside: bool) -> [f64; 2] {
        if outside {
            let k = (r + rho) / rho;
            [
                (r + rho) * theta.cos() - rho * (k * theta).cos(),
                (r + rho) * theta.sin() - rho * (k * theta).sin(),
            ]
        } else {
            let k = (r - rho) / rho;
            [
                (r - rho) * theta.cos() + rho * (k * theta).cos(),
                (r - rho) * theta.sin() - rho * (k * theta).sin(),
            ]
        }
    }

    /// The flank point at distance `radius` from the axis, as (polar angle offset from the pitch
    /// point, found by bisection on the monotone radius): returns the angle of the cycloid point
    /// at that radius measured from the x-axis.
    fn flank_angle(r: f64, rho: f64, radius: f64, outside: bool) -> f64 {
        // The radius is monotone in theta on the first arch (up to R + 2 rho outside, down to
        // R - 2 rho inside).
        let (mut lo, mut hi) = (
            0.0,
            PI * rho / (r + if outside { rho } else { -rho }).max(1e-9),
        );
        let at = |theta: f64| {
            let p = Self::cycloid(r, rho, theta, outside);
            p[0].hypot(p[1])
        };
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            let reached = if outside {
                at(mid) >= radius
            } else {
                at(mid) <= radius
            };
            if reached {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let p = Self::cycloid(r, rho, 0.5 * (lo + hi), outside);
        p[1].atan2(p[0])
    }

    /// The tooth outline, counter-clockwise (`samples` points per flank segment). Each tooth is
    /// centred on `i 2pi/z`; the flanks are mirrored about that line.
    pub fn generate_2d_profile(&self, samples: usize) -> Vec<[f64; 2]> {
        let z = self.teeth;
        let r = self.pitch_radius();
        let rho = (self.rolling_ratio.clamp(0.05, 0.5)) * r;
        let (rf, ra) = (self.root_radius(), self.tip_radius());
        let step = 2.0 * PI / z as f64;
        let psi = PI / (2.0 * z as f64); // half tooth thickness (angle) on the pitch circle
        let n = samples.max(4);
        // Half-angle of the tooth at radius `radius`: it widens inward along the hypocycloid and
        // narrows outward along the epicycloid.
        let half = |radius: f64| -> f64 {
            if radius <= r {
                psi + Self::flank_angle(r, rho, radius.max(r - 2.0 * rho + 1e-9), false)
            } else {
                psi - Self::flank_angle(r, rho, radius.min(r + 2.0 * rho - 1e-9), true)
            }
        };
        // Limit the tip so the tooth keeps at least a tenth of its width there, and the root so
        // the hypocycloid exists and neighbouring teeth do not overlap.
        let mut tip = ra.min(r + 2.0 * rho - 1e-6);
        while tip > r && half(tip) < 0.1 * psi {
            tip -= 0.01 * self.module;
        }
        let root = rf.max(r - 2.0 * rho + 1e-6);
        let limit = 0.5 * step * 0.98;
        let mut points = Vec::new();
        for i in 0..z {
            let c = i as f64 * step;
            let polar = |radius: f64, angle: f64| [radius * angle.cos(), radius * angle.sin()];
            // Root arc midpoint, then the leading flank from the root outwards to the tip.
            points.push(polar(rf, c - 0.5 * step));
            if rf < root {
                points.push(polar(rf, c - half(root).min(limit)));
            }
            let radii: Vec<f64> = (0..=2 * n)
                .map(|k| root + (tip - root) * k as f64 / (2 * n) as f64)
                .collect();
            for &radius in &radii {
                points.push(polar(radius, c - half(radius).min(limit)));
            }
            points.push(polar(tip, c));
            for &radius in radii.iter().rev() {
                points.push(polar(radius, c + half(radius).min(limit)));
            }
            if rf < root {
                points.push(polar(rf, c + half(root).min(limit)));
            }
        }
        points
    }

    /// The closed, outward-oriented solid of the gear (see [`InvoluteGear::generate_3d_mesh`]).
    pub fn generate_3d_mesh(&self) -> Mesh3D {
        let profile = self.generate_2d_profile(8);
        let bore = self.bore_radius.min(0.8 * self.root_radius()).max(0.0);
        toothed_gear_solid(
            format!("CycloidalGear_m{}_z{}", self.module, self.teeth),
            &profile,
            bore,
            self.face_width,
            2,
            0.0,
            false,
        )
    }
}

/// Internal (ring) involute spur gear: the teeth point inward from a cylindrical ring.
///
/// The tooth of an internal gear is the space of the external gear it meshes with, so its
/// thickness on the pitch circle is `pi m / 2` (zero profile shift), the addendum circle lies
/// *inside* the pitch circle at `R - m` and the root circle outside it at `R + 1.25 m`. Below
/// about 34 teeth the addendum circle would fall inside the base circle, where an involute does
/// not exist; the tooth tip is then limited to just outside the base circle.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalGear {
    /// Gear module $m$ (mm/tooth).
    pub module: f64,
    /// Number of teeth $z$ (must be large enough to keep the ring geometry valid, at least 20).
    pub teeth: usize,
    /// Pressure angle in degrees.
    pub pressure_angle_deg: f64,
    /// Face width.
    pub face_width: f64,
    /// Outer radius of the ring; it must exceed the root radius.
    pub outer_radius: f64,
}

impl InternalGear {
    /// An internal spur gear with a 20 degree pressure angle and a ring `3 m` thick behind the
    /// roots.
    pub fn new(module: f64, teeth: usize, face_width: f64) -> Self {
        let mut gear = Self {
            module,
            teeth,
            pressure_angle_deg: 20.0,
            face_width,
            outer_radius: 0.0,
        };
        gear.outer_radius = gear.root_radius() + 3.0 * module;
        gear
    }

    /// Pitch radius `m z / 2`.
    pub fn pitch_radius(&self) -> f64 {
        self.module * self.teeth as f64 / 2.0
    }

    /// Base radius `R cos(alpha)`.
    pub fn base_radius(&self) -> f64 {
        self.pitch_radius() * self.pressure_angle_deg.to_radians().cos()
    }

    /// Root radius `R + 1.25 m` (outside the pitch circle).
    pub fn root_radius(&self) -> f64 {
        self.pitch_radius() + 1.25 * self.module
    }

    /// Tooth tip radius `R - m`, limited to stay outside the base circle.
    pub fn tip_radius(&self) -> f64 {
        (self.pitch_radius() - self.module).max(self.base_radius() * 1.0005)
    }

    /// Center distance to an external pinion of `pinion_teeth` teeth of the same module:
    /// `m (z_ring - z_pinion) / 2`.
    pub fn center_distance(&self, pinion_teeth: usize) -> f64 {
        self.module * (self.teeth as f64 - pinion_teeth as f64) / 2.0
    }

    /// Half the angular width of a tooth on the circle of radius `r` (radians): it grows with the
    /// radius, `psi + inv(alpha_r) - inv(alpha)` with `psi = pi / (2 z)` the half thickness on the
    /// pitch circle and `cos(alpha_r) = r_b / r`.
    pub fn tooth_half_angle(&self, r: f64) -> f64 {
        let alpha = self.pressure_angle_deg.to_radians();
        let inv = |a: f64| a.tan() - a;
        let rb = self.base_radius();
        let alpha_r = (rb / r.max(rb)).clamp(-1.0, 1.0).acos();
        PI / (2.0 * self.teeth as f64) + inv(alpha_r) - inv(alpha)
    }

    /// The inner (toothed) outline, counter-clockwise about the axis: each tooth is centred on
    /// `i 2 pi / z`, with root arcs between teeth at the root radius, true involute flanks
    /// (`samples` points each) and a tip at the addendum radius.
    pub fn generate_2d_profile(&self, samples: usize) -> Vec<[f64; 2]> {
        let z = self.teeth;
        let (ra, rf) = (self.tip_radius(), self.root_radius());
        let step = 2.0 * PI / z as f64;
        let limit = 0.5 * step * 0.98;
        let n = samples.max(4);
        let polar = |r: f64, a: f64| [r * a.cos(), r * a.sin()];
        let half = |r: f64| self.tooth_half_angle(r).min(limit);
        let mut points = Vec::new();
        for i in 0..z {
            let c = i as f64 * step;
            points.push(polar(rf, c - 0.5 * step));
            // Leading flank from the root inwards to the tip: the angle rises as the radius falls.
            for k in 0..=n {
                let r = rf + (ra - rf) * k as f64 / n as f64;
                points.push(polar(r, c - half(r)));
            }
            points.push(polar(ra, c));
            for k in (0..=n).rev() {
                let r = rf + (ra - rf) * k as f64 / n as f64;
                points.push(polar(r, c + half(r)));
            }
        }
        points
    }

    /// The closed, outward-oriented ring solid: a cylindrical outer wall, the toothed inner wall
    /// and two flat annular end caps.
    pub fn generate_3d_mesh(&self) -> Mesh3D {
        let profile = self.generate_2d_profile(8);
        toothed_gear_solid(
            format!("InternalGear_m{}_z{}", self.module, self.teeth),
            &profile,
            self.outer_radius
                .max(self.root_radius() + 0.5 * self.module),
            self.face_width,
            2,
            0.0,
            true,
        )
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

    /// Minor (root) radius of the thread: the nominal radius minus the thread depth.
    pub fn minor_radius(&self) -> f64 {
        self.nominal_diameter / 2.0 - self.thread_depth()
    }

    /// Radial depth of the thread profile (crest to root), as a fraction of the pitch that
    /// depends on the standard: ISO 60 degrees `0.6134 P`, Acme and square `0.5 P`.
    pub fn thread_depth(&self) -> f64 {
        match self.thread_standard {
            ThreadStandard::IsoMetric60 => 0.6134 * self.pitch,
            ThreadStandard::Acme29 | ThreadStandard::Square => 0.5 * self.pitch,
        }
    }

    /// Radius of the threaded surface at axial phase `u` in `[0, pitch)` (the profile is periodic
    /// in the pitch): a root flat, a rising flank, a crest flat and a falling flank. The flank
    /// width follows the thread angle (30 degrees from the radial direction for ISO, 14.5 for
    /// Acme, 0 for the square thread).
    pub fn thread_radius(&self, u: f64) -> f64 {
        let u = u.rem_euclid(self.pitch);
        let (r_major, r_minor) = (self.nominal_diameter / 2.0, self.minor_radius());
        let depth = r_major - r_minor;
        let half_angle: f64 = match self.thread_standard {
            ThreadStandard::IsoMetric60 => 30.0,
            ThreadStandard::Acme29 => 14.5,
            ThreadStandard::Square => 0.0,
        };
        let flank = (depth * half_angle.to_radians().tan()).min(0.45 * self.pitch);
        let crest = match self.thread_standard {
            ThreadStandard::IsoMetric60 => 0.125 * self.pitch,
            ThreadStandard::Acme29 => 0.37 * self.pitch,
            ThreadStandard::Square => 0.5 * self.pitch,
        }
        .min(self.pitch - 2.0 * flank);
        let root = self.pitch - crest - 2.0 * flank;
        if u < root {
            r_minor
        } else if u < root + flank {
            r_minor + depth * (u - root) / flank.max(1e-12)
        } else if u < root + flank + crest {
            r_major
        } else {
            r_major - depth * (u - root - flank - crest) / flank.max(1e-12)
        }
    }

    /// Head outline radius at polar angle `theta` (hexagon with the given across-flats width, or a
    /// circle), and the head height.
    fn head_shape(&self) -> Option<(f64, bool, f64)> {
        let d = self.nominal_diameter;
        match self.head_type {
            BoltHeadType::Hexagonal => Some((1.6 * d, true, 0.65 * d)),
            BoltHeadType::SocketCap => Some((1.5 * d, false, d)),
            BoltHeadType::CountersunkFlat => Some((2.0 * d, false, 0.5 * d)),
            BoltHeadType::None => None,
        }
    }

    /// Generate the 3D mesh of the screw as a closed, outward-oriented solid: the threaded
    /// section (a helical profile of radius [`ThreadedScrew::thread_radius`]), the plain shank,
    /// the head (a hexagon or a cylinder; none for a stud) and flat end faces.
    ///
    /// The surface is one grid over polar angle and height: each row is a closed polar curve
    /// `r(theta)` at height `z`, consecutive rows are joined by quads, and a step in radius
    /// (shank to head, thread to shank) is a pair of rows at the same height, which becomes an
    /// annular face. The two end rows are closed with fans from the axis, valid because every
    /// row is star-shaped about the axis. The thread is a piecewise-linear approximation sampled
    /// at 96 angles and 24 heights per pitch.
    pub fn generate_3d_mesh(&self) -> Mesh3D {
        const N_THETA: usize = 96; // a multiple of 6 so a hexagon's corners are sampled exactly
        const ROWS_PER_PITCH: f64 = 24.0;
        let mut mesh = Mesh3D::new(format!("Screw_M{}_P{}", self.nominal_diameter, self.pitch));
        let r_major = self.nominal_diameter / 2.0;
        let thread_length = self.thread_length.clamp(0.0, self.total_length);
        let theta_at = |i: usize| 2.0 * PI * i as f64 / N_THETA as f64;

        // Rows: (height, radius as a function of the column's polar angle).
        type Radius<'a> = Box<dyn Fn(f64) -> f64 + 'a>;
        let mut rows: Vec<(f64, Radius)> = Vec::new();
        let pitch = self.pitch;
        let helical = move |z: f64| -> Radius<'_> {
            Box::new(move |theta: f64| self.thread_radius(z - pitch * theta / (2.0 * PI)))
        };
        let constant = |r: f64| -> Radius<'_> { Box::new(move |_| r) };

        if thread_length > 1e-9 {
            let n = (thread_length / pitch * ROWS_PER_PITCH).ceil().max(2.0) as usize;
            for k in 0..=n {
                let z = thread_length * k as f64 / n as f64;
                rows.push((z, helical(z)));
            }
        } else {
            rows.push((0.0, constant(r_major)));
        }
        if self.total_length > thread_length + 1e-9 {
            // Plain shank: a step from the thread to the full diameter, then up to the top.
            rows.push((thread_length, constant(r_major)));
            rows.push((self.total_length, constant(r_major)));
        }
        if let Some((across_flats, hexagonal, height)) = self.head_shape() {
            let z0 = self.total_length;
            let head = move |theta: f64| -> f64 {
                if hexagonal {
                    // Polar form of a regular hexagon with the given width across flats.
                    let apothem = across_flats / 2.0;
                    let sector = theta.rem_euclid(PI / 3.0) - PI / 6.0;
                    apothem / sector.cos()
                } else {
                    across_flats / 2.0
                }
            };
            rows.push((z0, Box::new(head)));
            rows.push((z0 + height, Box::new(head)));
        }

        for (z, radius) in &rows {
            for i in 0..N_THETA {
                let theta = theta_at(i);
                let r = radius(theta);
                mesh.add_vertex([r * theta.cos(), r * theta.sin(), *z]);
            }
        }
        let n_rows = rows.len();
        for j in 0..n_rows - 1 {
            for i in 0..N_THETA {
                let next = (i + 1) % N_THETA;
                let (a, b) = (j * N_THETA + i, j * N_THETA + next);
                let (c, d) = ((j + 1) * N_THETA + next, (j + 1) * N_THETA + i);
                mesh.add_triangle(a, b, c);
                mesh.add_triangle(a, c, d);
            }
        }
        let bottom = mesh.add_vertex([0.0, 0.0, rows[0].0]);
        let top = mesh.add_vertex([0.0, 0.0, rows[n_rows - 1].0]);
        let last = (n_rows - 1) * N_THETA;
        for i in 0..N_THETA {
            let next = (i + 1) % N_THETA;
            mesh.add_triangle(bottom, next, i);
            mesh.add_triangle(top, last + i, last + next);
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
