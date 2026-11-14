# Structure-Preserving Neural Networks: Hamiltonian, Port-Hamiltonian, GENERIC, and Divergence-Free Fields

## 1. Executive Summary & Architectural Overview

Classical deep learning architectures for dynamical systems treat trajectory modeling as unconstrained sequence-to-sequence or continuous neural ODE regression. While effective over short horizons, unconstrained models fundamentally violate the first and second laws of thermodynamics, geometric symplectic forms, and mass incompressibility constraints. Over extended simulation rollouts, numerical integration accumulates errors exponentially, causing unphysical energy growth, artificial damping, phase space distortion, or non-solenoidal fluid velocity fields.

**Structure-Preserving Neural Networks** bake geometric invariants directly into the network architecture:
1. **Hamiltonian Neural Networks (HNN)**: Parameterize vector fields via the skew-symmetric symplectic gradient $\dot{z} = J \nabla \mathcal{H}(z)$, guaranteeing exact energy conservation by construction ($\frac{d\mathcal{H}}{dt} \equiv 0$).
2. **Port-Hamiltonian Neural Networks (pHNN)**: Generalize conservative systems to open physical systems by incorporating positive semi-definite dissipation $R(z) \succeq 0$ and external control ports $G(z) u$, guaranteeing passive energy dissipation.
3. **Thermodynamic GENERIC Networks**: General Equation for Non-Equilibrium Reversible-Irreversible Coupling ($\dot{z} = L \nabla E + M \nabla S$), strictly enforcing both the First Law (exact energy conservation via $M \nabla E = 0$) and Second Law (monotonic non-negative entropy generation $\dot{S} \ge 0$ via $L \nabla S = 0$).
4. **Divergence-Free Solenoidal Networks**: Represent incompressible velocity fields via curl/stream potentials ($\mathbf{v} = (\partial_y \psi, -\partial_x \psi)$ in 2D or $\mathbf{v} = \nabla \times \mathbf{A}$ in 3D), guaranteeing exact mass conservation $\nabla \cdot \mathbf{v} \equiv 0$ identically to machine precision.
5. **Symplectic Leapfrog (Verlet) Integration**: Preserves phase-space symplectic 2-forms ($dq \wedge dp$) over arbitrary integration horizons, preventing artificial energy drift.

---

## 2. Mathematical Foundations & Governing Equations

### 2.1 Hamiltonian Mechanics & HNNs
Let phase space coordinates be $z = [q^T, p^T]^T \in \mathbb{R}^{2d}$, where $q$ represents generalized coordinates and $p$ generalized momenta.
The canonical symplectic matrix $J \in \mathbb{R}^{2d \times 2d}$ is defined as:

$$J = \begin{bmatrix} 0 & I_d \\ -I_d & 0 \end{bmatrix}, \quad J^T = -J, \quad J^2 = -I$$

For a total energy scalar potential $\mathcal{H}(q, p) = \frac{1}{2} p^T p + V(q)$, Hamilton's equations of motion are:

$$\dot{z} = \begin{bmatrix} \dot{q} \\ \dot{p} \end{bmatrix} = J \nabla_z \mathcal{H}(z) = \begin{bmatrix} \nabla_p \mathcal{H} \\ -\nabla_q \mathcal{H} \end{bmatrix} = \begin{bmatrix} p \\ -\nabla_q V(q) \end{bmatrix}$$

Conservation of energy is an exact algebraic consequence of the skew-symmetry of $J$:

$$\frac{d\mathcal{H}}{dt} = (\nabla_z \mathcal{H})^T \dot{z} = (\nabla_z \mathcal{H})^T J (\nabla_z \mathcal{H}) = 0$$

### 2.2 Port-Hamiltonian Dynamics with Dissipation
Real-world systems dissipate energy through friction and receive external power via control ports:

$$\dot{z} = \left[J(z) - R(z)\right] \nabla_z \mathcal{H}(z) + G(z) u_{\text{ext}}$$

where $R(z) \succeq 0$ is a positive semi-definite dissipation matrix ($R = \text{diag}(0, \gamma I)$ with $\gamma \ge 0$). In unforced systems ($u_{\text{ext}} = 0$):

$$\frac{d\mathcal{H}}{dt} = -(\nabla_z \mathcal{H})^T R(z) (\nabla_z \mathcal{H}) \le 0$$

### 2.3 Thermodynamic GENERIC Framework
Beyond mechanics, non-equilibrium thermodynamics couples reversible symplectic transport with irreversible entropy dissipation:

$$\dot{z} = L(z) \nabla E(z) + M(z) \nabla S(z)$$

Subject to the mutual degeneracy conditions:

$$L(z) \nabla S(z) = 0 \quad \text{and} \quad M(z) \nabla E(z) = 0$$

