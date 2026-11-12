//! # `urae_notebook::notebook::session`
//!
//! Session Persistence, Document Configuration, and BSON Export/Import.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[allow(dead_code)]
pub const SESSION_FILE_NAME: &str = "urae_notebook_session.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardDisplayMode {
    Inline,
    Hidden,
    PoppedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolRole {
    Variable,
    Parameter,
    Constant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolMetadata {
    pub name: String,
    pub role: SymbolRole,
    pub cur_val: f64,
    pub min_val: f64,
    pub max_val: f64,
    pub domain_type: String, // "Real", "Complex", "Integer", "Quaternion", "Grassmann", "Positive", "NonNegative"
    pub unit_str: Option<String>,
    #[serde(default)]
    pub parsed_text_val: Option<f64>,
    #[serde(default)]
    pub is_function: bool,
    #[serde(default)]
    pub function_args: Vec<String>,
    #[serde(default)]
    pub is_distribution: bool,
    #[serde(default)]
    pub tensor_rank: Option<usize>,
    #[serde(default)]
    pub tensor_shape: Vec<usize>,
    #[serde(default)]
    pub is_constant_locked: bool,
}

impl SymbolMetadata {
    pub fn new(name: impl Into<String>, val: f64, role: SymbolRole) -> Self {
        Self {
            name: name.into(),
            role,
            cur_val: val,
            min_val: -10.0,
            max_val: 10.0,
            domain_type: "Real".to_string(),
            unit_str: None,
            parsed_text_val: Some(val),
            is_function: false,
            function_args: Vec::new(),
            is_distribution: false,
            tensor_rank: None,
            tensor_shape: Vec::new(),
            is_constant_locked: role == SymbolRole::Constant,
        }
    }

    /// Retrieve all concurrent compound mathematical classification tags for this symbol.
    pub fn compound_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        match self.role {
            SymbolRole::Constant => tags.push("Constant".to_string()),
            SymbolRole::Parameter => tags.push("Parameter".to_string()),
            SymbolRole::Variable => tags.push("Variable".to_string()),
        }
        if self.is_function {
            if self.function_args.is_empty() {
                tags.push("Function".to_string());
            } else {
                tags.push(format!("Function of ({})", self.function_args.join(", ")));
            }
        }
        if self.is_distribution {
            tags.push("Distribution".to_string());
        }
        match self.tensor_rank {
            Some(0) | None => tags.push("Scalar".to_string()),
            Some(1) => {
                if let Some(dim) = self.tensor_shape.first() {
                    tags.push(format!("Vector ({}D)", dim));
                } else {
                    tags.push("Vector".to_string());
                }
            }
            Some(2) => {
                if self.tensor_shape.len() >= 2 {
                    tags.push(format!(
                        "Matrix ({}×{})",
                        self.tensor_shape[0], self.tensor_shape[1]
                    ));
                } else {
                    tags.push("Matrix".to_string());
                }
            }
            Some(r) => {
                let dims = self
                    .tensor_shape
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("×");
                if dims.is_empty() {
                    tags.push(format!("Rank-{} Tensor", r));
                } else {
                    tags.push(format!("Rank-{} Tensor ({})", r, dims));
                }
            }
        }
        tags.push(self.domain_type.clone());
        if let Some(u) = &self.unit_str {
            tags.push(format!("[{}]", u));
        }
        tags
    }

    /// Retrieve distinct, non-redundant mathematical classification badges for sidebar display.
    /// Excludes the base role (shown on the conversion button), base domain (shown as subtitle),
    /// the default "Scalar" tag (which applies to almost every mathematical variable and clutters cards),
    /// and raw unit strings (shown alongside the value or slider).
    pub fn distinct_badges(&self) -> Vec<String> {
        self.compound_tags()
            .into_iter()
            .filter(|tag| {
                tag != "Parameter"
                    && tag != "Variable"
                    && tag != "Constant"
                    && tag != "Scalar"
                    && tag != &self.domain_type
                    && !self
                        .unit_str
                        .as_ref()
                        .is_some_and(|u| tag == &format!("[{}]", u))
            })
            .collect()
    }

    /// Clamp current value within min_val..max_val and domain restrictions.
    pub fn clamp(&self, val: f64) -> f64 {
        let mut clamped = val.clamp(self.min_val, self.max_val);
        match self.domain_type.as_str() {
            "Positive" if clamped <= 0.0 => {
                clamped = 0.001;
            }
            "NonNegative" if clamped < 0.0 => {
                clamped = 0.0;
            }
            "Integer" => {
                clamped = clamped.round();
            }
            _ => {}
        }
        clamped
    }

    /// Switch symbol role while retaining all existing domain, bounds, and unit metadata.
    pub fn set_role(&mut self, new_role: SymbolRole) {
        self.role = new_role;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReactiveComputeMode {
    /// Adaptive Dual-Rate (Recommended Default):
    /// Instant 60 FPS plot curves and algebraic updates during drag, with soft-cap
    /// lightweight previews for heavy solvers, finalizing to full precision on release.
    #[default]
    AdaptiveDualRate,

    /// Soft-Cap Fast Drag:
    /// Clamps ODE steps and Newton iterations strictly during active mouse scrub,
    /// re-evaluating at full precision when the drag ends.
    SoftCapFastDrag,

    /// Frame-Budgeted Guard (8ms budget):
    /// Executes synchronously if DAG delta takes < 8ms; automatically delegates
    /// heavier sub-graphs to background workers if the frame budget is exceeded.
    FrameBudgeted,

    /// Full Synchronous:
    /// Re-evaluates exact full precision on every micro-tick (ideal for high-end multi-core desktop workstations).
    FullSynchronous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MatrixPresetKind {
    #[default]
    Custom,
    Identity,
    Zero,
    Diagonal,
    Symmetric,
    PauliX,
    PauliY,
    PauliZ,
}

/// Palette code generator for multi-dimensional matrices and tensors.
pub fn generate_matrix_syntax(
    rows: usize,
    cols: usize,
    preset: MatrixPresetKind,
    elements: Option<&Vec<Vec<String>>>,
) -> String {
    match preset {
        MatrixPresetKind::Identity => {
            format!("M = eye({})", rows.max(1))
        }
        MatrixPresetKind::Zero => {
            format!("M = zeros({}, {})", rows.max(1), cols.max(1))
        }
        MatrixPresetKind::Diagonal => {
            let diag_elems: Vec<String> = (1..=rows).map(|i| format!("d_{}", i)).collect();
            format!("M = diag([{}])", diag_elems.join(", "))
        }
        MatrixPresetKind::Symmetric => {
            let mut row_strs = Vec::new();
            for r in 0..rows {
                let mut col_strs = Vec::new();
                for c in 0..cols {
                    let (min_i, max_i) = if r <= c {
                        (r + 1, c + 1)
                    } else {
                        (c + 1, r + 1)
                    };
                    col_strs.push(format!("s_{}{}", min_i, max_i));
                }
                row_strs.push(format!("[{}]", col_strs.join(", ")));
            }
            format!("M = matrix([{}])", row_strs.join(", "))
        }
        MatrixPresetKind::PauliX => "sigma_x = matrix([[0, 1], [1, 0]])".to_string(),
        MatrixPresetKind::PauliY => "sigma_y = matrix([[0, -i], [i, 0]])".to_string(),
        MatrixPresetKind::PauliZ => "sigma_z = matrix([[1, 0], [0, -1]])".to_string(),
        MatrixPresetKind::Custom => {
            if let Some(grid) = elements {
                let mut row_strs = Vec::new();
                for r in grid {
                    let col_strs: Vec<String> = r
                        .iter()
                        .map(|s| {
                            if s.trim().is_empty() {
                                "0".to_string()
                            } else {
                                s.trim().to_string()
                            }
                        })
                        .collect();
                    row_strs.push(format!("[{}]", col_strs.join(", ")));
                }
                format!("M = matrix([{}])", row_strs.join(", "))
            } else {
                format!("M = zeros({}, {})", rows.max(1), cols.max(1))
            }
        }
    }
}

/// Palette code generator for set-builder intervals.
pub fn generate_interval_syntax(
    var: &str,
    domain: &str,
    min: f64,
    max: f64,
    inc_min: bool,
    inc_max: bool,
) -> String {
    let left_op = if inc_min { "<=" } else { "<" };
    let right_op = if inc_max { "<=" } else { "<" };
    format!(
        "{{ {} in {} | {:.2} {} {} {} {:.2} }}",
        var.trim(),
        domain.trim(),
        min,
        left_op,
        var.trim(),
        right_op,
        max
    )
}

/// Specification parameter bundle for universal number system variable & parameter builder.
#[derive(Debug, Clone)]
pub struct ParameterBuilderParams<'a> {
    pub var: &'a str,
    pub is_param: bool,
    pub category: usize, // 0 = Standard/Discrete, 1 = Cayley-Dickson, 2 = Adjoined, 3 = Non-Archimedean, 4 = Modular/Galois, 5 = Matrix/Tensor
    pub standard_kind: usize, // 0 = Reals, 1 = Positive, 2 = NonNegative, 3 = Integers, 4 = Naturals, 5 = Rationals, 6 = Complex
    pub cayley_depth: usize,  // 0..=5
    pub adjoin_i: bool,
    pub adjoin_eps: bool,
    pub adjoin_j: bool,
    pub adjoin_clifford: bool,
    pub padic_prime: u32,
    pub padic_valuation: i32,
    pub surreal_generation: u32,
    pub modulo_n: u64,
    pub galois_prime: u64,
    pub galois_power: u32,
    pub matrix_rows: usize,
    pub matrix_cols: usize,
    pub matrix_domain: &'a str,
    pub tensor_rank: u32,
    pub discrete_kind: usize, // 0 = Modulo, 1 = Integers/Step, 2 = GaloisField, 3 = Gaussian/Eisenstein, 4 = Boolean/BitVector
    pub modulo_rep: usize,    // 0 = Canonical [0, n-1], 1 = Balanced [-n/2, n/2], 2 = Units (Z/nZ)*
    pub congruence_rem: i64,
    pub congruence_mod: u64,
    pub has_congruence: bool,
    pub integer_step: u64,
    pub integer_parity: usize, // 0 = Any, 1 = Even (2Z), 2 = Odd (2Z+1), 3 = Multiple of k
    pub integer_multiple: u64,
    pub lattice_kind: usize, // 0 = Gaussian Z[i], 1 = Eisenstein Z[omega]
    pub bit_width: u32,
    pub bit_signed: bool,
    pub coords: &'a [(String, f64, f64, bool)],
}

impl<'a> Default for ParameterBuilderParams<'a> {
    fn default() -> Self {
        Self {
            var: "x",
            is_param: true,
            category: 0,
            standard_kind: 0,
            cayley_depth: 0,
            adjoin_i: false,
            adjoin_eps: false,
            adjoin_j: false,
            adjoin_clifford: false,
            padic_prime: 7,
            padic_valuation: 0,
            surreal_generation: 4,
            modulo_n: 12,
            galois_prime: 2,
            galois_power: 8,
            matrix_rows: 3,
            matrix_cols: 3,
            matrix_domain: "Reals",
            tensor_rank: 2,
            discrete_kind: 0,
            modulo_rep: 0,
            congruence_rem: 0,
            congruence_mod: 2,
            has_congruence: false,
            integer_step: 1,
            integer_parity: 0,
            integer_multiple: 2,
            lattice_kind: 0,
            bit_width: 8,
            bit_signed: false,
            coords: &[],
        }
    }
}

/// Universal code generator for Parameter & Variable Builder covering all mathematical number systems.
pub fn generate_universal_parameter_builder_syntax(params: &ParameterBuilderParams) -> String {
    let clean_var = if params.var.trim().is_empty() {
        "x"
    } else {
        params.var.trim()
    };
    let role_str = if params.is_param {
        "Parameter"
    } else {
        "Variable"
    };

    match params.category {
        0 => {
            // Category 0: Standard & Discrete Continuum
            let domain_name = match params.standard_kind {
                0 => "Reals",
                1 => "Positive",
                2 => "NonNegative",
                3 => "Integers",
                4 => "Naturals",
                5 => "Rationals",
                _ => "Complex",
            };

            if params.standard_kind == 6 {
                // Complex number system with Re/Im coordinate bounds
                let mut bound_parts = Vec::new();
                for (coord_name, min_v, max_v, enabled) in params.coords {
                    if *enabled {
                        bound_parts.push(format!(
                            "{}({}) in [{:.2}, {:.2}]",
                            coord_name, clean_var, min_v, max_v
                        ));
                    }
                }
                if bound_parts.is_empty() {
                    format!("{}: {} in Complex", clean_var, role_str)
                } else {
                    format!(
                        "{}: {} in Complex where {}",
                        clean_var,
                        role_str,
                        bound_parts.join(", ")
                    )
                }
            } else {
                // Scalar domains (Reals, Integers, Positive, etc.)
                if let Some((_, min_v, max_v, enabled)) = params.coords.first() {
                    if *enabled {
                        format!(
                            "{}: {} in {} [{:.2}, {:.2}]",
                            clean_var, role_str, domain_name, min_v, max_v
                        )
                    } else {
                        format!("{}: {} in {}", clean_var, role_str, domain_name)
                    }
                } else {
                    format!("{}: {} in {}", clean_var, role_str, domain_name)
                }
            }
        }
        1 => {
            // Category 1: Cayley-Dickson Algebra Class with Depth
            let class_name = match params.cayley_depth {
                0 => "Reals",
                1 => "Complex",
                2 => "Quaternion",
                3 => "Octonion",
                4 => "Sedenion",
                _ => "Trigintaduonion",
            };

            let mut bound_parts = Vec::new();
            for (coord_name, min_v, max_v, enabled) in params.coords {
                if *enabled {
                    bound_parts.push(format!(
                        "{}({}) in [{:.2}, {:.2}]",
                        coord_name, clean_var, min_v, max_v
                    ));
                }
            }

            if bound_parts.is_empty() {
                format!(
                    "{}: {} in CayleyDickson(depth={}) /* {} */",
                    clean_var, role_str, params.cayley_depth, class_name
                )
            } else {
                format!(
                    "{}: {} in CayleyDickson(depth={}) /* {} */ where {}",
                    clean_var,
                    role_str,
                    params.cayley_depth,
                    class_name,
                    bound_parts.join(", ")
                )
            }
        }
        2 => {
            // Category 2: Adjoin Elements / Generators to Base Field
            let algebra_desc = match (
                params.adjoin_i,
                params.adjoin_eps,
                params.adjoin_j,
                params.adjoin_clifford,
            ) {
                (true, false, false, false) => "Complex",
                (false, true, false, false) => "Dual(1, epsilon)",
                (true, true, false, false) => "DualComplex(1, i, epsilon)",
                (false, false, true, false) => "SplitComplex(1, j)",
                (false, false, false, true) => "Clifford(e1, e2)",
                (true, false, true, false) => "Bicomplex(1, i, j)",
                _ => "Reals",
            };

            let mut bound_parts = Vec::new();
            for (coord_name, min_v, max_v, enabled) in params.coords {
                if *enabled {
                    bound_parts.push(format!(
                        "{}({}) in [{:.2}, {:.2}]",
                        coord_name, clean_var, min_v, max_v
                    ));
                }
            }

            if bound_parts.is_empty() {
                format!("{}: {} in {}", clean_var, role_str, algebra_desc)
            } else {
                format!(
                    "{}: {} in {} where {}",
                    clean_var,
                    role_str,
                    algebra_desc,
                    bound_parts.join(", ")
                )
            }
        }
        3 => {
            // Category 3: Non-Archimedean & Infinitesimals (p-Adics, Surreals, Adeles)
            match params.standard_kind {
                0 => {
                    format!(
                        "{}: {} in PAdics(p={}) where valuation({}) >= {}",
                        clean_var, role_str, params.padic_prime, clean_var, params.padic_valuation
                    )
                }
                1 => {
                    format!(
                        "{}: {} in Surreals where generation({}) <= {}",
                        clean_var, role_str, clean_var, params.surreal_generation
                    )
                }
                _ => {
                    format!("{}: {} in Adeles", clean_var, role_str)
                }
            }
        }
        4 => {
            // Category 4: Discrete Number Systems & Modulo Arithmetic
            let discrete_kind = if params.discrete_kind == 0 && params.standard_kind == 1 {
                2
            } else {
                params.discrete_kind
            };
            match discrete_kind {
                0 => {
                    // Modular Arithmetic Z/nZ
                    let mod_n = params.modulo_n.max(2);
                    let cong_str = if params.has_congruence {
                        format!(
                            ", {} == {} (mod {})",
                            clean_var, params.congruence_rem, params.congruence_mod
                        )
                    } else {
                        String::new()
                    };

                    match params.modulo_rep {
                        0 => {
                            // Canonical residue: [0, n-1]
                            format!(
                                "{}: {} in Modulo(n={}) where {} in [0, {}]{}",
                                clean_var,
                                role_str,
                                mod_n,
                                clean_var,
                                mod_n.saturating_sub(1),
                                cong_str
                            )
                        }
                        1 => {
                            // Balanced / Symmetric residue: [-lower, upper]
                            let upper = (mod_n as i64) / 2;
                            let lower = -((mod_n as i64 - 1) / 2);
                            format!(
                                "{}: {} in Modulo(n={}, symmetric=true) where {} in [{}, {}]{}",
                                clean_var, role_str, mod_n, clean_var, lower, upper, cong_str
                            )
                        }
                        _ => {
                            // Units multiplicative group (Z/nZ)*
                            format!(
                                "{}: {} in ModuloUnits(n={}) /* (Z/{}Z)* gcd({}, {})=1 */{}",
                                clean_var, role_str, mod_n, mod_n, clean_var, mod_n, cong_str
                            )
                        }
                    }
                }
                1 => {
                    // Discrete Integers with step size and parity/divisibility
                    let min_v = params.coords.first().map(|c| c.1 as i64).unwrap_or(0);
                    let max_v = params.coords.first().map(|c| c.2 as i64).unwrap_or(100);
                    match params.integer_parity {
                        1 => {
                            format!(
                                "{}: {} in EvenIntegers [{}, {}]",
                                clean_var, role_str, min_v, max_v
                            )
                        }
                        2 => {
                            format!(
                                "{}: {} in OddIntegers [{}, {}]",
                                clean_var, role_str, min_v, max_v
                            )
                        }
                        3 => {
                            let k = params.integer_multiple.max(2);
                            format!(
                                "{}: {} in Integers where {} in {}*Integers [{}, {}]",
                                clean_var, role_str, clean_var, k, min_v, max_v
                            )
                        }
                        _ => {
                            let step = params.integer_step.max(1);
                            if step > 1 {
                                format!(
                                    "{}: {} in Integers [{}, {}] step {}",
                                    clean_var, role_str, min_v, max_v, step
                                )
                            } else {
                                format!(
                                    "{}: {} in Integers [{}, {}]",
                                    clean_var, role_str, min_v, max_v
                                )
                            }
                        }
                    }
                }
                2 => {
                    // Finite Galois Field GF(p^k)
                    format!(
                        "{}: {} in GaloisField(prime={}, power={})",
                        clean_var, role_str, params.galois_prime, params.galois_power
                    )
                }
                3 => {
                    // Discrete Complex Lattices (Gaussian Z[i] & Eisenstein Z[omega])
                    let min_re = params.coords.get(0).map(|c| c.1 as i64).unwrap_or(-5);
                    let max_re = params.coords.get(0).map(|c| c.2 as i64).unwrap_or(5);
                    let min_im = params.coords.get(1).map(|c| c.1 as i64).unwrap_or(-5);
                    let max_im = params.coords.get(1).map(|c| c.2 as i64).unwrap_or(5);

                    if params.lattice_kind == 0 {
                        format!(
                            "{}: {} in GaussianIntegers /* Z[i] */ where Re({}) in [{}, {}], Im({}) in [{}, {}]",
                            clean_var,
                            role_str,
                            clean_var,
                            min_re,
                            max_re,
                            clean_var,
                            min_im,
                            max_im
                        )
                    } else {
                        format!(
                            "{}: {} in EisensteinIntegers /* Z[omega] */ where Re({}) in [{}, {}], Im({}) in [{}, {}]",
                            clean_var,
                            role_str,
                            clean_var,
                            min_re,
                            max_re,
                            clean_var,
                            min_im,
                            max_im
                        )
                    }
                }
                _ => {
                    // Boolean Logic & Bit-Vectors
                    if params.bit_width <= 1 {
                        format!("{}: {} in Boolean", clean_var, role_str)
                    } else {
                        format!(
                            "{}: {} in BitVector(width={}, signed={})",
                            clean_var, role_str, params.bit_width, params.bit_signed
                        )
                    }
                }
            }
        }
        _ => {
            // Category 5: Matrix & Multilinear Tensor Spaces
            if params.standard_kind == 0 {
                let dom = if params.matrix_domain.trim().is_empty() {
                    "Reals"
                } else {
                    params.matrix_domain.trim()
                };
                format!(
                    "{}: {} in Matrix(rows={}, cols={}, domain={})",
                    clean_var, role_str, params.matrix_rows, params.matrix_cols, dom
                )
            } else {
                let dom = if params.matrix_domain.trim().is_empty() {
                    "Reals"
                } else {
                    params.matrix_domain.trim()
                };
                format!(
                    "{}: {} in Tensor(rank={}, domain={})",
                    clean_var, role_str, params.tensor_rank, dom
                )
            }
        }
    }
}

/// Interactive code generator for Parameter & Variable Builder (backwards-compatible wrapper).
pub fn generate_parameter_builder_syntax(
    var: &str,
    is_param: bool,
    mode: usize,
    adjoin_i: bool,
    adjoin_eps: bool,
    adjoin_j: bool,
    adjoin_clifford: bool,
    cayley_depth: usize,
    coords: &[(String, f64, f64, bool)],
) -> String {
    let params = ParameterBuilderParams {
        var,
        is_param,
        category: if mode == 0 { 2 } else { 1 },
        standard_kind: 0,
        cayley_depth,
        adjoin_i,
        adjoin_eps,
        adjoin_j,
        adjoin_clifford,
        padic_prime: 7,
        padic_valuation: 0,
        surreal_generation: 4,
        modulo_n: 12,
        galois_prime: 2,
        galois_power: 8,
        matrix_rows: 3,
        matrix_cols: 3,
        matrix_domain: "Reals",
        tensor_rank: 2,
        discrete_kind: 0,
        modulo_rep: 0,
        congruence_rem: 0,
        congruence_mod: 1,
        has_congruence: false,
        integer_step: 1,
        integer_parity: 0,
        integer_multiple: 2,
        lattice_kind: 0,
        bit_width: 8,
        bit_signed: false,
        coords,
    };
    generate_universal_parameter_builder_syntax(&params)
}

/// Palette code generator for Riemann branch cuts.
pub fn generate_branch_cut_syntax(fn_name: &str, sheet_k: i32, cut_pos: &str) -> String {
    format!(
        "branch_cut(\"{}\", sheet = {}, cut = \"{}\")",
        fn_name.trim(),
        sheet_k,
        cut_pos.trim()
    )
}

/// Palette code generator for ODE/PDE initial and boundary conditions.
pub fn generate_ode_bc_syntax(var_dep: &str, var_indep: &str, y0: f64, dy0: Option<f64>) -> String {
    if let Some(dy) = dy0 {
        format!(
            "{}(0) = {:.2}, {}'({}) = {:.2}",
            var_dep.trim(),
            y0,
            var_dep.trim(),
            var_indep.trim(),
            dy
        )
    } else {
        format!("{}(0) = {:.2}", var_dep.trim(), y0)
    }
}

/// Palette code generator for physical SI units.
pub fn generate_physical_unit_syntax(var: &str, val: f64, unit: &str) -> String {
    format!("{} = {:.2} [{}]", var.trim(), val, unit.trim())
}

/// Palette code generator for general n-rank tensors (scalars, vectors, matrices, rank-3+ tensors).
pub fn generate_tensor_syntax(
    var: &str,
    rank: usize,
    shape: &[usize],
    preset: &str,
    elements: Option<&[String]>,
) -> String {
    let clean_var = var.trim();
    let name_prefix = if clean_var.is_empty() { "T" } else { clean_var };

    match rank {
        0 => {
            let val = elements
                .and_then(|e| e.first())
                .map(|s| s.as_str())
                .unwrap_or("0");
            format!("{}: Scalar = {}", name_prefix, val)
        }
        1 => {
            let dim = shape.first().copied().unwrap_or(3);
            let elems = if let Some(e) = elements {
                e.iter().take(dim).cloned().collect::<Vec<_>>().join(", ")
            } else {
                vec!["0".to_string(); dim].join(", ")
            };
            format!("{}: Vector = [{}]", name_prefix, elems)
        }
        2 => {
            let rows = shape.first().copied().unwrap_or(2);
            let cols = shape.get(1).copied().unwrap_or(2);
            if preset == "Identity" {
                format!("{}: Matrix = eye({}, {})", name_prefix, rows, cols)
            } else if preset == "Zero" {
                format!("{}: Matrix = zeros({}, {})", name_prefix, rows, cols)
            } else if preset == "PauliX" {
                format!("{}: Matrix = [[0, 1], [1, 0]]", name_prefix)
            } else if preset == "PauliY" {
                format!("{}: Matrix = [[0, -i], [i, 0]]", name_prefix)
            } else if preset == "PauliZ" {
                format!("{}: Matrix = [[1, 0], [0, -1]]", name_prefix)
            } else if let Some(e) = elements {
                let mut row_strs = Vec::new();
                for r in 0..rows {
                    let mut row_items = Vec::new();
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let item = e.get(idx).map(|s| s.as_str()).unwrap_or("0");
                        row_items.push(item);
                    }
                    row_strs.push(format!("[{}]", row_items.join(", ")));
                }
                format!("{}: Matrix = [{}]", name_prefix, row_strs.join(", "))
            } else {
                format!("{}: Matrix = zeros({}, {})", name_prefix, rows, cols)
            }
        }
        _ => {
            let shape_str = shape
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}: Tensor = tensor(shape = [{}], preset = \"{}\")",
                name_prefix, shape_str, preset
            )
        }
    }
}

