# Changelog

All notable changes to `algebra-engine` (URAE) are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Semantic Versioning.

## [Unreleased]

### Added and fixed

- Added: arbitrary-precision evaluation of constant expressions: `N(pi, 50)`, `evalf(sqrt(2), 100)`, `float(exp(1), 30)`, `N pi, 25` give the requested number of correctly rounded significant digits (`pi`, `e`, `+ - * /`, integer and real powers, `sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`, `atan`, `asin`, `acos`, `sinh`, `cosh`, `tanh`, `abs`), up to 100,000 digits. Fixed: `N(expr, digits)` was a parse error, and there was no way to get a decimal value of `gamma(5)`-free constants beyond the 15 digits of an `f64`. Checked against the published digits of the standard constants and against identities that must hold to 85 to 95 of 100 digits (`sin^2 + cos^2 = 1`, `exp(ln x) = x`, `asin x + acos x = pi/2`, argument reduction of `sin(100)`).
- Added: `dsolve` solves homogeneous linear equations of any order up to six with constant coefficients (`y''' - y = 0`, `y'''' + 2 y'' + y = 0`, repeated and complex roots) and Euler equations (`x^2 y'' + x y' + y = 0`, `x^3 y''' - 3 x y' + 3 y = 0`) exactly from the factorisation of the characteristic polynomial, and Bernoulli equations (`y' + y = y^2`, `y' - y/x = x y^3`). Fixed: a product of powers of x with a negation (`x^2 * (-2 x)`) is merged into one power before integrating; an integrating factor such as `exp(2 ln x)` is `x^2`.
- Fixed: `dsolve(y' = y^2)` returned `y = C1 exp(x)` (the solution of `y' = y`) and `dsolve(y' = y*(1 - y))` returned `y = C1`: the command read the coefficients of `y''`, `y'` and `y` by substituting 0 and 1 and never checked that the equation is linear. It now checks the model against the equation at several points, solves separable equations `y' = f(x) g(y)` (`y' = y^2`, `x y^2`, `y (1 - y)`, `y / x`), reports an implicit solution when `y` cannot be isolated, and solves constant-coefficient non-homogeneous second-order equations by variation of parameters (`y'' + y = x`, `y'' - y = exp(x)`, `y'' - 2 y' + y = exp(x)`, `y'' + 9 y = cos(2 x)`). `exp(u) * exp(v)` integrates as `exp(u + v)`.
- Fixed: `SymbolicMatrix::eigenvectors` was a 2x2 formula: for a 3x3 matrix it returned vectors with two components, and for a diagonal matrix the zero vector. It now returns a basis of the null space of `A - lambda I` for each distinct eigenvalue (exactly, or numerically for a real irrational eigenvalue), with the right multiplicity (an identity matrix has an eigenspace of full dimension, a Jordan block of dimension one). Added: `RootOf(p, x, k)` evaluates numerically (the `k`-th root of a rational-coefficient polynomial in a canonical order), so eigenvalues and solutions without a closed form can be evaluated and checked.
- Added: the solver handles a polynomial of degree two or more in one function of the unknown by substitution and inversion (`cos(x)^2 = 1/4` gives `±π/3, ±2π/3` instead of an unresolved `RootOf`; also `tan(x)^2 = 3`, `ln(x)^2 - 3 ln(x) + 2`, `sin(x)^2 - sin(x)/2`).
- Added: limits at infinity that no asymptotic rule covers (`x*sin(1/x)`, `x*(sqrt(x^2+1) - x)`, `(x+1)^(1/ln(x+1))`, `sin(x)/x`, `exp(x)/x^3`) are estimated numerically from values at 10^2 .. 10^5 (divergence by growth or overflow, convergence by Aitken extrapolation, identified as a rational, a multiple of e or pi, or ln 2) and labelled "numerical estimate ... not a proof"; oscillating functions are still `Undefined`. Fixed: `lim ln(x)/x` printed `1 / oo` instead of `0`.
- Fixed: `series(-atan(-x^x))` ran for 281 seconds (debug build) and `taylor(x^x)` for 27 seconds, and the latter returned terms such as `ln(0)` and `1/0` as if they were a series. The series command now gives up with an error when the derivatives grow too fast for the requested order (a time budget on the differentiation method), and reports a function that is not analytic at the point (`x^x` at 0) instead of printing singular terms.
- Fixed: integrating a rational function whose denominator has a repeated irreducible quadratic factor (`1/(x^2+1)^2`, `1/(x^2+2*x+2)^2`, `x/(x^2+1)^2`) overflowed the stack and aborted the process. A new partial-fraction integrator over the rationals (complete factorisation of the denominator, reduction formula for `1/(t^2 + d)^k`) handles these first; it is also used when the older code gives up, which adds `1/(x^4+1)`, `x^2/(x^4+1)` and mixed denominators.
- Changed: Taylor series of expressions built from rational constants, the variable, `+ - * /`, powers and `exp`, `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`, `ln`, `atan`, `sqrt` are computed by exact power-series arithmetic instead of repeated differentiation. `series(exp(sin(x)), x, 0, 12)` took 22 seconds (release build); the order-15 series of `exp(exp(x) - 1)`, `ln(1 + tan(x))` and `sin(sin(exp(x) - 1))` now take milliseconds. Expressions outside that set use the old method.
- Added: integrals of a polynomial times or divided by `sqrt(q)` for a quadratic `q` (`sqrt(x^2+1)`, `sqrt(x^2+4*x+5)`, `x^2/sqrt(x^2+1)`, `1/sqrt(x^2+2*x+5)`, `x^2*sqrt(1-x^2)`), a polynomial times `atan`, `asin` or `acos` and times a power of `ln` (`x*atan(x)`, `x*asin(x)`, `x*ln(x)^2`), and integer powers of `sin` and `cos` of a linear argument with `1/sin` and `1/cos` (`sin(x)^4`, `1/cos(x)`); all were errors. Every result is checked by differentiating it.
- Added: closed-form integration of a polynomial times `exp`, `sin` or `cos` of a linear argument (tabular integration by parts: `x*exp(-x)`, `x^2*exp(3*x)`, `x*sin(2*x)`), of `exp(u)*sin(v)` and `exp(u)*cos(v)`, of products of two sines and cosines of linear arguments (`sin(x)*cos(2*x)`) and of `abs(a*x + b)`. These were errors ("Multi-factor product integration not matched"). Every result is checked by differentiating it.
- Fixed: a bare matrix or vector literal (`[[1,2],[3,4]]`, `[1,2,3]`) was echoed as `matrix(row(matrix(row(1, 2)), ...))`; it now prints as a grid.
- Added: integer functions `totient`, `next_prime`, `prev_prime`, `divisors`, `sigma`, `tau`, `mobius`, `mod_inverse`, `pow_mod` and `primitive_root`, as calls (`totient(36)`) or commands (`totient 36`); previously `nextprime(100)` and `totient(36)` were echoed back unevaluated.
- Fixed: the same product printed differently depending on what had been typed earlier in the session (`C1 · cos(x)` or `cos(x) · C1`), because the stored factor order follows the order symbols were first seen; products of commutative scalar factors now print in a fixed order (numbers, symbols and powers by name, functions by name). A product of two matrix literals (`[[1,2],[3,4]]*[[0,1],[1,0]]`) is evaluated as a matrix product instead of echoed.
- Fixed: the call spellings `series(f, x, 0, 5)`, `taylor(...)`, `isprime(n)`, `factorint(n)`, `apart(f)`, `inverse(M)`, `det(M)`, `rank(M)`, `limit(f, x, a)`, `dsolve(...)` and the infix `a mod b` were parsed as expressions and echoed back unevaluated; they now evaluate like their command spellings.
- Fixed: `integrate f from a to b` (and `integral of f from a to b`) returned the antiderivative with the bounds silently dropped; it now returns the value of the definite integral, accepts `dx` before `from`, and refuses a multi-variable integrand with bounds instead of guessing. `factor 360` returned `360`; it now returns the prime factorisation.
- Changed: `missing_docs` is denied in `urae`, `cat-tangent` and `cat-numbers` (the remaining crates are being documented; see `docs/PLANNING.md`).
- Changed: the `solve` command says which solutions are real and which are complex (`x^3 - 1 = 0`: "1 real, 2 non-real (complex) solutions") and marks equations with periodic or multivalued functions (`exp(x) = -1`, `sin(x) = a`) as giving the principal solution only. New `executor::classify_solution` and `SolutionKind`; the counts are in the result's metadata (`real_solutions`, `nonreal_solutions`, `coverage`).
- Changed: the `simplify` command now says where the result is valid when it dropped a restriction (`x/x` gives `1` with `valid where x ≠ 0`): in the CLI REPL, in the JSON response (`conditions`) and as the notebook's domain info.
- Added: `Simplifier::simplify_with_conditions` and `Expr::simplify_with_conditions` report the points a simplification excludes (`x / x` is `1` for `x ≠ 0`; `exp(ln x)` is `x` for `x > 0`; `tan(x) cos(x)` is `sin(x)` for `cos(x) ≠ 0`).
- Fixed: input nested a hundred levels deep (`sin(sin(...))`, `x^(x^(...))`) overflowed the stack and aborted the process; the parser and the simplifier now refuse nesting beyond 64 levels with an error.
- Added: a deterministic panic hunt (`panic_hunt_tests`) over the parser, simplifier, differentiation and integration; the production panic count is recounted with the compiler (three documented overload panics).
- Fixed: `SmtExpr::from_graph` turned big integers, big rationals, scientific notation and constants into `0.0`, rounded rationals through `f64` and accepted `NaN`; numbers are now exact, and a constant or non-finite float is an error. Negative numerals print as `(- n)`, as SMT-LIB requires.
- Added: arithmetic in `GF(p^n)` (`GaloisField::arithmetic`: validated field, `+`, `-`, `*`, inverse, powers, Frobenius, irreducibility test) and `ModuloInt::{inverse, div, sub, checked_pow, checked_new}`.
- Added: `poly::factor::factor_integer_polynomial` and `factor_rational_polynomial`, complete factorisation of univariate polynomials over `Z` and `Q` (Zassenhaus with Hensel lifting); `galois::galois_group_over_q`, the Galois group of any polynomial of degree at most 4 (and of products of quadratics). Fixed: `FieldExtension::quadratic(d)` for perfect squares.
- Added: `ReedSolomonCode` is a real code over `GF(2^8)`: systematic `encode`, `syndromes`, and `decode` (Berlekamp–Massey, Chien search, Forney) correcting up to `(n - k) / 2` wrong symbols. Fixed: elliptic-curve discriminant overflow, truncated factoring of the discriminant, the split-reduction criterion and the `F_2` point count in `crypto.rs`.
- Fixed: `cat-numbers` surreals carry exact dyadic values like `algebra-advanced`: sums, products and order are exact (`{0 | 3}` is `1`), tiny dyadics are no longer mistaken for the infinitesimal stand-in, and the `omega`/`epsilon` stand-ins are flagged `transfinite`.
- Fixed: `EquivalenceClassifier::classify` separates syntactic identity from algebraic equality (and says `None` when neither holds); Colombeau products carry their `epsilon` powers (`delta * delta` is `epsilon^-2`, `Theta * Theta` is `Theta`); `TangentFunctor::lift_hypergraph` covers every standard operation and refuses the rest with an error; the elaborator tracks the dimension of `^` and function calls and lowers the imaginary part of complex literals; `Domain::category` knows `N` is a semiring, `Z/nZ` is a field exactly for prime `n`, the adeles are not a field and `C` is algebraically closed.
- Fixed: `SimplicialComplex::betti_numbers` is exact (a circle is `[1, 1]`; it was `[1, 0]`) and `add_simplex` treats its input as a set; `SurrealNo` carries exact dyadic values and orders by them (the `f64` midpoint shortcut mis-ordered numbers such as `{0 | 3} = 1`); `PolynomialSystemSolver` branches over the roots at every level (`x^2 - y = 0, y^2 - 1 = 0` had no solutions, now four).
- Fixed: `solve_poisson_fem_1d` ignored non-zero Dirichlet values; the clamped B-spline basis was zero at its end point; `matrix_rank` of `[[0, 1], [0, 0]]` was 0 and `matrix_exponential` was unscaled (garbage for `e^{-50 I}`); `0.0` and `-0.0` (and NaNs with different payloads) were different numbers; the Smith normal form of `diag(2, 3)` was `[2, 3]` instead of `[1, 6]` (now `i128` with checked arithmetic); the Shirshov factorisation took the shortest suffix.
- Documentation reorganised into a master plan (`docs/PLANNING.md`), a design (`docs/DESIGN.md`) and a defect ledger (`docs/LEDGER.md`), with `docs/specs/` and `docs/audits/` for specifications and historical evidence; a documentation map (`docs/README.md`); the ledger's colliding ids renamed (`R1`–`R13`); relative links throughout; the test counts refreshed.
- Integration: `P(x) * (a x + b)^e` for a polynomial `P` and a non-integer rational `e` (`x * sqrt(x + 1)`, `x / sqrt(x + 1)`, `x^2 * (x + 1)^(3/2)`, ...) by exact substitution; it used to be an error.
- Fixed: `Ai`, `Y_n`, `K_n`, `J_n` for large arguments, `ζ`, `B(a, b)`, `ln Γ`, `erf` and `erfc` returned wrong values (`Ai(1)`, `Y0(1)`, `K0(1)`, `ζ(0.5)`, `J0(40) = -1.5e8`, `B(200, 200) = NaN`); they are now accurate to about 1e-12 and checked against tables and identities.
- Criterion benchmark suites for `algebra-core`, `algebra-engine`, `algebra-advanced` and `urae`, a rewritten `auto-promoters` benchmark and a `[profile.bench]` with thin LTO; fourteen strengthened test files.
- `big_prime_factors` factors semiprimes up to 127 bits with a Montgomery `u128` Pollard rho (an 89-bit product: 7.5 s to 0.6 s in a debug build; 97-bit products now finish) and splits prime squares exactly.
- `mcts::{LearnedPolicy, PolicyFeatures}`, `MctsPolicyHead::compute_policy_with`, `MctsSearchEngine::{with_policy, train_policy, search_with_root_stats}`: a trainable linear policy that starts equal to the heuristic and is trained from the engine's own search statistics; JSON persistence.
- `zeilberger_proof::{prove_recurrence, check_certificate, Poly2, SymbolicTerm}`: proves a Zeilberger recurrence for all `n` by rebuilding the certificate as a rational function of `n` and checking the telescoping identity symbolically.
- `CadEngine::decompose` supports three variables (exact trivariate resultants, McCallum projection twice, exact lifting); four or more are still an error.
- `CadFeaBridge::screw_to_fea` also meshes a screw with a key socket as a solid (the socket is a void in the mesh).

