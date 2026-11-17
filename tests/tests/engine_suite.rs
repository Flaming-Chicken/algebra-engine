//! Workspace Integration Test Suite for `algebra-engine`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "engine/analysis_tests.rs"]
mod analysis_tests;

#[path = "engine/branch_tests.rs"]
mod branch_tests;

#[path = "engine/calculus_calculus_tests.rs"]
mod calculus_calculus_tests;

#[path = "engine/cross_domain_stress_tests.rs"]
mod cross_domain_stress_tests;

#[path = "engine/discrete_and_applied_tests.rs"]
mod discrete_and_applied_tests;

#[path = "engine/distributions_distribution_tests.rs"]
mod distributions_distribution_tests;

#[path = "engine/dynamic_rule_tests.rs"]
mod dynamic_rule_tests;

#[path = "engine/executor_tests.rs"]
mod executor_tests;

#[path = "engine/geometry_geometry_tests.rs"]
mod geometry_geometry_tests;

#[path = "engine/grobner_tests.rs"]
mod grobner_tests;

#[path = "engine/hyperop_and_esoteric_tests.rs"]
mod hyperop_and_esoteric_tests;

#[path = "engine/lifting_tests.rs"]
mod lifting_tests;

#[path = "engine/logic_logic_and_assumptions.rs"]
mod logic_logic_and_assumptions;

#[path = "engine/logic_logic_tests.rs"]
mod logic_logic_tests;

#[path = "engine/matrix_matrix_tests.rs"]
mod matrix_matrix_tests;

#[path = "engine/numeric_numeric_tests.rs"]
mod numeric_numeric_tests;

#[path = "engine/ode_pde_tests.rs"]
mod ode_pde_tests;

#[path = "engine/parser_pipeline_tests.rs"]
mod parser_pipeline_tests;

#[path = "engine/phase6_cartan_tests.rs"]
mod phase6_cartan_tests;

#[path = "engine/physics_physics_tests.rs"]
mod physics_physics_tests;

#[path = "engine/poly_poly_tests.rs"]
mod poly_poly_tests;

#[path = "engine/poly_roots_tests.rs"]
mod poly_roots_tests;

#[path = "engine/polynomial_systems_tests.rs"]
mod polynomial_systems_tests;

#[path = "engine/series_series_tests.rs"]
mod series_series_tests;

#[path = "engine/sets_sets_and_numeric.rs"]
mod sets_sets_and_numeric;

#[path = "engine/sets_sets_tests.rs"]
mod sets_sets_tests;

#[path = "engine/simplify_simplify_tests.rs"]
mod simplify_simplify_tests;

#[path = "engine/solver_solver_tests.rs"]
mod solver_solver_tests;

#[path = "engine/sparse_sparse_tests.rs"]
mod sparse_sparse_tests;

#[path = "engine/special_special_tests.rs"]
mod special_special_tests;

#[path = "engine/systems_tests.rs"]
mod systems_tests;

#[path = "engine/tensor_tensor_tests.rs"]
mod tensor_tensor_tests;

#[path = "engine/transforms_transform_tests.rs"]
mod transforms_transform_tests;

#[path = "engine/transforms_transforms_tests.rs"]
mod transforms_transforms_tests;

#[path = "engine/phase7_performance_tests.rs"]
mod phase7_performance_tests;

#[path = "engine/phase8_risch_limits_tests.rs"]
mod phase8_risch_limits_tests;

#[path = "engine/phase10_probabilistic_heuristics_tests.rs"]
mod phase10_probabilistic_heuristics_tests;

#[path = "engine/phase12_cad_tests.rs"]
mod phase12_cad_tests;

#[path = "engine/phase13_cad_engineering_tests.rs"]
mod phase13_cad_engineering_tests;

#[path = "engine/phase14_fea_multiphysics_tests.rs"]
mod phase14_fea_multiphysics_tests;

#[path = "engine/phase15_simulation_pipeline_tests.rs"]
mod phase15_simulation_pipeline_tests;

#[path = "engine/phase16_curvature_diffalg_tests.rs"]
mod phase16_curvature_diffalg_tests;

#[path = "engine/phase17_visualizer_gpu_tests.rs"]
mod phase17_visualizer_gpu_tests;

#[path = "engine/phase20_symplectic_quantum_tests.rs"]
mod phase20_symplectic_quantum_tests;

#[path = "engine/phase21_autodiff_neural_ode_tests.rs"]
mod phase21_autodiff_neural_ode_tests;

#[path = "engine/phase22_tda_persistent_homology_tests.rs"]
mod phase22_tda_persistent_homology_tests;

#[path = "engine/phase23_noneuclidean_geometry_tests.rs"]
mod phase23_noneuclidean_geometry_tests;

#[path = "engine/phase24_smt_formal_verification_tests.rs"]
mod phase24_smt_formal_verification_tests;

#[path = "engine/phase25_geometric_deep_learning_tests.rs"]
mod phase25_geometric_deep_learning_tests;

#[path = "engine/phase26_qft_feynman_amplitudes_tests.rs"]
mod phase26_qft_feynman_amplitudes_tests;

#[path = "engine/phase27_holonomic_zeilberger_tests.rs"]
mod phase27_holonomic_zeilberger_tests;

#[path = "engine/phase28_cft_virasoro_kac_moody_tests.rs"]
mod phase28_cft_virasoro_kac_moody_tests;

#[path = "engine/phase29_exceptional_lie_octonions_tests.rs"]
mod phase29_exceptional_lie_octonions_tests;

#[path = "engine/phase30_padic_hodge_fontaine_tests.rs"]
mod phase30_padic_hodge_fontaine_tests;

#[path = "engine/phase31_iga_nurbs_cad_tests.rs"]
mod phase31_iga_nurbs_cad_tests;

#[path = "engine/interconnect_tests.rs"]
mod interconnect_tests;

#[path = "engine/lossless_solver_tests.rs"]
mod lossless_solver_tests;

#[path = "engine/radical_polynomial_tests.rs"]
mod radical_polynomial_tests;

#[path = "engine/analytical_pde_tests.rs"]
mod analytical_pde_tests;

#[path = "engine/diffeq_classification_tests.rs"]
mod diffeq_classification_tests;

#[path = "engine/control_systems_tests.rs"]
mod control_systems_tests;

#[path = "engine/reference_oracle_tests.rs"]
mod reference_oracle_tests;

#[path = "engine/audit_regression_tests.rs"]
mod audit_regression_tests;

#[path = "engine/audit_followup_tests.rs"]
mod audit_followup_tests;

#[path = "engine/uncertainty_multivalued_tests.rs"]
mod uncertainty_multivalued_tests;

#[path = "engine/unification_robustness_tests.rs"]
mod unification_robustness_tests;

#[path = "engine/api_stream_workbench_tests.rs"]
mod api_stream_workbench_tests;

#[path = "engine/known_defect_tests.rs"]
mod known_defect_tests;

#[path = "engine/control_general_order_tests.rs"]
mod control_general_order_tests;