/// Palette code generator for complex PDE Initial and Boundary Value Problems (IBVP).
pub fn generate_pde_bc_syntax(
    pde_kind: &str,
    var_dep: &str,
    spatial_vars: &[String],
    time_var: Option<&str>,
    bc_kind: &str,
    bcs: &[String],
    ics: &[String],
) -> String {
    let u = var_dep.trim();
    let spat = if spatial_vars.is_empty() {
        "x".to_string()
    } else {
        spatial_vars.join(", ")
    };
    let t_part = time_var.map(|t| format!(", {}", t)).unwrap_or_default();

    let mut parts = Vec::new();
    // 1. PDE Classification Header
    parts.push(format!(
        "// PDE Model: {} for {}({}{})",
        pde_kind, u, spat, t_part
    ));

    // 2. Boundary Conditions
    if !bcs.is_empty() {
        for bc in bcs {
            parts.push(format!("bc: {} | {}", bc_kind, bc));
        }
    } else {
        parts.push(format!(
            "bc: {} | {}(0{}) = 0, {}(L{}) = 0",
            bc_kind, u, t_part, u, t_part
        ));
    }

    // 3. Initial Conditions (if time-dependent)
    if time_var.is_some() {
        if !ics.is_empty() {
            for ic in ics {
                parts.push(format!("ic: {}", ic));
            }
        } else {
            parts.push(format!("ic: {}({}, 0) = f({})", u, spat, spat));
            if pde_kind.to_lowercase().contains("wave") {
                parts.push(format!("ic: {}_t({}, 0) = 0", u, spat));
            }
        }
    }

    parts.join("\n")
}