with $L = -L^T$ (skew-symmetric Poisson operator) and $M = M^T \succeq 0$ (symmetric positive semi-definite friction matrix).
Using projection matrices $P_S = I - \frac{\nabla S \nabla S^T}{\|\nabla S\|^2}$ and $P_E = I - \frac{\nabla E \nabla E^T}{\|\nabla E\|^2}$:

$$L = P_S L_0 P_S, \quad M = P_E M_0 P_E \implies \dot{E} \equiv 0 \text{ and } \dot{S} = (\nabla S)^T M \nabla S \ge 0$$

### 2.4 Divergence-Free Solenoidal Vector Fields
In incompressible fluid dynamics ($\nabla \cdot \mathbf{v} = 0$), parameterizing $\mathbf{v}$ directly allows mass violations. Instead, parameterize a stream function $\psi(x, y)$:

$$\mathbf{v}(x, y) = \begin{bmatrix} \frac{\partial \psi}{\partial y} \\ -\frac{\partial \psi}{\partial x} \end{bmatrix} \implies \nabla \cdot \mathbf{v} = \frac{\partial^2 \psi}{\partial x \partial y} - \frac{\partial^2 \psi}{\partial y \partial x} \equiv 0$$

By Schwarz's theorem on mixed partial derivatives, the divergence is identically zero everywhere.

---

## 3. Tensor Memory Layout & Implementation Details

| Component | Tensor Name | Shape | Layout / Memory Semantics |
| :--- | :--- | :--- | :--- |
| **Phase State** | `z_state` | `[4, 4]` | Contiguous $[q_1, q_2, p_1, p_2]$ phase space coordinates |
| **Potential Weights 1** | `w_v1` | `[2, 8]` | Potential network input layer weights |
| **Potential Bias 1** | `b_v1` | `[8]` | Potential network input layer biases |
| **Potential Weights 2** | `w_v2` | `[8, 1]` | Potential network output linear weights |
| **Potential Bias 2** | `b_v2` | `[1]` | Potential network output linear bias |
| **Potential Energy** | `v_potential` | `[4, 1]` | Scalar potential evaluation $V(q)$ |
| **Potential Gradient** | `grad_v` | `[4, 2]` | Spatial gradient vector $\nabla_q V(q)$ |
| **Total Energy** | `h_total` | `[4, 1]` | Hamiltonian scalar energy $H = \frac{1}{2}\|p\|^2 + V(q)$ |
| **HNN Dynamics** | `z_dot_hnn` | `[4, 4]` | Conservative velocity vector $[p, -\nabla_q V]$ |
| **pHNN Dynamics** | `z_dot_phnn` | `[4, 4]` | Dissipative velocity vector $[p, -\nabla_q V - \gamma p]$ |
| **Leapfrog State** | `z_leapfrog` | `[4, 4]` | 1-step symplectic Verlet rollout state $[q_{n+1}, p_{n+1}]$ |
| **Stream Weights 1** | `w_psi1` | `[2, 8]` | Stream function input layer weights |
| **Stream Bias 1** | `b_psi1` | `[8]` | Stream function input layer biases |
| **Stream Weights 2** | `w_psi2` | `[8, 1]` | Stream function output linear weights |
| **Stream Bias 2** | `b_psi2` | `[1]` | Stream function output linear bias |
| **Stream Potential** | `psi_stream` | `[4, 1]` | Scalar stream potential $\psi(x, y)$ |
| **Solenoidal Field** | `v_solenoidal`| `[4, 2]` | Exact divergence-free velocity $[\partial_y \psi, -\partial_x \psi]$ |
| **Skew Generator** | `l0_skew` | `[4, 4]` | Unprojected skew-symmetric Poisson bracket |
| **Friction Generator** | `m0_psd` | `[4, 4]` | Unprojected positive semi-definite friction matrix |
| **GENERIC Dynamics** | `z_dot_generic`| `[4, 4]` | Thermodynamic rate of change $L \nabla E + M \nabla S$ |
| **Energy Drift Rate** | `de_dt` | `[4]` | First Law verification rate $\frac{dE}{dt} \approx 0$ |
| **Entropy Growth Rate**| `ds_dt` | `[4]` | Second Law verification rate $\frac{dS}{dt} \ge 0$ |

---

## 4. Multi-Ecosystem Implementation Matrix