## [0.6.0] - 2026-11-18

### Added

- `ThreadedScrew::thread_radius`, `thread_depth` and `minor_radius`; the Acme 29 degree and square thread profiles are now distinct from the ISO 60 degree profile.
- FEA Dirichlet constraints are eliminated exactly rather than penalised with 1e12, and `reaction_forces` now reports the support reactions (was all zeros).
- `CustomShape::BooleanCSG` of implicit surfaces now meshes (`implicit_field`, `Mesh3D::from_field`); Booleans involving meshes/extrusions/polytopes still report unsupported.
- `InvoluteGear` teeth are true involutes (the pitch-circle thickness was too small before) and support a profile shift coefficient (`with_profile_shift`).
- FEA assembly is sparse (`SparseAssembly`); constrained systems are built from it directly, so large problems never form a dense matrix.
- FEA: the plane-strain constitutive matrix was wrong and element von Mises stress was computed from absolute nodal differences; stress is now `C B u` for the constant-strain triangle (plane stress and strain), verified exactly by a uniaxial patch test.
- FEA: linear tetrahedral solid elements (`Tet4`, `FeaMesh::box_3d`); 2-D body forces are applied; unimplemented element types are an error instead of being skipped.
- FEA: bilinear quadrilateral elasticity elements (`Quad4`, `FeaMesh::plate_2d_quad`).
- FEA heat conduction on `Quad4` and `Tet4` elements; unsupported element types are an error instead of being skipped.
- FEA: trilinear hexahedral elements (`Hex8`, `FeaMesh::box_3d_hex`) for elasticity and heat conduction.
- The Cartan operation accepts symbolic coefficients (`d(x^2*y dx)`, wedge, interior product, Lie derivative) via `SymbolicForm::parse`.
- `SymbolicForm::{hodge_star, codifferential, laplace_de_rham}`; the Cartan operation's Hodge star accepts symbolic coefficients.
- Notebook: "Print / Export" in the presentation view now produces a printable HTML report (`export::to_html_report`); it only set a status message before.
- Geometric constraint solver: `Point3D` unknowns with distance and coincidence constraints.
- Symbolic form coefficients accept juxtaposed products (`x^2 y dx`).
- FEA: quadratic triangle elements (`Tri6`, `FeaMesh::plate_2d_tri6`) for elasticity and heat conduction.
- Notebook: workspace tabs and layout presets are routed to views (`WorkspaceTab::route`, `apply_layout`), with palette commands for the Presentation Report and Dependency Graph layouts.
- Geometric constraint solver: a circle with no stated radius gets the radius solved from tangency.
- FEA: quadratic tetrahedral elements (`Tet10`, `FeaMesh::box_3d_tet10`) for elasticity and heat conduction.
- `CycloidalGear`: cycloidal spur gears with a closed, oriented solid (shared generator with the involute gear).
- `InternalGear`: internal (ring) involute spur gears as closed solids.
- `ThreadedScrew`: optional head chamfer and hexagonal key socket (closed solids, volumes verified).
- Gear end caps no longer contain folded triangles (one per radial root segment on small pinions); `gear_to_fea` meshes a bored gear as a solid of linear tetrahedra for real 3-D elasticity.
- Fixed: `NacaAirfoil`, `HelicalSpring` and `Mesh3D::cube` were inside out (normals into the solid); added `Mesh3D::{signed_volume, is_outward_oriented, reverse_orientation}`.
- `airfoil_to_fea` meshes the wing as a solid of linear tetrahedra (shared prism stacker with the gear mesh).
- Fixed: gear FEA boundary groups are selected by geometry (`bore_hole` = the bore cylinder, `teeth_flanks` = outline above the pitch circle) instead of by radial-range fractions.
- `spring_to_fea`: helical springs are meshed as solids of linear tetrahedra with end-face groups.
- `gosper::{gosper, QPoly, GosperSum}`: Gosper's algorithm for indefinite hypergeometric summation in exact rational arithmetic (the building block of Zeilberger's creative telescoping, which is still open).
- `summation` handles polynomial times geometric summands (`k*2^k`, `(1/2)^k`, ...) in closed form via Gosper's algorithm.
- Geometric constraint solver: `Line3D` with parallel, perpendicular and angle constraints in space.
- Geometric constraint solver: point-to-line distances (2-D and 3-D); a distance of zero puts a point on a line.
- Geometric constraint solver: `Plane3D` with point-plane distance and line/plane parallel and perpendicular constraints.
- Geometric constraint solver: `PointND` unknowns in any dimension for distance and coincidence.
- Cartan operations with symbolic vector fields (`InteriorProductSymbolic`, `LieDerivativeSymbolic`).
- `guess::{guess_recurrence, guess_minimal_recurrence}`: P-recursive recurrences guessed from exact sequence values (a conjecture, not a proof).
- `SymbolicForm::hodge_star_diagonal`: Hodge star for a diagonal metric with symbolic entries (spherical, Minkowski, ...).
- Cartan operation `HodgeStarMetric`: Hodge star for a diagonal metric given as expressions.
- `SymbolicForm::hodge_star_metric`: Hodge star for a general symmetric metric with symbolic entries.
- Cartan operation `HodgeStarMatrix`: Hodge star for a general symmetric metric matrix given as expressions.
- `zeilberger::{zeilberger, telescoping_at}`: creative telescoping by specialisation and interpolation (recurrences for binomial, squared-binomial and Delannoy sums, verified beyond the fitted range; not an all-n proof).
- Fixed: the hard-coded Zeilberger certificates for the binomial and Vandermonde sums were wrong (sign; plus an extra factor for Vandermonde); a vacuous test (`valid || order == 1`) had hidden it. Both are now verified exactly.
- Fixed: `SpinorHelicity::mandelstam_s` had the wrong sign and returned 0 for momenta along -z (spinor construction singular there); two always-true test assertions that hid this and an untested uncertainty path were replaced.
- Fixed: `CadEngine::decompose` is now sound for one and two variables (exact McCallum projection, Sturm root isolation); it used to cut the base line at the roots of the polynomials with the other variables set to 0. Three or more variables, repeated factors and common factors are errors.
- Fixed: `lim (1+x)^(1/x)` at a finite point (and every `1^oo` form) was NaN; constant folding in the limit engine could overflow `i64` (a panic for a high-order Taylor series of `sqrt(1+x)`).
- `CadFeaBridge::screw_to_fea` meshes a screw without a key socket as a solid of linear tetrahedra (radius steps become thin cones); a socketed screw keeps the surface mesh.
- Fixed: inverse trig/hyperbolic function names are canonicalised (`arctan` is `atan`, ...), so `d/dx arctan(x)` is evaluated; the arctan integral of a perfect-square constant is simplified (`1/2 atan(1/2 x)` for `1/(x^2 + 4)`). Expressions typed with the long names are now displayed with the short ones.
- `cartan_symbolic::SymbolicForm`: differential forms with symbolic coefficients (exterior derivative, wedge, interior product, Lie derivative).
- `CsrMatrix` (CSR storage, matvec, Jacobi-preconditioned conjugate gradients with a componentwise backward-error stop); the FEA solver uses it above 400 unknowns.
- `Mesh3D::from_implicit` meshes `f(x,y,z)=0` inside a box by marching tetrahedra (closed, outward-oriented; volumes of a sphere and torus within a few percent), and `CustomShape::ParametricImplicit` now uses it.
- `CustomShape::to_tri_mesh` returns `Result`; N-D polytopes and Boolean CSG are reported as unsupported instead of being replaced by a unit cube.
- `ThreadedScrew::generate_3d_mesh` returned an open ribbon surface (hundreds of open edges) regardless of the thread standard; it is now a closed solid with a threaded section, shank, hex or round head and end faces.
- `CatExprEvaluator::eval_with_fidelity` ignored its `AnytimeBudget`; `ApproxResult::budget_exceeded` is now set when a tier runs past the hard timeout.