/// Palette code generator for Parametric CAD & Gear Machinery.
pub fn generate_gear_cad_syntax(
    gear_kind: &str,
    teeth: usize,
    module_val: f64,
    pressure_angle: f64,
    face_width: f64,
    extra_params: &[(&str, f64)],
) -> String {
    let mut args = vec![
        format!("type = \"{}\"", gear_kind),
        format!("teeth = {}", teeth),
        format!("module = {:.2}", module_val),
        format!("pressure_angle = {:.1}", pressure_angle),
        format!("face_width = {:.1}", face_width),
    ];
    for (k, v) in extra_params {
        args.push(format!("{} = {:.2}", k, v));
    }
    format!("gear!({})", args.join(", "))
}

/// Palette code generator for Dimensional Analysis across unit systems.
pub fn generate_dimensional_unit_syntax(
    var: &str,
    val: f64,
    unit_system: &str,
    l: i32,
    m: i32,
    t: i32,
    i_curr: i32,
    th: i32,
    n: i32,
    j_lum: i32,
) -> (String, String) {
    let (u_l, u_m, u_t, u_i, u_th, u_n, u_j) = match unit_system {
        "Imperial" => ("ft", "lb", "s", "A", "degF", "mol", "cd"),
        "CGS" => ("cm", "g", "s", "A", "K", "mol", "cd"),
        "Natural" => ("l_p", "m_p", "t_p", "q_p", "T_p", "mol", "cd"),
        _ => ("m", "kg", "s", "A", "K", "mol", "cd"),
    };

    let mut numerators = Vec::new();
    let mut denominators = Vec::new();

    let push_dim = |arr_num: &mut Vec<String>, arr_den: &mut Vec<String>, name: &str, exp: i32| {
        if exp > 0 {
            if exp == 1 {
                arr_num.push(name.to_string());
            } else {
                arr_num.push(format!("{}^{}", name, exp));
            }
        } else if exp < 0 {
            let pos = exp.abs();
            if pos == 1 {
                arr_den.push(name.to_string());
            } else {
                arr_den.push(format!("{}^{}", name, pos));
            }
        }
    };

    push_dim(&mut numerators, &mut denominators, u_m, m);
    push_dim(&mut numerators, &mut denominators, u_l, l);
    push_dim(&mut numerators, &mut denominators, u_t, t);
    push_dim(&mut numerators, &mut denominators, u_i, i_curr);
    push_dim(&mut numerators, &mut denominators, u_th, th);
    push_dim(&mut numerators, &mut denominators, u_n, n);
    push_dim(&mut numerators, &mut denominators, u_j, j_lum);

    let unit_str = if numerators.is_empty() && denominators.is_empty() {
        "1".to_string()
    } else if denominators.is_empty() {
        numerators.join("·")
    } else if numerators.is_empty() {
        format!("1/({})", denominators.join("·"))
    } else {
        format!("{}/({})", numerators.join("·"), denominators.join("·"))
    };

    let clean_unit = unit_str.replace("/(1)", "");
    let syntax = format!("{} = {:.4} [{}]", var.trim(), val, clean_unit);
    (clean_unit, syntax)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotebookSettings {
    pub auto_sliders: bool,
    pub auto_plots: bool,
    pub show_hover_tooltips: bool,
    pub show_derivatives: bool,
    pub default_range_min: f64,
    pub default_range_max: f64,
    pub font_size: f32,
    pub logging_level: String,
    pub ai_config: urae::agent::AiConfig,
    pub reactive_mode: ReactiveComputeMode,
    pub target_frame_budget_ms: f64,
    pub auto_throttle_enabled: bool,
    pub theme: crate::ui::ThemeKind,
    pub workspace_preset: crate::ui::WorkspaceLayoutPreset,
    pub show_cell_line_numbers: bool,

    // Left Panel Features
    pub left_panel_show_sliders: bool,
    pub left_panel_show_domains: bool,
    pub left_panel_show_values: bool,
    pub left_panel_show_badges: bool,

    // Text Editor Features
    pub show_editor: bool,
    pub editor_syntax_highlighting: bool,
    pub editor_word_wrap: bool,
    pub editor_alt_scrubbing: bool,

    // Right Panel Features
    pub right_panel_show_plots: bool,
    pub right_panel_show_3d: bool,
    pub right_panel_show_cad: bool,
    pub right_panel_show_solutions: bool,
    pub right_panel_compact_mode: bool,
    pub right_panel_show_inbound_refs: bool,

    // Terminal Features
    pub terminal_show_timing: bool,
}

pub fn default_logging_level() -> String {
    if cfg!(test) {
        "warn".to_string()
    } else if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "error".to_string()
    }
}