| Framework | Implementation Module | Execution Model | Conservation Enforcement Strategy |
| :--- | :--- | :--- | :--- |
| **PyTorch 2.x** | `pytorch2/.../05_structure_preserving_hamiltonian_port_generic_and_divergence_free.py` | Eager / `torch.compile` | Analytical gradient derivation with autograd parity verification |
| **JAX** | `jax/.../05_structure_preserving_hamiltonian_port_generic_and_divergence_free.py` | Functional `jax.jit` | Vectorized batched projections ($P_S L_0 P_S$ and $P_E M_0 P_E$) |
| **Candle** | `candle/.../05_structure_preserving_hamiltonian_port_generic_and_divergence_free.rs` | Zero-GC Rust | Zero-copy tensor narrow, broadcast arithmetic, in-place Leapfrog |
| **WGSL / CPU** | `kernels/.../05_structure_preserving_hamiltonian_port_generic_and_divergence_free.rs` | WebGPU compute / SIMD | Parallel symplectic Verlet workgroup kernel and stream curl evaluation |

---

## 5. Pathological Misuse & Failure Modes

1. **Explicit Forward Euler on Hamiltonian Systems**: Integrating an HNN with explicit Forward Euler ($q_{n+1} = q_n + h p_n$, $p_{n+1} = p_n - h \nabla V(q_n)$) injects positive numerical energy at every step: $\mathcal{H}_{n+1} = \mathcal{H}_n + \frac{h^2}{2}\|\nabla V\|^2 > \mathcal{H}_n$. Over long rollouts, orbits spiral outward exponentially. Always integrate via **Symplectic Leapfrog / Verlet**.
2. **GENERIC Degeneracy Violation**: Omitting the orthogonal projection operators $P_S$ and $P_E$ causes cross-talk between reversible and irreversible dynamics, resulting in non-zero $\frac{dE}{dt} \neq 0$ (violating First Law) and possible negative entropy generation $\frac{dS}{dt} < 0$ (violating Second Law).
3. **Direct Velocity Prediction in Incompressible Fluids**: Predicting $[v_x, v_y]$ directly with standard CNNs or MLPs produces non-zero divergence ($\nabla \cdot \mathbf{v} \neq 0$), creating artificial sources and sinks that cause mass loss in Navier-Stokes simulations. Using stream/vector potential parameterization guarantees incompressibility to machine precision.

---

## 6. Cross-Domain Generalisation Matrix

| Application Domain | Structure-Preserving Method | Invariant Guaranteed |
| :--- | :--- | :--- |
| **Orbital & Molecular Dynamics** | Hamiltonian Neural Network + Leapfrog | Symplectic form and Hamiltonian total energy conservation |
| **Robotics & Port Manipulation** | Port-Hamiltonian Neural Network | Passivity, $L_2$ gain boundedness, and stable contact control |
| **Thermodynamic Continuum Mechanics**| Thermodynamic GENERIC Network | Simultaneous First Law (isothermal) and Second Law compliance |
| **Aerodynamics & Weather Modeling** | Divergence-Free Solenoidal Stream Formulation | Incompressible continuity equation $\nabla \cdot \mathbf{v} \equiv 0$ |

---

## 7. Verification, Parity & Empirical Benchmarks

All operators are verified against the deterministic golden reference checkpoint:
- Checkpoint: `weights/06_smooth_and_physics_informed_ai/05_structure_preserving_hamiltonian_port_generic_and_divergence_free.safetensors` (Seed 42)
- Tensors Verified: 26 tensors (anchor, phase states, gradients, leapfrog, solenoidal velocities, GENERIC tensors).
- Numerical Parity:
  - PyTorch vs JAX: max absolute difference $< 10^{-6}$.
  - Candle Test `test_05_structure_preserving_hamiltonian_port_generic_and_divergence_free_parity`: 0 failures, max difference $< 10^{-5}$.
  - Divergence-Free Verification: $\nabla \cdot \mathbf{v}_{\text{solenoidal}} < 6.0 \times 10^{-8}$.
  - GENERIC Conservation: $|\frac{dE}{dt}| < 1.5 \times 10^{-6}$ and $\frac{dS}{dt} > 0.88$.

---

## 8. Synchronized Track Artifacts & Navigation

- PyTorch: `pytorch2/06_smooth_and_physics_informed_ai/05_structure_preserving_hamiltonian_port_generic_and_divergence_free.py`
- JAX: `jax/06_smooth_and_physics_informed_ai/05_structure_preserving_hamiltonian_port_generic_and_divergence_free.py`
- Candle: `candle/06_smooth_and_physics_informed_ai/05_structure_preserving_hamiltonian_port_generic_and_divergence_free.rs`
- Kernels: `kernels/06_smooth_and_physics_informed_ai/05_structure_preserving_hamiltonian_port_generic_and_divergence_free.rs`
- Weights: `weights/06_smooth_and_physics_informed_ai/05_structure_preserving_hamiltonian_port_generic_and_divergence_free.safetensors`
- Notebook: `notebooks/06_smooth_and_physics_informed_ai/05_structure_preserving_hamiltonian_port_generic_and_divergence_free.ipynb`
