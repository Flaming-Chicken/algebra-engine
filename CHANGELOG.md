# Changelog

All notable changes to `algebra-engine` (URAE) are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Semantic Versioning.

## [Unreleased]

### Added
- Salvaged `Magnitude` (`cat-numbers`), `Hecs<T>` hexagonal coordinates, and `Pose<T>` SE(3) kinematics (`urscript_pose`), re-exported via `urae::prelude`.
- `NumericalOdeMethod::SelfOptimizingMeta`, `ButcherTableau::tsit5()`, and `NumericalPdeSolver::solve_burgers_1d_mol`.
- `urae-py` (`pyproject.toml`, `OdeTrajectorySummary`, `py_*` bindings), `urae-kernel` Jupyter v5.3 message dispatcher, and `urae-wasm` JSON plotting/ODE endpoints.
- Typed `SolveOutcome` (`solveset_outcome`), general $N \times N$ cofactor/adjugate/charpoly + Bareiss RREF/nullspace, and `SymbolicCalculus::integrate_with_steps`.
- `linprog_minimize` (Simplex with Bland's anti-cycling rule) and `yun_square_free_z` (Yun square-free decomposition over $\mathbb{Z}[x]$).
- Truthful `Feature Matrix (URAE vs. SymPy vs. Symbolica)` in `README.md`.
