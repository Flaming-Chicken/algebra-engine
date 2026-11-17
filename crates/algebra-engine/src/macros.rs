//! # Ergonomic CAS DSL Macro Suite
//!
//! Provides declarative, zero-cost domain-specific macros for mathematical computations
//! in the Universal Rust Algebra Engine (URAE).
//!
//! ## Macro Categories
//! 1. **Calculus, Limits, Series & Padé Approximants**:
//!    - [`integrate!`](crate::integrate!), [`risch!`](crate::risch!), [`diff!`](crate::diff!),
//!      [`limit!`](crate::limit!), [`taylor!`](crate::taylor!), [`laurent!`](crate::laurent!),
//!      [`puiseux!`](crate::puiseux!), [`pade!`](crate::pade!)
//! 2. **Vector Calculus & Differential Operators**:
//!    - [`grad!`](crate::grad!), [`div!`](crate::div!), [`curl!`](crate::curl!),
//!      [`laplacian!`](crate::laplacian!), [`jacobian!`](crate::jacobian!), [`hessian!`](crate::hessian!)
//! 3. **Equation Solvers, Gröbner & Diophantine**:
//!    - [`solve!`](crate::solve!), [`solve_certified!`](crate::solve_certified!),
//!      [`simplify!`](crate::simplify!), [`simplify_certified!`](crate::simplify_certified!),
//!      [`groebner!`](crate::groebner!), [`diophantine!`](crate::diophantine!)
//! 4. **Differential Equations & Numerical Steppers**:
//!    - [`ode_linear!`](crate::ode_linear!), [`pde_heat_1d!`](crate::pde_heat_1d!),
//!      [`verlet!`](crate::verlet!), [`dopri5!`](crate::dopri5!), [`radau5!`](crate::radau5!)
//! 5. **Clifford Algebra, Quantum & Differential Forms**:
//!    - [`rotor!`](crate::rotor!), [`clifford_sandwich!`](crate::clifford_sandwich!),
//!      [`ext_diff!`](crate::ext_diff!), [`hodge!`](crate::hodge!),
//!      [`commutator!`](crate::commutator!), [`anticommutator!`](crate::anticommutator!)
//! 6. **Integral Transforms, PDE Representations & Block Diagrams**:
//!    - [`laplace!`](crate::laplace!), [`fourier!`](crate::fourier!), [`transfer_fn!`](crate::transfer_fn!),
//!      [`block_feedback!`](crate::block_feedback!), [`block_series!`](crate::block_series!),
//!      [`block_parallel!`](crate::block_parallel!)
//! 7. **Probabilistic Verification & Proof Certification**:
//!    - [`prob_test!`](crate::prob_test!), [`certify_zero!`](crate::certify_zero!)

// ============================================================================
// 1. Calculus, Limits, Series & Padé Approximants
// ============================================================================

/// Analytical integration using calculus extensions on `ExprGraph`.
#[macro_export]
macro_rules! integrate {
    ($graph:expr, $expr:expr, $var:expr) => {
        $crate::calculus::SymbolicCalculus::integrate($graph, $expr, $var)
    };
}

/// Transcendental integration using the complete Risch differential field algorithm.
#[macro_export]
macro_rules! risch {
    ($graph:expr, $expr:expr, $var:expr) => {
        $crate::risch::RischIntegrator::integrate($graph, $expr, $var)
    };
}

/// Symbolic differentiation with optional derivative order.
#[macro_export]
macro_rules! diff {
    ($graph:expr, $expr:expr, $var:expr) => {
        $crate::calculus::SymbolicCalculus::diff($graph, $expr, $var)
    };
    ($graph:expr, $expr:expr, $var:expr, $order:expr) => {{
        let mut curr = $expr;
        for _ in 0..$order {
            curr = $crate::calculus::SymbolicCalculus::diff($graph, curr, $var);
        }
        curr
    }};
}

/// Directional and two-sided limits using the Gruntz algorithm.
#[macro_export]
macro_rules! limit {
    ($graph:expr, $expr:expr, $var:expr, $point:expr) => {
        $crate::limits::LimitEngine::limit($graph, $expr, $var, $point)
    };
}

/// Taylor series expansion of an analytical expression.
#[macro_export]
macro_rules! taylor {
    ($graph:expr, $expr:expr, $var:expr, $point:expr, $order:expr) => {
        $crate::calculus::SymbolicCalculus::taylor_series($graph, $expr, $var, $point, $order)
    };
}

/// Laurent series expansion around isolated singular points.
#[macro_export]
macro_rules! laurent {
    ($graph:expr, $expr:expr, $var:expr, $point:expr, $pole_order:expr, $regular_order:expr) => {
        $crate::series::SymbolicSeries::laurent_series(
            $graph,
            $expr,
            $var,
            $point,
            $pole_order,
            $regular_order,
        )
    };
}

/// Puiseux fractional-power series expansion around branch points.
#[macro_export]
macro_rules! puiseux {
    ($graph:expr, $expr:expr, $var:expr, $point:expr, $denom_q:expr, $order:expr) => {
        $crate::series::SymbolicSeries::puiseux_series(
            $graph, $expr, $var, $point, $denom_q, $order,
        )
    };
}