impl Default for NotebookSettings {
    fn default() -> Self {
        Self {
            auto_sliders: true,
            auto_plots: true,
            show_hover_tooltips: true,
            show_derivatives: true,
            default_range_min: -5.0,
            default_range_max: 5.0,
            font_size: 15.0,
            logging_level: default_logging_level(),
            ai_config: urae::agent::AiConfig::default(),
            reactive_mode: ReactiveComputeMode::default(),
            target_frame_budget_ms: 8.0,
            auto_throttle_enabled: true,
            theme: crate::ui::ThemeKind::default(),
            workspace_preset: crate::ui::WorkspaceLayoutPreset::default(),
            show_cell_line_numbers: true,

            left_panel_show_sliders: true,
            left_panel_show_domains: true,
            left_panel_show_values: true,
            left_panel_show_badges: true,
            show_editor: true,
            editor_syntax_highlighting: true,
            editor_word_wrap: true,
            editor_alt_scrubbing: true,
            right_panel_show_plots: true,
            right_panel_show_3d: true,
            right_panel_show_cad: true,
            right_panel_show_solutions: true,
            right_panel_compact_mode: false,
            right_panel_show_inbound_refs: true,
            terminal_show_timing: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionData {
    pub raw_document_text: String,
    pub slider_values: HashMap<String, f64>,
    pub symbol_metadata: HashMap<String, SymbolMetadata>,
    pub card_modes: HashMap<String, CardDisplayMode>,
    pub explicit_plots: HashSet<String>,
    pub settings: NotebookSettings,
}

/// Export `SessionData` to zlib-compressed BSON binary blob
pub fn export_session_to_compressed_bson(session: &SessionData) -> Result<Vec<u8>, String> {
    spodeian_export::export_to_compressed_bson(session).map_err(|e| format!("BSON serialization error: {e}"))
}

/// Import `SessionData` from compressed or uncompressed BSON binary blob
pub fn import_session_from_compressed_bson(bytes: &[u8]) -> Result<SessionData, String> {
    spodeian_export::import_from_compressed_bson(bytes).map_err(|e| format!("BSON deserialization error: {e}"))
}


impl Default for SessionData {
    fn default() -> Self {
        let mut metadata = HashMap::new();
        metadata.insert(
            "a".to_string(),
            SymbolMetadata::new("a", 2.0, SymbolRole::Parameter),
        );
        metadata.insert(
            "c".to_string(),
            SymbolMetadata::new("c", -3.0, SymbolRole::Parameter),
        );
        metadata.insert(
            "x".to_string(),
            SymbolMetadata::new("x", 0.0, SymbolRole::Variable),
        );

        let mut slider_values = HashMap::new();
        slider_values.insert("a".to_string(), 2.0);
        slider_values.insert("c".to_string(), -3.0);

        Self {
            raw_document_text: [
                "# URAE Mathematics Notepad",
                "// Declare variables, parameters, set builder domains, and physical units below.",
                "",
                "a: Parameter = 2.00 [m]",
                "c: Parameter = -3.00 [m]",
                "x: Variable",
                "{ x in Reals | -5 <= x <= 5 }",
                "{ q in Quaternion | norm(q) == 1 }",
                "{ v in Grassmann | v^2 == 0 }",
                "f(x) = a * x^2 + c",
                "",
                "g = 9.81 [m/s^2]",
                "y = 1.50 * cos(2.00 * x)",
                "g(x) = x^3 - 3 * x + 2",
            ]
            .join("\n"),
            slider_values,
            symbol_metadata: metadata,
            card_modes: HashMap::new(),
            explicit_plots: HashSet::new(),
            settings: NotebookSettings::default(),
        }
    }
}

impl SessionData {
    pub fn load_from_disk() -> Option<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if std::env::var("URAE_DISABLE_SESSION_FILE").is_ok()
                || std::env::var("CARGO_MANIFEST_DIR").is_ok()
            {
                return None;
            }
            if let Ok(content) = std::fs::read_to_string(SESSION_FILE_NAME) {
                if let Ok(data) = serde_json::from_str::<Self>(&content) {
                    return Some(data);
                }
            }
        }
        None
    }

    pub fn save_to_disk(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if std::env::var("URAE_DISABLE_SESSION_FILE").is_ok()
                || std::env::var("CARGO_MANIFEST_DIR").is_ok()
            {
                return;
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(SESSION_FILE_NAME, json);
            }
        }
    }

    /// Export the notebook session as clean GitHub-Flavored Markdown.
    pub fn export_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# URAE Mathematics Notepad\n\n");
        md.push_str("```urae\n");
        md.push_str(&self.raw_document_text);
        md.push_str("\n```\n");
        md
    }
}
