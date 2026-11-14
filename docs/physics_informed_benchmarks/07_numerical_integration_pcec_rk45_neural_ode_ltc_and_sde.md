# Numerical Integration Schemes: Predictor-Corrector (PCEC), Adaptive RK45, Neural ODEs, Liquid Time-Constant (LTC), and Neural SDEs

## 1. Executive Summary & Architectural Overview

Standard recurrent neural networks and discrete sequence models discretize time into arbitrary unit steps ($\Delta t = 1$), ignoring irregular sampling intervals, stiff continuous-time dynamics, and underlying differential equations. **Continuous-Time Neural Models** parameterize continuous physical trajectories $x(t)$ governed by differential equations $\frac{dx}{dt} = f_\theta(t, x)$.

However, the efficacy of continuous-time neural models depends crucially on the **numerical integration scheme**:
1. **Explicit Multi-Step & Predictor-Corrector (`PECE` & `PCEC`)**: Combines explicit Adams-Bashforth predictors with implicit Adams-Moulton correctors, dramatically expanding the numerical stability domain on stiff vector fields without requiring expensive algebraic matrix inversions.
2. **Embedded Runge-Kutta (`RK4` & `Dormand-Prince DP54 / RK45`)**: Computes multi-stage intermediate derivative evaluations with local truncation error estimation ($\mathcal{O}(h^5)$) for adaptive step-size control.
3. **Continuous Adjoint Sensitivity Method**: Computes exact parameter gradients $\frac{d\mathcal{L}}{d\theta}$ in $\mathcal{O}(1)$ memory by integrating an augmented adjoint ODE backwards in time, bypassing intermediate forward activation caching.
4. **Liquid Time-Constant (LTC) & Closed-Form Continuous-Time (CfC)**: Overcomes computational bottlenecks of iterative ODE solvers by formulating continuous-time neural dynamics with an analytical closed-form solution evaluating in $\mathcal{O}(1)$ constant time.
5. **Neural Stochastic Differential Equations (Neural SDE)**: Extends neural dynamics to stochastic Brownian motion diffusion ($dx = f(x)dt + g(x)dW$), comparing first-order Euler-Maruyama with second-order Milstein integration.

---

## 2. Mathematical Foundations & Governing Equations

### 2.1 Numerical Integrator Zoo
Given initial state $x_0$ and step size $h$:
1. **Explicit Forward Euler**:
   $$x_{n+1} = x_n + h f(t_n, x_n)$$
2. **Heun's 2nd-Order Runge-Kutta (RK2)**:
   $$k_1 = f(t_n, x_n), \quad k_2 = f(t_n + h, x_n + h k_1)$$
   $$x_{n+1} = x_n + \frac{h}{2}(k_1 + k_2)$$
3. **Predictor-Corrector Scheme (`PCEC` Adams-Bashforth-Moulton)**:
   $$\text{Predict } (P): \quad x_{n+1}^{(P)} = x_n + h f(t_n, x_n)$$
   $$\text{Evaluate } (E): \quad f_{n+1}^{(P)} = f(t_{n+1}, x_{n+1}^{(P)})$$
   $$\text{Correct } (C): \quad x_{n+1}^{(C)} = x_n + \frac{h}{2}\left( f(t_n, x_n) + f_{n+1}^{(P)} \right)$$
   $$\text{Evaluate } (E): \quad f_{n+1}^{(C)} = f(t_{n+1}, x_{n+1}^{(C)})$$
   $$\text{Final Correct } (C): \quad x_{n+1} = x_n + \frac{h}{2}\left( f(t_n, x_n) + f_{n+1}^{(C)} \right)$$
4. **Classical Runge-Kutta 4th-Order (RK4)**:
   $$k_1 = f(t_n, x_n), \quad k_2 = f(t_n + \tfrac{h}{2}, x_n + \tfrac{h}{2}k_1)$$
   $$k_3 = f(t_n + \tfrac{h}{2}, x_n + \tfrac{h}{2}k_2), \quad k_4 = f(t_n + h, x_n + h k_3)$$
   $$x_{n+1} = x_n + \frac{h}{6}(k_1 + 2k_2 + 2k_3 + k_4)$$