/// Padé Approximant $[L/M]$ of `expr` about `point`, from its exact Taylor coefficients.
///
/// Evaluates to `AlgebraResult<ExprId>`; see [`crate::calculus::pade_approximant`].
#[macro_export]
macro_rules! pade {
    ($graph:expr, $expr:expr, $var:expr, $point:expr, $l:expr, $m:expr) => {
        $crate::calculus::pade_approximant($graph, $expr, $var, $point, $l, $m)
    };
}

// ============================================================================
// 2. Vector Calculus & Differential Operators
// ============================================================================

/// Gradient $\nabla f = \left[\frac{\partial f}{\partial x_1}, \dots, \frac{\partial f}{\partial x_n}\right]$.
#[macro_export]
macro_rules! grad {
    ($graph:expr, $expr:expr, [$($var:expr),* $(,)?]) => {{
        let mut grads = Vec::new();
        $(
            grads.push($crate::calculus::SymbolicCalculus::diff($graph, $expr, $var));
        )*
        grads
    }};
}

/// Divergence $\nabla \cdot \mathbf{F} = \sum_{i=1}^n \frac{\partial F_i}{\partial x_i}$.
#[macro_export]
macro_rules! div {
    ($graph:expr, [$($f:expr),* $(,)?], [$($var:expr),* $(,)?]) => {{
        let components = vec![$($f),*];
        let vars = vec![$($var),*];
        assert_eq!(components.len(), vars.len(), "Vector components and variables dimension mismatch");
        let mut terms = Vec::new();
        for (f_comp, v) in components.into_iter().zip(vars.into_iter()) {
            terms.push($crate::calculus::SymbolicCalculus::diff($graph, f_comp, v));
        }
        $graph.add(terms)
    }};
}

/// 3D Curl $\nabla \times \mathbf{F} = \left[\frac{\partial F_z}{\partial y} - \frac{\partial F_y}{\partial z}, \frac{\partial F_x}{\partial z} - \frac{\partial F_z}{\partial x}, \frac{\partial F_y}{\partial x} - \frac{\partial F_x}{\partial y}\right]$.
#[macro_export]
macro_rules! curl {
    ($graph:expr, [$fx:expr, $fy:expr, $fz:expr], [$x:expr, $y:expr, $z:expr]) => {{
        let dfz_dy = $crate::calculus::SymbolicCalculus::diff($graph, $fz, $y);
        let dfy_dz = $crate::calculus::SymbolicCalculus::diff($graph, $fy, $z);
        let cx = $graph.sub(dfz_dy, dfy_dz);

        let dfx_dz = $crate::calculus::SymbolicCalculus::diff($graph, $fx, $z);
        let dfz_dx = $crate::calculus::SymbolicCalculus::diff($graph, $fz, $x);
        let cy = $graph.sub(dfx_dz, dfz_dx);

        let dfy_dx = $crate::calculus::SymbolicCalculus::diff($graph, $fy, $x);
        let dfx_dy = $crate::calculus::SymbolicCalculus::diff($graph, $fx, $y);
        let cz = $graph.sub(dfy_dx, dfx_dy);

        vec![cx, cy, cz]
    }};
}

/// Laplacian $\nabla^2 f = \sum_{i=1}^n \frac{\partial^2 f}{\partial x_i^2}$.
#[macro_export]
macro_rules! laplacian {
    ($graph:expr, $expr:expr, [$($var:expr),* $(,)?]) => {{
        let mut second_derivs = Vec::new();
        $(
            let d1 = $crate::calculus::SymbolicCalculus::diff($graph, $expr, $var);
            let d2 = $crate::calculus::SymbolicCalculus::diff($graph, d1, $var);
            second_derivs.push(d2);
        )*
        $graph.add(second_derivs)
    }};
}

/// Jacobian matrix $J_{ij} = \frac{\partial f_i}{\partial x_j}$.
#[macro_export]
macro_rules! jacobian {
    ($graph:expr, [$($f:expr),* $(,)?], [$($var:expr),* $(,)?]) => {{
        let f_vec = vec![$($f),*];
        let var_vec = vec![$($var),*];
        let mut rows = Vec::new();
        for f_comp in f_vec {
            let mut row = Vec::new();
            for &v in &var_vec {
                row.push($crate::calculus::SymbolicCalculus::diff($graph, f_comp, v));
            }
            rows.push(row);
        }
        rows
    }};
}

/// Hessian matrix $H_{ij} = \frac{\partial^2 f}{\partial x_i \partial x_j}$.
#[macro_export]
macro_rules! hessian {
    ($graph:expr, $expr:expr, [$($var:expr),* $(,)?]) => {{
        let var_vec = vec![$($var),*];
        let mut rows = Vec::new();
        for &vi in &var_vec {
            let mut row = Vec::new();
            let d1 = $crate::calculus::SymbolicCalculus::diff($graph, $expr, vi);
            for &vj in &var_vec {
                let d2 = $crate::calculus::SymbolicCalculus::diff($graph, d1, vj);
                row.push(d2);
            }
            rows.push(row);
        }
        rows
    }};
}

// ============================================================================
// 3. Equation Solvers, Gröbner & Diophantine
// ============================================================================

/// Exact algebraic equation root solver.
#[macro_export]
macro_rules! solve {
    ($graph:expr, $eq:expr, $target:expr) => {
        $crate::solver::SymbolicSolver::solveset($graph, $eq, $target)
    };
}