## [0.5.0] - 2026-11-17

### Added
- `Mesh3D::boundary_edge_count` and `Mesh3D::is_closed`: detect open boundaries and inconsistent winding.
- `GpuKernelDescriptor::validate` (structural WGSL checks) and `SimdPolyVector::{checked_add, checked_multiply}`.
- `cat_egraph::EGraph::rebuild` (congruence closure) and `extract_term`/`ExtractedTerm` (the cheapest expression as a tree). `EGraph::add` now hash-conses.
- `IgaStiffnessMatrix::assemble_2d`, `NurbsPatchND::rational_basis` and `tangents`: the isogeometric Laplace-Beltrami stiffness matrix with analytic NURBS derivatives.
- `RittWuReducer::reduce` (differential pseudo-division) and `is_reduced_wrt`; `DiffPolynomial` normalisation and arithmetic (`add`, `sub`, `mul`, `scale`, `coefficient_of`, `initial`).
- `UraeLanguageServer::completions_at`: offers names defined above the cursor, newest first, before the catalogue.
- `SymbolicOdeSolver::frobenius_series_at_zero`: Frobenius series at a regular singular point (the module documentation previously claimed this method without implementing it).
- `poly::exact`: exact Gröbner bases over the rationals (`ExactPoly`, `ExactGrobner`) with a decision procedure for ideal membership. The floating-point `GrobnerBasis` is documented as numeric.
- Notebook export (`urae_notebook::export`): LaTeX, SymPy and Lean 4 files are built from the parsed document. The LaTeX export previously wrapped the raw buffer, the Python export put it in a docstring, and the Lean export was a two-line stub.
- `DysonSeries::first_order_term_between` (integral from `t0` to `t`) and `WeilJet::ito_drift`.
- `ProofTrace::export_lean4_with` and `export_coq_with`: each step is a theorem about the real expressions it rewrote (closed by `ring`; unsupported steps are listed as comments). The CLI and agent export use them.
- `calculus::pade_approximant`: exact Padé `[l/m]` approximants from rational Taylor coefficients.
- Closed forms for second-order linear recurrences with distinct or repeated real roots.
- Cartan operations (wedge, `d`, interior product, Lie derivative, Hodge star) read the form that was typed (`dx`, `2 dx + dz`, `dx^dy`).
- Summation of any polynomial summand by Faulhaber's formula (degree up to 12).
- Clifford vector products and rotor rotations compute real results; `export <format> <expr>` renders the expression in LaTeX, Unicode or MathML.