### 2.2 Continuous Adjoint Sensitivity Method (Pontryagin's Maximum Principle)
To minimize scalar loss $\mathcal{L}(x(T))$ subject to $\dot{x} = f(t, x, \theta)$, define the adjoint state $a(t) = \frac{\partial \mathcal{L}}{\partial x(t)}$.
The adjoint satisfies the continuous differential equation:
$$\frac{da(t)}{dt} = -a(t)^T \frac{\partial f(t, x(t), \theta)}{\partial x}$$
The backward discrete adjoint step evaluates:
$$a(t - h) = a(t) + h \cdot a(t)^T J_f(x(t))$$
where $J_{ij} = \frac{\partial f_j}{\partial x_i}$ is the spatial Jacobian matrix.

### 2.3 Closed-Form Continuous-Time Networks (CfC)
Liquid Time-Constant networks govern states via input-dependent time-constants:
$$\frac{dx}{dt} = -\left[\frac{1}{\tau} + f_\theta(x, I)\right] x + f_\theta(x, I) A$$
Hasani et al. (2022) derived the approximate closed-form continuous-time (CfC) solution:
$$x(t) \approx \sigma(-|f(x_0, I)| t) \odot (x_0 W_1) + \left(1 - \sigma(-|f(x_0, I)| t)\right) \odot (x_0 W_2)$$
evaluating continuous trajectories in $\mathcal{O}(1)$ operations without numerical ODE solver loops.

### 2.4 Neural Stochastic Differential Equations (SDE)
For stochastic dynamics driven by standard Brownian motion $W(t)$:
$$dx = f(t, x) dt + g(t, x) dW$$
- **Euler-Maruyama Scheme** (strong order 0.5):
  $$x_{n+1} = x_n + f(x_n) \Delta t + g(x_n) \Delta W_n$$
- **Milstein Scheme** (strong order 1.0):
  $$x_{n+1} = x_n + f(x_n) \Delta t + g(x_n) \Delta W_n + \frac{1}{2} g(x_n) g'(x_n) \left( \Delta W_n^2 - \Delta t \right)$$
where the spatial derivative correction $g(x) g'(x)$ accounts for Itô's lemma non-linear diffusion noise geometry.

---

## 3. Tensor Memory Layout & Implementation Details

| Component | Tensor Name | Shape | Layout / Memory Semantics |
| :--- | :--- | :--- | :--- |
| **Initial State** | `x0` | `[2, 2]` | Initial state vector in $\mathbb{R}^2$ ($B=2$) |
| **Vector Field Weights 1**| `w1` | `[2, 4]` | Neural ODE input projection weights |
| **Vector Field Bias 1** | `b1` | `[4]` | Neural ODE input projection biases |
| **Vector Field Weights 2**| `w2` | `[4, 2]` | Neural ODE output linear weights |
| **Vector Field Bias 2** | `b2` | `[2]` | Neural ODE output linear biases |
| **Initial Velocity** | `f0` | `[2, 2]` | Initial vector field evaluation $f(x_0)$ |
| **Euler State** | `x_euler` | `[2, 2]` | Explicit Euler 1-step integrated state |
| **Heun (RK2) State** | `x_heun` | `[2, 2]` | 2nd-order Runge-Kutta integrated state |
| **PCEC State** | `x_pcec` | `[2, 2]` | Predictor-Corrector integrated state |
| **RK4 State** | `x_rk4` | `[2, 2]` | Classical 4th-order Runge-Kutta state |
| **Terminal Adjoint** | `adj_T` | `[2, 2]` | Backward sensitivity loss gradient $a(T)$ |
| **Spatial Jacobian** | `j0` | `[2, 2, 2]` | Analytical Jacobian matrix $J_{ij} = \frac{\partial f_j}{\partial x_i}$ |
| **Initial Adjoint** | `adj_0` | `[2, 2]` | Backward integrated adjoint state $a(0)$ |
| **CfC Weights 1** | `w_cfc1` | `[2, 2]` | Closed-form continuous branch 1 weights |
| **CfC Weights 2** | `w_cfc2` | `[2, 2]` | Closed-form continuous branch 2 weights |
| **CfC Output State** | `x_cfc` | `[2, 2]` | $\mathcal{O}(1)$ closed-form continuous-time state |
| **Brownian Increment** | `dW` | `[2, 2]` | Gaussian Brownian noise increment $\mathcal{N}(0, \Delta t)$ |
| **Euler-Maruyama SDE** | `x_em` | `[2, 2]` | Strong-order 0.5 stochastic state |
| **Milstein SDE** | `x_milstein` | `[2, 2]` | Strong-order 1.0 corrected stochastic state |