/// Algebraic equation root solver with probabilistic certification.
#[macro_export]
macro_rules! solve_certified {
    ($graph:expr, $eq:expr, $target:expr, $eps:expr) => {
        $crate::solver::SymbolicSolver::solveset_certified($graph, $eq, $target, $eps)
    };
}

/// Algebraic E-Graph simplification.
#[macro_export]
macro_rules! simplify {
    ($graph:expr, $expr:expr) => {
        $crate::simplify::Simplifier::simplify($graph, $expr)
    };
    ($graph:expr, $expr:expr, goal = $goal:expr) => {
        $crate::simplify::Simplifier::simplify_with_goal($graph, $expr, $goal)
    };
}

/// Algebraic E-Graph simplification certified with Schwartz-Zippel upper error bound.
#[macro_export]
macro_rules! simplify_certified {
    ($graph:expr, $expr:expr, $eps:expr) => {
        $crate::simplify::Simplifier::simplify_certified($graph, $expr, $eps)
    };
}

/// Multi-variate polynomial Gröbner basis computation using Buchberger's algorithm.
#[macro_export]
macro_rules! groebner {
    ($polys:expr, $vars:expr, $order:expr) => {
        $crate::poly::GrobnerBasis::buchberger($polys, $vars, $order)
    };
}

/// Linear Diophantine equation solver $a x + b y = c$.
#[macro_export]
macro_rules! diophantine {
    ($a:expr, $b:expr, $c:expr) => {
        $crate::solver::DiophantineSolver::solve_linear_2var($a, $b, $c)
    };
}

// ============================================================================
// 4. Differential Equations & Numerical Steppers
// ============================================================================

/// Solve first-order linear ODE $y'(x) + P(x) y(x) = Q(x)$.
#[macro_export]
macro_rules! ode_linear {
    ($graph:expr, $p:expr, $q:expr, $x:expr) => {
        $crate::solver::SymbolicSolver::solve_ode_linear($graph, $p, $q, $x)
    };
}

/// Solve 1D Heat PDE $u_t = \alpha^2 u_{xx}$ via separation of variables.
#[macro_export]
macro_rules! pde_heat_1d {
    ($graph:expr, $alpha:expr, $x:expr, $t:expr) => {
        $crate::solver::SymbolicSolver::solve_pde_heat_1d($graph, $alpha, $x, $t)
    };
}

/// Symplectic Velocity-Verlet integrator for Hamiltonian systems.
#[macro_export]
macro_rules! verlet {
    ($f:expr, $t_span:expr, $y0:expr) => {{
        let cfg = $crate::ode::NumericalOdeConfig {
            method: $crate::ode::NumericalOdeMethod::VelocityVerlet,
            ..$crate::ode::NumericalOdeConfig::default()
        };
        $crate::ode::NumericalOdeSolver::solve($f, $t_span, $y0, &cfg)
    }};
}

/// Adaptive Runge-Kutta Dormand-Prince 5(4) integrator for non-stiff ODEs.
#[macro_export]
macro_rules! dopri5 {
    ($f:expr, $t_span:expr, $y0:expr, $tol:expr) => {{
        let cfg = $crate::ode::NumericalOdeConfig {
            method: $crate::ode::NumericalOdeMethod::DormandPrince54,
            rtol: $tol,
            atol: $tol * 1e-3,
            ..$crate::ode::NumericalOdeConfig::default()
        };
        $crate::ode::NumericalOdeSolver::solve($f, $t_span, $y0, &cfg)
    }};
}

/// Implicit Radau IIA 5th-order integrator for stiff differential-algebraic equations.
#[macro_export]
macro_rules! radau5 {
    ($f:expr, $t_span:expr, $y0:expr, $tol:expr) => {{
        let cfg = $crate::ode::NumericalOdeConfig {
            method: $crate::ode::NumericalOdeMethod::RadauIIA5,
            rtol: $tol,
            atol: $tol * 1e-3,
            ..$crate::ode::NumericalOdeConfig::default()
        };
        $crate::ode::NumericalOdeSolver::solve($f, $t_span, $y0, &cfg)
    }};
}

// ============================================================================
// 5. Clifford Algebra, Quantum & Differential Forms
// ============================================================================

/// Rotor $R = \cos(\theta/2) + \sin(\theta/2) B$ in Geometric Clifford Algebra.
#[macro_export]
macro_rules! rotor {
    ($theta:expr, $bivector_norm:expr) => {{
        let half_theta = ($theta) * 0.5;
        (half_theta.cos(), half_theta.sin() * ($bivector_norm))
    }};
}

/// Rotor sandwich rotation $v' = R v R^\dagger$.
#[macro_export]
macro_rules! clifford_sandwich {
    ($rotor:expr, $vector:expr) => {{
        let (c, s) = $rotor;
        let angle = 2.0 * s.atan2(c);
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        (
            cos_a * $vector.0 - sin_a * $vector.1,
            sin_a * $vector.0 + cos_a * $vector.1,
        )
    }};
}

/// Exterior derivative $\mathrm{d}\omega$ on Cartan differential forms.
#[macro_export]
macro_rules! ext_diff {
    ($form:expr, $coords:expr, $graph:expr) => {
        $form.exterior_derivative($coords, $graph)
    };
}