### Changed
- `CustomShape::to_tri_mesh` returns `Result`; implicit surfaces, N-D polytopes and Boolean CSG are reported as unsupported instead of being replaced by a unit cube.
- `CodeGenerator::compile_fn1`, `compile_fn2` and `compile_fn_n` compile to bytecode once and evaluate on `f64` per call instead of cloning the graph and substituting symbolically on every call.
- `VmCompiler` reports an unbound symbol as a compilation error; it used to compile it as the variable and return wrong numbers.
- `AdjointSensitivitySolver::compute_gradient` back-propagates exactly through the RK4 steps of `NeuralOde::forward` (fourth-order accurate gradients). It was a first-order scheme with an error of about 1% at 100 steps, and it ignored its `vector_field` argument.
- FEA solves report an under-constrained (singular) system as an error and verify the solution's residual. Previously a singular equation was skipped and its unknown reported as `0`, so a structure with rigid-body motion returned plausible-looking displacements.
- `WavefrontSchedule::build` and `evaluate` return `Result` (`WavefrontError`): levels follow dependencies rather than edge listing order, cycles are detected, and a missing wire, an unsupported operation or a division by zero is an error instead of silently `0`. `Inv` and `Identity` are now evaluated.
- `RittWuReducer::characteristic_set` computes a characteristic set (Ritt's procedure) instead of sorting its input; `{y'', y, y'}` now gives `{y}`.
- `UniversalBridge::weyl_to_diff_polynomial` returns `Result` and `DiffPolynomial` to `WeylDOperator` is now `TryFrom`: conversions that would drop an `x` power, flatten a nonlinear term or merge indeterminates are errors instead of silently wrong output.
- Differential-equation classification reports PDEs as linear, semilinear, quasilinear or nonlinear by the location of the nonlinearity (the KdV equation is now semilinear, Burgers quasilinear). Powers and products of derivatives are detected as nonlinear.
- Production code no longer contains reachable `unwrap`, `expect` or `panic!` calls outside documented operator panics; the workspace lints `clippy::unwrap_used`, `expect_used` and `panic` are on (test code exempt). A failed notebook launch is reported instead of panicking, and a poisoned token lock is recovered.
- `KnotInvariants::kauffman_bracket` and `jones_polynomial` return `Result` and compute a state sum (previously a lookup table for three knots and a product of `A^±1` otherwise). A positive `σ₁³` now gives `A⁻⁷ − A⁻³ − A⁵`, the standard right-handed trefoil (the old table held its mirror image).
- `AnytimeHandle::refine_step` and `run_to_budget` take a re-evaluation closure and `refine_with` accepts only sound (contained, tighter) balls; enclosures no longer shrink by a fixed factor.
- Planning consolidated: `docs/PLANNING.md` indexes the planning documents and tracks every known stub and defect against the code; `ROADMAP.md` marks phases 12-16, 22, 23 and 27 as Partial and lists the scope that has no implementation.
- `pade!` returns `AlgebraResult<ExprId>` and computes a real approximant (it returned a placeholder quotient).
- `creative_telescoping_integral`, `EmpiricalDistributionMut::{pdf, cdf}` and an unreachable AI provider return errors instead of a fixed Gaussian ODE, constants and a canned `f(x)` explanation.
- `puiseux_series` with a fractional exponent, `log`, `preset` and AI queries in the executor return errors instead of placeholder output.
- Unsupported summands, complex-root and higher-order recurrences return an error instead of a wrong formula.

### Fixed
- `poly::groebner_basis` returned its input with zero polynomials removed; it now computes the reduced Groebner basis over the rationals. `SparseMonomial::cmp_grevlex` is a genuine graded reverse lexicographic order.
- `Mesh3D::extrude_profile` capped non-convex outlines with a fan from one vertex (overlapping or inverted faces); it now ear-clips the polygon, handles either winding, and rejects degenerate or self-intersecting profiles.
- `NacaAirfoil::generate_3d_mesh` and `HelicalSpring::generate_3d_mesh` returned open shells without end caps (so `compute_volume` was meaningless); they are now closed solids. The airfoil outline no longer repeats its leading-edge point. `ThreadedScrew` meshes are documented as open surfaces.
- `GpuComputeKernel::enabled` was ignored; it now selects the parallel or sequential path. The `algebra_advanced::gpu` documentation no longer claims SIMD or GPU offload (the kernels are Rayon-parallel CPU code).
- `EGraph::extract_min_cost` costed only the chosen node's operator, not its operands, so the cheapest-looking node was not the cheapest expression; it now returns the total cost of the cheapest expression, and handles cyclic classes.
- `InvoluteGear::generate_3d_mesh` ignored `bore_radius` and capped the ends with a fan from one vertex of the non-convex tooth outline, giving overlapping or inverted faces. It is now a closed solid with a real bore and annular end caps (volume equals the outline area minus the bore, times the face width).
- `GrobnerProver` (lemma mining) reduced the difference of two expressions modulo an empty ideal, so it could not use identities such as `sin^2 + cos^2 = 1`. It now extracts the side relations of the transcendental atoms present and decides membership exactly over the rationals.
- `GeometricConstraintSolver::solve` ignored every constraint except distance and coincidence (placing a point along the x-axis); it now solves parallel, perpendicular, angle, tangent and fixed-area constraints too by damped Gauss-Newton, and reports inconsistent systems.
- `IgaStiffnessMatrix::integrate_stiffness_entry_2d` ignored the NURBS basis and returned a fixed weight of 1.0 on the diagonal and 0.1 elsewhere times the area; it now integrates the real gradient inner product (the unit square gives the textbook Q1 matrix). `metric_determinant_2d` uses analytic tangents.
- `DiffPolynomial::differentiate` left repeated indeterminates in a term (`y'` twice), which made degrees and leaders wrong; it now normalises.
- LSP hover evaluated only the hovered line, so any line using an earlier definition showed an error or a wrong value; it now evaluates the document up to that line in order.
- `StateSpaceSystem::is_stable` judged systems with three or more states by the sign of the trace; it now applies Routh-Hurwitz (continuous) or Schur-Cohn (discrete) to the characteristic polynomial.
- `frequency_response` returned zeros for every system that was not a two-state SISO one, and ignored the sample time of discrete systems; it now evaluates `C(sI - A)^-1 B + D` in general.
- `pole_placement_ackermann` supports any controllable single-input system (it stopped at two states).
- `UnificationEngine::unify` only handled a top-level symbol or number and failed on everything else. It is now a structural first-order unifier with an occurs check; `SubstitutionMap::apply` applies the result.
- The CLI proof export gated its step on unifying an expression with its own simplification, which almost never succeeded and is not an equivalence test; it now uses the equivalence check.
- `lagrange_inversion` divided `w / f` before differentiating, which is `0/0` at the expansion point for every `f` except the identity. It now computes the exact series of the inverse function and rejects non-invertible input.
- Forced 2×2 linear systems omitted the Duhamel convolution of the forcing term.
- The symbolic tier labelled a tangent-line approximation as order 3; it is a first-order expansion with an order-2 remainder.
- `DysonSeries::first_order_term` returned the integrand when the integral had no closed form.
- `sum k^2` returned `k^2 * n` (any non-symbol summand was treated as constant).
- `y' = 1` returned an expression containing `1 / 0`.
- The Fibonacci recurrence returned `0`.
- Caputo derivative used the wrong kernel exponent for orders other than n - 1/2.
- `solve_pde_heat_1d` returned `cos(k_x)` with no `x` and a decay rate of `k_x` instead of `k_x^2`.

