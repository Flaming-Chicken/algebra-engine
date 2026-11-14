# URAE Mathematical & Engineering Documentation Standards

**Universal Rust Algebra Engine (URAE)**  
*Authoritative Documentation Guide, KaTeX Math Syntax Conventions, Doctest Standards, and Quality Enforcement*

---

## 1. Overview & Core Philosophy

URAE is a mathematical computing system designed for formal rigor, extreme performance, and educational transparency. Every public symbol (crate, module, trait, struct, enum, function, method, macro) must be documented to peer-reviewed mathematical publication quality.

### The Four Pillars of URAE Documentation:
1. **Mathematical Precision**: Every algorithm and algebraic structure must state its mathematical definition, axioms, governing equations, and domain/range in standard LaTeX/KaTeX notation.
2. **Algorithmic Transparency**: Document time complexity ($O$), space complexity, numerical stability/conditioning, convergence criteria, and potential failure modes.
3. **Actionable Examples**: Provide self-contained, compile-tested doctests (`# Examples`) demonstrating standard usage and edge cases.
4. **Zero-Warning Hygiene**: All rustdoc builds (`cargo doc --workspace`) must compile with zero broken intra-doc links, zero ambiguities, and zero markdown warnings.

---

## 2. KaTeX Mathematical Typesetting Conventions

URAE documentation utilizes KaTeX for rendering mathematical typography. To ensure seamless rendering in both IDE hovers and browser rustdocs, follow these formatting rules:

### 2.1 Inline and Display Delimiters
- **Inline Math**: Enclose mathematical expressions between single dollar signs: `$f(x) = \sin(x)$`.
- **Display Math**: Place multi-line equations and complex matrices inside double dollar signs on their own lines:
  ```rust
  //! $$
  //! \int_{-\infty}^{\infty} e^{-x^2} \mathrm{d}x = \sqrt{\pi}
  //! $$
  ```

### 2.2 Escaping Characters in Rustdoc Markdown
Rustdoc interprets square brackets `[...]` as intra-doc link targets. When writing roots, fractions, or intervals inside KaTeX formulas in doc comments, always escape the brackets with a backslash:
- ❌ **Incorrect**: `/// $\sqrt[n]{z}$` (Causes `broken_intra_doc_links` error: unresolved link to `n`).
- ✅ **Correct**: `/// $\sqrt\[n\]{z}$`
- ❌ **Incorrect**: `/// Interval $[a, b]$`
- ✅ **Correct**: `/// Interval $\$[a, b]\$$` or `/// Interval $[a, b]$` with literal brackets escaped when necessary.

### 2.3 Mathematical Typography Standards
- **Differentials**: Write Roman $\mathrm{d}$ for integration and differentiation: `\mathrm{d}x`, `\frac{\mathrm{d}y}{\mathrm{d}x}`.
- **Vectors & Matrices**: Use bold Roman for vectors and tensors: `\mathbf{v}, \mathbf{A}, \mathbf{x}`.
- **Sets & Spaces**: Use blackboard bold for standard number fields: `\mathbb{R}, \mathbb{C}, \mathbb{Z}, \mathbb{Q}, \mathbb{F}_p`.
- **Operators**: Use `\operatorname{...}` for custom named functions: `\operatorname{Tr}(A), \operatorname{rank}(M), \operatorname{sgn}(x), \operatorname{p.v.}`.
- **Multi-line Alignments**: Use `\begin{aligned} ... \end{aligned}` inside `$$...$$` for aligned equation systems.

---

## 3. Standard Documentation Templates

### 3.1 Module-Level Template (`//!`)

Every `mod.rs` and top-level module file must begin with:

```rust
//! # `algebra_engine::module_name`
//!
//! One-line concise summary of the module's mathematical role.
//!
//! ## Mathematical Foundations
//! - **Core Concept 1**: LaTeX definition and theoretical explanation:
//!   $$
//!   \mathcal{L}_X \omega = (\mathrm{d}\iota_X + \iota_X\mathrm{d})\omega
//!   $$
//! - **Core Concept 2**: Invariants, theorems, and algebraic properties.
//!
//! ## Supported Capabilities & Algorithms
//! - **Algorithm A**: Name, paper reference, time complexity.
//! - **Algorithm B**: Numerical method, error bounds, stability properties.
//!
//! ## Examples
//! ```rust
//! use urae::prelude::*;
//!
//! let graph = ExprGraph::new();
//! let x = graph.symbol("x");
//! // Demonstrative operations
//! ```
```

---

### 3.2 Struct & Enum Template (`///`)

Every public struct and enum must explain what it represents mathematically:

```rust
/// Metric signature $(p, q, r)$ defining the quadratic form of Clifford Geometric Algebra $\operatorname{Cl}(p,q,r)$.
///
/// ## Mathematical Invariants
/// - Total manifold dimension $n = p + q + r$.
/// - Basis vectors satisfy:
///   $$
///   e_i^2 = \begin{cases} +1 & 1 \le i \le p \\ -1 & p < i \le p+q \\ 0 & p+q < i \le p+q+r \end{cases}
///   $$
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CliffordSignature {
    /// Number of positive Euclidean basis vectors ($e_i^2 = +1$).
    pub p: usize,
    /// Number of negative Minkowski basis vectors ($e_j^2 = -1$).
    pub q: usize,
    /// Number of degenerate null basis vectors ($e_k^2 = 0$).
    pub r: usize,
}
```

---

### 3.3 Function & Method Template (`///`)

Every public function and method must detail:
1. Short mathematical action description.
2. Formal equation / transformation.
3. Computational complexity ($O$).
4. Invariant preconditions and failure states.
5. Compile-tested doctests.

```rust
/// Compute the continuous Fourier transform $\mathcal{F}\{f(t)\}(\omega)$ of an analytical expression.
///
/// ## Mathematical Definition
/// $$
/// \mathcal{F}\{f(t)\}(\omega) = \int_{-\infty}^{\infty} f(t) e^{-i \omega t} \mathrm{d}t
/// $$
///
/// ## Complexity
/// - Time Complexity: $O(N)$ lookup for table forms, $O(D^3)$ for Risch differential field reduction.
/// - Space Complexity: $O(N)$ new DAG nodes allocated in `ExprGraph`.
///
/// ## Errors
/// Returns [`AlgebraError::EvaluationError`] if the integral diverges or contains non-integrable singularities along the real axis.
///
/// ## Examples
/// ```rust
/// use urae::prelude::*;
///
/// let graph = ExprGraph::new();
/// let t = graph.symbols.get_or_intern("t");
/// let w = graph.symbols.get_or_intern("w");
/// let one = graph.integer(1);
///
/// let ft = graph.fourier_transform(one, t, w);
/// assert!(ft.is_ok());
/// ```
```

---

## 4. Doctest Verification & CI Integration

### 4.1 Running All Doctests
All doc examples are compiled and run automatically during testing:
```bash
cargo test --doc --workspace
```

### 4.2 Building Documentation with KaTeX Rendering
To build local HTML documentation with KaTeX math formula rendering enabled:
```bash
RUSTDOCFLAGS="--html-in-header docs/katex-header.html" cargo doc --no-deps --workspace --open
```

### 4.3 Documentation Quality Checklist
Before submitting changes or advancing phases:
- [ ] `cargo doc --no-deps --workspace` completes with **0 warnings** and **0 broken links**.
- [ ] `cargo test --doc --workspace` passes **100% of doc-tests**.
- [ ] Android mobile build (`scripts/build-android.ps1` / `scripts/build-android.sh`) verifies with `cargo-ndk`.
- [ ] All public structs, enums, and functions have explicit KaTeX formulas and doc comments.
- [ ] All square brackets in KaTeX formulas are properly escaped (`\[n\]`).
- [ ] Links to macros use `crate::macro_name!` syntax to prevent ambiguous intra-doc link warnings.