/// Hodge star operator $\star \omega$.
#[macro_export]
macro_rules! hodge {
    ($form:expr, $dim:expr, $graph:expr) => {
        $form.hodge_star($dim, $graph)
    };
}

/// Quantum Lie commutator $[A, B] = A B - B A$.
#[macro_export]
macro_rules! commutator {
    ($graph:expr, $a:expr, $b:expr) => {{
        let ab = $graph.mul([$a, $b]);
        let ba = $graph.mul([$b, $a]);
        $graph.sub(ab, ba)
    }};
}

/// Quantum anticommutator $\{A, B\} = A B + B A$.
#[macro_export]
macro_rules! anticommutator {
    ($graph:expr, $a:expr, $b:expr) => {{
        let ab = $graph.mul([$a, $b]);
        let ba = $graph.mul([$b, $a]);
        $graph.add([ab, ba])
    }};
}

// ============================================================================
// 6. Integral Transforms, PDE Representations & Block Diagrams
// ============================================================================

/// Laplace transform $\mathcal{L}\{f(t)\} = \int_0^\infty f(t) e^{-s t} \mathrm{d}t$.
#[macro_export]
macro_rules! laplace {
    ($graph:expr, $expr:expr, $t:expr, $s:expr) => {
        $crate::transforms::SymbolicTransforms::laplace_transform($graph, $expr, $t, $s)
    };
}

/// Continuous Fourier transform $\mathcal{F}\{f(t)\} = \int_{-\infty}^\infty f(t) e^{-i \omega t} \mathrm{d}t$.
#[macro_export]
macro_rules! fourier {
    ($graph:expr, $expr:expr, $t:expr, $w:expr) => {
        $crate::transforms::SymbolicTransforms::fourier_transform($graph, $expr, $t, $w)
    };
}

/// Transfer function representation $G(s) = \frac{N(s)}{D(s)}$ in Laplace control theory.
#[macro_export]
macro_rules! transfer_fn {
    ($num:expr, $den:expr) => {
        ($num, $den)
    };
}

/// Negative feedback loop block diagram algebra $\frac{G}{1 + G H}$.
#[macro_export]
macro_rules! block_feedback {
    ($graph:expr, $g:expr, $h:expr) => {{
        let one = $graph.integer(1);
        let gh = $graph.mul([$g, $h]);
        let den = $graph.add([one, gh]);
        $graph.div($g, den)
    }};
}

/// Series cascade block diagram algebra $G_1 \cdot G_2$.
#[macro_export]
macro_rules! block_series {
    ($graph:expr, $g1:expr, $g2:expr) => {
        $graph.mul([$g1, $g2])
    };
}

/// Parallel sum block diagram algebra $G_1 + G_2$.
#[macro_export]
macro_rules! block_parallel {
    ($graph:expr, $g1:expr, $g2:expr) => {
        $graph.add([$g1, $g2])
    };
}

// ============================================================================
// 7. Probabilistic Verification & Proof Certification
// ============================================================================

/// Schwartz-Zippel multi-prime equivalence verification test.
#[macro_export]
macro_rules! prob_test {
    ($graph:expr, $lhs:expr, $rhs:expr, $eps:expr) => {
        algebra_core::probabilistic::ProbabilisticVerifier::verify_equivalence(
            $graph, $lhs, $rhs, $eps,
        )
    };
}

/// Schwartz-Zippel multi-prime test for zero equivalence $f(\mathbf{x}) \stackrel{?}{=} 0$.
#[macro_export]
macro_rules! certify_zero {
    ($graph:expr, $expr:expr, $eps:expr) => {
        algebra_core::probabilistic::ProbabilisticVerifier::verify_zero_schwartz_zippel(
            $graph, $expr, $eps,
        )
    };
}

// ============================================================================
// 8. Cylindrical Algebraic Decomposition & Quantifier Elimination
// ============================================================================

/// Construct a Cylindrical Algebraic Decomposition of $\mathbb{R}^n$.
#[macro_export]
macro_rules! cad {
    ($polys:expr, $vars:expr) => {
        $crate::cad::CadEngine::decompose($polys, $vars, 1e-10)
    };
    ($polys:expr, $vars:expr, $eps:expr) => {
        $crate::cad::CadEngine::decompose($polys, $vars, $eps)
    };
}

/// Real Quantifier Elimination using Tarski-Seidenberg decision procedure.
#[macro_export]
macro_rules! quantifier_elim {
    ($quantifiers:expr, $polys:expr, $conditions:expr, $vars:expr) => {
        $crate::cad::QuantifierEliminator::eliminate($quantifiers, $polys, $conditions, $vars)
    };
}

// ============================================================================
// 9. Complex Engineering Shape Generators
// ============================================================================

/// Involute Spur or Helical Gear Generator macro.
#[macro_export]
macro_rules! gear {
    (module = $m:expr, teeth = $z:expr, width = $b:expr) => {
        $crate::cad::InvoluteGear::spur($m as f64, $z as usize, $b as f64)
    };
    (module = $m:expr, teeth = $z:expr, width = $b:expr, helix = $beta:expr) => {
        $crate::cad::InvoluteGear::helical($m as f64, $z as usize, $b as f64, $beta as f64)
    };
}