## [0.4.0] - 2026-11-17

### Added
- Overhaul (`23f508b`): `cat-core` 4-fidelity evaluation, BSGS tensor canonicalisation, generalised transforms and a polyglot DSL.
- CAS depth overhaul with multivalued and uncertainty support, and an audit regression suite (`4c2a293`).
- Unified API, stream and workbench layers, naming clean-up, and a GUI architecture plan (`ab4d0f7`).
- Integration by parts for `x^n` times `exp`, `sin`, `cos` and `ln` for any degree, and `1/(x^2 - b^2)` integrated as a logarithm.
- Limits at +infinity by dominant-term analysis (radicals, variable exponents); two-sided limits through `abs`, `sign`, `floor` and `ceil` compare the one-sided values.
- Factoring of a difference of squares with multi-term roots (`a^2 + 2ab + b^2 - c^2`), using an exact multivariate polynomial square root.
- The simplifier merges repeated bases (`x * x` is `x^2`) and combines square roots of positive rationals (`sqrt(2) * sqrt(8)` is `4`).

### Changed
- All three formatters (LaTeX, Unicode, MathML) print with operator precedence: parentheses where needed, negative terms as subtraction. Output text changes, so anything matching printed strings must be updated.
- `ROADMAP.md` marks phases 8, 28 and 30 as Partial or Foundation, with what the code does.

