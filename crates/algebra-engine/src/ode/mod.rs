//! # `algebra_engine::ode`
//!
//! Ordinary Differential Equation (ODE) computational subsystem for URAE.
//!
//! Encompasses:
//! - **Symbolic Solvers**: Exact 1st-order linear, separable, Bernoulli, exact, 2nd-order constant coefficients, Cauchy-Euler, systems of linear ODEs.
//! - **Numerical Solvers**: Adaptive DOPRI5, Tsitouras 5(4), Radau IIA (stiff), Symplectic Velocity Verlet, custom Butcher Tableaus.
//!
//! ## References
//! - Dormand, J. R., & Prince, P. J. (1980). A family of embedded Runge-Kutta formulae. *J. Comput. Appl. Math.*, 6(1), 19–26.
//! - Tsitouras, C. (2011). Runge-Kutta pairs of order 5(4) satisfying only the first column simplifying assumption. *Comput. Math. Appl.*, 62(2), 770–775.
//! - Hairer, E., Nørsett, S. P., & Wanner, G. (1993/1996). *Solving Ordinary Differential Equations I & II*. Springer.
//! - Butcher, J. C. (2016). *Numerical Methods for Ordinary Differential Equations* (3rd ed.). Wiley.

pub mod numerical;
pub mod symbolic;

pub use numerical::{
    ButcherTableau, NumericalOdeConfig, NumericalOdeMethod, NumericalOdeSolver, OdeTrajectory,
};
pub use symbolic::{OdeType, SymbolicOdeSolver};