/// Parametric Threaded Fastener / Bolt Generator macro.
#[macro_export]
macro_rules! screw {
    (dia = $d:expr, pitch = $p:expr, length = $l:expr) => {
        $crate::cad::ThreadedScrew::metric_bolt($d as f64, $p as f64, $l as f64)
    };
    (socket, dia = $d:expr, pitch = $p:expr, length = $l:expr) => {
        $crate::cad::ThreadedScrew::socket_screw($d as f64, $p as f64, $l as f64)
    };
    (lead, dia = $d:expr, pitch = $p:expr, length = $l:expr) => {
        $crate::cad::ThreadedScrew::lead_screw($d as f64, $p as f64, $l as f64)
    };
}

/// NACA Aerodynamic Airfoil Generator macro.
#[macro_export]
macro_rules! airfoil {
    (code = $c:expr, chord = $chord:expr, span = $span:expr) => {
        $crate::cad::NacaAirfoil::new($c, $chord as f64, $span as f64)
    };
    (code = $c:expr, chord = $chord:expr, span = $span:expr, twist = $tw:expr) => {{
        let mut foil = $crate::cad::NacaAirfoil::new($c, $chord as f64, $span as f64);
        foil.twist_deg = $tw as f64;
        foil
    }};
}

/// Helical Compression/Tension Spring Generator macro.
#[macro_export]
macro_rules! spring {
    (wire = $d:expr, mean_dia = $D:expr, pitch = $p:expr, coils = $n:expr) => {
        $crate::cad::HelicalSpring::new($d as f64, $D as f64, $p as f64, $n as f64)
    };
}

// ============================================================================
// 10. Multiphysics Finite Element Analysis (FEA)
// ============================================================================

/// Execute Multiphysics FEA simulation.
#[macro_export]
macro_rules! fea_solve {
    ($mesh:expr, $physics:expr, $bcs:expr) => {
        $crate::pde::FeaEngine::solve($mesh, $physics, $bcs)
    };
}

/// Linear Elasticity FEA solver macro.
#[macro_export]
macro_rules! fea_elasticity {
    ($mesh:expr, E = $e:expr, nu = $nu:expr, bcs = $bcs:expr) => {{
        let physics = $crate::pde::PhysicsDiscipline::LinearElasticity {
            youngs_modulus: $e as f64,
            poissons_ratio: $nu as f64,
            density: 1.0,
            body_force: vec![0.0; $mesh.dim],
            plane_stress: true,
        };
        $crate::pde::FeaEngine::solve($mesh, &physics, $bcs)
    }};
}

/// Heat Conduction FEA solver macro.
#[macro_export]
macro_rules! fea_heat {
    ($mesh:expr, k = $k:expr, Q = $q:expr, bcs = $bcs:expr) => {{
        let physics = $crate::pde::PhysicsDiscipline::HeatConduction {
            thermal_conductivity: $k as f64,
            heat_capacity: 1.0,
            density: 1.0,
            internal_heat_source: $q as f64,
        };
        $crate::pde::FeaEngine::solve($mesh, &physics, $bcs)
    }};
}

// ============================================================================
// 11. Cross-Tooling Simulation Pipeline Bridge
// ============================================================================

/// End-to-End CAD-to-FEA Simulation Pipeline macro.
#[macro_export]
macro_rules! simulate_cad {
    ($cad_mesh:expr, $physics:expr, $bcs:expr) => {{
        let mut pipeline = $crate::pipeline::SimulationPipeline::from_cad($cad_mesh);
        pipeline.physics = $physics;
        pipeline.boundary_conditions = $bcs;
        pipeline.run_analysis()
    }};
}

// ============================================================================
// 12. Riemannian Curvature & Differential Algebra
// ============================================================================

/// Compute Christoffel symbols, Riemann, Ricci, and Einstein curvature tensors.
#[macro_export]
macro_rules! metric_curvature {
    ($metric:expr) => {
        $metric.compute_curvature()
    };
}

/// Compute Ritt-Wu characteristic set of a differential polynomial system.
#[macro_export]
macro_rules! char_set {
    ($system:expr) => {
        $crate::diffalg::RittWuReducer::characteristic_set($system)
    };
}

// ============================================================================
// 13. Complex Visualizer & WebGPU Shaders
// ============================================================================

/// Complex domain coloring RGB evaluation macro.
#[macro_export]
macro_rules! domain_color {
    ($re:expr, $im:expr) => {
        $crate::visualizer::DomainColoring::color_at($re as f64, $im as f64)
    };
}

/// 2D Vector field streamline tracing macro.
#[macro_export]
macro_rules! streamline_2d {
    ($seed:expr, $field:expr, $dt:expr, $steps:expr) => {
        $crate::visualizer::VectorFieldVisualizer::trace_streamline_2d(
            $seed, $field, $dt as f64, $steps,
        )
    };
}

/// WebGPU WGSL Grid Compute Shader generator macro.
#[macro_export]
macro_rules! wgsl_grid {
    ($expr_str:expr) => {
        $crate::gpu::WgslShaderGenerator::generate_grid_evaluation_shader($expr_str, 64)
    };
}

// ============================================================================
// 14. Symplectic Mechanics & Quantum Circuits
// ============================================================================