### Fixed
- `factor_primes` printed a composite that Pollard rho could not split as a prime; it is now marked `[composite, not factored]`.
- `exp(2*x) = 7` returned `ln(-sqrt(7))` as a root; roots of equations quadratic in `e^x` print as `ln(7)/2` and `ln(7)/2 + i*pi`.
- An equation that is an identity after clearing denominators (`(x - 1)/(x - 1) = 1`) returned no solutions; it returns the variable, as `x = x` does.
- The Unicode parser converted only the first `sqrt(` in an input.

## [0.3.2] - 2026-11-15

### Added
- Salvaged `Magnitude` (`cat-numbers`), `Hecs<T>` hexagonal coordinates, and `Pose<T>` SE(3) kinematics (`urscript_pose`), re-exported via `urae::prelude`.
- `NumericalOdeMethod::SelfOptimizingMeta`, `ButcherTableau::tsit5()`, and `NumericalPdeSolver::solve_burgers_1d_mol`.
- `urae-py` (`pyproject.toml`, `OdeTrajectorySummary`, `py_*` bindings), `urae-kernel` Jupyter v5.3 message dispatcher, and `urae-wasm` JSON plotting/ODE endpoints.
- Typed `SolveOutcome` (`solveset_outcome`), general $N \times N$ cofactor/adjugate/charpoly + Bareiss RREF/nullspace, and `SymbolicCalculus::integrate_with_steps`.
- `linprog_minimize` (Simplex with Bland's anti-cycling rule) and `yun_square_free_z` (Yun square-free decomposition over $\mathbb{Z}[x]$).
- Truthful `Feature Matrix (URAE vs. SymPy vs. Symbolica)` in `README.md`.