---

## 4. Multi-Ecosystem Implementation Matrix

| Framework | Implementation Module | Execution Model | Solver Implementation Strategy |
| :--- | :--- | :--- | :--- |
| **PyTorch 2.x** | `pytorch2/.../07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.py` | Eager / `torch.compile` | Multi-step unrolling, analytical Jacobian, torch autograd parity |
| **JAX** | `jax/.../07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.py` | Functional `jax.jit` | Vectorized `jnp.einsum`, functional stages, XLA fusion |
| **Candle** | `candle/.../07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.rs` | Zero-GC Rust | Pure tensor arithmetic, stack row concatenation, in-place RK4 |
| **WGSL / CPU** | `kernels/.../07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.rs` | WebGPU compute / SIMD | Parallel RK4 intermediate stage aggregation workgroup shader |

---

## 5. Pathological Misuse & Failure Modes

1. **Explicit Euler on Stiff Dynamics**: Integrating stiff differential equations ($\lambda_{\max}(J) \gg 1$) with explicit Euler requires step sizes $h < 2/\lambda_{\max}$. When $h > 2/\lambda_{\max}$, Euler diverges exponentially to infinity. Multi-step **PCEC** and implicit BDF methods stabilize stiff ODEs.
2. **Continuous Adjoint Divergence Without Checkpointing**: For chaotic, dissipative, or turbulence vector fields, reverse-time adjoint integration is numerically ill-conditioned and diverges exponentially from the forward trajectory. Checkpointed adjoints (interpolating saved forward checkpoints) are required for chaotic ODEs.
3. **Euler-Maruyama Underestimation in Non-Linear SDEs**: For state-dependent diffusion $g(x)$, Euler-Maruyama drops the spatial derivative Itô correction $\frac{1}{2} g(x) g'(x)(\Delta W^2 - \Delta t)$, introducing first-order bias in financial option pricing and molecular Langevin simulations. Use **Milstein integration**.

---

## 6. Cross-Domain Generalisation Matrix

| Problem Domain | Continuous Dynamic Architecture | Practical Advantage |
| :--- | :--- | :--- |
| **Medical ICU Vital Sensors** | Neural ODE / CfC | Handles irregular, missing, and asynchronous time-series observations naturally |
| **Robotics & Autonomous Drones** | Liquid Time-Constant (LTC) / CfC | $100\times$ faster closed-loop control with provable out-of-distribution stability |
| **Quantitative Finance (Volatility)**| Neural SDE (Milstein) | Arbitrage-free continuous stochastic volatility and asset jump modeling |
| **Celestial & Molecular Mechanics** | Symplectic RK / Verlet | Bounded energy oscillation over millions of simulation time steps |

---

## 7. Verification, Parity & Empirical Benchmarks

All operators and integrators are verified against the deterministic golden reference checkpoint:
- Checkpoint: `weights/06_smooth_and_physics_informed_ai/07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.safetensors` (Seed 42)
- Tensors Verified: 23 tensors (anchor, Euler, Heun, PCEC, RK4, Adjoint, CfC, SDE).
- Numerical Parity:
  - PyTorch vs JAX: max absolute difference $< 10^{-6}$.
  - Candle Test `test_07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde_parity`: 0 failures, max difference $< 10^{-5}$.
  - Analytical vs Autograd Jacobian parity: $\Delta < 1.0 \times 10^{-7}$.

---

## 8. Synchronized Track Artifacts & Navigation

- PyTorch: `pytorch2/06_smooth_and_physics_informed_ai/07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.py`
- JAX: `jax/06_smooth_and_physics_informed_ai/07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.py`
- Candle: `candle/06_smooth_and_physics_informed_ai/07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.rs`
- Kernels: `kernels/06_smooth_and_physics_informed_ai/07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.rs`
- Weights: `weights/06_smooth_and_physics_informed_ai/07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.safetensors`
- Notebook: `notebooks/06_smooth_and_physics_informed_ai/07_numerical_integration_pcec_rk45_neural_ode_ltc_and_sde.ipynb`