/// Symplectic energy-preserving Hamiltonian integration macro.
#[macro_export]
macro_rules! symplectic_integrate {
    ($sys:expr, q0 = $q0:expr, p0 = $p0:expr, h = $h:expr, steps = $steps:expr) => {
        $crate::symplectic::SymplecticIntegrator::integrate(
            $sys,
            $q0,
            $p0,
            $h as f64,
            $steps,
            $crate::symplectic::SymplecticOrder::YoshidaOrder4,
        )
    };
}

/// Quantum Bell state pair macro.
#[macro_export]
macro_rules! quantum_bell_state {
    () => {
        $crate::quantum_circuit::QuantumCircuit::bell_pair()
    };
}

/// Quantum GHZ state macro for N qubits.
#[macro_export]
macro_rules! quantum_ghz_state {
    ($n:expr) => {
        $crate::quantum_circuit::QuantumCircuit::ghz_state($n)
    };
}

// ============================================================================
// 15. Automatic Differentiation, Neural ODEs & PINNs
// ============================================================================

/// Forward-mode gradient evaluation macro: returns `(val, grad)`.
#[macro_export]
macro_rules! autodiff_forward {
    ($f:expr, $x:expr) => {
        $crate::autodiff::ForwardGradient::gradient($f, $x)
    };
}

/// Forward-mode exact Hessian matrix evaluation macro: returns `(val, grad, hess)`.
#[macro_export]
macro_rules! autodiff_hessian {
    ($f:expr, $x:expr) => {
        $crate::autodiff::ForwardGradient::hessian($f, $x)
    };
}

/// Continuous Neural ODE forward trajectory integration macro.
#[macro_export]
macro_rules! neural_ode_step {
    ($vector_field:expr, x0 = $x0:expr, theta = $theta:expr, t0 = $t0:expr, t1 = $t1:expr, steps = $steps:expr) => {
        $crate::neural_ode::NeuralOde::forward(
            $vector_field,
            $x0,
            $theta,
            $t0 as f64,
            $t1 as f64,
            $steps,
        )
    };
}

/// Physics-Informed Neural Network (PINN) Burgers residual macro.
#[macro_export]
macro_rules! pinn_burgers_residual {
    ($u_fn:expr, x = $x:expr, t = $t:expr, nu = $nu:expr) => {
        $crate::pinn::PinnResidual::burgers_residual($u_fn, $x as f64, $t as f64, $nu as f64)
    };
}

// ============================================================================
// 16. Topological Data Analysis (TDA) & Discrete Hodge Laplacian
// ============================================================================

/// Vietoris-Rips filtration macro.
#[macro_export]
macro_rules! vietoris_rips {
    ($points:expr, max_dim = $dim:expr, max_edge = $max_edge:expr) => {
        $crate::tda::VietorisRipsFiltration::from_point_cloud($points, $dim, $max_edge as f64)
    };
}

/// Persistent homology diagram computation macro.
#[macro_export]
macro_rules! persistence_diagram {
    ($simplices:expr) => {
        $crate::tda::PersistenceDiagram::compute($simplices)
    };
}

/// Discrete 0-Laplacian (Graph Laplacian) matrix macro.
#[macro_export]
macro_rules! discrete_laplacian_0 {
    ($n:expr, $edges:expr) => {
        $crate::tda::DiscreteHodge::laplacian_0($n, $edges)
    };
}

// ============================================================================
// 17. Non-Euclidean Computational Geometry (Hyperbolic & Spherical)
// ============================================================================

/// Poincaré disk geodesic distance macro: `hyperbolic_dist!(p1, p2)`.
#[macro_export]
macro_rules! hyperbolic_dist {
    ($p1:expr, $p2:expr) => {
        $p1.distance($p2)
    };
}

/// Spherical great-circle distance macro: `spherical_dist!(s1, s2)`.
#[macro_export]
macro_rules! spherical_dist {
    ($s1:expr, $s2:expr) => {
        $s1.great_circle_distance($s2)
    };
}

/// Hyperbolic triangle area defect macro (Gauss-Bonnet): `hyperbolic_triangle_area!(alpha, beta, gamma)`.
#[macro_export]
macro_rules! hyperbolic_triangle_area {
    ($alpha:expr, $beta:expr, $gamma:expr) => {
        $crate::noneuclidean::HyperbolicTriangle::area_defect(
            $alpha as f64,
            $beta as f64,
            $gamma as f64,
        )
    };
}

/// Spherical triangle area excess macro (Girard): `spherical_triangle_area!(alpha, beta, gamma)`.
#[macro_export]
macro_rules! spherical_triangle_area {
    ($alpha:expr, $beta:expr, $gamma:expr) => {
        $crate::noneuclidean::SphericalPoint::triangle_excess_area(
            $alpha as f64,
            $beta as f64,
            $gamma as f64,
        )
    };
}

// ============================================================================
// 18. SMT-LIB2 Automated Theorem Prover (ATP) Bridge & Formal Proofs
// ============================================================================

/// SMT Problem builder macro: `smt_problem!(logic)`.
#[macro_export]
macro_rules! smt_problem {
    ($logic:expr) => {
        $crate::smt::SmtProblem::with_logic($logic)
    };
}

/// SMT Assertion macro: `smt_assert!(problem, expr)`.
#[macro_export]
macro_rules! smt_assert {
    ($problem:expr, $expr:expr) => {
        $problem.assert($expr);
    };
}

/// Formal verification constructive proof certificate macro: `proof_certificate!(name, [(var, typ), ...], conclusion, tactic)`.
#[macro_export]
macro_rules! proof_certificate {
    ($name:expr, [ $( ($var:expr, $typ:expr) ),* ], $conclusion:expr, $tactic:expr) => {
        $crate::smt::ProofCertificate::new(
            $name,
            vec![ $( ($var.to_string(), $typ.to_string()) ),* ],
            $conclusion,
            $tactic,
        )
    };
}

// ============================================================================
// 19. Geometric Deep Learning & SE(3) Equivariance
// ============================================================================

/// Real Spherical Harmonic evaluation macro: `spherical_harmonic!(l, m, theta, phi)`.
#[macro_export]
macro_rules! spherical_harmonic {
    ($l:expr, $m:expr, $theta:expr, $phi:expr) => {
        $crate::geometric_dl::SphericalHarmonics::real_y_lm($l, $m, $theta as f64, $phi as f64)
    };
}

/// Clebsch-Gordan coefficient macro: `clebsch_gordan!(l1, m1, l2, m2, l, m)`.
#[macro_export]
macro_rules! clebsch_gordan {
    ($l1:expr, $m1:expr, $l2:expr, $m2:expr, $l:expr, $m:expr) => {
        $crate::geometric_dl::ClebschGordan::coefficient($l1, $m1, $l2, $m2, $l, $m)
    };
}

/// Clifford Multivector Layer forward transformation macro: `clifford_layer_forward!(layer, mv)`.
#[macro_export]
macro_rules! clifford_layer_forward {
    ($layer:expr, $mv:expr) => {
        $layer.forward($mv)
    };
}

// ============================================================================
// 20. Quantum Field Theory (QFT) & Feynman Diagram Amplitude Calculus
// ============================================================================

/// Dirac gamma matrix trace macro: `dirac_trace!(indices)`.
#[macro_export]
macro_rules! dirac_trace {
    ($indices:expr) => {
        $crate::qft::DiracGamma::trace_gamma($indices)
    };
}

/// Slashed 4-momentum product trace macro: `dirac_slashed_trace!(momenta)`.
#[macro_export]
macro_rules! dirac_slashed_trace {
    ($momenta:expr) => {
        $crate::qft::DiracGamma::trace_slashed_product($momenta)
    };
}

/// Invariant angle bracket macro: `spinor_bracket_angle!(s1, s2)`.
#[macro_export]
macro_rules! spinor_bracket_angle {
    ($s1:expr, $s2:expr) => {
        $crate::qft::SpinorHelicity::angle_bracket($s1, $s2)
    };
}

/// Invariant square bracket macro: `spinor_bracket_square!(s1, s2)`.
#[macro_export]
macro_rules! spinor_bracket_square {
    ($s1:expr, $s2:expr) => {
        $crate::qft::SpinorHelicity::square_bracket($s1, $s2)
    };
}

/// Parke-Taylor tree-level MHV gluon amplitude macro: `parke_taylor_mhv!(spinors, i, j)`.
#[macro_export]
macro_rules! parke_taylor_mhv {
    ($spinors:expr, $i:expr, $j:expr) => {
        $crate::qft::SpinorHelicity::parke_taylor_tree_mhv($spinors, $i, $j)
    };
}

/// Passarino-Veltman vector reduction coefficient macro: `passarino_veltman_b1!(p_sq, m1_sq, m2_sq, eps, mu_sq)`.
#[macro_export]
macro_rules! passarino_veltman_b1 {
    ($p_sq:expr, $m1_sq:expr, $m2_sq:expr, $eps:expr, $mu_sq:expr) => {
        $crate::qft::PassarinoVeltman::b1(
            $p_sq as f64,
            $m1_sq as f64,
            $m2_sq as f64,
            $eps as f64,
            $mu_sq as f64,
        )
    };
}

// ============================================================================
// 21. Holonomic D-Modules, Non-Commutative Weyl Algebra & Zeilberger
// ============================================================================

/// Weyl coordinate operator macro: `weyl_x!(n_vars, var_idx, power)`.
#[macro_export]
macro_rules! weyl_x {
    ($n:expr, $var:expr, $pow:expr) => {
        $crate::weyl_dmodules::WeylOperator::x($n, $var, $pow)
    };
}

/// Weyl derivative operator macro: `weyl_d!(n_vars, var_idx, power)`.
#[macro_export]
macro_rules! weyl_d {
    ($n:expr, $var:expr, $pow:expr) => {
        $crate::weyl_dmodules::WeylOperator::d($n, $var, $pow)
    };
}

/// Weyl operator commutator macro: `weyl_commute!(op1, op2)`.
#[macro_export]
macro_rules! weyl_commute {
    ($op1:expr, $op2:expr) => {
        $op1.commutator(&$op2)
    };
}

/// Zeilberger binomial sum proof macro: `zeilberger_binomial_proof!()`.
#[macro_export]
macro_rules! zeilberger_binomial_proof {
    () => {
        $crate::weyl_dmodules::ZeilbergerAlgorithm::prove_binomial_sum()
    };
}

/// Almkvist-Zeilberger Gaussian integral ODE macro: `almkvist_zeilberger_gaussian!()`.
#[macro_export]
macro_rules! almkvist_zeilberger_gaussian {
    () => {
        $crate::weyl_dmodules::AlmkvistZeilberger::gaussian_integral_ode()
    };
}

// ============================================================================
// 22. Conformal Field Theory (CFT), Virasoro & Affine Kac-Moody Algebras
// ============================================================================

/// Virasoro Lie bracket macro: `virasoro_bracket!(m, n, c)`.
#[macro_export]
macro_rules! virasoro_bracket {
    ($m:expr, $n:expr, $c:expr) => {
        $crate::cft::VirasoroAlgebra::bracket($m as i64, $n as i64, $c as f64)
    };
}

/// Affine Kac-Moody su(2)_k current bracket macro: `kac_moody_bracket!(a, m, b, n, k)`.
#[macro_export]
macro_rules! kac_moody_bracket {
    ($a:expr, $m:expr, $b:expr, $n:expr, $k:expr) => {
        $crate::cft::KacMoodyAlgebra::su2_bracket($a, $m as i64, $b, $n as i64, $k as f64)
    };
}

/// Sugawara central charge macro: `sugawara_central_charge!(dim_g, dual_coxeter, k)`.
#[macro_export]
macro_rules! sugawara_central_charge {
    ($dim_g:expr, $dual_coxeter:expr, $k:expr) => {
        $crate::cft::KacMoodyAlgebra::sugawara_central_charge(
            $dim_g as f64,
            $dual_coxeter as f64,
            $k as f64,
        )
    };
}

/// Minimal model Kac table conformal weight macro: `kac_conformal_weight!(m, r, s)`.
#[macro_export]
macro_rules! kac_conformal_weight {
    ($m:expr, $r:expr, $s:expr) => {
        $crate::cft::VirasoroAlgebra::kac_conformal_weight($m, $r, $s)
    };
}

// ============================================================================
// 23. Exceptional Lie Groups, Octonions & E8 Root System
// ============================================================================

/// Octonion constructor macro: `octonion!(c0, c1, c2, c3, c4, c5, c6, c7)`.
#[macro_export]
macro_rules! octonion {
    ($c0:expr, $c1:expr, $c2:expr, $c3:expr, $c4:expr, $c5:expr, $c6:expr, $c7:expr) => {
        $crate::exceptional_lie::Octonion::new([
            $c0 as f64, $c1 as f64, $c2 as f64, $c3 as f64, $c4 as f64, $c5 as f64, $c6 as f64,
            $c7 as f64,
        ])
    };
}

/// Octonion multiplication macro: `octonion_mul!(x, y)`.
#[macro_export]
macro_rules! octonion_mul {
    ($x:expr, $y:expr) => {
        $x.mul(&$y)
    };
}

/// Octonion associator macro: `octonion_associator!(x, y, z)`.
#[macro_export]
macro_rules! octonion_associator {
    ($x:expr, $y:expr, $z:expr) => {
        $x.associator(&$y, &$z)
    };
}

/// Albert Exceptional Jordan algebra determinant macro: `albert_det!(albert)`.
#[macro_export]
macro_rules! albert_det {
    ($albert:expr) => {
        $albert.determinant()
    };
}

/// E8 Weyl reflection macro: `e8_weyl_reflect!(v, alpha)`.
#[macro_export]
macro_rules! e8_weyl_reflect {
    ($v:expr, $alpha:expr) => {
        $crate::exceptional_lie::E8Lattice::weyl_reflect(&$v, &$alpha)
    };
}

// ============================================================================
// 24. p-Adic Hodge Theory, Fontaine Modules & Galois Representations
// ============================================================================

/// Fontaine Filtered (Phi, N)-module constructor macro.
#[macro_export]
macro_rules! fontaine_module {
    ($dim:expr, $p:expr, $frob:expr, $mono:expr, $weights:expr) => {
        $crate::padic_hodge::FontaineModule::new($dim, $p as u64, $frob, $mono, $weights)
    };
}

/// Tate twist on Hodge-Tate weights macro: `tate_twist_weights!(weights, r)`.
#[macro_export]
macro_rules! tate_twist_weights {
    ($weights:expr, $r:expr) => {
        $crate::padic_hodge::TateTwist::twist_hodge_tate_weights(&$weights, $r as i64)
    };
}

/// Tate twist on Frobenius eigenvalues macro: `tate_twist_frob!(eigenvalues, p, r)`.
#[macro_export]
macro_rules! tate_twist_frob {
    ($evals:expr, $p:expr, $r:expr) => {
        $crate::padic_hodge::TateTwist::twist_frobenius_eigenvalues(&$evals, $p as u64, $r as i64)
    };
}

// ============================================================================
// 25. N-Dimensional CAD Topology & Isogeometric Analysis (IGA)
// ============================================================================

/// Cox-de Boor B-spline basis evaluation macro: `cox_de_boor_basis!(i, p, u, knots)`.
#[macro_export]
macro_rules! cox_de_boor_basis {
    ($i:expr, $p:expr, $u:expr, $knots:expr) => {
        $crate::iga::CoxDeBoor::basis_function($i, $p, $u as f64, &$knots)
    };
}

/// IGA stiffness entry integration macro: `iga_stiffness_entry_2d!(patch, a, b)`.
#[macro_export]
macro_rules! iga_stiffness_entry_2d {
    ($patch:expr, $a:expr, $b:expr) => {
        $crate::iga::IgaStiffnessMatrix::integrate_stiffness_entry_2d(&$patch, $a, $b)
    };
}
