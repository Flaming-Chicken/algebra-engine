//! `egui` Multiplatform UI Application for URAE Continuous Mathematics Notepad.

use crate::notebook::{
    CardDisplayMode, LineKind, MatrixPresetKind, NotebookState, ParameterBuilderParams,
    ReactiveComputeMode, SymbolMetadata, SymbolRole, generate_branch_cut_syntax,
    generate_matrix_syntax, generate_ode_bc_syntax, generate_physical_unit_syntax,
    generate_universal_parameter_builder_syntax,
};
use base64::Engine;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::HashSet;
use urae::calculus::SymbolicCalculus;
use urae::format::Formatter;
use urae::geometry::CoordinateSystem;

pub fn format_latex_as_pretty_unicode(latex_str: &str) -> String {
    let mut s = latex_str.trim().to_string();

    if s.starts_with("$$") && s.ends_with("$$") {
        s = s[2..s.len() - 2].trim().to_string();
    }

    s = s
        .replace("\\operatorname", "")
        .replace("\\left(", "(")
        .replace("\\right)", ")")
        .replace("\\left\\{", "{")
        .replace("\\right\\}", "}")
        .replace("\\mid", "|")
        .replace("\\cdot", "·")
        .replace("\\times", "×")
        .replace("\\in", "∈")
        .replace("\\mathbb{R}", "ℝ")
        .replace("\\mathbb{C}", "ℂ")
        .replace("\\mathbb{Z}", "ℤ")
        .replace("\\mathbb{H}", "ℍ")
        .replace("\\Lambda(V)", "Λ(V)")
        .replace("\\leq", "≤")
        .replace("\\geq", "≥")
        .replace("\\neq", "≠")
        .replace("\\sqrt", "√")
        .replace("\\int", "∫")
        .replace("\\sum", "∑")
        .replace("\\prod", "∏");

    s = s
        .replace("{x}^{2}", "x²")
        .replace("{x}^{3}", "x³")
        .replace("^{2}", "²")
        .replace("^{3}", "³")
        .replace("^{n}", "ⁿ");

    s
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    SmartStream,
    FocusEditor,
    DependencyGraph,
    PresentationReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowDockPosition {
    #[default]
    Floating,
    DockBottom,
    DockRight,
    DockLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlotScaleMode {
    #[default]
    Linear,
    SemiLogY,
    SemiLogX,
    LogLog,
}

/// Main App State struct implementing `eframe::App`.
pub struct UraeNotebookApp {
    pub state: NotebookState,
    pub view_mode: ViewMode,
    pub active_coord_system: CoordinateSystem,
    pub active_math_context: urae::MathContext,
    pub copilot_prompt: String,
    pub copilot_response: Option<String>,
    pub export_notice: Option<String>,
    pub show_cli_terminal: bool,
    pub show_left_sidebar: bool,
    pub show_editor: bool,
    pub show_right_sidebar: bool,
    pub left_panel_floating: bool,
    pub editor_floating: bool,
    pub right_panel_floating: bool,
    pub show_help_modal: bool,
    pub cli_input: String,
    pub cli_history: Vec<(String, String)>,
    pub terminal_dock: WindowDockPosition,
    pub settings_dock: WindowDockPosition,
    pub native_viewport_graphs: HashSet<String>,
    pub show_role_details: HashSet<String>,
    pub fonts_initialized: bool,
    pub fullscreen: bool,
    pub available_notebooks: Vec<String>,
    pub active_notebook: String,
    pub plot_scale_modes: std::collections::HashMap<usize, PlotScaleMode>,
    pub builder_snapshot: Option<String>,
    pub builder_active_line: Option<usize>,
    pub new_notebook_name: String,
    pub show_save_modal: bool,
    pub selected_save_format: String,

    // Storage Hardening & Diagnostics
    pub show_storage_modal: bool,
    pub dismissed_ephemeral_warning: bool,
    pub dismissed_quota_warning: bool,
    pub dismissed_combined_warning: bool,
    pub quota_exceeded: bool,
    pub storage_backend: String,
    pub is_persisted: Option<bool>,
    pub pwa_install_available: bool,
    pub is_pwa_installed: bool,
    pub last_diag_poll_time: f64,
    pub b64_import_input: String,
    pub b64_import_feedback: Option<String>,

    // Interactive Builder Palettes State
    pub show_matrix_builder: bool,
    pub show_domain_picker: bool,
    pub show_branch_cut_config: bool,
    pub show_ode_wizard: bool,
    pub show_units_palette: bool,

    pub matrix_builder_rows: usize,
    pub matrix_builder_cols: usize,
    pub matrix_builder_preset: MatrixPresetKind,
    pub matrix_builder_elements: Vec<Vec<String>>,

    pub domain_picker_var: String,
    pub domain_picker_domain: String,
    pub domain_picker_min: f64,
    pub domain_picker_max: f64,
    pub domain_picker_inc_min: bool,
    pub domain_picker_inc_max: bool,

    // Parameter & Variable Builder State
    pub param_builder_var: String,
    pub param_builder_is_param: bool,
    pub param_builder_category: usize, // 0 = Standard, 1 = Cayley-Dickson, 2 = Adjoined, 3 = Non-Archimedean, 4 = Modular/Galois, 5 = Matrix/Tensor
    pub param_builder_standard_kind: usize, // 0 = Reals, 1 = Positive, 2 = NonNegative, 3 = Integers, 4 = Naturals, 5 = Rationals, 6 = Complex
    pub param_builder_mode: usize,          // 0 = Adjoin, 1 = Cayley-Dickson
    pub param_builder_adjoin_i: bool,
    pub param_builder_adjoin_eps: bool,
    pub param_builder_adjoin_j: bool,
    pub param_builder_adjoin_clifford: bool,
    pub param_builder_cayley_depth: usize,
    pub param_builder_padic_prime: u32,
    pub param_builder_padic_valuation: i32,
    pub param_builder_surreal_generation: u32,
    pub param_builder_modulo_n: u64,
    pub param_builder_galois_prime: u64,
    pub param_builder_galois_power: u32,
    pub param_builder_matrix_rows: usize,
    pub param_builder_matrix_cols: usize,
    pub param_builder_matrix_domain: String,
    pub param_builder_tensor_rank: u32,
    pub param_builder_discrete_kind: usize, // 0 = Modulo, 1 = Integers/Step, 2 = GaloisField, 3 = Gaussian/Eisenstein, 4 = Boolean/BitVector
    pub param_builder_modulo_rep: usize, // 0 = Canonical [0, n-1], 1 = Balanced [-n/2, n/2], 2 = Units (Z/nZ)*
    pub param_builder_congruence_rem: i64,
    pub param_builder_congruence_mod: u64,
    pub param_builder_has_congruence: bool,
    pub param_builder_integer_step: u64,
    pub param_builder_integer_parity: usize, // 0 = Any, 1 = Even (2Z), 2 = Odd (2Z+1), 3 = Multiple of k
    pub param_builder_integer_multiple: u64,
    pub param_builder_lattice_kind: usize, // 0 = Gaussian Z[i], 1 = Eisenstein Z[omega]
    pub param_builder_bit_width: u32,
    pub param_builder_bit_signed: bool,
    pub param_builder_coords: Vec<(String, f64, f64, bool)>,

    pub branch_cut_fn: String,
    pub branch_cut_sheet: i32,
    pub branch_cut_pos: String,

    pub ode_var: String,
    pub ode_indep: String,
    pub ode_y0: f64,
    pub ode_dy0: Option<f64>,
    pub ode_has_deriv: bool,

    pub units_palette_var: String,
    pub units_palette_val: f64,
    pub units_palette_unit: String,
    pub units_system_index: usize, // 0 = SI, 1 = Imperial, 2 = CGS, 3 = Natural
    pub dim_length: i32,
    pub dim_mass: i32,
    pub dim_time: i32,
    pub dim_current: i32,
    pub dim_temp: i32,
    pub dim_substance: i32,
    pub dim_luminosity: i32,

    pub ode_pde_mode: usize, // 0 = ODE, 1 = PDE
    pub pde_model_type: String,
    pub pde_spatial_dim: usize,
    pub pde_bc_type: String,
    pub pde_has_time: bool,

    pub tensor_builder_rank: usize,
    pub tensor_builder_dims: Vec<usize>,
    pub tensor_builder_preset: String,
    pub param_builder_is_constant: bool,

    // Phase 17 UX Enhancements: 3D Viewport & Command Palette
    pub show_viewport_3d: bool,
    pub viewport_3d_state: crate::ui::Viewport3DState,
    pub command_palette_state: crate::ui::CommandPaletteState,
    pub show_cad_machinery_palette: bool,

    // In-App Example Notebook Gallery & Interactive Tutorials
    pub show_example_gallery: bool,
    pub selected_example_category: Option<crate::examples::ExampleCategory>,
    pub example_search_query: String,
    pub selected_example_id: String,

    // Modernized UI Theming & Workspace Subsystems
    pub theme: crate::ui::ThemeKind,
    pub workspace: crate::ui::WorkspaceState,
    pub inspected_symbol: Option<String>,
    pub active_open_inspectors: Vec<String>,
    pub hovered_line_idx: Option<usize>,
    pub right_sidebar_width: f32,
    pub collapsed_plot_lines: HashSet<usize>,
    pub line_screen_positions: std::collections::HashMap<usize, egui::Pos2>,
    pub inspector_to_focus: Option<String>,
    pub active_scrubbing: Option<ActiveScrubbing>,

    // Performance Debouncing and Fast Auto-Save (<500ms)
    pub last_keystroke_time: web_time::Instant,
    pub is_edit_dirty: bool,
    pub last_autosave_time: web_time::Instant,
    pub is_save_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct ActiveScrubbing {
    pub byte_range: (usize, usize),
    pub original_val: f64,
    pub has_decimal: bool,
    pub decimal_places: usize,
    pub start_pointer_x: f32,
}

impl Default for UraeNotebookApp {
    fn default() -> Self {
        let state = NotebookState::default();
        let theme = state.session.settings.theme;
        let mut workspace = crate::ui::WorkspaceState::default();
        workspace.set_layout(state.session.settings.workspace_preset);

        Self {
            state,
            view_mode: ViewMode::SmartStream,
            active_coord_system: CoordinateSystem::Cartesian,
            active_math_context: urae::MathContext::default(),
            copilot_prompt: String::new(),
            copilot_response: None,
            export_notice: None,
            show_cli_terminal: false,
            show_left_sidebar: true,
            show_editor: true,
            show_right_sidebar: true,
            left_panel_floating: false,
            editor_floating: false,
            right_panel_floating: false,
            show_help_modal: false,
            cli_input: String::new(),
            cli_history: Vec::new(),
            terminal_dock: WindowDockPosition::DockBottom,
            settings_dock: WindowDockPosition::Floating,
            native_viewport_graphs: HashSet::new(),
            show_role_details: HashSet::new(),
            fonts_initialized: false,
            fullscreen: false,
            available_notebooks: Vec::new(),
            active_notebook: "urae_notebook_session.json".to_string(),
            plot_scale_modes: std::collections::HashMap::new(),
            builder_snapshot: None,
            builder_active_line: None,
            new_notebook_name: String::new(),
            show_save_modal: false,
            selected_save_format: ".json".to_string(),
            inspected_symbol: None,
            active_open_inspectors: Vec::new(),
            hovered_line_idx: None,
            right_sidebar_width: 280.0,
            collapsed_plot_lines: HashSet::new(),
            line_screen_positions: std::collections::HashMap::new(),
            inspector_to_focus: None,
            active_scrubbing: None,

            // Storage defaults
            show_storage_modal: false,
            dismissed_ephemeral_warning: false,
            dismissed_quota_warning: false,
            dismissed_combined_warning: false,
            quota_exceeded: false,
            storage_backend: "Local Storage (Tier 1)".to_string(),
            is_persisted: None,
            pwa_install_available: false,
            is_pwa_installed: false,
            last_diag_poll_time: 0.0,
            b64_import_input: String::new(),
            b64_import_feedback: None,

            // Builder defaults
            show_matrix_builder: false,
            show_domain_picker: false,
            show_branch_cut_config: false,
            show_ode_wizard: false,
            show_units_palette: false,

            matrix_builder_rows: 2,
            matrix_builder_cols: 2,
            matrix_builder_preset: MatrixPresetKind::Custom,
            matrix_builder_elements: vec![
                vec!["1".to_string(), "0".to_string()],
                vec!["0".to_string(), "1".to_string()],
            ],

            domain_picker_var: "x".to_string(),
            domain_picker_domain: "Reals".to_string(),
            domain_picker_min: -5.0,
            domain_picker_max: 5.0,
            domain_picker_inc_min: true,
            domain_picker_inc_max: true,

            param_builder_var: "z".to_string(),
            param_builder_is_param: false,
            param_builder_category: 0,
            param_builder_standard_kind: 0,
            param_builder_mode: 0,
            param_builder_adjoin_i: true,
            param_builder_adjoin_eps: false,
            param_builder_adjoin_j: false,
            param_builder_adjoin_clifford: false,
            param_builder_cayley_depth: 1,
            param_builder_padic_prime: 7,
            param_builder_padic_valuation: 0,
            param_builder_surreal_generation: 4,
            param_builder_modulo_n: 12,
            param_builder_galois_prime: 2,
            param_builder_galois_power: 8,
            param_builder_matrix_rows: 3,
            param_builder_matrix_cols: 3,
            param_builder_matrix_domain: "Reals".to_string(),
            param_builder_tensor_rank: 2,
            param_builder_discrete_kind: 0,
            param_builder_modulo_rep: 0,
            param_builder_congruence_rem: 0,
            param_builder_congruence_mod: 1,
            param_builder_has_congruence: false,
            param_builder_integer_step: 1,
            param_builder_integer_parity: 0,
            param_builder_integer_multiple: 2,
            param_builder_lattice_kind: 0,
            param_builder_bit_width: 8,
            param_builder_bit_signed: false,
            param_builder_coords: vec![
                ("Re".to_string(), -5.0, 5.0, true),
                ("Im".to_string(), -5.0, 5.0, true),
                ("Dual".to_string(), -1.0, 1.0, false),
                ("j".to_string(), -1.0, 1.0, false),
            ],

            branch_cut_fn: "log".to_string(),
            branch_cut_sheet: 0,
            branch_cut_pos: "(-infinity, 0]".to_string(),

            ode_var: "y".to_string(),
            ode_indep: "x".to_string(),
            ode_y0: 1.0,
            ode_dy0: Some(0.0),
            ode_has_deriv: true,

            units_palette_var: "v".to_string(),
            units_palette_val: 10.0,
            units_palette_unit: "m/s".to_string(),
            units_system_index: 0,
            dim_length: 1,
            dim_mass: 0,
            dim_time: -1,
            dim_current: 0,
            dim_temp: 0,
            dim_substance: 0,
            dim_luminosity: 0,

            ode_pde_mode: 0,
            pde_model_type: "Wave Equation".to_string(),
            pde_spatial_dim: 1,
            pde_bc_type: "Dirichlet (u=0)".to_string(),
            pde_has_time: true,

            tensor_builder_rank: 2,
            tensor_builder_dims: vec![2, 2],
            tensor_builder_preset: "Identity".to_string(),
            param_builder_is_constant: false,

            show_viewport_3d: false,
            viewport_3d_state: crate::ui::Viewport3DState::default(),
            command_palette_state: crate::ui::CommandPaletteState::default(),
            show_cad_machinery_palette: false,

            show_example_gallery: false,
            selected_example_category: None,
            example_search_query: String::new(),
            selected_example_id: "01_calculus_and_differential_equations".to_string(),

            theme,
            workspace,

            last_keystroke_time: web_time::Instant::now(),
            is_edit_dirty: false,
            last_autosave_time: web_time::Instant::now(),
            is_save_dirty: false,
        }
    }
}

impl UraeNotebookApp {
    /// Open a dedicated floating inspector tooltip for `symbol`, or bring it to the front if already open (1 per object).
    pub fn open_or_focus_inspector(&mut self, symbol: &str) {
        let clean = symbol.trim().to_string();
        if clean.is_empty() {
            return;
        }
        if let Some(pos) = self.active_open_inspectors.iter().position(|s| s == &clean) {
            let sym = self.active_open_inspectors.remove(pos);
            self.active_open_inspectors.push(sym);
        } else {
            self.active_open_inspectors.push(clean.clone());
        }
        self.inspector_to_focus = Some(clean);
    }

    /// Create new `UraeNotebookApp` instance initializing fonts and available notebooks before the first frame.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        crate::log_debug("UraeNotebookApp::new() started");

        let mut app = Self::default();

        #[cfg(not(target_arch = "wasm32"))]
        crate::log_debug("UraeNotebookApp::default() created, calling setup_fonts()...");

        app.setup_fonts(&cc.egui_ctx);

        #[cfg(not(target_arch = "wasm32"))]
        crate::log_debug("setup_fonts() completed, calling scan_available_notebooks()...");

        app.scan_available_notebooks();

        #[cfg(not(target_arch = "wasm32"))]
        crate::log_debug(&format!(
            "scan_available_notebooks() found {} notebooks: {:?}",
            app.available_notebooks.len(),
            app.available_notebooks
        ));

        app.fonts_initialized = true;
        app
    }

    /// Embed Noto Sans Math TTF font into egui for rendering full Unicode mathematical symbols.
    pub fn setup_fonts(&self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "NotoSansMath".to_string(),
            egui::FontData::from_static(include_bytes!("../assets/NotoSansMath-Regular.ttf"))
                .into(),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("NotoSansMath".to_string());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("NotoSansMath".to_string());

        ctx.set_fonts(fonts);
    }

    /// Scan active directory (or local storage on web) for saved mathematical notebook files.
    pub fn scan_available_notebooks(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(".") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "json" || ext == "math" || ext == "txt" || ext == "bson" {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if name != "package.json"
                                    && name != "Cargo.json"
                                    && !name.starts_with('.')
                                {
                                    files.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
            files.sort();
            self.available_notebooks = files;
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    let mut keys = Vec::new();
                    if let Ok(len) = local_storage.length() {
                        for i in 0..len {
                            if let Ok(Some(key)) = local_storage.key(i) {
                                if key.ends_with(".json")
                                    || key.ends_with(".math")
                                    || key.ends_with(".txt")
                                    || key.ends_with(".bson")
                                {
                                    keys.push(key);
                                }
                            }
                        }
                    }
                    keys.sort();
                    self.available_notebooks = keys;
                }
            }
        }
    }

    /// Export active session as zlib-compressed BSON binary blob
    pub fn export_current_session_compressed_bson(&self) -> Result<Vec<u8>, String> {
        crate::notebook::export_session_to_compressed_bson(&self.state.session)
    }

    /// Save active mathematical notebook in optimal BSON, JSON or human-readable Plain Text format.
    pub fn save_notebook(&mut self, filename: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if filename.ends_with(".bson") {
                if let Ok(bson_data) = self.export_current_session_compressed_bson() {
                    let _ = std::fs::write(filename, bson_data);
                }
            } else if filename.ends_with(".json") {
                if let Ok(json) = serde_json::to_string_pretty(&self.state.session) {
                    let _ = std::fs::write(filename, json);
                }
            } else {
                let _ = std::fs::write(filename, &self.state.session.raw_document_text);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut save_success = false;
            let (payload, _is_bson) = if filename.ends_with(".bson") {
                if let Ok(bson_data) = self.export_current_session_compressed_bson() {
                    use base64::Engine;
                    (
                        base64::engine::general_purpose::STANDARD.encode(&bson_data),
                        true,
                    )
                } else {
                    (String::new(), true)
                }
            } else if filename.ends_with(".json") {
                (
                    serde_json::to_string(&self.state.session).unwrap_or_default(),
                    false,
                )
            } else {
                (self.state.session.raw_document_text.clone(), false)
            };

            if !payload.is_empty() {
                // Tier 1: Try LocalStorage
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(local_storage)) = window.local_storage() {
                        if local_storage.set_item(filename, &payload).is_ok() {
                            save_success = true;
                            self.storage_backend = "Local Storage (Tier 1)".to_string();
                        }
                    }
                }

                // Tier 2: If LocalStorage fails (QuotaExceededError or disabled), fallback to IndexedDB
                if !save_success {
                    self.quota_exceeded = true;
                    self.storage_backend = "IndexedDB (Tier 2 Fallback)".to_string();
                    let js_code = format!(
                        "if (window.__saveNotebookToIndexedDB) {{ window.__saveNotebookToIndexedDB({:?}, {:?}); }}",
                        filename, payload
                    );
                    let _ = js_sys::eval(&js_code);
                }
            }
        }
    }

    /// Load active mathematical notebook in optimal BSON, JSON or human-readable Plain Text format.
    pub fn load_notebook(&mut self, filename: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if filename.ends_with(".bson") {
                if let Ok(bytes) = std::fs::read(filename) {
                    if let Ok(data) = crate::notebook::import_session_from_compressed_bson(&bytes) {
                        self.state.session = data;
                        self.state.evaluate_all();
                    }
                }
            } else if filename.ends_with(".json") {
                if let Ok(content) = std::fs::read_to_string(filename) {
                    if let Ok(data) = serde_json::from_str::<crate::notebook::SessionData>(&content)
                    {
                        self.state.session = data;
                        self.state.evaluate_all();
                    }
                }
            } else {
                if let Ok(content) = std::fs::read_to_string(filename) {
                    self.state.session.raw_document_text = content;
                    // Reset associated sliders to defaults
                    self.state.session.slider_values.clear();
                    self.state.session.symbol_metadata.clear();
                    self.state.evaluate_all();
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut content_opt = None;

            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    if let Ok(Some(content)) = local_storage.get_item(filename) {
                        content_opt = Some(content);
                    }
                }
            }

            if let Some(content) = content_opt {
                if filename.ends_with(".bson") {
                    use base64::Engine;
                    if let Ok(bytes) =
                        base64::engine::general_purpose::STANDARD.decode(content.trim())
                    {
                        if let Ok(data) =
                            crate::notebook::import_session_from_compressed_bson(&bytes)
                        {
                            self.state.session = data;
                            self.state.evaluate_all();
                        }
                    }
                } else if filename.ends_with(".json") {
                    if let Ok(data) = serde_json::from_str::<crate::notebook::SessionData>(&content)
                    {
                        self.state.session = data;
                        self.state.evaluate_all();
                    }
                } else {
                    self.state.session.raw_document_text = content;
                    self.state.session.slider_values.clear();
                    self.state.session.symbol_metadata.clear();
                    self.state.evaluate_all();
                }
            }
        }
    }

    /// Periodic diagnostics polling for storage persistence, quota, and PWA installation state.
    pub fn poll_storage_diagnostics(&mut self, _ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        {
            let now = _ctx.input(|i| i.time);
            if now - self.last_diag_poll_time > 4.0 || self.last_diag_poll_time == 0.0 {
                self.last_diag_poll_time = now;

                if let Some(window) = web_sys::window() {
                    // Check persistence status
                    if let Ok(persisted_val) = js_sys::Reflect::get(
                        &window,
                        &wasm_bindgen::JsValue::from_str("__isStoragePersisted"),
                    ) {
                        if !persisted_val.is_undefined() {
                            self.is_persisted = Some(persisted_val.is_truthy());
                        }
                    }

                    // Check PWA install prompt readiness
                    if let Ok(prompt_val) = js_sys::Reflect::get(
                        &window,
                        &wasm_bindgen::JsValue::from_str("__pwaInstallPrompt"),
                    ) {
                        self.pwa_install_available =
                            !prompt_val.is_null() && !prompt_val.is_undefined();
                    }

                    // Check if PWA already installed
                    if let Ok(installed_val) = js_sys::Reflect::get(
                        &window,
                        &wasm_bindgen::JsValue::from_str("__isPWAInstalled"),
                    ) {
                        self.is_pwa_installed = installed_val.is_truthy();
                    }
                }
            }
        }
    }

    /// Trigger persistent storage request prompt
    pub fn request_persistence(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval(
                "if (window.__requestPersistentStorage) { window.__requestPersistentStorage(); }",
            );
        }
    }

    /// Trigger PWA app installation prompt
    pub fn trigger_pwa_install(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ =
                js_sys::eval("if (window.__triggerPWAInstall) { window.__triggerPWAInstall(); }");
        }
    }

    /// Triggers a cross-platform file download (browser download in WASM, filesystem write on native desktop).
    pub fn trigger_file_download(&self, filename: &str, data: &[u8], _mime_type: &str) {
        spodeian_web_utils::trigger_binary_download(filename, data, _mime_type);
    }


    /// Download active session as a compressed .bson binary file
    pub fn download_bson_backup(&self) {
        if let Ok(bytes) = self.export_current_session_compressed_bson() {
            self.trigger_file_download("notebook_backup.bson", &bytes, "application/octet-stream");
        }
    }

    /// Render Ephemeral Storage, Quota Exceeded, and Combined Warning Banners
    pub fn render_warning_banners(&mut self, #[allow(unused_variables)] ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Desktop builds run on the local filesystem with full persistence; no browser quota/eviction banners needed.
        }

        #[cfg(target_arch = "wasm32")]
        {
            let is_ephemeral = self.is_persisted == Some(false);
            let is_quota = self.quota_exceeded;

            let show_combined = is_ephemeral && is_quota && !self.dismissed_combined_warning;
            let show_ephemeral =
                is_ephemeral && !show_combined && !self.dismissed_ephemeral_warning;
            let show_quota = is_quota && !show_combined && !self.dismissed_quota_warning;

            if show_combined || show_ephemeral || show_quota {
                let (msg, fill_color, stroke_color, text_color) = if show_combined {
                    (
                        "Storage Warning: Running in ephemeral storage and quota is full.",
                        egui::Color32::from_rgba_premultiplied(35, 20, 20, 245),
                        egui::Color32::from_rgb(240, 80, 80),
                        egui::Color32::from_rgb(255, 120, 120),
                    )
                } else if show_ephemeral {
                    (
                        "Ephemeral Storage: Browser may clear local notebooks under storage pressure.",
                        egui::Color32::from_rgba_premultiplied(35, 28, 15, 245),
                        egui::Color32::from_rgb(220, 160, 30),
                        egui::Color32::from_rgb(255, 200, 80),
                    )
                } else {
                    (
                        "Quota Exceeded: Local storage is full. Notebook migrated to IndexedDB fallback.",
                        egui::Color32::from_rgba_premultiplied(35, 28, 15, 245),
                        egui::Color32::from_rgb(220, 160, 30),
                        egui::Color32::from_rgb(255, 200, 80),
                    )
                };

                egui::Area::new(egui::Id::new("storage_warning_banner_area"))
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
                    .show(ctx, |ui| {
                        egui::Frame::NONE
                            .fill(fill_color)
                            .stroke(egui::Stroke::new(1.0_f32, stroke_color))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(16, 10))
                            .show(ui, |ui| {
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(msg).strong().color(text_color));
                                    ui.add_space(6.0);
                                    ui.with_layout(
                                        egui::Layout::left_to_right(egui::Align::Center)
                                            .with_main_align(egui::Align::Center),
                                        |ui| {
                                            if ui.button("Backup .bson").clicked() {
                                                self.download_bson_backup();
                                            }
                                            if ui.button("Request Persistence").clicked() {
                                                self.request_persistence();
                                            }
                                            if ui.button("Dismiss").clicked() {
                                                if show_combined {
                                                    self.dismissed_combined_warning = true;
                                                } else if show_ephemeral {
                                                    self.dismissed_ephemeral_warning = true;
                                                } else {
                                                    self.dismissed_quota_warning = true;
                                                }
                                            }
                                        },
                                    );
                                });
                            });
                    });
            }
        }
    }

    /// Render Storage Diagnostics & Backup Modal
    pub fn render_storage_modal(&mut self, ctx: &egui::Context) {
        if self.show_storage_modal {
            let mut is_open = self.show_storage_modal;
            let mut close_requested = false;

            egui::Window::new("Storage Diagnostics & Data Management")
                .open(&mut is_open)
                .default_size([580.0, 380.0])
                .resizable(true)
                .collapsible(true)
                .show(&ctx, |ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.subheading("🖥️ Desktop Local Filesystem & Storage Health");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• Environment:").strong());
                            ui.colored_label(egui::Color32::GREEN, "Native Desktop (Direct Filesystem Access - No browser storage quota/eviction)");
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• Working Directory:").strong());
                            let cwd = std::env::current_dir()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|_| ".".to_string());
                            ui.monospace(cwd);
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• Active Notebook:").strong());
                            ui.monospace(&self.active_notebook);
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• Autosave Engine:").strong());
                            ui.colored_label(egui::Color32::from_rgb(80, 220, 140), "⚡ Low-Latency Disk Persistence (<500ms debounce)");
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• Document Memory:").strong());
                            ui.weak(format!(
                                "{} bytes ({} lines, {} declared symbols)",
                                self.state.session.raw_document_text.len(),
                                self.state.parsed_lines.len(),
                                self.state.session.symbol_metadata.len()
                            ));
                        });

                        ui.add_space(8.0);
                        ui.subheading("Backup & Export Actions");
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Export Compressed .bson Backup").clicked() {
                                self.download_bson_backup();
                            }
                        });
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        ui.subheading("🌐 Web Browser Storage Health & Durability");
                        ui.separator();

                        let status_color = match self.is_persisted {
                            Some(true) => egui::Color32::GREEN,
                            Some(false) => egui::Color32::from_rgb(240, 160, 40),
                            None => egui::Color32::GRAY,
                        };
                        let status_str = match self.is_persisted {
                            Some(true) => "Persistent (Immune to browser storage eviction)",
                            Some(false) => "Ephemeral (May be cleared under storage pressure)",
                            None => "Unknown / Querying browser...",
                        };

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• Durability Status:").strong());
                            ui.colored_label(status_color, status_str);
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• Active Storage Tier:").strong());
                            ui.monospace(&self.storage_backend);
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("• PWA App State:").strong());
                            if self.is_pwa_installed {
                                ui.colored_label(
                                    egui::Color32::GREEN,
                                    "Installed (Permanent Standalone Application)",
                                );
                            } else if self.pwa_install_available {
                                ui.colored_label(
                                    egui::Color32::from_rgb(100, 180, 255),
                                    "Ready to Install as App",
                                );
                            } else {
                                ui.weak("Running in Web Tab");
                            }
                        });

                        ui.add_space(8.0);
                        ui.subheading("Storage Actions");
                        ui.separator();

                        ui.horizontal_wrapped(|ui| {
                            if self.is_persisted != Some(true)
                                && ui.button("Request Persistent Storage").clicked()
                            {
                                self.request_persistence();
                            }

                            if self.pwa_install_available
                                && !self.is_pwa_installed
                                && ui.button("Install Web App").clicked()
                            {
                                self.trigger_pwa_install();
                            }

                            if ui.button("Export Compressed .bson Backup").clicked() {
                                self.download_bson_backup();
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.subheading("Import Base64 / BSON State");
                    ui.separator();

                    ui.label(
                        "Paste Base64 encoded compressed BSON or JSON workspace string below:",
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut self.b64_import_input)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );

                    ui.horizontal(|ui| {
                        if ui.button("Import & Apply State").clicked() {
                            let input_trim = self.b64_import_input.trim();
                            if input_trim.is_empty() {
                                self.b64_import_feedback =
                                    Some("Please paste Base64 data first.".to_string());
                            } else {
                                use base64::Engine;
                                if let Ok(bytes) =
                                    base64::engine::general_purpose::STANDARD.decode(input_trim)
                                {
                                    if let Ok(data) =
                                        crate::notebook::import_session_from_compressed_bson(&bytes)
                                    {
                                        self.state.session = data;
                                        self.state.evaluate_all();
                                        self.b64_import_feedback = Some(
                                            "Successfully imported session from compressed BSON!"
                                                .to_string(),
                                        );
                                    } else {
                                        self.b64_import_feedback = Some(
                                            "Error: Failed to deserialize BSON payload."
                                                .to_string(),
                                        );
                                    }
                                } else if let Ok(data) =
                                    serde_json::from_str::<crate::notebook::SessionData>(input_trim)
                                {
                                    self.state.session = data;
                                    self.state.evaluate_all();
                                    self.b64_import_feedback = Some(
                                        "Successfully imported session from JSON!".to_string(),
                                    );
                                } else {
                                    self.b64_import_feedback =
                                        Some("Error: Invalid Base64 or JSON payload.".to_string());
                                }
                            }
                        }

                        if let Some(msg) = &self.b64_import_feedback {
                            if !msg.starts_with("Error") {
                                ui.colored_label(egui::Color32::GREEN, msg);
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), msg);
                            }
                        }
                    });

                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });

            if !is_open || close_requested {
                self.show_storage_modal = false;
            }
        }
    }
}

/// Configuration for consistently formatted column/panel headers across docked and floating windows.
pub struct ColumnHeaderConfig<'a> {
    pub title_full: &'a str,
    pub title_standard: &'a str,
    pub title_short: &'a str,
    pub title_tooltip: &'a str,
    pub float_icon: &'a str,
    pub float_full: &'a str,
    pub float_tooltip: &'a str,
    pub hide_icon: &'a str,
    pub hide_full: &'a str,
    pub hide_tooltip: &'a str,
}

/// Render a consistently formatted column/panel header with dynamic symbol-only button fallback
/// when columns are shrunk to prevent squishing or overlapping with the title.
pub fn render_column_header(
    ui: &mut egui::Ui,
    palette: &crate::ui::ThemePalette,
    config: &ColumnHeaderConfig<'_>,
) -> (bool, bool) {
    let avail_w = ui.available_width();
    // When columns are shrunk such that buttons would squish or overlap the header text,
    // buttons drop their textual label and become just symbols.
    let use_symbol_buttons = avail_w < 320.0;
    let float_label = if use_symbol_buttons {
        config.float_icon
    } else {
        config.float_full
    };
    let hide_label = if use_symbol_buttons {
        config.hide_icon
    } else {
        config.hide_full
    };

    let title_text = if avail_w >= 380.0 {
        config.title_full
    } else if avail_w >= 200.0 || (use_symbol_buttons && avail_w >= 160.0) {
        config.title_standard
    } else {
        config.title_short
    };

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let hide_clicked = ui
                .button(
                    egui::RichText::new(hide_label)
                        .size(12.0)
                        .color(palette.text_muted),
                )
                .on_hover_text(config.hide_tooltip)
                .clicked();

            let float_clicked = ui
                .button(
                    egui::RichText::new(float_label)
                        .size(12.0)
                        .color(palette.text_muted),
                )
                .on_hover_text(config.float_tooltip)
                .clicked();

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(title_text)
                            .strong()
                            .size(13.0)
                            .color(palette.text_primary),
                    )
                    .truncate(),
                )
                .on_hover_text(config.title_tooltip);
            });

            (float_clicked, hide_clicked)
        })
        .inner
    })
    .inner
}

impl eframe::App for UraeNotebookApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        static FRAME_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let frame_num = FRAME_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        #[cfg(not(target_arch = "wasm32"))]
        {
            if frame_num < 5 || frame_num.is_multiple_of(120) {
                let rect = ctx.input(|i| i.raw.screen_rect);
                crate::log_debug(&format!(
                    "[FRAME #{frame_num}] Screen Rect: {:.1}x{:.1}, Zoom: {:.2}, PixelsPerPoint: {:.2}",
                    rect.unwrap_or(egui::Rect::EVERYTHING).width(),
                    rect.unwrap_or(egui::Rect::EVERYTHING).height(),
                    ctx.zoom_factor(),
                    ctx.pixels_per_point()
                ));
            }
        }

        if frame_num == 0 {
            #[cfg(not(target_arch = "wasm32"))]
            crate::log_info("First frame rendering: displaying window and requesting UI paint...");
            ctx.set_style_of(ctx.theme(), self.theme.to_style());
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.request_repaint();
        }

        if !self.fonts_initialized {
            self.setup_fonts(&ctx);
            self.scan_available_notebooks();
            self.fonts_initialized = true;
        }

        self.poll_storage_diagnostics(&ctx);

        let mut state_changed = false;

        // Key bindings: F5 to re-evaluate, F11 to toggle fullscreen
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            state_changed = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        }
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::S)) {
            self.show_save_modal = true;
            if self.new_notebook_name.is_empty() {
                self.new_notebook_name = self.active_notebook.clone();
            }
        }
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::K)) {
            self.command_palette_state.is_open = !self.command_palette_state.is_open;
        }

        // Export shortcuts: Ctrl+Shift+C (Copy with results) & Ctrl+Shift+E (Export Markdown)
        if ctx.input(|i| {
            (i.modifiers.command || i.modifiers.ctrl)
                && i.modifiers.shift
                && i.key_pressed(egui::Key::C)
        }) {
            let mut export_text = String::new();
            for pl in &self.state.parsed_lines {
                export_text.push_str(&pl.raw_text);
                if !pl.output_unicode.is_empty() && pl.raw_text.trim() != pl.output_unicode.trim() {
                    export_text.push_str(&format!("  // => {}", pl.output_unicode));
                }
                export_text.push('\n');
            }
            ctx.copy_text(export_text);
            self.export_notice =
                Some("Copied notepad with live evaluated results to clipboard!".to_string());
        }

        if ctx.input(|i| {
            (i.modifiers.command || i.modifiers.ctrl)
                && i.modifiers.shift
                && i.key_pressed(egui::Key::E)
        }) {
            let md = self.state.session.export_markdown();
            ctx.copy_text(md);
            self.export_notice = Some("Exported clean Markdown document to clipboard!".to_string());
        }

        // Keystroke evaluation debounce (200ms)
        if self.is_edit_dirty && self.last_keystroke_time.elapsed().as_millis() >= 200 {
            state_changed = true;
            self.is_edit_dirty = false;
            self.is_save_dirty = true;
            self.last_autosave_time = web_time::Instant::now();
        }

        // Debounced Autosave (<500ms -> 400ms)
        if self.is_save_dirty && self.last_autosave_time.elapsed().as_millis() >= 400 {
            self.state.session.save_to_disk();
            self.is_save_dirty = false;
        }

        // Keyboard zoom / scale (Command/Ctrl + / - / 0)
        let mut zoom = ctx.zoom_factor();
        if ctx.input(|i| {
            (i.modifiers.command || i.modifiers.ctrl)
                && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals))
        }) {
            zoom += 0.1;
        }
        if ctx
            .input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Minus))
        {
            zoom -= 0.1;
        }
        if ctx
            .input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Num0))
        {
            zoom = 1.0;
        }
        ctx.set_zoom_factor(zoom.clamp(0.5, 3.0));

        // Global Undo / Redo Keybindings
        if ctx.input(|i| {
            (i.modifiers.command || i.modifiers.ctrl)
                && !i.modifiers.shift
                && i.key_pressed(egui::Key::Z)
        }) && self.state.undo()
        {
            state_changed = true;
        }
        if ctx.input(|i| {
            (i.modifiers.command || i.modifiers.ctrl)
                && (i.key_pressed(egui::Key::Y)
                    || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))
        }) && self.state.redo()
        {
            state_changed = true;
        }

        // Top Control Bar with Organized Dropdown Menus
        egui::Panel::top("top_header_panel").show(ui, |ui| {
            let palette = self.theme.palette();

            ui.horizontal_wrapped(|ui| {
                ui.heading("URAE");
                ui.colored_label(palette.accent_secondary, concat!("v", env!("CARGO_PKG_VERSION")));
                ui.separator();

                // 1. File Menu Dropdown
                ui.menu_button("📁 File", |ui| {
                    ui.menu_button(format!("Notebook: {}", self.active_notebook), |ui| {
                        let available = self.available_notebooks.clone();
                        for nb in &available {
                            if ui.selectable_label(self.active_notebook == *nb, nb).clicked() {
                                self.active_notebook = nb.clone();
                                self.load_notebook(nb);
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.weak("New:");
                        ui.text_edit_singleline(&mut self.new_notebook_name);
                        if ui.button("Create").clicked() && !self.new_notebook_name.trim().is_empty() {
                            let name = self.new_notebook_name.trim().to_string();
                            let full_name = if name.ends_with(".json") || name.ends_with(".math") || name.ends_with(".txt") || name.ends_with(".bson") {
                                name
                            } else {
                                format!("{}.math", name)
                            };
                            self.save_notebook(&full_name);
                            self.active_notebook = full_name;
                            self.scan_available_notebooks();
                            self.new_notebook_name.clear();
                            ui.close();
                        }
                    });
                    ui.separator();
                    if ui.button("💾 Save (Ctrl+S)").clicked() {
                        let active_nb = self.active_notebook.clone();
                        self.save_notebook(&active_nb);
                        ui.close();
                    }
                    if ui.button("Save As...").clicked() {
                        self.show_save_modal = true;
                        if self.new_notebook_name.is_empty() {
                            self.new_notebook_name = self.active_notebook.clone();
                        }
                        ui.close();
                    }
                    ui.menu_button("📤 Export", |ui| {
                        if ui.button("LaTeX (.tex)").clicked() {
                            let tex = format!("\\documentclass{{article}}\n\\begin{{document}}\n{}\n\\end{{document}}", self.state.session.raw_document_text);
                            self.trigger_file_download("notebook_export.tex", tex.as_bytes(), "text/x-tex");
                            self.export_notice = Some("Exported to 'notebook_export.tex'".to_string());
                            ui.close();
                        }
                        if ui.button("Python SymPy (.py)").clicked() {
                            let py = format!("# URAE Exported Python Script\nimport sympy as sp\nx = sp.Symbol('x')\n# Document Text:\n\"\"\"{}\"\"\"", self.state.session.raw_document_text);
                            self.trigger_file_download("notebook_export.py", py.as_bytes(), "text/x-python");
                            self.export_notice = Some("Exported to 'notebook_export.py'".to_string());
                            ui.close();
                        }
                        if ui.button("Lean 4 Proof (.lean)").clicked() {
                            let lean = "-- URAE Exported Lean 4 Formal Script\nimport Mathlib\n";
                            self.trigger_file_download("notebook_export.lean", lean.as_bytes(), "text/plain");
                            self.export_notice = Some("Exported to 'notebook_export.lean'".to_string());
                            ui.close();
                        }
                        if ui.button("Compressed BSON (.bson)").clicked() {
                            self.download_bson_backup();
                            self.export_notice = Some("Exported compressed BSON backup".to_string());
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("🌐 WASM Embed IFrame Snippet").on_hover_text("Generate embeddable iframe tag for blogs, textbooks & documentation").clicked() {
                            let doc_bytes = self.state.session.raw_document_text.as_bytes();
                            let encoded = base64::engine::general_purpose::STANDARD.encode(doc_bytes);
                            let iframe_html = format!(
                                r#"<iframe src="https://urae.app/embed?data={}" width="800" height="600" frameborder="0" style="border: 1px solid #334155; border-radius: 8px;" allow="clipboard-write"></iframe>"#,
                                encoded
                            );
                            ctx.copy_text(iframe_html);
                            self.export_notice = Some("Copied WASM embed iframe snippet to clipboard!".to_string());
                            ui.close();
                        }
                    });
                    ui.separator();
                    let storage_label = match self.is_persisted {
                        Some(true) => "Storage (Persistent)",
                        Some(false) => "Storage (Ephemeral)",
                        None => "Storage Diagnostics",
                    };
                    if ui.button(storage_label).clicked() {
                        self.show_storage_modal = true;
                        ui.close();
                    }
                });

                // 2. Edit Menu Dropdown
                ui.menu_button("✏ Edit", |ui| {
                    let can_undo = self.state.can_undo();
                    let undo_lbl = match self.state.history.undo_description() {
                        Some(desc) => format!("⮌ Undo: {} (Ctrl+Z)", desc),
                        None => "⮌ Undo (Ctrl+Z)".to_string(),
                    };
                    if ui.add_enabled(can_undo, egui::Button::new(undo_lbl)).clicked() {
                        self.state.undo();
                        state_changed = true;
                        ui.close();
                    }

                    let can_redo = self.state.can_redo();
                    let redo_lbl = match self.state.history.redo_description() {
                        Some(desc) => format!("⮎ Redo: {} (Ctrl+Y)", desc),
                        None => "⮎ Redo (Ctrl+Y)".to_string(),
                    };
                    if ui.add_enabled(can_redo, egui::Button::new(redo_lbl)).clicked() {
                        self.state.redo();
                        state_changed = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("⚡ Re-evaluate Document").clicked() {
                        state_changed = true;
                        ui.close();
                    }
                });

                // 3. View Menu Dropdown (Panels Visibility & Floating Windows)
                ui.menu_button("👁 View", |ui| {
                    ui.label(egui::RichText::new("Panels Visibility & Windows").strong());
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_left_sidebar, "◀ Left Parameters (Ctrl+B)");
                        if self.show_left_sidebar {
                            let float_lbl = if self.left_panel_floating { "⤓ Dock" } else { "⤢ Float" };
                            if ui.small_button(float_lbl).clicked() {
                                self.left_panel_floating = !self.left_panel_floating;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.show_editor, "📝 Text Editor (Ctrl+E)").changed() {
                            self.state.session.settings.show_editor = self.show_editor;
                        }
                        if self.show_editor {
                            let float_lbl = if self.editor_floating { "⤓ Dock" } else { "⤢ Float" };
                            if ui.small_button(float_lbl).clicked() {
                                self.editor_floating = !self.editor_floating;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_right_sidebar, "Right Results ▶ (Ctrl+J)");
                        if self.show_right_sidebar {
                            let float_lbl = if self.right_panel_floating { "⤓ Dock" } else { "⤢ Float" };
                            if ui.small_button(float_lbl).clicked() {
                                self.right_panel_floating = !self.right_panel_floating;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_cli_terminal, "💻 Terminal CLI (Ctrl+`)");
                        if self.show_cli_terminal {
                            let is_floating = self.terminal_dock == WindowDockPosition::Floating;
                            let float_lbl = if is_floating { "⤓ Dock" } else { "⤢ Float" };
                            if ui.small_button(float_lbl).clicked() {
                                self.terminal_dock = if is_floating {
                                    WindowDockPosition::DockBottom
                                } else {
                                    WindowDockPosition::Floating
                                };
                            }
                        }
                    });
                    ui.separator();
                    if ui.checkbox(&mut self.state.session.settings.show_cell_line_numbers, "Show Line Numbers").clicked() {
                        self.state.save_session();
                        ui.close();
                    }
                    if ui.button("⚙ Panel Feature Settings...").clicked() {
                        self.state.show_settings_modal = true;
                        ui.close();
                    }
                });

                // 4. Windows & Tools Menu Dropdown (for opening sub-windows)
                ui.menu_button("🪟 Windows", |ui| {
                    if ui.checkbox(&mut self.show_viewport_3d, "📐 3D Surface Viewport").clicked() {
                        ui.close();
                    }
                    if ui.checkbox(&mut self.show_cli_terminal, "💻 Interactive CLI Terminal").clicked() {
                        ui.close();
                    }
                    if ui.button("📚 Example Notebook Gallery").clicked() {
                        self.show_example_gallery = true;
                        ui.close();
                    }
                    if ui.button("🔍 Command Palette (Cmd+K)").clicked() {
                        self.command_palette_state.is_open = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("⚙ Notebook Settings").clicked() {
                        self.state.show_settings_modal = true;
                        ui.close();
                    }
                    if ui.button("❓ Help & Syntax Guide").clicked() {
                        self.show_help_modal = true;
                        ui.close();
                    }
                });

                // 5. Builders Menu Dropdown
                ui.menu_button("🛠 Builders", |ui| {
                    if ui.button("📐 Parameter & Variable Builder").clicked() {
                        self.show_domain_picker = true;
                        ui.close();
                    }
                    if ui.button("⚙ Parametric CAD Machinery").clicked() {
                        self.show_cad_machinery_palette = true;
                        ui.close();
                    }
                    if ui.button("🧮 Matrix & Tensor Builder").clicked() {
                        self.show_matrix_builder = true;
                        ui.close();
                    }
                    if ui.button("✂ Riemann Branch Cut Configurator").clicked() {
                        self.show_branch_cut_config = true;
                        ui.close();
                    }
                    if ui.button("📈 ODE/PDE Boundary Wizard").clicked() {
                        self.show_ode_wizard = true;
                        ui.close();
                    }
                    if ui.button("⚖ Physical Units Palette").clicked() {
                        self.show_units_palette = true;
                        ui.close();
                    }
                });

                // 7. Theme Menu Dropdown
                ui.menu_button(format!("🎨 Theme: {}", self.theme.name()), |ui| {
                    for t in crate::ui::ThemeKind::all() {
                        if ui.selectable_value(&mut self.theme, *t, t.name()).clicked() {
                            self.state.session.settings.theme = self.theme;
                            ctx.set_style_of(ctx.theme(), self.theme.to_style());
                            self.state.save_session();
                            ui.close();
                        }
                    }
                });

                ui.separator();

                if ui.button(egui::RichText::new("🔍 Palette (Cmd+K)").color(palette.accent_secondary)).on_hover_text("Open Command Palette").clicked() {
                    self.command_palette_state.is_open = true;
                }
            });
        });

        // Bottom AI Copilot Bar (Only rendered when AI Copilot is enabled in settings)
        if self.state.session.settings.ai_config.enabled {
            egui::Panel::bottom("copilot_panel").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("AI Copilot:").strong().color(egui::Color32::from_rgb(160, 120, 240)));
                    let res = ui.add(
                        egui::TextEdit::singleline(&mut self.copilot_prompt)
                            .hint_text("Ask a math question e.g. 'Explain how to differentiate x^2 + 3'...")
                            .desired_width(500.0),
                    );

                    if (res.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Ask").clicked() {
                        let prompt = self.copilot_prompt.trim().to_string();
                        if !prompt.is_empty() {
                            let agent = urae::agent::AlgebraAgentInterface::new();
                            match agent.query(&self.state.session.settings.ai_config, &prompt, &self.state.graph) {
                                Ok(response) => {
                                    self.copilot_response = Some(response);
                                }
                                Err(err) => {
                                    self.copilot_response = Some(format!("Error: {}", err));
                                }
                            }
                            self.copilot_prompt.clear();
                        }
                    }

                    if let Some(notice) = &self.export_notice {
                        ui.colored_label(egui::Color32::GREEN, notice);
                    }
                });

                if let Some(resp) = &self.copilot_response {
                    ui.add_space(2.0);
                    ui.monospace(resp);
                }
            });
        } else if self.export_notice.is_some() {
            egui::Panel::bottom("export_notice_panel").show(ui, |ui| {
                if let Some(notice) = &self.export_notice {
                    ui.colored_label(egui::Color32::GREEN, notice);
                }
            });
        }

        // Render Help and Storage Modals
        self.render_help_modal(&ctx);
        self.render_storage_modal(&ctx);

        // Settings Window (Floating or Docked)
        if self.state.show_settings_modal {
            match self.settings_dock {
                WindowDockPosition::Floating => {
                    let mut settings_open = self.state.show_settings_modal;
                    let mut close_requested = false;
                    egui::Window::new("Notebook Settings")
                        .open(&mut settings_open)
                        .collapsible(true)
                        .resizable(true)
                        .show(&ctx, |ui| {
                            close_requested = self.render_settings_contents(ui);
                        });
                    if !settings_open || close_requested {
                        self.state.show_settings_modal = false;
                    }
                }
                WindowDockPosition::DockLeft => {
                    egui::Panel::left("settings_dock_left_panel")
                        .resizable(true)
                        .default_size(300.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("Settings");
                                if ui.button("Float").clicked() {
                                    self.settings_dock = WindowDockPosition::Floating;
                                }
                                if ui.button("✕").clicked() {
                                    self.state.show_settings_modal = false;
                                }
                            });
                            ui.separator();
                            let _ = self.render_settings_contents(ui);
                        });
                }
                WindowDockPosition::DockRight => {
                    egui::Panel::right("settings_dock_right_panel")
                        .resizable(true)
                        .default_size(300.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("Settings");
                                if ui.small_button("Float").clicked() {
                                    self.settings_dock = WindowDockPosition::Floating;
                                }
                                if ui.small_button("✕").clicked() {
                                    self.state.show_settings_modal = false;
                                }
                            });
                            ui.separator();
                            let _ = self.render_settings_contents(ui);
                        });
                }
                _ => {}
            }
        }

        // Interactive Terminal CLI (Floating or Docked)
        if self.show_cli_terminal {
            match self.terminal_dock {
                WindowDockPosition::Floating => {
                    let mut terminal_open = true;
                    egui::Window::new("URAE Interactive CLI Terminal")
                        .open(&mut terminal_open)
                        .default_size([680.0, 360.0])
                        .resizable(true)
                        .collapsible(true)
                        .show(&ctx, |ui| {
                            self.render_cli_terminal_contents(&ctx, ui);
                        });
                    if !terminal_open {
                        self.show_cli_terminal = false;
                    }
                }
                WindowDockPosition::DockBottom => {
                    egui::Panel::bottom("terminal_dock_bottom_panel")
                        .resizable(true)
                        .default_size(260.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("Terminal CLI");
                                if ui.button("Float").clicked() {
                                    self.terminal_dock = WindowDockPosition::Floating;
                                }
                                if ui.button("Dock Right").clicked() {
                                    self.terminal_dock = WindowDockPosition::DockRight;
                                }
                                if ui.button("✕").clicked() {
                                    self.show_cli_terminal = false;
                                }
                            });
                            ui.separator();
                            self.render_cli_terminal_contents(&ctx, ui);
                        });
                }
                WindowDockPosition::DockRight => {
                    egui::Panel::right("terminal_dock_right_panel")
                        .resizable(true)
                        .default_size(380.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("Terminal CLI");
                                if ui.small_button("Float").clicked() {
                                    self.terminal_dock = WindowDockPosition::Floating;
                                }
                                if ui.small_button("Bottom").clicked() {
                                    self.terminal_dock = WindowDockPosition::DockBottom;
                                }
                                if ui.small_button("✕").clicked() {
                                    self.show_cli_terminal = false;
                                }
                            });
                            ui.separator();
                            self.render_cli_terminal_contents(&ctx, ui);
                        });
                }
                _ => {}
            }
        }

        // Left Sidebar: Symbol Role Conversion & Multi-Component Parameter Controls
        if self.show_left_sidebar {
            let palette = self.theme.palette();
            if self.left_panel_floating {
                let mut open = true;
                egui::Window::new("⚙ Parameters & Variables")
                    .default_size([300.0, 500.0])
                    .resizable(true)
                    .open(&mut open)
                    .show(ui, |ui| {
                        let (dock_clicked, hide_clicked) = render_column_header(
                            ui,
                            &palette,
                            &ColumnHeaderConfig {
                                title_full: "⚙ Parameters & Variables",
                                title_standard: "⚙ Parameters",
                                title_short: "⚙ Params",
                                title_tooltip: "Parameters & Variables (Symbol controls, sliders, and domains)",
                                float_icon: "⤓",
                                float_full: "⤓ Dock",
                                float_tooltip: "Dock parameters sidebar to left",
                                hide_icon: "✕",
                                hide_full: "✕ Hide",
                                hide_tooltip: "Hide parameters sidebar (Ctrl+B)",
                            },
                        );
                        if dock_clicked {
                            self.left_panel_floating = false;
                        }
                        if hide_clicked {
                            self.show_left_sidebar = false;
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if self.render_left_sidebar_contents(ui) {
                                    state_changed = true;
                                }
                            });
                    });
                if !open {
                    self.show_left_sidebar = false;
                }
            } else {
                egui::Panel::left("sidebar_panel")
                    .resizable(true)
                    .default_size(280.0)
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        let (float_clicked, hide_clicked) = render_column_header(
                            ui,
                            &palette,
                            &ColumnHeaderConfig {
                                title_full: "⚙ Parameters & Variables",
                                title_standard: "⚙ Parameters",
                                title_short: "⚙ Params",
                                title_tooltip: "Parameters & Variables (Symbol controls, sliders, and domains)",
                                float_icon: "⤢",
                                float_full: "⤢ Float",
                                float_tooltip: "Pop out parameters into a floating window",
                                hide_icon: "◀",
                                hide_full: "◀ Hide",
                                hide_tooltip: "Hide parameters sidebar (Ctrl+B)",
                            },
                        );
                        if float_clicked {
                            self.left_panel_floating = true;
                        }
                        if hide_clicked {
                            self.show_left_sidebar = false;
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if self.render_left_sidebar_contents(ui) {
                                    state_changed = true;
                                }
                            });
                    });
            }
        }

        // Main Central View (Unified Continuous Mathematical Notepad)
        egui::CentralPanel::default().show(ui, |ui| match self.view_mode {
            ViewMode::SmartStream | ViewMode::FocusEditor => {
                if self.render_smart_stream_view(ui, &ctx) {
                    state_changed = true;
                }
            }
            ViewMode::DependencyGraph => {
                self.render_dependency_graph_view(ui, &ctx);
            }
            ViewMode::PresentationReport => {
                self.render_presentation_report_view(ui, &ctx);
            }
        });

        // Render floating popped-out windows and interactive builder modals
        self.render_popped_out_windows(&ctx);
        self.render_save_modal(&ctx);
        self.render_matrix_builder_modal(&ctx);
        self.render_domain_picker_modal(&ctx);
        self.render_branch_cut_modal(&ctx);
        self.render_ode_wizard_modal(&ctx);
        self.render_units_palette_modal(&ctx);
        self.render_example_gallery_modal(&ctx);
        self.render_open_inspectors(&ctx);

        // Phase 17 UX: Render Command Palette (Cmd+K)
        if let Some(syntax) = crate::ui::CommandPalette::show(&ctx, &mut self.command_palette_state)
        {
            match syntax.as_str() {
                "__URAE_CMD_UNDO__" => {
                    self.state.undo();
                    state_changed = true;
                }
                "__URAE_CMD_REDO__" => {
                    self.state.redo();
                    state_changed = true;
                }
                "__URAE_CMD_THEME_DARK__" => {
                    self.theme = crate::ui::ThemeKind::Dark;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_THEME_LIGHT__" => {
                    self.theme = crate::ui::ThemeKind::Light;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_THEME_CATPPUCCIN__" => {
                    self.theme = crate::ui::ThemeKind::CatppuccinMocha;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_THEME_NORD__" => {
                    self.theme = crate::ui::ThemeKind::Nord;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_THEME_DRACULA__" => {
                    self.theme = crate::ui::ThemeKind::Dracula;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_THEME_SOLARIZED_DARK__" => {
                    self.theme = crate::ui::ThemeKind::SolarizedDark;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_THEME_SOLARIZED_LIGHT__" => {
                    self.theme = crate::ui::ThemeKind::SolarizedLight;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_THEME_CYBERPUNK__" => {
                    self.theme = crate::ui::ThemeKind::Cyberpunk;
                    self.state.session.settings.theme = self.theme;
                    ctx.set_style_of(ctx.theme(), self.theme.to_style());
                    self.state.save_session();
                }
                "__URAE_CMD_LAYOUT_FLUID__" => {
                    self.workspace
                        .set_layout(crate::ui::WorkspaceLayoutPreset::FluidNotepad);
                    self.view_mode = ViewMode::SmartStream;
                    self.state.session.settings.workspace_preset = self.workspace.layout_preset;
                    self.state.save_session();
                }
                "__URAE_CMD_LAYOUT_DUAL__" => {
                    self.workspace
                        .set_layout(crate::ui::WorkspaceLayoutPreset::SplitDual);
                    self.view_mode = ViewMode::SmartStream;
                    self.state.session.settings.workspace_preset = self.workspace.layout_preset;
                    self.state.save_session();
                }
                "__URAE_CMD_LAYOUT_TRIPLE__" => {
                    self.workspace
                        .set_layout(crate::ui::WorkspaceLayoutPreset::TripleIDE);
                    self.view_mode = ViewMode::SmartStream;
                    self.show_cli_terminal = true;
                    self.show_viewport_3d = true;
                    self.state.session.settings.workspace_preset = self.workspace.layout_preset;
                    self.state.save_session();
                }
                "__URAE_CMD_LAYOUT_ZEN__" => {
                    self.workspace
                        .set_layout(crate::ui::WorkspaceLayoutPreset::ZenMode);
                    self.view_mode = ViewMode::SmartStream;
                    self.state.session.settings.workspace_preset = self.workspace.layout_preset;
                    self.state.save_session();
                }
                _ => {
                    self.state.insert_text_at_active_line(&syntax);
                    state_changed = true;
                }
            }
        }

        // Phase 17 UX: Render CAD Machinery Builder Palette
        crate::ui::PalettesUi::render_cad_machinery_palette(
            &ctx,
            &mut self.show_cad_machinery_palette,
            &mut self.state,
        );

        // Phase 17 UX: Render 3D CAD & Simulation Viewport Window
        if self.show_viewport_3d {
            let mut is_open = self.show_viewport_3d;
            egui::Window::new("3D CAD & Simulation Viewport")
                .open(&mut is_open)
                .default_size([540.0, 420.0])
                .resizable(true)
                .show(&ctx, |ui| {
                    crate::ui::Viewport3D::show(ui, &mut self.viewport_3d_state, None);
                });
            self.show_viewport_3d = is_open;

            if let Some(req) = self.viewport_3d_state.export_request.take() {
                self.trigger_file_download(&req.filename, &req.data, req.format.mime_type());
                self.export_notice = Some(format!("Exported 3D model to '{}'", req.filename));
            }
        }

        // Render Ephemeral Storage & Quota Warning Banner in foreground anchored bottom-center
        self.render_warning_banners(&ctx);

        if state_changed {
            self.state.evaluate_all();
            ctx.request_repaint();
        }
    }
}

impl UraeNotebookApp {
    fn render_save_modal(&mut self, ctx: &egui::Context) {
        if self.show_save_modal {
            let mut is_open = self.show_save_modal;
            egui::Window::new("Save Notebook As...")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label("Enter notebook name and select format:");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let text_edit = egui::TextEdit::singleline(&mut self.new_notebook_name)
                            .desired_width(200.0);
                        let res = ui.add(text_edit);
                        res.request_focus(); // automatically focus input on Ctrl+S
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        egui::ComboBox::from_id_salt("save_format_combo")
                            .selected_text(match self.selected_save_format.as_str() {
                                ".json" => "JSON Workspace (.json)",
                                ".math" => "Plain Text (.math)",
                                ".txt" => "Plain Text (.txt)",
                                ".bson" => "Binary BSON (.bson)",
                                _ => "JSON Workspace (.json)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.selected_save_format,
                                    ".json".to_string(),
                                    "JSON Workspace (.json)",
                                );
                                ui.selectable_value(
                                    &mut self.selected_save_format,
                                    ".math".to_string(),
                                    "Plain Text (.math)",
                                );
                                ui.selectable_value(
                                    &mut self.selected_save_format,
                                    ".txt".to_string(),
                                    "Plain Text (.txt)",
                                );
                                ui.selectable_value(
                                    &mut self.selected_save_format,
                                    ".bson".to_string(),
                                    "Binary BSON (.bson)",
                                );
                            });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let name = self.new_notebook_name.trim().to_string();

                        if ui.button("Save").clicked() && !name.is_empty() {
                            let mut full_name = name;
                            let target_ext = &self.selected_save_format;

                            // Strip existing known extensions to swap cleanly
                            if full_name.ends_with(".json")
                                || full_name.ends_with(".math")
                                || full_name.ends_with(".txt")
                                || full_name.ends_with(".bson")
                            {
                                if let Some(dot_idx) = full_name.rfind('.') {
                                    full_name = full_name[..dot_idx].to_string();
                                }
                            }
                            full_name.push_str(target_ext);

                            self.save_notebook(&full_name);
                            self.active_notebook = full_name;
                            self.scan_available_notebooks();
                            self.show_save_modal = false;
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_save_modal = false;
                        }
                    });
                });
            self.show_save_modal = is_open;
        }
    }

    fn render_help_modal(&mut self, ctx: &egui::Context) {
        if self.show_help_modal {
            let mut help_open = self.show_help_modal;
            egui::Window::new("URAE Notebook - Comprehensive User Guide")
                .open(&mut help_open)
                .default_size([720.0, 560.0])
                .resizable(true)
                .collapsible(true)
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("URAE Interactive Mathematics Tutorial & Syntax Reference");
                        ui.weak("URAE is a multi-dimensional, context-aware symbolic Computer Algebra System.");
                        ui.separator();

                        ui.collapsing("1. NATURAL MATHEMATICAL PHRASING", |ui| {
                            ui.label("Express mathematical tasks naturally without being locked into strict functional names:");
                            ui.monospace("• Equation Solvers  : given g(x) = 3 find x");
                            ui.monospace("                     solve x^2 - 16 = 0 for x");
                            ui.monospace("                     find x where x + 5 = 11");
                            ui.monospace("• Derivatives       : d f(x, q) / dx");
                            ui.monospace("                     derivative of f(x) by x");
                            ui.monospace("                     derive x^4 wrt x");
                            ui.monospace("                     total derivative of x^2 * y");
                            ui.monospace("• Integrals         : integral of cos(x) along [2, 7)");
                            ui.monospace("                     integrate exp(-x^2) by x along (-infty, +inf)");
                            ui.monospace("                     suma cos(x) dx");
                            ui.monospace("• Limits            : limit of sin(x)/x as x -> 0");
                            ui.monospace("                     lim_(x -> 0) (1 + x)^(1/x)");
                            ui.monospace("• Summations        : sum of k^2 from k=1 to n");
                            ui.monospace("                     sum_{i=1}^{10} i^2");
                            ui.monospace("• Substitution      : evaluate x^2 + 3 at x = 2");
                            ui.monospace("                     substitute x = 4 in 2*x + 10");
                        });

                        ui.collapsing("2. MULTI-DIMENSIONAL ALGEBRAS & CONTEXTS", |ui| {
                            ui.label("Configure active algebras dynamically per-line or using the top header dropdown:");
                            ui.monospace("• Real Field (ℝ)     : standard real scalar arithmetic");
                            ui.monospace("• Complex Field (ℂ)  : inserts slider components for Real (Re) & Imaginary (Im) parts");
                            ui.monospace("• Quaternions (ℍ)    : 4D hypercomplex algebra (w, i, j, k) with non-commutative multiplication rules");
                            ui.monospace("• Clifford Algebra   : geometric Clifford numbers (e.g. p is a 3D clifford number with signature (1,1,1))");
                            ui.monospace("• Matrix Algebras    : symbolic and numeric matrix structures (LU, QR, SVD, Cholesky, Pauli generators)");
                            ui.monospace("• Boolean Logic      : SAT, predicate logic, multi-valued logic systems, three-valued truth tables");
                        });

                        ui.collapsing("3. CURVILINEAR COORDINATE SYSTEMS", |ui| {
                            ui.label("Switch between continuous coordinate spaces seamlessly:");
                            ui.monospace("• Cartesian          : (x, y, z)");
                            ui.monospace("• Polar              : (r, theta) — suitable for cylindrical and circular mappings");
                            ui.monospace("• Cylindrical        : (r, theta, z)");
                            ui.monospace("• Spherical          : (r, theta, phi)");
                            ui.monospace("• Method Chaining    : Access subcomponents directly: e.g., v.curvilinear(\"spherical\").r");
                        });

                        ui.collapsing("4. SET BUILDER BOUNDS & ROLE SWITCHING", |ui| {
                            ui.label("Bind interval restrictions and easily toggle variables to slider parameters with full metadata retention:");
                            ui.monospace("• Set-Builder bounds : { x in Reals | -5 <= x <= 5 }");
                            ui.monospace("                     { q in Quaternion | norm(q) == 1 }");
                            ui.monospace("• Role Switching     : Change symbols from Variables (e.g., x: Variable) to Parameters");
                            ui.monospace("                     (e.g., a: Parameter = 2.0 [m]) using the left sidebar conversion panel.");
                        });

                        ui.collapsing("5. PHYSICAL SI UNITS & DIMENSIONAL CONSISTENCY", |ui| {
                            ui.label("Bind unit tracking and prevent dimensional algebra violations:");
                            ui.monospace("• Syntax             : a = 9.81 [m/s^2] (Acceleration)");
                            ui.monospace("                     F = 100.0 [N]     (Force)");
                            ui.monospace("                     E = 50.0 [J]      (Energy / Work)");
                            ui.monospace("• Dimensional Check  : The engine automatically raises a DimensionMismatchError if you perform");
                            ui.monospace("                     illegal dimensional arithmetic (e.g., adding Force [N] to Length [m]).");
                        });

                        ui.collapsing("6. KEYBOARD SHORTCUTS & USER EXPERIENCE", |ui| {
                            ui.label("Accelerate your mathematical prototyping using physical key binds:");
                            ui.monospace("• F5                 : Refresh/Re-evaluate all mathematical notepad cells");
                            ui.monospace("• F11                : Toggle Fullscreen / Windowed display mode");
                            ui.monospace("• Command/Ctrl +     : Zoom in (increase font and spacing scale)");
                            ui.monospace("• Command/Ctrl -     : Zoom out (decrease font and spacing scale)");
                            ui.monospace("• Command/Ctrl 0     : Reset zoom scale back to default 1.0");
                        });

                        ui.collapsing("7. EMBEDDED CONSOLE TERMINAL", |ui| {
                            ui.label("Interact directly with your current math workspace context:");
                            ui.monospace("• vars               : Lists all active variables, parameters, domains, and bounds");
                            ui.monospace("• params             : Lists parameter sliders and values");
                            ui.monospace("• eval a * x^2       : Evaluates expressions utilizing active parameter slider values");
                            ui.monospace("• simplify <expr>    : Manually run E-Graph pattern-matching simplifications");
                        });
                    });
                });
            self.show_help_modal = help_open;
        }
    }

    fn render_settings_contents(&mut self, ui: &mut egui::Ui) -> bool {
        let mut close_settings = false;
        ui.heading("Comprehensive URAE Workspace Preferences");
        ui.separator();

        egui::ScrollArea::vertical()
            .max_height(480.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("🎨 Visual Theme & Interface Styling").strong());
                ui.horizontal(|ui| {
                    ui.label("Active Theme:");
                    let mut theme_changed = false;
                    egui::ComboBox::from_id_salt("settings_theme_combo")
                        .selected_text(self.theme.name())
                        .show_ui(ui, |ui| {
                            for t in crate::ui::ThemeKind::all() {
                                if ui.selectable_value(&mut self.theme, *t, t.name()).clicked() {
                                    theme_changed = true;
                                }
                            }
                        });
                    if theme_changed {
                        self.state.session.settings.theme = self.theme;
                        ui.ctx().set_style_of(ui.ctx().theme(), self.theme.to_style());
                        self.state.save_session();
                    }
                });

                // Theme color swatch preview
                let palette = self.theme.palette();
                ui.horizontal(|ui| {
                    ui.weak("Palette Swatch:");
                    for color in &palette.plot_palette {
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 3.0, *color);
                        let c = *color;
                        resp.widget_info(move || {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Other,
                                true,
                                format!("Color swatch rgb({}, {}, {})", c.r(), c.g(), c.b()),
                            )
                        });
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("🎛 Panel Visibility & Modular Display Features").strong());
                ui.weak("Each panel can be independently hidden, docked, or popped out into a floating window, with customized display controls.");

                // 1. Left Panel (Parameters & Variables)
                egui::CollapsingHeader::new("◀ Left Panel: Parameters & Variables")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_left_sidebar, "Visible (Ctrl+B)");
                            if self.show_left_sidebar {
                                if self.left_panel_floating {
                                    if ui.button("⤓ Dock to Left").clicked() {
                                        self.left_panel_floating = false;
                                    }
                                } else if ui.button("⤢ Pop Out to Floating Window").clicked() {
                                    self.left_panel_floating = true;
                                }
                            }
                        });
                        ui.separator();
                        ui.checkbox(
                            &mut self.state.session.settings.left_panel_show_sliders,
                            "Show interactive parameter sliders",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.left_panel_show_domains,
                            "Show domain restrictions (: Real, : Complex, : Positive...)",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.left_panel_show_values,
                            "Show numerical evaluated values (= 5.000)",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.left_panel_show_badges,
                            "Show compound classification badges ([Constant], [Scalar]...)",
                        );
                    });

                // 2. Text Editor (Continuous Math Notepad)
                egui::CollapsingHeader::new("📝 Text Editor: Continuous Math Notepad")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut self.show_editor, "Visible (Ctrl+E)").changed() {
                                self.state.session.settings.show_editor = self.show_editor;
                            }
                            if self.show_editor {
                                if self.editor_floating {
                                    if ui.button("⤓ Dock to Center").clicked() {
                                        self.editor_floating = false;
                                    }
                                } else if ui.button("⤢ Pop Out to Floating Window").clicked() {
                                    self.editor_floating = true;
                                }
                            }
                        });
                        ui.separator();
                        ui.checkbox(
                            &mut self.state.session.settings.show_cell_line_numbers,
                            "Show cell line number gutter badges (#1, #2...)",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.editor_syntax_highlighting,
                            "Mathematical syntax highlighting & operator tinting",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.editor_word_wrap,
                            "Word wrap editor lines at panel boundary",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.editor_alt_scrubbing,
                            "Enable Alt+Drag number scrubbing on numeric literals",
                        );
                    });

                // 3. Right Panel (Results & Visualizations)
                egui::CollapsingHeader::new("▶ Right Panel: Results & Visualizations")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_right_sidebar, "Visible (Ctrl+J)");
                            if self.show_right_sidebar {
                                if self.right_panel_floating {
                                    if ui.button("⤓ Dock to Right").clicked() {
                                        self.right_panel_floating = false;
                                    }
                                } else if ui.button("⤢ Pop Out to Floating Window").clicked() {
                                    self.right_panel_floating = true;
                                }
                            }
                        });
                        ui.separator();
                        ui.checkbox(
                            &mut self.state.session.settings.right_panel_show_plots,
                            "Show 2D function graphs and plots",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.right_panel_show_3d,
                            "Show 3D surfaces and mesh viewports",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.right_panel_show_cad,
                            "Show 2D CAD canvas geometry projections",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.right_panel_show_solutions,
                            "Show algebraic solutions and root breakdowns",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.right_panel_compact_mode,
                            "Compact single-line output stream",
                        );
                        ui.checkbox(
                            &mut self.state.session.settings.right_panel_show_inbound_refs,
                            "Show inbound dependency indicators",
                        );
                    });

                // 4. Terminal (Interactive CLI)
                egui::CollapsingHeader::new("💻 Terminal: Interactive CLI")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_cli_terminal, "Visible (Ctrl+`)");
                            if self.show_cli_terminal {
                                egui::ComboBox::from_id_salt("settings_terminal_dock_combo")
                                    .selected_text(match self.terminal_dock {
                                        WindowDockPosition::Floating => "⤢ Floating Window",
                                        WindowDockPosition::DockBottom => "⤓ Docked Bottom",
                                        WindowDockPosition::DockRight => "▶ Docked Right",
                                        WindowDockPosition::DockLeft => "◀ Docked Left",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.terminal_dock, WindowDockPosition::Floating, "⤢ Floating Window");
                                        ui.selectable_value(&mut self.terminal_dock, WindowDockPosition::DockBottom, "⤓ Docked Bottom");
                                        ui.selectable_value(&mut self.terminal_dock, WindowDockPosition::DockRight, "▶ Docked Right");
                                    });
                            }
                        });
                        ui.separator();
                        ui.checkbox(
                            &mut self.state.session.settings.terminal_show_timing,
                            "Display execution latency / timing per command",
                        );
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Document Behavior & Auto-Generation").strong());
                ui.checkbox(
                    &mut self.state.session.settings.show_cell_line_numbers,
                    "Show cell line number gutter badges (#1, #2...)",
                );
                ui.checkbox(
                    &mut self.state.session.settings.auto_sliders,
                    "Automatically generate parameter sliders for free variables",
                );
                ui.checkbox(
                    &mut self.state.session.settings.auto_plots,
                    "Automatically display inline borderless function plots",
                );
                ui.checkbox(
                    &mut self.state.session.settings.show_hover_tooltips,
                    "Show hover popover tooltips over mathematical expressions",
                );
                ui.checkbox(
                    &mut self.state.session.settings.show_derivatives,
                    "Compute and display automatic derivatives in details dropdown",
                );

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Evaluation & Solver Settings").strong());

                ui.horizontal(|ui| {
                    ui.label("Logging Verbosity Level:");
                    egui::ComboBox::from_id_salt("logging_level_combo")
                        .selected_text(&self.state.session.settings.logging_level)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.state.session.settings.logging_level,
                                "off".to_string(),
                                "Off (Optimal Performance)",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.logging_level,
                                "error".to_string(),
                                "Error",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.logging_level,
                                "warn".to_string(),
                                "Warn",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.logging_level,
                                "info".to_string(),
                                "Info (Context Promotion Logs)",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.logging_level,
                                "debug".to_string(),
                                "Debug (Verbose Simplification Steps)",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.logging_level,
                                "trace".to_string(),
                                "Trace",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Default Numerical Plot Bounds:");
                    ui.add(
                        egui::DragValue::new(&mut self.state.session.settings.default_range_min)
                            .speed(0.1),
                    );
                    ui.label("to");
                    ui.add(
                        egui::DragValue::new(&mut self.state.session.settings.default_range_max)
                            .speed(0.1),
                    );
                });

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("AI Copilot Provider Setup").strong());
                ui.label(
                    "AI components are turned off by default for local privacy and performance.",
                );

                let ai_config = &mut self.state.session.settings.ai_config;

                ui.checkbox(&mut ai_config.enabled, "Enable AI Copilot Features");

                if ai_config.enabled {
                    ui.horizontal(|ui| {
                        ui.label("AI Service Provider:");
                        egui::ComboBox::from_id_salt("ai_provider_combo")
                            .selected_text(ai_config.provider.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut ai_config.provider,
                                    urae::agent::AiProvider::GoogleAi,
                                    "Google AI (Gemini)",
                                );
                                ui.selectable_value(
                                    &mut ai_config.provider,
                                    urae::agent::AiProvider::Anthropic,
                                    "Anthropic (Claude)",
                                );
                                ui.selectable_value(
                                    &mut ai_config.provider,
                                    urae::agent::AiProvider::LmStudio,
                                    "LM Studio (Local Host)",
                                );
                                ui.selectable_value(
                                    &mut ai_config.provider,
                                    urae::agent::AiProvider::OpenCode,
                                    "OpenCode",
                                );
                                ui.selectable_value(
                                    &mut ai_config.provider,
                                    urae::agent::AiProvider::OpenRouter,
                                    "OpenRouter Gateway",
                                );
                            });
                    });

                    let mut key_str = ai_config.api_key.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("Private API Key:");
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut key_str)
                                    .password(true)
                                    .hint_text("Enter Key..."),
                            )
                            .changed()
                        {
                            ai_config.api_key = if key_str.trim().is_empty() {
                                None
                            } else {
                                Some(key_str)
                            };
                        }
                    });

                    let mut url_str = ai_config.access_url.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("API Access URL:");
                        if ui
                            .add(egui::TextEdit::singleline(&mut url_str).hint_text(format!(
                                "Default: {}",
                                ai_config.provider.default_access_url()
                            )))
                            .changed()
                        {
                            ai_config.access_url = if url_str.trim().is_empty() {
                                None
                            } else {
                                Some(url_str)
                            };
                        }
                    });

                    let mut model_str = ai_config.model_name.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("Model Identifier:");
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut model_str).hint_text(format!(
                                    "Default: {}",
                                    ai_config.provider.default_model()
                                )),
                            )
                            .changed()
                        {
                            ai_config.model_name = if model_str.trim().is_empty() {
                                None
                            } else {
                                Some(model_str)
                            };
                        }
                    });
                }

                ui.colored_label(
                    if ai_config.is_configured() {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::YELLOW
                    },
                    ai_config.status_summary(),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("Reactive Invalidation & Performance (60 FPS)").strong(),
                );
                ui.horizontal(|ui| {
                    ui.label("Reactive Compute Mode:");
                    egui::ComboBox::from_id_salt("reactive_mode_combo")
                        .selected_text(match self.state.session.settings.reactive_mode {
                            ReactiveComputeMode::AdaptiveDualRate => {
                                "Adaptive Dual-Rate (60 FPS + Async Convergence)"
                            }
                            ReactiveComputeMode::SoftCapFastDrag => {
                                "Soft-Cap Fast Drag (Approx Drag, Exact Release)"
                            }
                            ReactiveComputeMode::FrameBudgeted => {
                                "Frame-Budgeted Guard (8ms Budget)"
                            }
                            ReactiveComputeMode::FullSynchronous => {
                                "Full Synchronous (Heavy Multi-Core)"
                            }
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.state.session.settings.reactive_mode,
                                ReactiveComputeMode::AdaptiveDualRate,
                                "Adaptive Dual-Rate (Recommended)",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.reactive_mode,
                                ReactiveComputeMode::SoftCapFastDrag,
                                "Soft-Cap Fast Drag",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.reactive_mode,
                                ReactiveComputeMode::FrameBudgeted,
                                "Frame-Budgeted Guard (8ms)",
                            );
                            ui.selectable_value(
                                &mut self.state.session.settings.reactive_mode,
                                ReactiveComputeMode::FullSynchronous,
                                "Full Synchronous",
                            );
                        });
                });
                ui.checkbox(
                    &mut self.state.session.settings.auto_throttle_enabled,
                    "Enable dynamic auto-throttling during rapid typing/scrubbing",
                );
                ui.horizontal(|ui| {
                    ui.label("Measured Evaluation Latency:");
                    ui.colored_label(
                        if self.state.measured_eval_latency_ms < 8.0 {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::YELLOW
                        },
                        format!(
                            "{:.2} ms (60 FPS target: <16.6ms)",
                            self.state.measured_eval_latency_ms
                        ),
                    );
                });

                ui.add_space(8.0);
                if ui
                    .button("Reset Workspace Preferences to Factory Defaults")
                    .clicked()
                {
                    self.state.session.settings = crate::notebook::NotebookSettings::default();
                }
            });

        ui.separator();
        ui.add_space(4.0);
        if ui.button("Apply & Close Settings").clicked() {
            close_settings = true;
        }
        close_settings
    }

    /// Render the parameters and variables sidebar contents respecting user settings toggles.
    pub fn render_left_sidebar_contents(&mut self, ui: &mut egui::Ui) -> bool {
        let mut state_changed = false;
        if self.state.session.symbol_metadata.is_empty() {
            ui.weak("No active symbols.");
            return false;
        }

        let mut updated_meta: Vec<(String, SymbolMetadata)> = Vec::new();

        // Sort symbols alphabetically for a stable, organized panel
        let mut symbol_entries: Vec<(String, SymbolMetadata)> = self
            .state
            .session
            .symbol_metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        symbol_entries.sort_by(|a, b| a.0.cmp(&b.0));

        for (sym, meta) in &symbol_entries {
            ui.group(|ui| {
                let mut mut_meta = meta.clone();

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(sym).strong());
                    if self.state.session.settings.left_panel_show_domains {
                        ui.colored_label(
                            egui::Color32::from_rgb(180, 140, 255),
                            format!(": {}", mut_meta.domain_type),
                        );
                    }

                    let role_btn_label = match mut_meta.role {
                        SymbolRole::Parameter => "Param",
                        SymbolRole::Variable => "Var",
                        SymbolRole::Constant => "Const",
                    };
                    if ui
                        .button(role_btn_label)
                        .on_hover_text("Convert Role (retains metadata)")
                        .clicked()
                    {
                        mut_meta.role = match mut_meta.role {
                            SymbolRole::Parameter => SymbolRole::Variable,
                            SymbolRole::Variable => SymbolRole::Parameter,
                            SymbolRole::Constant => SymbolRole::Variable,
                        };
                    }
                });

                if self.state.session.settings.left_panel_show_values
                    && mut_meta.role != SymbolRole::Parameter
                {
                    ui.horizontal(|ui| {
                        ui.monospace(format!("= {:.4}", mut_meta.cur_val));
                        if let Some(u) = &mut_meta.unit_str {
                            ui.weak(format!("[{}]", u));
                        }
                    });
                }
                let badges = mut_meta.distinct_badges();
                if self.state.session.settings.left_panel_show_badges && !badges.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for tag in badges {
                            ui.weak(format!("[{}]", tag));
                        }
                    });
                }

                if self.state.session.settings.left_panel_show_sliders
                    && mut_meta.role == SymbolRole::Parameter
                {
                    match mut_meta.domain_type.as_str() {
                        "Complex" => {
                            ui.weak("Complex Components (Re + Im i):");
                            let real_key = format!("{}_real", sym);
                            let imag_key = format!("{}_imag", sym);
                            let mut r_val = *self
                                .state
                                .session
                                .slider_values
                                .get(&real_key)
                                .unwrap_or(&mut_meta.cur_val);
                            let mut i_val = *self
                                .state
                                .session
                                .slider_values
                                .get(&imag_key)
                                .unwrap_or(&0.0);

                            ui.horizontal(|ui| {
                                ui.weak("Re:");
                                if ui
                                    .add(egui::Slider::new(
                                        &mut r_val,
                                        mut_meta.min_val..=mut_meta.max_val,
                                    ))
                                    .changed()
                                {
                                    self.state.session.slider_values.insert(real_key, r_val);
                                    mut_meta.cur_val = r_val;
                                    state_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.weak("Im:");
                                if ui
                                    .add(egui::Slider::new(
                                        &mut i_val,
                                        mut_meta.min_val..=mut_meta.max_val,
                                    ))
                                    .changed()
                                {
                                    self.state.session.slider_values.insert(imag_key, i_val);
                                    state_changed = true;
                                }
                            });
                        }
                        "Quaternion" => {
                            ui.weak("Quaternion Components (w + i i + j j + k k):");
                            for component in ["w", "i", "j", "k"] {
                                let comp_key = format!("{}_{}", sym, component);
                                let mut comp_val = *self
                                    .state
                                    .session
                                    .slider_values
                                    .get(&comp_key)
                                    .unwrap_or(&0.0);
                                ui.horizontal(|ui| {
                                    ui.weak(format!("{}:", component));
                                    if ui
                                        .add(egui::Slider::new(
                                            &mut comp_val,
                                            mut_meta.min_val..=mut_meta.max_val,
                                        ))
                                        .changed()
                                    {
                                        self.state.session.slider_values.insert(comp_key, comp_val);
                                        state_changed = true;
                                    }
                                });
                            }
                        }
                        "Boolean" => {
                            let mut is_true = *self
                                .state
                                .session
                                .slider_values
                                .get(sym)
                                .unwrap_or(&mut_meta.cur_val)
                                > 0.5;
                            ui.horizontal(|ui| {
                                ui.weak("Bool:");
                                let lbl = if is_true { "1 (True)" } else { "0 (False)" };
                                if ui.checkbox(&mut is_true, lbl).changed() {
                                    let val = if is_true { 1.0 } else { 0.0 };
                                    self.state.session.slider_values.insert(sym.clone(), val);
                                    mut_meta.cur_val = val;
                                    state_changed = true;
                                }
                            });
                        }
                        "GaussianIntegers" | "EisensteinIntegers" => {
                            let lattice_name = if mut_meta.domain_type == "GaussianIntegers" {
                                "ℤ[i]"
                            } else {
                                "ℤ[ω]"
                            };
                            ui.weak(format!("Lattice {} (Re + Im):", lattice_name));
                            let real_key = format!("{}_real", sym);
                            let imag_key = format!("{}_imag", sym);
                            let mut r_val = self
                                .state
                                .session
                                .slider_values
                                .get(&real_key)
                                .copied()
                                .unwrap_or(mut_meta.cur_val)
                                .round();
                            let mut i_val = self
                                .state
                                .session
                                .slider_values
                                .get(&imag_key)
                                .copied()
                                .unwrap_or(0.0)
                                .round();

                            ui.horizontal(|ui| {
                                ui.weak("Re:");
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut r_val,
                                            mut_meta.min_val.round()..=mut_meta.max_val.round(),
                                        )
                                        .step_by(1.0)
                                        .integer(),
                                    )
                                    .changed()
                                {
                                    self.state.session.slider_values.insert(real_key, r_val);
                                    mut_meta.cur_val = r_val;
                                    state_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.weak("Im:");
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut i_val,
                                            mut_meta.min_val.round()..=mut_meta.max_val.round(),
                                        )
                                        .step_by(1.0)
                                        .integer(),
                                    )
                                    .changed()
                                {
                                    self.state.session.slider_values.insert(imag_key, i_val);
                                    state_changed = true;
                                }
                            });
                        }
                        "Integers" | "Integer" | "Naturals" | "Modulo" | "ModuloUnits"
                        | "BitVector" | "EvenIntegers" | "OddIntegers" | "GaloisField" => {
                            let mut val = self
                                .state
                                .session
                                .slider_values
                                .get(sym)
                                .copied()
                                .unwrap_or(mut_meta.cur_val)
                                .round();
                            let step = if mut_meta.domain_type == "EvenIntegers"
                                || mut_meta.domain_type == "OddIntegers"
                            {
                                2.0
                            } else {
                                1.0
                            };
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut val,
                                            mut_meta.min_val.round()..=mut_meta.max_val.round(),
                                        )
                                        .step_by(step)
                                        .integer(),
                                    )
                                    .changed()
                                {
                                    self.state.session.slider_values.insert(sym.clone(), val);
                                    mut_meta.cur_val = val;
                                    state_changed = true;
                                }
                                if let Some(unit) = &mut_meta.unit_str {
                                    ui.weak(format!("[{}]", unit));
                                }
                            });
                        }
                        _ => {
                            let mut val = *self
                                .state
                                .session
                                .slider_values
                                .get(sym)
                                .unwrap_or(&mut_meta.cur_val);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(egui::Slider::new(
                                        &mut val,
                                        mut_meta.min_val..=mut_meta.max_val,
                                    ))
                                    .changed()
                                {
                                    self.state.session.slider_values.insert(sym.clone(), val);
                                    mut_meta.cur_val = val;
                                    state_changed = true;
                                }
                                if let Some(unit) = &mut_meta.unit_str {
                                    ui.weak(format!("[{}]", unit));
                                }
                            });
                        }
                    }
                }

                updated_meta.push((sym.clone(), mut_meta));
            });
        }

        for (sym, meta) in updated_meta {
            self.state.session.symbol_metadata.insert(sym, meta);
        }

        state_changed
    }

    fn render_cli_terminal_contents(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.weak("Notebook Context: Direct access to active variables, parameters, and DAG state.");
        ui.separator();

        egui::ScrollArea::vertical()
            .max_height(240.0)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.cli_history.is_empty() {
                    ui.monospace("Universal Rust Algebra Engine (URAE) REPL Terminal v0.1.0");
                    ui.monospace("Type 'help' for commands, 'vars' to list variables, or 'eval a * x^2' to evaluate parameters.");
                    ui.separator();
                }
                for (cmd, out) in &self.cli_history {
                    ui.monospace(egui::RichText::new(format!("urae> {}", cmd)).strong().color(egui::Color32::from_rgb(120, 200, 255)));
                    ui.monospace(format!("= {}\n", out));
                }
            });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("urae>")
                    .strong()
                    .color(egui::Color32::from_rgb(120, 200, 255)),
            );
            let res = ui.add(
                egui::TextEdit::singleline(&mut self.cli_input)
                    .hint_text("Enter CLI command e.g. 'diff x^3, x', 'vars', 'eval a * x^2'...")
                    .desired_width(520.0)
                    .font(egui::TextStyle::Monospace),
            );

            let execute_command = (res.lost_focus()
                && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.button("Run").clicked();

            if execute_command {
                let input_cmd = self.cli_input.trim().to_string();
                if !input_cmd.is_empty() {
                    let bindings = self.state.session.slider_values.clone();

                    let mut summary = Vec::new();
                    for (sym, meta) in &self.state.session.symbol_metadata {
                        let role_str = match meta.role {
                            SymbolRole::Variable => "Variable",
                            SymbolRole::Parameter => "Parameter",
                            SymbolRole::Constant => "Constant",
                        };
                        let cur_val = *self
                            .state
                            .session
                            .slider_values
                            .get(sym)
                            .unwrap_or(&meta.cur_val);
                        summary.push((
                            sym.clone(),
                            role_str.to_string(),
                            cur_val,
                            meta.unit_str.clone().unwrap_or_default(),
                        ));
                    }

                    // 1. Collect evaluated numeric line results from active document
                    let mut line_results = std::collections::HashMap::new();
                    for (idx, pl) in self.state.parsed_lines.iter().enumerate() {
                        if let Some(val) = pl.linearized_estimate {
                            line_results.insert(idx, val);
                        }
                    }
                    let last_val = self
                        .state
                        .parsed_lines
                        .iter()
                        .rev()
                        .find_map(|pl| pl.linearized_estimate);

                    // 2. Resolve $N, ans, range mathematical aggregations: sum($1..$4), mean($1..$4), $1..$3
                    let mut resolved_cmd = crate::notebook::NotebookState::resolve_line_references(
                        &input_cmd,
                        last_val,
                        &line_results,
                    );

                    // 3. Fallback for non-numeric references or formula tokens (e.g. diff $1, x where $1 is an expression)
                    for (idx, pl) in self.state.parsed_lines.iter().enumerate() {
                        let pat_dollar = format!("${}", idx + 1);
                        let pat_line = format!("line({})", idx + 1);
                        let pat_line_alt = format!("Line {}", idx + 1);
                        if resolved_cmd.contains(&pat_dollar)
                            || resolved_cmd.contains(&pat_line)
                            || resolved_cmd.contains(&pat_line_alt)
                        {
                            let formula = if let Some((_, rhs)) = pl.raw_text.split_once('=') {
                                rhs.trim()
                            } else {
                                pl.raw_text.trim()
                            };
                            if !formula.is_empty() {
                                let wrapped = format!("({})", formula);
                                resolved_cmd = resolved_cmd.replace(&pat_dollar, &wrapped);
                                resolved_cmd = resolved_cmd.replace(&pat_line, &wrapped);
                                resolved_cmd = resolved_cmd.replace(&pat_line_alt, &wrapped);
                            }
                        }
                    }

                    // 4. Special commands: "lines" or "doc" to inspect active document lines
                    let result = if input_cmd.eq_ignore_ascii_case("lines")
                        || input_cmd.eq_ignore_ascii_case("doc")
                    {
                        let mut doc_summary = String::new();
                        doc_summary.push_str("Active Document Lines:\n");
                        for (idx, pl) in self.state.parsed_lines.iter().enumerate() {
                            let val_str = if let Some(v) = pl.linearized_estimate {
                                format!(" => {}", v)
                            } else if !pl.output_unicode.is_empty() {
                                format!(" => {}", pl.output_unicode)
                            } else {
                                String::new()
                            };
                            doc_summary.push_str(&format!(
                                "  Line #{}: {}{}\n",
                                idx + 1,
                                pl.raw_text,
                                val_str
                            ));
                        }
                        Ok(doc_summary)
                    } else {
                        let start_t = web_time::Instant::now();
                        let exec_res = urae::cli::process_input_with_context(
                            &self.state.graph,
                            &self.state.formatter,
                            &resolved_cmd,
                            &mut self.state.session.settings.ai_config,
                            Some(&bindings),
                            Some(&summary),
                        );
                        let elapsed_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                        match exec_res {
                            Ok(out) => {
                                if self.state.session.settings.terminal_show_timing {
                                    Ok(format!("{} (in {:.2}ms)", out, elapsed_ms))
                                } else {
                                    Ok(out)
                                }
                            }
                            Err(err) => Err(err),
                        }
                    };

                    let output_text = match result {
                        Ok(out) => out,
                        Err(err) => format!("Error: {}", err),
                    };

                    self.cli_history.push((input_cmd, output_text));
                    self.cli_input.clear();
                    res.request_focus();
                }
            }
        });
    }

    #[allow(dead_code)]
    fn render_live_preview_lines(&mut self, ui: &mut egui::Ui) {
        if self.state.parsed_lines.is_empty() {
            ui.weak(
                "Type mathematical formulas in the notepad editor to see live formatted output.",
            );
            return;
        }

        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

        let default_mode = CardDisplayMode::Inline;
        let mut mode_changes = Vec::new();

        for pl in &self.state.parsed_lines {
            ui.add_space(2.0);

            match pl.kind {
                LineKind::Markdown => {
                    ui.label(egui::RichText::new(&pl.raw_text).heading());
                }
                LineKind::Formula => {
                    let expr_key = pl.raw_text.trim();
                    let card_mode = *self
                        .state
                        .session
                        .card_modes
                        .get(expr_key)
                        .unwrap_or(&default_mode);
                    let is_expanded = self.show_role_details.contains(expr_key);

                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            if !pl.output_unicode.is_empty() {
                                let mut text = egui::RichText::new(&pl.output_unicode).strong();
                                if pl.error_msg.is_some() {
                                    text = text.color(egui::Color32::RED);
                                }
                                let lbl = ui.label(text);
                                if pl.is_pending {
                                    lbl.on_hover_text("waiting for solution...");
                                }
                            } else {
                                ui.monospace(&pl.raw_text);
                            }

                            let btn_label = if is_expanded {
                                "Hide Details"
                            } else {
                                "Details"
                            };
                            if ui.small_button(btn_label).clicked() {
                                if is_expanded {
                                    self.show_role_details.remove(expr_key);
                                } else {
                                    self.show_role_details.insert(expr_key.to_string());
                                }
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if pl.is_function {
                                        match card_mode {
                                            CardDisplayMode::Inline => {
                                                if ui
                                                    .small_button("Pop Out")
                                                    .on_hover_text(
                                                        "Pop out graph into a floating window",
                                                    )
                                                    .clicked()
                                                {
                                                    mode_changes.push((
                                                        expr_key.to_string(),
                                                        CardDisplayMode::PoppedOut,
                                                    ));
                                                }
                                                if ui.small_button("Hide Plot").clicked() {
                                                    mode_changes.push((
                                                        expr_key.to_string(),
                                                        CardDisplayMode::Hidden,
                                                    ));
                                                }
                                            }
                                            CardDisplayMode::Hidden => {
                                                if ui.small_button("Show Plot").clicked() {
                                                    mode_changes.push((
                                                        expr_key.to_string(),
                                                        CardDisplayMode::Inline,
                                                    ));
                                                }
                                            }
                                            CardDisplayMode::PoppedOut => {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(180, 140, 255),
                                                    "Popped Out",
                                                );
                                                if ui.small_button("Dock").clicked() {
                                                    mode_changes.push((
                                                        expr_key.to_string(),
                                                        CardDisplayMode::Inline,
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                },
                            );
                        });

                        if is_expanded {
                            ui.separator();
                            ui.weak(format!("• Scoped Context: {}", pl.scoped_context.summary()));
                            ui.weak(format!("• Raw Expression: {}", pl.raw_text));
                            if let Some(simp) = &pl.simplified_unicode {
                                ui.colored_label(
                                    egui::Color32::from_rgb(140, 220, 180),
                                    format!("• Background Simplified Form: {}", simp),
                                );
                            }
                            if let Some(unit) = &pl.physical_unit {
                                ui.weak(format!("• Physical Unit: [{}]", unit));
                            }
                            if let Some(domain) = &pl.domain_info {
                                ui.weak(format!("• Domain Info: {}", domain));
                            }
                            if let Some(roots) = &pl.numerical_roots {
                                let roots_fmt: Vec<String> =
                                    roots.iter().map(|r| format!("{:.6}", r)).collect();
                                ui.weak(format!(
                                    "• Numerical Roots (f64): [{}]",
                                    roots_fmt.join(", ")
                                ));
                            }
                            if let Some(est) = pl.linearized_estimate {
                                ui.weak(format!("• Linearized Taylor Estimate: x ≈ {:.6}", est));
                            }
                            if pl.eval_time_ms > 0 {
                                ui.weak(format!("• Evaluation Time: {} ms", pl.eval_time_ms));
                            }

                            // Compute derivative and integral info on demand
                            let parser = urae::parser::ExprParser::new(&self.state.graph);
                            if let Ok(expr_id) = parser.parse(expr_key) {
                                let x_sym = self.state.graph.symbols.get_or_intern("x");
                                let diff_id = self.state.graph.diff(expr_id, x_sym);
                                if let Ok(diff_fmt) = self
                                    .state
                                    .unicode_formatter
                                    .format(&self.state.graph, diff_id)
                                {
                                    ui.weak(format!("• Derivative d/dx: {}", diff_fmt));
                                }

                                if let Ok(int_id) = self.state.graph.integrate(expr_id, x_sym) {
                                    if let Ok(int_fmt) = self
                                        .state
                                        .unicode_formatter
                                        .format(&self.state.graph, int_id)
                                    {
                                        ui.weak(format!("• Indefinite Integral ∫ dx: {}", int_fmt));
                                    }
                                }
                            }
                        }

                        if card_mode == CardDisplayMode::Inline && pl.is_function {
                            let (smart_min, smart_max) =
                                self.state.compute_smart_plot_bounds(expr_key, "x");
                            let mut min_r = smart_min;
                            let mut max_r = smart_max;

                            let plot_response = Plot::new(format!("inline_plot_{}", pl.line_idx))
                                .height(140.0)
                                .allow_drag(true)
                                .allow_zoom(true)
                                .allow_scroll(false)
                                .include_x(smart_min)
                                .include_x(smart_max)
                                .auto_bounds(egui::Vec2b::new(false, true))
                                .show(ui, |plot_ui| {
                                    let scroll_y = plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                                    let smooth_y = plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                                    let scroll_delta = if scroll_y.abs() > 0.0 {
                                        scroll_y
                                    } else {
                                        smooth_y
                                    };
                                    if plot_ui.response().hovered() && scroll_delta.abs() > 0.0 {
                                        let zoom_factor =
                                            (scroll_delta * 0.0025).clamp(-0.5, 0.5).exp();
                                        plot_ui.zoom_bounds_around_hovered(egui::Vec2::splat(
                                            zoom_factor,
                                        ));
                                    }

                                    let bounds = plot_ui.plot_bounds();
                                    min_r = bounds.min()[0];
                                    max_r = bounds.max()[0];

                                    // Stabilise: don't let bounds drift to nonsensical ranges
                                    if !min_r.is_finite()
                                        || !max_r.is_finite()
                                        || (max_r - min_r).abs() < 1e-12
                                    {
                                        min_r = smart_min;
                                        max_r = smart_max;
                                    }

                                    let pts_data = self.state.cached_evaluate_plot_points(
                                        expr_key, "x", min_r, max_r, 200,
                                    );

                                    if !pts_data.is_empty() {
                                        let y_clamp = (max_r - min_r).abs() * 25.0;
                                        let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
                                        let mut cur_seg: Vec<[f64; 2]> = Vec::new();

                                        for p in pts_data {
                                            if !p[0].is_finite()
                                                || !p[1].is_finite()
                                                || p[1].abs() > y_clamp
                                            {
                                                if !cur_seg.is_empty() {
                                                    segments.push(std::mem::take(&mut cur_seg));
                                                }
                                                continue;
                                            }
                                            if let Some(prev) = cur_seg.last() {
                                                let dx = (p[0] - prev[0]).abs();
                                                let dy = (p[1] - prev[1]).abs();
                                                if dx > 1e-9
                                                    && (dy / dx) > 1e5
                                                    && (p[1] * prev[1] < 0.0)
                                                {
                                                    segments.push(std::mem::take(&mut cur_seg));
                                                }
                                            }
                                            cur_seg.push(p);
                                        }
                                        if !cur_seg.is_empty() {
                                            segments.push(cur_seg);
                                        }

                                        for seg in segments {
                                            if !seg.is_empty() {
                                                let plot_points: PlotPoints =
                                                    seg.into_iter().collect();
                                                let line = Line::new("", plot_points)
                                                    .color(egui::Color32::from_rgb(100, 180, 240))
                                                    .width(2.0_f32);
                                                plot_ui.line(line);
                                            }
                                        }
                                    }
                                });

                            let expr_label = expr_key.to_string();
                            plot_response.response.widget_info(move || {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Other,
                                    true,
                                    format!(
                                        "Mathematical function plot of {}. Domain: x in [{:.2}, {:.2}]. Drag to pan, scroll to zoom.",
                                        expr_label, min_r, max_r
                                    ),
                                )
                            });

                            let pts_summary = self.state.cached_evaluate_plot_points(
                                expr_key, "x", min_r, max_r, 80,
                            );
                            if !pts_summary.is_empty() {
                                let valid_pts: Vec<[f64; 2]> = pts_summary.into_iter().filter(|p| p[0].is_finite() && p[1].is_finite()).collect();
                                if !valid_pts.is_empty() {
                                    let min_y = valid_pts.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
                                    let max_y = valid_pts.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);

                                    let mut roots = Vec::new();
                                    for i in 1..valid_pts.len() {
                                        let y0 = valid_pts[i-1][1];
                                        let y1 = valid_pts[i][1];
                                        if y0 * y1 <= 0.0 && (y1 - y0).abs() < (max_y - min_y).abs() * 0.5 {
                                            let x_mid = (valid_pts[i-1][0] + valid_pts[i][0]) * 0.5;
                                            roots.push(format!("{:.2}", x_mid));
                                            if roots.len() >= 4 { break; }
                                        }
                                    }
                                    let root_summary = if roots.is_empty() {
                                        "none in interval".to_string()
                                    } else {
                                        roots.join(", ")
                                    };

                                    ui.collapsing("📊 Plot Data Summary (Screen Reader & Accessible)", |ui| {
                                        ui.small(format!("Function: {}", expr_key));
                                        ui.small(format!("Domain X: [{:.2}, {:.2}]", min_r, max_r));
                                        ui.small(format!("Range Y: [{:.2}, {:.2}]", min_y, max_y));
                                        ui.small(format!("Approx. roots: {}", root_summary));
                                    });
                                }
                            }

                            if plot_response.response.dragged() || plot_response.response.hovered()
                            {
                                ui.ctx().request_repaint();
                            }
                        }
                    });
                }
                LineKind::SliderDef {
                    ref name,
                    val,
                    ref unit,
                } => {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Slider: {}", name)).strong());
                            ui.monospace(format!("= {:.2}", val));
                            if let Some(u) = unit {
                                ui.weak(format!("[{}]", u));
                            }
                        });
                    });
                }
                LineKind::DomainRestriction {
                    ref name,
                    ref rule_summary,
                } => {
                    let is_expanded = self.show_role_details.contains(name);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(220, 180, 100), format!("Domain Rule for '{}': {}", name, rule_summary));

                            let btn_label = if is_expanded { "Hide Details" } else { "Details" };
                            if ui.small_button(btn_label).clicked() {
                                if is_expanded {
                                    self.show_role_details.remove(name);
                                } else {
                                    self.show_role_details.insert(name.clone());
                                }
                            }
                        });

                        if is_expanded {
                            ui.separator();
                            ui.weak(format!("• Symbol: {}", name));
                            ui.weak(format!("• Rule Summary: {}", rule_summary));
                            ui.weak("• Enforcement Status: Active Domain Bound Enforced in Simplification Engine");
                        }
                    });
                }
                LineKind::RoleDeclaration { ref name, role } => {
                    let role_str = match role {
                        SymbolRole::Parameter => "Parameter",
                        SymbolRole::Variable => "Variable",
                        SymbolRole::Constant => "Constant",
                    };
                    let is_expanded = self.show_role_details.contains(name);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                egui::Color32::from_rgb(160, 140, 240),
                                format!("Declared '{}' as {}", name, role_str),
                            );

                            let btn_label = if is_expanded {
                                "Hide Details"
                            } else {
                                "Details"
                            };
                            if ui.small_button(btn_label).clicked() {
                                if is_expanded {
                                    self.show_role_details.remove(name);
                                } else {
                                    self.show_role_details.insert(name.clone());
                                }
                            }
                        });

                        if is_expanded {
                            if let Some(meta) = self.state.session.symbol_metadata.get(name) {
                                ui.separator();
                                ui.weak(format!("• Domain Type: {}", meta.domain_type));
                                ui.weak(format!("• Current Value: {:.4}", meta.cur_val));
                                ui.weak(format!(
                                    "• Slider Bounds: [{:.1} .. {:.1}]",
                                    meta.min_val, meta.max_val
                                ));
                                if let Some(u) = &meta.unit_str {
                                    ui.weak(format!("• Physical Unit: [{}]", u));
                                }
                            }
                        }
                    });
                }
                LineKind::SetBuilder {
                    ref var_name,
                    ref domain_type,
                    ref condition,
                } => {
                    let is_expanded = self.show_role_details.contains(var_name);

                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{{ {} in {} | {} }}",
                                    var_name, domain_type, condition
                                ))
                                .strong()
                                .color(egui::Color32::from_rgb(180, 140, 255)),
                            );

                            let btn_label = if is_expanded {
                                "Hide Details"
                            } else {
                                "Details"
                            };
                            if ui.small_button(btn_label).clicked() {
                                if is_expanded {
                                    self.show_role_details.remove(var_name);
                                } else {
                                    self.show_role_details.insert(var_name.clone());
                                }
                            }
                        });

                        if is_expanded {
                            ui.separator();
                            ui.weak(format!("• Target Variable: {}", var_name));
                            ui.weak(format!("• Domain / Algebraic Structure: {}", domain_type));
                            ui.weak(format!("• Set Filter Condition: {}", condition));
                            ui.weak(
                                "• Set Classification: Bounded Interval / Subspace Restriction",
                            );
                        }
                    });
                }
                LineKind::CadMesh {
                    ref model_kind,
                    ref summary,
                } => {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("⚙ CAD Model: {}", model_kind))
                                    .strong()
                                    .color(egui::Color32::from_rgb(56, 189, 248)),
                            );
                            ui.weak(summary);
                        });
                        ui.separator();
                        crate::ui::Viewport3D::show_with_height(
                            ui,
                            &mut self.viewport_3d_state,
                            None,
                            220.0,
                        );
                    });
                }
                LineKind::Plot3D {
                    ref expr_str,
                    ref x_var,
                    ref y_var,
                } => {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "📊 3D Surface: z = f({}, {})",
                                    x_var, y_var
                                ))
                                .strong()
                                .color(egui::Color32::from_rgb(52, 211, 153)),
                            );
                            ui.monospace(expr_str);
                        });
                        ui.separator();
                        crate::ui::Viewport3D::show_with_height(
                            ui,
                            &mut self.viewport_3d_state,
                            None,
                            220.0,
                        );
                    });
                }
            }

            ui.add_space(4.0);
        }

        for (expr_key, mode) in mode_changes {
            self.state.set_card_mode(&expr_key, mode);
        }
    }

    /// Render popped-out floating windows for graphs in `CardDisplayMode::PoppedOut`.
    fn render_popped_out_windows(&mut self, ctx: &egui::Context) {
        let mut mode_changes = Vec::new();
        let mut native_to_remove = Vec::new();

        for pl in &self.state.parsed_lines {
            if pl.is_function {
                let expr_key = pl.raw_text.trim();
                if self.state.get_card_mode(expr_key) == CardDisplayMode::PoppedOut {
                    let (smart_min, smart_max) =
                        self.state.compute_smart_plot_bounds(expr_key, "x");
                    if self.native_viewport_graphs.contains(expr_key) {
                        let viewport_id = egui::ViewportId::from_hash_of(expr_key);
                        let mut close_native = false;
                        let min_r = smart_min;
                        let max_r = smart_max;
                        let pl_idx = pl.line_idx;

                        ctx.show_viewport_immediate(
                            viewport_id,
                            egui::ViewportBuilder::default()
                                .with_title(format!("URAE Standalone Graph: {}", expr_key))
                                .with_inner_size([480.0, 320.0]),
                            |ui, _class| {
                                egui::CentralPanel::default().show(ui, |ui| {
                                    let mut local_min_r = min_r;
                                    let mut local_max_r = max_r;
                                    let plot_response =
                                        Plot::new(format!("native_viewport_plot_{}", pl_idx))
                                            .height(260.0)
                                            .allow_drag(true)
                                            .allow_zoom(true)
                                            .allow_scroll(false)
                                            .include_x(smart_min)
                                            .include_x(smart_max)
                                            .auto_bounds(egui::Vec2b::new(false, true))
                                            .show(ui, |plot_ui| {
                                                let scroll_y = plot_ui
                                                    .ctx()
                                                    .input(|i| i.smooth_scroll_delta.y);
                                                let smooth_y = plot_ui
                                                    .ctx()
                                                    .input(|i| i.smooth_scroll_delta.y);
                                                let scroll_delta = if scroll_y.abs() > 0.0 {
                                                    scroll_y
                                                } else {
                                                    smooth_y
                                                };
                                                if plot_ui.response().hovered()
                                                    && scroll_delta.abs() > 0.0
                                                {
                                                    let zoom_factor = (scroll_delta * 0.0025)
                                                        .clamp(-0.5, 0.5)
                                                        .exp();
                                                    plot_ui.zoom_bounds_around_hovered(
                                                        egui::Vec2::splat(zoom_factor),
                                                    );
                                                }

                                                let bounds = plot_ui.plot_bounds();
                                                local_min_r = bounds.min()[0];
                                                local_max_r = bounds.max()[0];

                                                if !local_min_r.is_finite()
                                                    || !local_max_r.is_finite()
                                                    || (local_max_r - local_min_r).abs() < 1e-12
                                                {
                                                    local_min_r = min_r;
                                                    local_max_r = max_r;
                                                }

                                                let pts_data =
                                                    self.state.cached_evaluate_plot_points(
                                                        expr_key,
                                                        "x",
                                                        local_min_r,
                                                        local_max_r,
                                                        200,
                                                    );

                                                if !pts_data.is_empty() {
                                                    let y_clamp =
                                                        (local_max_r - local_min_r).abs() * 25.0;
                                                    let mut segments: Vec<Vec<[f64; 2]>> =
                                                        Vec::new();
                                                    let mut cur_seg: Vec<[f64; 2]> = Vec::new();

                                                    for p in pts_data {
                                                        if !p[0].is_finite()
                                                            || !p[1].is_finite()
                                                            || p[1].abs() > y_clamp
                                                        {
                                                            if !cur_seg.is_empty() {
                                                                segments.push(std::mem::take(
                                                                    &mut cur_seg,
                                                                ));
                                                            }
                                                            continue;
                                                        }
                                                        if let Some(prev) = cur_seg.last() {
                                                            let dx = (p[0] - prev[0]).abs();
                                                            let dy = (p[1] - prev[1]).abs();
                                                            if dx > 1e-9
                                                                && (dy / dx) > 1e5
                                                                && (p[1] * prev[1] < 0.0)
                                                            {
                                                                segments.push(std::mem::take(
                                                                    &mut cur_seg,
                                                                ));
                                                            }
                                                        }
                                                        cur_seg.push(p);
                                                    }
                                                    if !cur_seg.is_empty() {
                                                        segments.push(cur_seg);
                                                    }

                                                    for seg in segments {
                                                        if !seg.is_empty() {
                                                            let plot_points: PlotPoints =
                                                                seg.into_iter().collect();
                                                            let line = Line::new("", plot_points)
                                                                .color(egui::Color32::from_rgb(
                                                                    240, 160, 50,
                                                                ))
                                                                .width(2.0_f32);
                                                            plot_ui.line(line);
                                                        }
                                                    }
                                                }
                                            });

                                    let expr_label = expr_key.to_string();
                                    plot_response.response.widget_info(move || {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Other,
                                            true,
                                            format!(
                                                "Standalone window plot of {}. Domain: x in [{:.2}, {:.2}]. Drag to pan, scroll to zoom.",
                                                expr_label, local_min_r, local_max_r
                                            ),
                                        )
                                    });

                                    if plot_response.response.dragged()
                                        || plot_response.response.hovered()
                                    {
                                        ctx.request_repaint();
                                    }
                                });

                                if ctx.input(|i| i.viewport().close_requested()) {
                                    close_native = true;
                                    mode_changes
                                        .push((expr_key.to_string(), CardDisplayMode::Hidden));
                                }
                            },
                        );

                        if close_native {
                            native_to_remove.push(expr_key.to_string());
                        }
                    } else {
                        let mut window_open = true;
                        egui::Window::new(format!("Graph: {}", expr_key))
                            .open(&mut window_open)
                            .default_size([380.0, 260.0])
                            .resizable(true)
                            .collapsible(true)
                            .show(&ctx, |ui| {
                                let mut local_min_r = smart_min;
                                let mut local_max_r = smart_max;

                                let plot_response =
                                    Plot::new(format!("floating_plot_{}", pl.line_idx))
                                        .height(220.0)
                                        .allow_drag(true)
                                        .allow_zoom(true)
                                        .allow_scroll(false)
                                        .include_x(smart_min)
                                        .include_x(smart_max)
                                        .auto_bounds(egui::Vec2b::new(false, true))
                                        .show(ui, |plot_ui| {
                                            let scroll_y =
                                                plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                                            let smooth_y =
                                                plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                                            let scroll_delta = if scroll_y.abs() > 0.0 {
                                                scroll_y
                                            } else {
                                                smooth_y
                                            };
                                            if plot_ui.response().hovered()
                                                && scroll_delta.abs() > 0.0
                                            {
                                                let zoom_factor =
                                                    (scroll_delta * 0.0025).clamp(-0.5, 0.5).exp();
                                                plot_ui.zoom_bounds_around_hovered(
                                                    egui::Vec2::splat(zoom_factor),
                                                );
                                            }

                                            let bounds = plot_ui.plot_bounds();
                                            local_min_r = bounds.min()[0];
                                            local_max_r = bounds.max()[0];

                                            if !local_min_r.is_finite()
                                                || !local_max_r.is_finite()
                                                || (local_max_r - local_min_r).abs() < 1e-12
                                            {
                                                local_min_r = smart_min;
                                                local_max_r = smart_max;
                                            }

                                            let pts_data = self.state.cached_evaluate_plot_points(
                                                expr_key,
                                                "x",
                                                local_min_r,
                                                local_max_r,
                                                200,
                                            );

                                            if !pts_data.is_empty() {
                                                let y_clamp =
                                                    (local_max_r - local_min_r).abs() * 25.0;
                                                let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
                                                let mut cur_seg: Vec<[f64; 2]> = Vec::new();

                                                for p in pts_data {
                                                    if !p[0].is_finite()
                                                        || !p[1].is_finite()
                                                        || p[1].abs() > y_clamp
                                                    {
                                                        if !cur_seg.is_empty() {
                                                            segments
                                                                .push(std::mem::take(&mut cur_seg));
                                                        }
                                                        continue;
                                                    }
                                                    if let Some(prev) = cur_seg.last() {
                                                        let dx = (p[0] - prev[0]).abs();
                                                        let dy = (p[1] - prev[1]).abs();
                                                        if dx > 1e-9
                                                            && (dy / dx) > 1e5
                                                            && (p[1] * prev[1] < 0.0)
                                                        {
                                                            segments
                                                                .push(std::mem::take(&mut cur_seg));
                                                        }
                                                    }
                                                    cur_seg.push(p);
                                                }
                                                if !cur_seg.is_empty() {
                                                    segments.push(cur_seg);
                                                }

                                                for seg in segments {
                                                    if !seg.is_empty() {
                                                        let plot_points: PlotPoints =
                                                            seg.into_iter().collect();
                                                        let line = Line::new("", plot_points)
                                                            .color(egui::Color32::from_rgb(
                                                                120, 200, 255,
                                                            ))
                                                            .width(2.0_f32);
                                                        plot_ui.line(line);
                                                    }
                                                }
                                            }
                                        });

                                let expr_label = expr_key.to_string();
                                plot_response.response.widget_info(move || {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Other,
                                        true,
                                        format!(
                                            "Floating window plot of {}. Domain: x in [{:.2}, {:.2}]. Drag to pan, scroll to zoom.",
                                            expr_label, local_min_r, local_max_r
                                        ),
                                    )
                                });

                                if plot_response.response.dragged()
                                    || plot_response.response.hovered()
                                {
                                    ctx.request_repaint();
                                }
                            });

                        if !window_open {
                            mode_changes.push((expr_key.to_string(), CardDisplayMode::Hidden));
                        }
                    }
                }
            }
        }

        for expr_key in native_to_remove {
            self.native_viewport_graphs.remove(&expr_key);
        }

        for (expr_key, mode) in mode_changes {
            self.state.set_card_mode(&expr_key, mode);
        }
    }

    /// Render the unified, continuous notepad (Line numbers, Full/Split Editor, and Reactive Gutter).
    fn render_smart_stream_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        let mut state_changed = false;
        let palette = self.theme.palette();

        // Keyboard shortcuts: Ctrl+B (Left sidebar), Ctrl+J (Right stream), Ctrl+E (Text editor), Ctrl+` (Terminal)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::B)) {
            self.show_left_sidebar = !self.show_left_sidebar;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::J)) {
            self.show_right_sidebar = !self.show_right_sidebar;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::E)) {
            self.show_editor = !self.show_editor;
            self.state.session.settings.show_editor = self.show_editor;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Backtick)) {
            self.show_cli_terminal = !self.show_cli_terminal;
        }

        let total_width = ui.available_width();
        let line_num_width = if self.state.session.settings.show_cell_line_numbers {
            36.0
        } else {
            0.0
        };
        let unhide_width = if !self.show_left_sidebar { 68.0 } else { 0.0 };
        let is_dual_halves =
            self.workspace.layout_preset == crate::ui::WorkspaceLayoutPreset::DualHalves;
        let available_space = (total_width - line_num_width - unhide_width - 32.0).max(300.0);
        let has_docked_right =
            (self.show_right_sidebar || is_dual_halves) && !self.right_panel_floating;
        let gutter_width = if has_docked_right {
            if is_dual_halves && (self.right_sidebar_width - 280.0).abs() < 1.0 {
                self.right_sidebar_width = (available_space * (1.0 - self.workspace.split_ratio))
                    .clamp(200.0, (available_space - 150.0).max(200.0));
            }
            if !self.show_editor || self.editor_floating {
                available_space
            } else {
                self.right_sidebar_width
                    .clamp(180.0, (available_space - 120.0).max(200.0))
            }
        } else {
            0.0
        };
        let editor_width = if has_docked_right {
            (total_width - gutter_width - line_num_width - unhide_width - 32.0).max(120.0)
        } else {
            (total_width - line_num_width - unhide_width - 80.0).max(200.0)
        };

        let line_count = self
            .state
            .session
            .raw_document_text
            .split('\n')
            .count()
            .max(1);
        let parsed_lines = self.state.parsed_lines.clone();

        // Floating Popped-Out Text Editor Window
        if self.editor_floating && self.show_editor {
            let mut open = true;
            egui::Window::new("📝 Continuous Math Editor")
                .default_size([550.0, 600.0])
                .resizable(true)
                .open(&mut open)
                .show(ctx, |ui| {
                    let (dock_clicked, hide_clicked) = render_column_header(
                        ui,
                        &palette,
                        &ColumnHeaderConfig {
                            title_full: "📝 Continuous Math Editor",
                            title_standard: "📝 Editor",
                            title_short: "📝 Edit",
                            title_tooltip: "Continuous Mathematical Notepad Editor",
                            float_icon: "⤓",
                            float_full: "⤓ Dock",
                            float_tooltip: "Dock text editor to center",
                            hide_icon: "✕",
                            hide_full: "✕ Hide",
                            hide_tooltip: "Hide text editor",
                        },
                    );
                    if dock_clicked {
                        self.editor_floating = false;
                    }
                    if hide_clicked {
                        self.show_editor = false;
                        self.state.session.settings.show_editor = false;
                    }
                    ui.separator();
                    let mut math_layouter =
                        |ui: &egui::Ui, string: &dyn egui::TextBuffer, wrap_width: f32| {
                            let string = string.as_str();
                            let eff_wrap = if self.state.session.settings.editor_word_wrap {
                                wrap_width
                            } else {
                                f32::INFINITY
                            };
                            if self.state.session.settings.editor_syntax_highlighting {
                                palette.math_syntax_layouter(ui, string, eff_wrap)
                            } else {
                                palette.plain_monospace_layouter(ui, string, eff_wrap)
                            }
                        };
                    let text_edit =
                        egui::TextEdit::multiline(&mut self.state.session.raw_document_text)
                            .layouter(&mut math_layouter)
                            .desired_width(ui.available_width())
                            .desired_rows(line_count.max(20))
                            .lock_focus(true);
                    let edit_resp = ui.add(text_edit);
                    if edit_resp.changed() {
                        self.is_edit_dirty = true;
                        self.last_keystroke_time = web_time::Instant::now();
                        state_changed = true;
                    }
                });
            if !open {
                self.show_editor = false;
                self.state.session.settings.show_editor = false;
            }
        }

        // Floating Popped-Out Right Results & Viewport Window
        if self.right_panel_floating && self.show_right_sidebar {
            let mut open = true;
            egui::Window::new("⚡ Live Results, Graphs & 3D Surfaces")
                .default_size([550.0, 600.0])
                .resizable(true)
                .open(&mut open)
                .show(ctx, |ui| {
                    let (dock_clicked, hide_clicked) = render_column_header(
                        ui,
                        &palette,
                        &ColumnHeaderConfig {
                            title_full: "⚡ Results & 3D Surfaces",
                            title_standard: "⚡ Results",
                            title_short: "⚡ Results",
                            title_tooltip: "Live Results Stream and Visualizations",
                            float_icon: "⤓",
                            float_full: "⤓ Dock",
                            float_tooltip: "Dock results to right half",
                            hide_icon: "✕",
                            hide_full: "✕ Hide",
                            hide_tooltip: "Hide results window",
                        },
                    );
                    if dock_clicked {
                        self.right_panel_floating = false;
                    }
                    if hide_clicked {
                        self.show_right_sidebar = false;
                    }
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for pl in &parsed_lines {
                                let is_left_entity = match &pl.kind {
                                    LineKind::SliderDef { .. }
                                    | LineKind::RoleDeclaration { .. }
                                    | LineKind::DomainRestriction { .. }
                                    | LineKind::SetBuilder { .. } => true,
                                    LineKind::Formula => {
                                        !pl.is_function
                                            && !pl.is_surface_3d
                                            && pl.error_msg.is_none()
                                            && is_variable_or_param_assignment(&pl.raw_text)
                                    }
                                    _ => false,
                                };
                                if is_left_entity {
                                    continue;
                                }
                                match &pl.kind {
                                    LineKind::Formula => {
                                        if pl.is_surface_3d
                                            && self.state.session.settings.right_panel_show_3d
                                        {
                                            crate::ui::Viewport3D::show_with_height(
                                                ui,
                                                &mut self.viewport_3d_state,
                                                None,
                                                240.0,
                                            );
                                        } else if !pl.output_unicode.is_empty() {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.monospace(&pl.output_unicode);
                                            });
                                        }
                                    }
                                    LineKind::CadMesh {
                                        model_kind,
                                        summary,
                                    } => {
                                        if self.state.session.settings.right_panel_show_cad {
                                            ui.label(format!("CAD: {}", model_kind));
                                            ui.weak(summary);
                                            crate::ui::Viewport3D::show_with_height(
                                                ui,
                                                &mut self.viewport_3d_state,
                                                None,
                                                240.0,
                                            );
                                        } else {
                                            ui.label(format!("CAD: {}", model_kind));
                                        }
                                    }
                                    LineKind::Plot3D {
                                        expr_str,
                                        x_var,
                                        y_var,
                                    } => {
                                        if self.state.session.settings.right_panel_show_3d {
                                            ui.label(format!(
                                                "z = f({}, {}) = {}",
                                                x_var, y_var, expr_str
                                            ));
                                            crate::ui::Viewport3D::show_with_height(
                                                ui,
                                                &mut self.viewport_3d_state,
                                                None,
                                                240.0,
                                            );
                                        } else {
                                            ui.label(format!(
                                                "z = f({}, {}) = {}",
                                                x_var, y_var, expr_str
                                            ));
                                        }
                                    }
                                    _ => {}
                                }
                                ui.add_space(4.0);
                            }
                        });
                });
            if !open {
                self.show_right_sidebar = false;
            }
        }

        let mut slider_updates = Vec::new();
        let mut plot_to_open = None;
        let mut refactor_target: Option<(usize, String)> = None;
        let mut inspector_to_open: Option<String> = None;
        let mut new_hovered_line: Option<usize> = None;

        let dep_highlighted_lines: HashSet<usize> = if let Some(h_idx) = self.hovered_line_idx {
            compute_line_upstream_dependencies(h_idx, &parsed_lines)
        } else {
            HashSet::new()
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // Left parameters unhide button (if hidden)
                    if !self.show_left_sidebar
                        && ui
                            .button(egui::RichText::new("▶ Params (Ctrl+B)").size(12.0).color(palette.text_muted))
                            .on_hover_text("Show parameters sidebar (Ctrl+B)")
                            .clicked()
                    {
                        self.show_left_sidebar = true;
                    }

                    // Editor unhide button (if hidden)
                    if !self.show_editor
                        && ui
                            .button(egui::RichText::new("📝 Editor (Ctrl+E)").size(12.0).color(palette.text_muted))
                            .on_hover_text("Show text editor (Ctrl+E)")
                            .clicked()
                    {
                        self.show_editor = true;
                        self.state.session.settings.show_editor = true;
                    }

                    // Main Continuous Text Editor & Line Numbers (if visible and docked)
                    if self.show_editor && !self.editor_floating {
                        let total_editor_w = line_num_width + editor_width;
                        ui.allocate_ui_with_layout(
                            egui::vec2(total_editor_w, ui.available_height()),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                ui.add_space(4.0);
                                let (float_clicked, hide_clicked) = render_column_header(
                                    ui,
                                    &palette,
                                    &ColumnHeaderConfig {
                                        title_full: "📝 Continuous Math Editor",
                                        title_standard: "📝 Editor",
                                        title_short: "📝 Edit",
                                        title_tooltip: "Continuous Mathematical Notepad Editor (Ctrl+E)",
                                        float_icon: "⤢",
                                        float_full: "⤢ Float",
                                        float_tooltip: "Pop out text editor into floating window",
                                        hide_icon: "✕",
                                        hide_full: "✕ Hide",
                                        hide_tooltip: "Hide text editor (Ctrl+E)",
                                    },
                                );
                                if float_clicked {
                                    self.editor_floating = true;
                                }
                                if hide_clicked {
                                    self.show_editor = false;
                                    self.state.session.settings.show_editor = false;
                                }
                                ui.separator();

                                ui.horizontal_top(|ui| {
                                    // 1. Line Numbers Column (Interactive Click & Drag explicitly aligned with editor rows, wrapped lines unnumbered)
                                    if self.state.session.settings.show_cell_line_numbers {
                                        let mono_font = egui::FontId::monospace(13.0);
                                        let default_row_height = ui.fonts_mut(|f| f.row_height(&mono_font));
                                        let frame_padding_y = 2.0;

                                        let galley = palette.math_syntax_layouter(ui, &self.state.session.raw_document_text, editor_width);

                                        ui.allocate_ui_with_layout(
                                            egui::vec2(line_num_width, ui.available_height()),
                                            egui::Layout::top_down(egui::Align::RIGHT),
                                            |ui| {
                                                ui.spacing_mut().item_spacing.y = 0.0;
                                                ui.add_space(frame_padding_y);

                                                if galley.rows.is_empty() {
                                                    let (rect, _) = ui.allocate_exact_size(
                                                        egui::vec2(line_num_width, default_row_height),
                                                        egui::Sense::hover(),
                                                    );
                                                    ui.painter().text(
                                                        egui::pos2(rect.right(), rect.center().y),
                                                        egui::Align2::RIGHT_CENTER,
                                                        " 1 ",
                                                        mono_font.clone(),
                                                        palette.text_muted,
                                                    );
                                                } else {
                                                    let mut logical_line_idx = 0;
                                                    let mut is_new_logical_line = true;

                                                    for row in &galley.rows {
                                                        let row_height = row.rect().height();
                                                        let (rect, resp) = ui.allocate_exact_size(
                                                            egui::vec2(line_num_width, row_height),
                                                            egui::Sense::click_and_drag(),
                                                        );

                                                        if is_new_logical_line {
                                                            logical_line_idx += 1;
                                                            let line_no = logical_line_idx;
                                                            resp.widget_info(move || {
                                                                egui::WidgetInfo::labeled(
                                                                    egui::WidgetType::Button,
                                                                    true,
                                                                    format!("Line number {}", line_no),
                                                                )
                                                            });
                                                            let is_dep_highlighted = dep_highlighted_lines.contains(&(logical_line_idx - 1));
                                                            if is_dep_highlighted {
                                                                ui.painter().rect_filled(
                                                                    rect,
                                                                    2.0,
                                                                    palette.accent_secondary.linear_multiply(0.35),
                                                                );
                                                            }
                                                            let text_color = if resp.hovered() || is_dep_highlighted {
                                                                palette.accent_primary
                                                            } else {
                                                                palette.text_muted
                                                            };
                                                            let num_text = format!("{:2} ", logical_line_idx);
                                                            ui.painter().text(
                                                                egui::pos2(rect.right(), rect.center().y),
                                                                egui::Align2::RIGHT_CENTER,
                                                                num_text,
                                                                mono_font.clone(),
                                                                text_color,
                                                            );
                                                            if resp.clicked() {
                                                                let token = format!("${}", logical_line_idx);
                                                                if let Some(pos) = self.state.cursor_char_idx {
                                                                    let doc = &mut self.state.session.raw_document_text;
                                                                    let clamped = pos.min(doc.len());
                                                                    let insert_str = if clamped > 0 && !doc[..clamped].ends_with(' ') && !doc[..clamped].ends_with('\n') {
                                                                        format!(" {}", token)
                                                                    } else {
                                                                        token.clone()
                                                                    };
                                                                    doc.insert_str(clamped, &insert_str);
                                                                    self.state.cursor_char_idx = Some(clamped + insert_str.len());
                                                                } else {
                                                                    self.state.session.raw_document_text.push_str(&format!(" ${}", logical_line_idx));
                                                                }
                                                                self.is_edit_dirty = true;
                                                                self.last_keystroke_time = web_time::Instant::now();
                                                                state_changed = true;
                                                            }
                                                            if resp.hovered() {
                                                                new_hovered_line = Some(logical_line_idx - 1);
                                                                self.inspected_symbol = Some(format!("${}", logical_line_idx));
                                                            }
                                                            resp.on_hover_text(format!("Line {} — Click to insert ${} into notepad", logical_line_idx, logical_line_idx));
                                                        } else {
                                                            // Wrapped continuation lines do NOT receive a line number
                                                            if resp.clicked() {
                                                                let token = format!("${}", logical_line_idx);
                                                                if let Some(pos) = self.state.cursor_char_idx {
                                                                    let doc = &mut self.state.session.raw_document_text;
                                                                    let clamped = pos.min(doc.len());
                                                                    let insert_str = if clamped > 0 && !doc[..clamped].ends_with(' ') && !doc[..clamped].ends_with('\n') {
                                                                        format!(" {}", token)
                                                                    } else {
                                                                        token.clone()
                                                                    };
                                                                    doc.insert_str(clamped, &insert_str);
                                                                    self.state.cursor_char_idx = Some(clamped + insert_str.len());
                                                                } else {
                                                                    self.state.session.raw_document_text.push_str(&format!(" ${}", logical_line_idx));
                                                                }
                                                                self.is_edit_dirty = true;
                                                                self.last_keystroke_time = web_time::Instant::now();
                                                                state_changed = true;
                                                            }
                                                            if resp.hovered() {
                                                                new_hovered_line = Some(logical_line_idx - 1);
                                                                self.inspected_symbol = Some(format!("${}", logical_line_idx));
                                                            }
                                                            resp.on_hover_text(format!("Line {} (wrapped continuation)", logical_line_idx));
                                                        }

                                                        is_new_logical_line = row.ends_with_newline;
                                                    }
                                                }
                                            },
                                        );
                                    }

                                    // 2. Main Continuous Text Editor
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(editor_width, ui.available_height()),
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        |ui| {
                                            let mut math_layouter = |ui: &egui::Ui, string: &dyn egui::TextBuffer, wrap_width: f32| {
                                                let string = string.as_str();
                                                let eff_wrap = if self.state.session.settings.editor_word_wrap {
                                                    wrap_width
                                                } else {
                                                    f32::INFINITY
                                                };
                                                if self.state.session.settings.editor_syntax_highlighting {
                                                    palette.math_syntax_layouter(ui, string, eff_wrap)
                                                } else {
                                                    palette.plain_monospace_layouter(ui, string, eff_wrap)
                                                }
                                            };

                                            let text_edit = egui::TextEdit::multiline(&mut self.state.session.raw_document_text)
                                                .layouter(&mut math_layouter)
                                                .desired_width(editor_width)
                                                .desired_rows(line_count.max(20))
                                                .lock_focus(true);

                                            let edit_resp = ui.add(text_edit);
                                            if let Some(text_state) = egui::text_edit::TextEditState::load(ui.ctx(), edit_resp.id) {
                                                if let Some(char_range) = text_state.cursor.char_range() {
                                                    let char_idx = char_range.primary.index;
                                                    let text = &self.state.session.raw_document_text;
                                                    let clamped = usize::from(char_idx).min(text.len());
                                                    let current_line_idx = text[..clamped].chars().filter(|&c| c == '\n').count();
                                                    self.state.focused_line = Some(current_line_idx);
                                                    self.state.cursor_char_idx = Some(clamped);
                                                }
                                            }
                                            if edit_resp.changed() {
                                                self.is_edit_dirty = true;
                                                self.last_keystroke_time = web_time::Instant::now();
                                                state_changed = true;
                                            }

                                            // Inline Number Scrubbing via Alt + Drag
                                            let is_alt_down = self.state.session.settings.editor_alt_scrubbing && ui.input(|i| i.modifiers.alt);
                                            if is_alt_down && (edit_resp.hovered() || self.active_scrubbing.is_some()) {
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                            }

                                            if is_alt_down && ui.input(|i| i.pointer.primary_down()) {
                                                if self.active_scrubbing.is_none() {
                                                    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
                                                    let char_idx = egui::text_edit::TextEditState::load(ui.ctx(), edit_resp.id)
                                                        .and_then(|s| s.cursor.char_range().map(|cr| cr.primary.index));
                                                    if let Some(c_idx) = char_idx {
                                                        if let Some((byte_start, byte_end, val, has_dec, dec_places)) =
                                                            find_numeric_literal_at(&self.state.session.raw_document_text, usize::from(c_idx))
                                                        {
                                                            let start_x = pointer_pos.map(|p| p.x).unwrap_or(0.0);
                                                            self.active_scrubbing = Some(ActiveScrubbing {
                                                                byte_range: (byte_start, byte_end),
                                                                original_val: val,
                                                                has_decimal: has_dec,
                                                                decimal_places: dec_places,
                                                                start_pointer_x: start_x,
                                                            });
                                                        }
                                                    }
                                                }

                                                if let Some(ref mut scrub) = self.active_scrubbing {
                                                    let curr_x = ui.input(|i| {
                                                        i.pointer
                                                            .interact_pos()
                                                            .map(|p| p.x)
                                                            .unwrap_or(scrub.start_pointer_x)
                                                    });
                                                    let delta_x = curr_x - scrub.start_pointer_x;
                                                    let is_shift = ui.input(|i| i.modifiers.shift);
                                                    let step = if is_shift {
                                                        if scrub.has_decimal {
                                                            10.0_f64.powi(-(scrub.decimal_places as i32)) * 0.1
                                                        } else {
                                                            0.1
                                                        }
                                                    } else if scrub.has_decimal {
                                                        10.0_f64.powi(-(scrub.decimal_places as i32))
                                                    } else {
                                                        1.0
                                                    };
                                                    let new_val = scrub.original_val + (delta_x as f64 * step * 0.2);

                                                    let new_str = if scrub.has_decimal {
                                                        format!("{:.precision$}", new_val, precision = scrub.decimal_places)
                                                    } else {
                                                        format!("{}", new_val.round() as i64)
                                                    };

                                                    let doc = &mut self.state.session.raw_document_text;
                                                    let (start_b, end_b) = scrub.byte_range;
                                                    if start_b <= end_b && end_b <= doc.len() {
                                                        doc.replace_range(start_b..end_b, &new_str);
                                                        scrub.byte_range = (start_b, start_b + new_str.len());
                                                        self.is_edit_dirty = true;
                                                        self.last_keystroke_time = web_time::Instant::now();
                                                        state_changed = true;
                                                    }
                                                }
                                            } else if !ui.input(|i| i.pointer.primary_down()) {
                                                self.active_scrubbing = None;
                                            }
                                        },
                                    );
                                });
                            },
                        );
                    }

                    // 3. Reactive Gutter Column (if show_right_sidebar is true and not floating, or in DualHalves mode)
                    if has_docked_right {
                        let (res_rect, res_resp) = ui.allocate_exact_size(
                            egui::vec2(8.0, ui.available_height().max(400.0)),
                            egui::Sense::click_and_drag(),
                        );
                        if res_resp.dragged() {
                            let delta_x = res_resp.drag_delta().x;
                            self.right_sidebar_width = (self.right_sidebar_width - delta_x)
                                .clamp(180.0, (available_space - 120.0).max(200.0));
                            self.workspace.split_ratio = (1.0 - (self.right_sidebar_width / available_space)).clamp(0.15, 0.85);
                            ui.ctx().request_repaint();
                        }
                        if res_resp.hovered() || res_resp.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            ui.painter().rect_filled(res_rect, 2.0, palette.accent_primary);
                        } else {
                            ui.painter().rect_filled(res_rect, 0.0, palette.border);
                        }
                        res_resp.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Other,
                                true,
                                "Sidebar resize handle",
                            )
                        });
                        let _ = res_resp.on_hover_text("Drag to resize results column");

                        ui.allocate_ui_with_layout(
                            egui::vec2(gutter_width, ui.available_height()),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                ui.add_space(4.0);
                                let (float_clicked, hide_clicked) = render_column_header(
                                    ui,
                                    &palette,
                                    &ColumnHeaderConfig {
                                        title_full: "⚡ Results & 3D Surfaces",
                                        title_standard: "⚡ Results",
                                        title_short: "⚡ Results",
                                        title_tooltip: "Live Results Stream and Visualizations (Ctrl+J)",
                                        float_icon: "⤢",
                                        float_full: "⤢ Float",
                                        float_tooltip: "Pop out results and graphs into floating window",
                                        hide_icon: "▶",
                                        hide_full: "Hide ▶",
                                        hide_tooltip: "Hide results stream (Ctrl+J)",
                                    },
                                );
                                if float_clicked {
                                    self.right_panel_floating = !self.right_panel_floating;
                                }
                                if hide_clicked {
                                    self.show_right_sidebar = false;
                                }
                                ui.separator();

                                if parsed_lines.is_empty() {
                                    ui.weak("Type math formulas to see live results...");
                                    return;
                                }

                                for pl in &parsed_lines {
                                    if pl.raw_text.trim().is_empty() {
                                        continue;
                                    }

                                    // Variables, parameters, and constants default exclusively to the LEFT half.
                                    // They are never rendered in the right column results stream.
                                    let is_left_entity = match &pl.kind {
                                        LineKind::SliderDef { .. }
                                        | LineKind::RoleDeclaration { .. }
                                        | LineKind::DomainRestriction { .. }
                                        | LineKind::SetBuilder { .. } => true,
                                        LineKind::Formula => {
                                            !pl.is_function
                                                && !pl.is_surface_3d
                                                && pl.error_msg.is_none()
                                                && is_variable_or_param_assignment(&pl.raw_text)
                                        }
                                        _ => false,
                                    };
                                    if is_left_entity {
                                        continue;
                                    }

                                    let line_token = format!("${}", pl.line_idx + 1);
                                    let is_dep_highlighted = dep_highlighted_lines.contains(&pl.line_idx);

                                    // Check if downstream lines reference this line
                                    let referencing_lines: Vec<usize> = parsed_lines
                                        .iter()
                                        .filter(|other| {
                                            other.line_idx > pl.line_idx
                                                 && (other.raw_text.contains(&line_token)
                                                    || (other.line_idx == pl.line_idx + 1
                                                        && other.raw_text.contains("ans")))
                                        })
                                        .map(|other| other.line_idx + 1)
                                        .collect();

                                    let frame_color = if is_dep_highlighted {
                                        palette.accent_secondary.linear_multiply(0.25)
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    let frame_stroke = if is_dep_highlighted {
                                        egui::Stroke::new(1.0_f32, palette.accent_primary.linear_multiply(0.7))
                                    } else {
                                        egui::Stroke::NONE
                                    };

                                    let margin = if self.state.session.settings.right_panel_compact_mode {
                                        egui::Margin::symmetric(2, 0)
                                    } else {
                                        egui::Margin::symmetric(4, 2)
                                    };
                                    let line_resp = egui::Frame::NONE
                                        .fill(frame_color)
                                        .stroke(frame_stroke)
                                        .corner_radius(4)
                                        .inner_margin(margin)
                                        .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            match &pl.kind {
                                                LineKind::Markdown => {
                                                    let trimmed = pl.raw_text.trim();
                                                    if trimmed.starts_with('#') {
                                                        let level = trimmed.chars().take_while(|c| *c == '#').count();
                                                        let header_txt = trimmed.trim_start_matches('#').trim();
                                                        match level {
                                                             1 => {
                                                                ui.heading(egui::RichText::new(format!("§ {}", header_txt)).color(palette.accent_primary).strong());
                                                            }
                                                            2 => {
                                                                ui.label(egui::RichText::new(format!("§ {}", header_txt)).size(14.0).color(palette.accent_secondary).strong());
                                                            }
                                                            _ => {
                                                                ui.label(egui::RichText::new(format!("§ {}", header_txt)).strong().color(palette.text_primary));
                                                            }
                                                        }
                                                    } else {
                                                        let is_expanded = self.show_role_details.contains(&pl.raw_text);
                                                        let btn_lbl = if is_expanded { "▼ Note" } else { "▶ Note" };
                                                        if ui.small_button(btn_lbl).on_hover_text("Click to expand/collapse note content").clicked() {
                                                            if is_expanded {
                                                                self.show_role_details.remove(&pl.raw_text);
                                                            } else {
                                                                self.show_role_details.insert(pl.raw_text.clone());
                                                            }
                                                        }
                                                        if is_expanded {
                                                            ui.add_space(2.0);
                                                            let note_clean = trimmed.trim_start_matches("//").trim();
                                                            ui.label(egui::RichText::new(note_clean).color(palette.math_comment).italics());
                                                        }
                                                    }
                                                }
                                                LineKind::DomainRestriction { name, rule_summary } => {
                                                    ui.horizontal_wrapped(|ui| {
                                                        let resp = ui.colored_label(
                                                            palette.text_muted,
                                                            format!("✓ {} ∈ {}", name, rule_summary),
                                                        );
                                                        if resp.hovered() {
                                                            new_hovered_line = Some(pl.line_idx);
                                                            self.inspected_symbol = Some(name.clone());
                                                        }
                                                        if let Some(card) = self.state.get_symbol_info_card(name) {
                                                            let _ = resp.on_hover_ui(|ui| render_symbol_info_card_content(ui, &card, &palette, &mut inspector_to_open));
                                                        }
                                                    });
                                                }
                                                LineKind::SetBuilder { var_name, domain_type, .. } => {
                                                    ui.horizontal_wrapped(|ui| {
                                                        let resp = ui.colored_label(palette.text_success, format!("✓ {} ∈ {}", var_name, domain_type));
                                                        if resp.hovered() {
                                                            new_hovered_line = Some(pl.line_idx);
                                                            self.inspected_symbol = Some(var_name.clone());
                                                        }
                                                        if let Some(card) = self.state.get_symbol_info_card(var_name) {
                                                            let _ = resp.on_hover_ui(|ui| render_symbol_info_card_content(ui, &card, &palette, &mut inspector_to_open));
                                                        }
                                                    });
                                                }
                                                LineKind::SliderDef { name, val, unit } => {
                                                    ui.horizontal_wrapped(|ui| {
                                                        let (min_v, max_v, step_v) = if let Some(m) = self.state.session.symbol_metadata.get(name) {
                                                            let step = match m.domain_type.as_str() {
                                                                "EvenIntegers" | "OddIntegers" => 2.0,
                                                                "Integer" | "Integers" | "Naturals" | "Modulo"
                                                                | "ModuloUnits" | "BitVector" | "GaussianIntegers"
                                                                | "EisensteinIntegers" | "GaloisField" | "Boolean" => 1.0,
                                                                _ => 0.05,
                                                            };
                                                            (m.min_val, m.max_val, step)
                                                        } else {
                                                            (-10.0, 10.0, 0.05)
                                                        };
                                                        let mut cur = self.state.session.slider_values.get(name).copied().unwrap_or(*val);
                                                        let mut lbl_resp = ui.colored_label(palette.math_var, format!("{} =", name));
                                                        if let Some(card) = self.state.get_symbol_info_card(name) {
                                                            lbl_resp = lbl_resp.on_hover_ui(|ui| render_symbol_info_card_content(ui, &card, &palette, &mut inspector_to_open));
                                                        }
                                                        if lbl_resp.hovered() {
                                                            new_hovered_line = Some(pl.line_idx);
                                                            self.inspected_symbol = Some(name.clone());
                                                        }

                                                        // Alt+Drag live scrubbing
                                                        if lbl_resp.hovered() && ctx.input(|i| i.modifiers.alt && i.pointer.is_decidedly_dragging()) {
                                                            let delta = ctx.input(|i| i.pointer.delta().x as f64) * 0.05;
                                                            cur = (cur + delta).clamp(min_v, max_v);
                                                            slider_updates.push((name.clone(), cur));
                                                        }

                                                        if ui.add(egui::Slider::new(&mut cur, min_v..=max_v).step_by(step_v).show_value(false)).changed() {
                                                            slider_updates.push((name.clone(), cur));
                                                        }
                                                        let u_str = unit.as_deref().unwrap_or("");
                                                        ui.monospace(format!("{:.2} {}", cur, u_str));
                                                    });
                                                }
                                                LineKind::RoleDeclaration { name, role } => {
                                                    ui.horizontal_wrapped(|ui| {
                                                        let resp = ui.weak(format!("{}: {:?}", name, role));
                                                        if resp.hovered() {
                                                            new_hovered_line = Some(pl.line_idx);
                                                            self.inspected_symbol = Some(name.clone());
                                                        }
                                                        if let Some(card) = self.state.get_symbol_info_card(name) {
                                                            let _ = resp.on_hover_ui(|ui| render_symbol_info_card_content(ui, &card, &palette, &mut inspector_to_open));
                                                        }
                                                    });
                                                }
                                                LineKind::Formula => {
                                                    if let Some(err) = &pl.error_msg {
                                                        ui.colored_label(palette.text_error, format!("⚠ {}", err));
                                                    } else if pl.is_function {
                                                        let line_idx = pl.line_idx;
                                                        let is_collapsed = self.collapsed_plot_lines.contains(&line_idx);
                                                        let line_token = format!("${}", line_idx + 1);
                                                        let cur_scale = self.plot_scale_modes.get(&line_idx).copied().unwrap_or(PlotScaleMode::Linear);
                                                        let mut scale_to_set = None;
                                                        let mut do_pop_out = false;
                                                        let mut toggle_collapse = false;

                                                        // Fold-out header bar
                                                        ui.horizontal_wrapped(|ui| {
                                                            let toggle_lbl = if is_collapsed { "📈 ▶" } else { "📈 ▼" };
                                                            if ui.small_button(toggle_lbl).on_hover_text("Toggle inline plot fold-out").clicked() {
                                                                toggle_collapse = true;
                                                            }
                                                            let display_txt = if !pl.output_unicode.is_empty() {
                                                                &pl.output_unicode
                                                            } else {
                                                                &pl.raw_text
                                                            };
                                                            let scale_suffix = match cur_scale {
                                                                PlotScaleMode::Linear => "",
                                                                PlotScaleMode::SemiLogY => " (Log-Y)",
                                                                PlotScaleMode::SemiLogX => " (Log-X)",
                                                                PlotScaleMode::LogLog => " (Log-Log)",
                                                            };
                                                            let full_lbl = if scale_suffix.is_empty() {
                                                                display_txt.to_string()
                                                            } else {
                                                                format!("{}{}", display_txt, scale_suffix)
                                                            };
                                                            let mut fn_lbl = ui.colored_label(palette.math_result, full_lbl);
                                                            if let Some(card) = self.state.get_symbol_info_card(&line_token) {
                                                                fn_lbl = fn_lbl.on_hover_ui(|ui| render_symbol_info_card_content(ui, &card, &palette, &mut inspector_to_open));
                                                            }
                                                            if fn_lbl.hovered() {
                                                                new_hovered_line = Some(line_idx);
                                                                self.inspected_symbol = Some(line_token.clone());
                                                            }
                                                            fn_lbl.context_menu(|ui| {
                                                                ui.label(egui::RichText::new("📊 Graph Options").strong());
                                                                ui.separator();
                                                                ui.label(egui::RichText::new("Axis Scaling:").weak());
                                                                if ui.selectable_label(cur_scale == PlotScaleMode::Linear, "Linear (Standard)").clicked() {
                                                                    scale_to_set = Some(PlotScaleMode::Linear);
                                                                }
                                                                if ui.selectable_label(cur_scale == PlotScaleMode::SemiLogY, "Semi-log Y (log₁₀ y)").clicked() {
                                                                    scale_to_set = Some(PlotScaleMode::SemiLogY);
                                                                }
                                                                if ui.selectable_label(cur_scale == PlotScaleMode::SemiLogX, "Semi-log X (log₁₀ x)").clicked() {
                                                                    scale_to_set = Some(PlotScaleMode::SemiLogX);
                                                                }
                                                                if ui.selectable_label(cur_scale == PlotScaleMode::LogLog, "Log-Log (log₁₀ x, log₁₀ y)").clicked() {
                                                                    scale_to_set = Some(PlotScaleMode::LogLog);
                                                                }
                                                                ui.separator();
                                                                if ui.button("⤢ Pop Out to Window").clicked() {
                                                                    do_pop_out = true;
                                                                }
                                                                if ui.button(if is_collapsed { "📈 Expand Plot" } else { "📈 Collapse Plot" }).clicked() {
                                                                    toggle_collapse = true;
                                                                }
                                                            });
                                                        });

                                                        // Plot renders directly BELOW the fold-out header in the right panel
                                                        if !is_collapsed {
                                                            ui.add_space(2.0);
                                                            if pl.is_surface_3d {
                                                                if self.state.session.settings.right_panel_show_3d {
                                                                    let lbl_resp = ui.label(egui::RichText::new("🌐 3D Surface Viewport").strong().color(palette.accent_secondary));
                                                                    lbl_resp.context_menu(|ui| {
                                                                        ui.label(egui::RichText::new("🌐 3D Surface Options").strong());
                                                                        ui.separator();
                                                                        if ui.button("⤢ Pop Out to Window").clicked() {
                                                                            do_pop_out = true;
                                                                        }
                                                                        if ui.button("📈 Collapse 3D Viewport").clicked() {
                                                                            toggle_collapse = true;
                                                                        }
                                                                    });
                                                                    crate::ui::Viewport3D::show_with_height(
                                                                        ui,
                                                                        &mut self.viewport_3d_state,
                                                                        None,
                                                                        200.0,
                                                                    );
                                                                }
                                                            } else if self.state.session.settings.right_panel_show_plots {
                                                                let (smart_min, smart_max) = self.state.compute_smart_plot_bounds(&pl.raw_text, "x");
                                                                let mut min_r = smart_min;
                                                                let mut max_r = smart_max;

                                                                let plot_resp = Plot::new(format!("stream_plot_{}", pl.line_idx))
                                                                    .height(130.0)
                                                                    .width((gutter_width - 24.0).max(140.0))
                                                                    .allow_drag(true)
                                                                    .allow_zoom(true)
                                                                    .allow_scroll(false)
                                                                    .include_x(smart_min)
                                                                    .include_x(smart_max)
                                                                    .auto_bounds(egui::Vec2b::new(false, true))
                                                                    .show(ui, |plot_ui| {
                                                                        let scroll_y = plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                                                                        let smooth_y = plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                                                                        let scroll_delta = if scroll_y.abs() > 0.0 { scroll_y } else { smooth_y };
                                                                        if plot_ui.response().hovered() && scroll_delta.abs() > 0.0 {
                                                                            let zoom_factor = (scroll_delta * 0.0025).clamp(-0.5, 0.5).exp();
                                                                            plot_ui.zoom_bounds_around_hovered(egui::Vec2::splat(zoom_factor));
                                                                        }

                                                                        let bounds = plot_ui.plot_bounds();
                                                                        min_r = bounds.min()[0];
                                                                        max_r = bounds.max()[0];
                                                                        if !min_r.is_finite() || !max_r.is_finite() || (max_r - min_r).abs() < 1e-12 {
                                                                            min_r = smart_min;
                                                                            max_r = smart_max;
                                                                        }

                                                                        let pts_data = self.state.cached_evaluate_plot_points(
                                                                            &pl.raw_text, "x", min_r, max_r, 200,
                                                                        );
                                                                        if !pts_data.is_empty() {
                                                                            let y_clamp = (max_r - min_r).abs() * 25.0;
                                                                            let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
                                                                            let mut cur_seg: Vec<[f64; 2]> = Vec::new();

                                                                            for p in pts_data {
                                                                                let mut px = p[0];
                                                                                let mut py = p[1];
                                                                                match cur_scale {
                                                                                    PlotScaleMode::Linear => {}
                                                                                    PlotScaleMode::SemiLogY => {
                                                                                        if py > 0.0 { py = py.log10(); } else { continue; }
                                                                                    }
                                                                                    PlotScaleMode::SemiLogX => {
                                                                                        if px > 0.0 { px = px.log10(); } else { continue; }
                                                                                    }
                                                                                    PlotScaleMode::LogLog => {
                                                                                        if px > 0.0 && py > 0.0 { px = px.log10(); py = py.log10(); } else { continue; }
                                                                                    }
                                                                                }
                                                                                if !px.is_finite() || !py.is_finite() || py.abs() > y_clamp {
                                                                                    if !cur_seg.is_empty() {
                                                                                        segments.push(std::mem::take(&mut cur_seg));
                                                                                    }
                                                                                    continue;
                                                                                }
                                                                                if let Some(prev) = cur_seg.last() {
                                                                                    let dx = (px - prev[0]).abs();
                                                                                    let dy = (py - prev[1]).abs();
                                                                                    if dx > 1e-9 && (dy / dx) > 1e5 && (py * prev[1] < 0.0) {
                                                                                        segments.push(std::mem::take(&mut cur_seg));
                                                                                    }
                                                                                }
                                                                                cur_seg.push([px, py]);
                                                                            }
                                                                            if !cur_seg.is_empty() {
                                                                                segments.push(cur_seg);
                                                                            }

                                                                            for seg in segments {
                                                                                if !seg.is_empty() {
                                                                                    let plot_points: PlotPoints = seg.into_iter().collect();
                                                                                    let line = Line::new("", plot_points)
                                                                                        .color(palette.accent_primary)
                                                                                        .width(2.0_f32);
                                                                                    plot_ui.line(line);
                                                                                }
                                                                            }
                                                                        }
                                                                    });

                                                                let mut resp = plot_resp.response;
                                                                let token_label = line_token.to_string();
                                                                resp.widget_info(move || {
                                                                    egui::WidgetInfo::labeled(
                                                                        egui::WidgetType::Other,
                                                                        true,
                                                                        format!(
                                                                            "Side stream plot of {}. Domain: x in [{:.2}, {:.2}]. Drag to pan, scroll to zoom.",
                                                                            token_label, min_r, max_r
                                                                        ),
                                                                    )
                                                                });
                                                                if resp.hovered() {
                                                                    new_hovered_line = Some(line_idx);
                                                                }
                                                                if let Some(card) = self.state.get_symbol_info_card(&line_token) {
                                                                    resp = resp.on_hover_ui(|ui| render_symbol_info_card_content(ui, &card, &palette, &mut inspector_to_open));
                                                                }
                                                                resp.context_menu(|ui| {
                                                                    ui.label(egui::RichText::new("📊 Graph Options").strong());
                                                                    ui.separator();
                                                                    ui.label(egui::RichText::new("Axis Scaling:").weak());
                                                                    if ui.selectable_label(cur_scale == PlotScaleMode::Linear, "Linear (Standard)").clicked() {
                                                                        scale_to_set = Some(PlotScaleMode::Linear);
                                                                    }
                                                                    if ui.selectable_label(cur_scale == PlotScaleMode::SemiLogY, "Semi-log Y (log₁₀ y)").clicked() {
                                                                        scale_to_set = Some(PlotScaleMode::SemiLogY);
                                                                    }
                                                                    if ui.selectable_label(cur_scale == PlotScaleMode::SemiLogX, "Semi-log X (log₁₀ x)").clicked() {
                                                                        scale_to_set = Some(PlotScaleMode::SemiLogX);
                                                                    }
                                                                    if ui.selectable_label(cur_scale == PlotScaleMode::LogLog, "Log-Log (log₁₀ x, log₁₀ y)").clicked() {
                                                                        scale_to_set = Some(PlotScaleMode::LogLog);
                                                                    }
                                                                    ui.separator();
                                                                    if ui.button("⤢ Pop Out to Window").clicked() {
                                                                        do_pop_out = true;
                                                                    }
                                                                    if ui.button(if is_collapsed { "📈 Expand Plot" } else { "📈 Collapse Plot" }).clicked() {
                                                                        toggle_collapse = true;
                                                                    }
                                                                });
                                                                if resp.dragged() || resp.hovered() {
                                                                    ui.ctx().request_repaint();
                                                                }
                                                            }
                                                        }

                                                        if let Some(s) = scale_to_set {
                                                            self.plot_scale_modes.insert(line_idx, s);
                                                        }
                                                        if do_pop_out {
                                                            plot_to_open = Some(pl.raw_text.trim().to_string());
                                                        }
                                                        if toggle_collapse {
                                                            if is_collapsed {
                                                                self.collapsed_plot_lines.remove(&line_idx);
                                                            } else {
                                                                self.collapsed_plot_lines.insert(line_idx);
                                                            }
                                                        }
                                                    } else if !pl.output_unicode.is_empty() {
                                                        if self.state.session.settings.right_panel_show_solutions {
                                                            ui.horizontal_wrapped(|ui| {
                                                            let mut text = egui::RichText::new(&pl.output_unicode).monospace();
                                                            text = text.color(palette.math_result);
                                                            let mut res_btn = ui.add(egui::Button::new(text).frame(false));
                                                            let line_token = format!("${}", pl.line_idx + 1);
                                                            if let Some(card) = self.state.get_symbol_info_card(&line_token) {
                                                                res_btn = res_btn.on_hover_ui(|ui| render_symbol_info_card_content(ui, &card, &palette, &mut inspector_to_open));
                                                            }
                                                            if res_btn.clicked() {
                                                                self.state.session.raw_document_text.push_str(&format!(" ${}", pl.line_idx + 1));
                                                                self.is_edit_dirty = true;
                                                                self.last_keystroke_time = web_time::Instant::now();
                                                                state_changed = true;
                                                            }
                                                            if res_btn.hovered() {
                                                                new_hovered_line = Some(pl.line_idx);
                                                                self.inspected_symbol = Some(line_token);
                                                            }

                                                            if let Some(unit) = &pl.physical_unit {
                                                                if !pl.output_unicode.contains(&format!("[{}]", unit)) {
                                                                    ui.weak(format!("[{}]", unit));
                                                                }
                                                            }

                                                            // Inbound reference indicators & quick auto-naming
                                                            if self.state.session.settings.right_panel_show_inbound_refs && !referencing_lines.is_empty() {
                                                                let ref_str = referencing_lines.iter().map(|n| format!("L{}", n)).collect::<Vec<_>>().join(",");
                                                                ui.colored_label(palette.accent_secondary, format!("↖{}", ref_str))
                                                                    .on_hover_text(format!("Referenced by line(s) {}", ref_str));

                                                                if !pl.raw_text.contains('=')
                                                                    && ui.small_button("💡 Name").on_hover_text("Convert this result into a named variable across document").clicked() {
                                                                    refactor_target = Some((pl.line_idx, format!("val_{}", pl.line_idx + 1)));
                                                                }
                                                            }
                                                        });
                                                        }
                                                    } else {
                                                        ui.weak("…");
                                                    }
                                                }
                                                LineKind::CadMesh { model_kind, summary } => {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(format!("⚙ {}", model_kind))
                                                                .strong()
                                                                .color(palette.accent_primary),
                                                        );
                                                        ui.weak(summary);
                                                    });
                                                    if self.state.session.settings.right_panel_show_cad {
                                                        crate::ui::Viewport3D::show_with_height(
                                                            ui,
                                                            &mut self.viewport_3d_state,
                                                            None,
                                                            200.0,
                                                        );
                                                    }
                                                }
                                                LineKind::Plot3D { expr_str, x_var, y_var } => {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(format!("📊 3D: z = f({}, {})", x_var, y_var))
                                                                .strong()
                                                                .color(palette.accent_secondary),
                                                        );
                                                        ui.monospace(expr_str);
                                                    });
                                                    if self.state.session.settings.right_panel_show_3d {
                                                        crate::ui::Viewport3D::show_with_height(
                                                            ui,
                                                            &mut self.viewport_3d_state,
                                                            None,
                                                            220.0,
                                                        );
                                                    }
                                                }
                                            }
                                        });
                                    });

                                    self.line_screen_positions.insert(pl.line_idx, line_resp.response.rect.left_bottom());
                                    if line_resp.response.hovered() {
                                        new_hovered_line = Some(pl.line_idx);
                                        self.inspected_symbol = Some(line_token.clone());
                                    }
                                }
                            },
                        );
                    } else {
                        ui.separator();
                        if ui
                            .button(egui::RichText::new("◀ Results").size(12.0).color(palette.text_muted))
                            .on_hover_text("Show right results stream (Ctrl+J)")
                            .clicked()
                        {
                            self.show_right_sidebar = true;
                        }
                    }
                });
            });

        if let Some(to_open) = inspector_to_open {
            self.open_or_focus_inspector(&to_open);
        }
        self.hovered_line_idx = new_hovered_line;

        if let Some((target_idx, var_name)) = refactor_target {
            self.state
                .refactor_line_to_named_variable(target_idx, &var_name);
            state_changed = true;
        }

        // 4. Inline Slash Command Menu (Phase D)
        let slash_line_info = self
            .state
            .session
            .raw_document_text
            .lines()
            .enumerate()
            .find(|(_, l)| l.trim().starts_with('/') && !l.trim().starts_with("//"));

        if let Some((line_idx, line_str)) = slash_line_info {
            let query = line_str.trim().trim_start_matches('/').to_lowercase();
            let all_cmds = crate::ui::CommandPalette::all_commands();
            let matches: Vec<_> = all_cmds
                .into_iter()
                .filter(|cmd| {
                    query.is_empty()
                        || cmd.title.to_lowercase().contains(&query)
                        || cmd
                            .shortcut
                            .map(|s| s.to_lowercase().contains(&query))
                            .unwrap_or(false)
                        || cmd.syntax_template.to_lowercase().contains(&query)
                })
                .take(6)
                .collect();

            if !matches.is_empty() {
                egui::Window::new("✨ Insert Math Command")
                    .anchor(
                        egui::Align2::RIGHT_TOP,
                        egui::vec2(-20.0, 80.0 + (line_idx as f32 * 18.0).min(300.0)),
                    )
                    .collapsible(false)
                    .resizable(false)
                    .default_size([320.0, 320.0])
                    .show(&ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(palette.accent_primary, "Commands matching:");
                            ui.monospace(format!("/{}", query));
                        });
                        ui.separator();
                        for cmd in matches {
                            if ui
                                .button(format!("{}: {}", cmd.category, cmd.title))
                                .on_hover_text(cmd.description)
                                .clicked()
                            {
                                let lines: Vec<&str> =
                                    self.state.session.raw_document_text.lines().collect();
                                let mut new_lines = Vec::new();
                                for (i, l) in lines.into_iter().enumerate() {
                                    if i == line_idx {
                                        new_lines.push(cmd.syntax_template.to_string());
                                    } else {
                                        new_lines.push(l.to_string());
                                    }
                                }
                                self.state.session.raw_document_text = new_lines.join("\n");
                                state_changed = true;
                            }
                        }
                    });
            }
        }

        for (sym, val) in slider_updates {
            self.state.session.slider_values.insert(sym.clone(), val);
            if let Some(meta) = self.state.session.symbol_metadata.get_mut(&sym) {
                meta.cur_val = val;
            }
            state_changed = true;
        }

        if let Some(expr_key) = plot_to_open {
            self.state
                .set_card_mode(&expr_key, CardDisplayMode::PoppedOut);
            state_changed = true;
        }

        state_changed
    }

    /// Render Interactive Matrix & Tensor Grid Builder Modal
    fn render_matrix_builder_modal(&mut self, ctx: &egui::Context) {
        if self.show_matrix_builder {
            if self.builder_snapshot.is_none() {
                self.builder_snapshot = Some(self.state.session.raw_document_text.clone());
            }
            let mut is_open = self.show_matrix_builder;
            egui::Window::new("Matrix & Tensor Grid Builder")
                .open(&mut is_open)
                .default_size([440.0, 360.0])
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.subheading("Matrix Dimensions & Presets");
                    ui.horizontal(|ui| {
                        ui.label("Rows:");
                        ui.add(egui::Slider::new(&mut self.matrix_builder_rows, 1..=6));
                        ui.label("Cols:");
                        ui.add(egui::Slider::new(&mut self.matrix_builder_cols, 1..=6));
                    });

                    // Resize matrix grid elements if rows/cols changed
                    while self.matrix_builder_elements.len() < self.matrix_builder_rows {
                        let mut new_row = Vec::new();
                        for _ in 0..self.matrix_builder_cols {
                            new_row.push("0".to_string());
                        }
                        self.matrix_builder_elements.push(new_row);
                    }
                    self.matrix_builder_elements
                        .truncate(self.matrix_builder_rows);
                    for r in &mut self.matrix_builder_elements {
                        while r.len() < self.matrix_builder_cols {
                            r.push("0".to_string());
                        }
                        r.truncate(self.matrix_builder_cols);
                    }

                    ui.horizontal(|ui| {
                        ui.label("Preset:");
                        egui::ComboBox::from_id_salt("matrix_preset_combo")
                            .selected_text(format!("{:?}", self.matrix_builder_preset))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::Custom,
                                    "Custom Grid",
                                );
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::Identity,
                                    "Identity Matrix",
                                );
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::Zero,
                                    "Zero Matrix",
                                );
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::Diagonal,
                                    "Diagonal Matrix",
                                );
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::Symmetric,
                                    "Symmetric Matrix",
                                );
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::PauliX,
                                    "Pauli σ_x",
                                );
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::PauliY,
                                    "Pauli σ_y",
                                );
                                ui.selectable_value(
                                    &mut self.matrix_builder_preset,
                                    MatrixPresetKind::PauliZ,
                                    "Pauli σ_z",
                                );
                            });
                    });

                    if self.matrix_builder_preset == MatrixPresetKind::Custom {
                        ui.separator();
                        ui.subheading("Matrix Grid Elements");
                        for r in 0..self.matrix_builder_rows {
                            ui.horizontal(|ui| {
                                for c in 0..self.matrix_builder_cols {
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.matrix_builder_elements[r][c],
                                        )
                                        .desired_width(45.0),
                                    );
                                }
                            });
                        }
                    }

                    ui.separator();
                    let syntax = generate_matrix_syntax(
                        self.matrix_builder_rows,
                        self.matrix_builder_cols,
                        self.matrix_builder_preset,
                        Some(&self.matrix_builder_elements),
                    );

                    ui.label("Live Syntax Preview:");
                    ui.monospace(
                        egui::RichText::new(&syntax)
                            .strong()
                            .color(egui::Color32::from_rgb(120, 200, 255)),
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Insert at Active Line").clicked() {
                            self.state.insert_text_at_active_line(&syntax);
                            self.builder_snapshot = None;
                            self.show_matrix_builder = false;
                        }
                        if ui.button("Cancel").clicked() {
                            if let Some(orig) = self.builder_snapshot.take() {
                                self.state.session.raw_document_text = orig;
                                self.state.evaluate_all();
                            }
                            self.show_matrix_builder = false;
                        }
                    });
                });
            self.show_matrix_builder = is_open;
        }
    }

    /// Render Interactive Parameter & Variable Builder Modal
    fn render_domain_picker_modal(&mut self, ctx: &egui::Context) {
        if self.show_domain_picker {
            if self.builder_snapshot.is_none() {
                self.builder_snapshot = Some(self.state.session.raw_document_text.clone());
            }
            let mut is_open = self.show_domain_picker;
            egui::Window::new("🎛 Parameter & Variable Builder")
                .open(&mut is_open)
                .default_size([520.0, 560.0])
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.subheading("Symbol Declaration & Number System Specification");

                    // 1. Symbol Name and Role
                    ui.horizontal(|ui| {
                        ui.label("Symbol Name:");
                        ui.text_edit_singleline(&mut self.param_builder_var);

                        ui.separator();
                        ui.label("Role:");
                        if ui.selectable_label(!self.param_builder_is_param, "Variable").clicked() {
                            self.param_builder_is_param = false;
                        }
                        if ui.selectable_label(self.param_builder_is_param, "Parameter (Slider)").clicked() {
                            self.param_builder_is_param = true;
                        }
                    });

                    ui.add_space(4.0);
                    ui.separator();

                    // 2. Universal Number System Categories
                    ui.subheading("Number System & Algebraic Category");
                    ui.horizontal_wrapped(|ui| {
                        if ui.selectable_label(self.param_builder_category == 0, "Standard (ℝ, ℤ, ℂ)").clicked() {
                            self.param_builder_category = 0;
                        }
                        if ui.selectable_label(self.param_builder_category == 1, "Cayley-Dickson (ℝ..𝕊)").clicked() {
                            self.param_builder_category = 1;
                        }
                        if ui.selectable_label(self.param_builder_category == 2, "Adjoined (i, ε, j, eₖ)").clicked() {
                            self.param_builder_category = 2;
                        }
                        if ui.selectable_label(self.param_builder_category == 3, "Non-Archimedean (ℚₚ, 𝐍𝐨, 𝔸)").clicked() {
                            self.param_builder_category = 3;
                        }
                        if ui.selectable_label(self.param_builder_category == 4, "Modular & Galois (ℤₙ, GF)").clicked() {
                            self.param_builder_category = 4;
                        }
                        if ui.selectable_label(self.param_builder_category == 5, "Matrix & Tensor (Mₘₓₙ, Tʳ)").clicked() {
                            self.param_builder_category = 5;
                        }
                    });

                    ui.add_space(4.0);

                    // 3. Category-specific configuration panels
                    match self.param_builder_category {
                        0 => {
                            // Standard & Discrete Continuum
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Classical Number Continuum & Discrete Subsets:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    let options = [
                                        ("ℝ Reals", 0),
                                        ("ℝ⁺ Positive", 1),
                                        ("ℝ⁺₀ NonNeg", 2),
                                        ("ℤ Integers", 3),
                                        ("ℕ Naturals", 4),
                                        ("ℚ Rationals", 5),
                                        ("ℂ Complex", 6),
                                    ];
                                    for (lbl, idx) in options {
                                        if ui.selectable_label(self.param_builder_standard_kind == idx, lbl).clicked() {
                                            self.param_builder_standard_kind = idx;
                                        }
                                    }
                                });

                                let notes = match self.param_builder_standard_kind {
                                    0 => "Real number line ℝ: Complete ordered Archimedean field",
                                    1 => "Positive reals ℝ⁺: (0, ∞) strictly greater than zero",
                                    2 => "Non-negative reals ℝ⁺₀: [0, ∞) including zero",
                                    3 => "Integers ℤ: Integral domain with unit elements {-1, 1}",
                                    4 => "Natural numbers ℕ: Non-negative counting numbers {0, 1, 2, ...}",
                                    5 => "Rationals ℚ: Exact fraction quotient field p/q",
                                    _ => "Complex field ℂ: Algebraically closed 2D plane with i² = -1",
                                };
                                ui.weak(notes);
                            });
                        }
                        1 => {
                            // Cayley-Dickson Algebra Hierarchy
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Cayley-Dickson Algebra Family:").strong());
                                ui.horizontal(|ui| {
                                    ui.label("Integer Depth (0..5):");
                                    ui.add(egui::Slider::new(&mut self.param_builder_cayley_depth, 0..=5).text("Depth"));
                                });

                                let (name, dim, notes) = match self.param_builder_cayley_depth {
                                    0 => ("ℝ (Reals)", 1, "Commutative, Associative, Ordered Field"),
                                    1 => ("ℂ (Complex)", 2, "Commutative, Associative, Algebraically Closed"),
                                    2 => ("ℍ (Quaternions)", 4, "Non-Commutative Division Ring (3D Spatial Rotations)"),
                                    3 => ("𝕆 (Octonions)", 8, "Non-Associative Alternative Division Algebra"),
                                    4 => ("𝕊 (Sedenions)", 16, "Non-Alternative, Non-Division, Contains Zero Divisors"),
                                    _ => ("𝕋 (Trigintaduonions)", 32, "32-Dimensional Hypercomplex Algebra"),
                                };

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("Class: {} (Dimension: {})", name, dim)).strong());
                                });
                                ui.weak(format!("Properties: {}", notes));
                            });
                        }
                        2 => {
                            // Adjoined Algebras & Generators
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Adjoin Algebraic Units to Base Field ℝ:").strong());
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.param_builder_adjoin_i, "i (Imaginary: i² = -1)");
                                    ui.checkbox(&mut self.param_builder_adjoin_eps, "ε (Dual: ε² = 0, AutoDiff)");
                                });
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.param_builder_adjoin_j, "j (Split-Complex: j² = +1)");
                                    ui.checkbox(&mut self.param_builder_adjoin_clifford, "e₁, e₂ (Grassmann/Clifford)");
                                });

                                let derived_algebra = match (
                                    self.param_builder_adjoin_i,
                                    self.param_builder_adjoin_eps,
                                    self.param_builder_adjoin_j,
                                    self.param_builder_adjoin_clifford,
                                ) {
                                    (true, false, false, false) => "ℂ (Complex Field, dim 2)",
                                    (false, true, false, false) => "𝔻 (Dual Numbers for Forward AutoDiff, dim 2)",
                                    (true, true, false, false) => "Dual-Complex (Kinematics & Screw Theory, dim 4)",
                                    (false, false, true, false) => "ℝ[j] (Split-Complex / Hyperbolic Numbers, dim 2)",
                                    (false, false, false, true) => "Clifford Cl(2, 0) Multivector Geometric Algebra",
                                    (true, false, true, false) => "Bicomplex Commutative Quaternions",
                                    _ => "ℝ (Real Base Field, dim 1)",
                                };

                                ui.weak(format!("Resulting Algebra: {}", derived_algebra));
                            });
                        }
                        3 => {
                            // Non-Archimedean & Infinitesimals
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Non-Archimedean & Infinitesimal Fields:").strong());
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.param_builder_standard_kind == 0, "p-Adic ℚₚ").clicked() {
                                        self.param_builder_standard_kind = 0;
                                    }
                                    if ui.selectable_label(self.param_builder_standard_kind == 1, "Surreal 𝐍𝐨").clicked() {
                                        self.param_builder_standard_kind = 1;
                                    }
                                    if ui.selectable_label(self.param_builder_standard_kind == 2, "Adeles 𝔸").clicked() {
                                        self.param_builder_standard_kind = 2;
                                    }
                                });

                                match self.param_builder_standard_kind {
                                    0 => {
                                        ui.horizontal(|ui| {
                                            ui.label("Prime p:");
                                            ui.add(egui::DragValue::new(&mut self.param_builder_padic_prime).range(2..=997));
                                            ui.separator();
                                            ui.label("Valuation Bound vₚ(x) ≥:");
                                            ui.add(egui::DragValue::new(&mut self.param_builder_padic_valuation).range(-10..=10));
                                        });
                                        ui.weak(format!("Ultrametric field ℚ_{} with non-Archimedean norm |x|_{} = {}^{{-v(x)}}", self.param_builder_padic_prime, self.param_builder_padic_prime, self.param_builder_padic_prime));
                                    }
                                    1 => {
                                        ui.horizontal(|ui| {
                                            ui.label("Generation / Birth Day Depth ≤:");
                                            ui.add(egui::Slider::new(&mut self.param_builder_surreal_generation, 0..=10));
                                        });
                                        ui.weak("Conway Surreal numbers: contains infinitesimals ε and transfinite ordinals ω");
                                    }
                                    _ => {
                                        ui.weak("Topological Adele Ring 𝔸: restricted product of ℝ and all p-adic completions ℚₚ");
                                    }
                                }
                            });
                        }
                        4 => {
                            // Discrete Number Systems & Modulo Arithmetic
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Discrete Number Systems & Modulo Arithmetic:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    let options = [
                                        ("Modulo (ℤ/nℤ)", 0),
                                        ("Integers & Step (ℤ, kℤ)", 1),
                                        ("Galois Field (GF)", 2),
                                        ("Lattices (ℤ[i], ℤ[ω])", 3),
                                        ("Boolean & Words (𝔹)", 4),
                                    ];
                                    for (lbl, idx) in options {
                                        if ui.selectable_label(self.param_builder_discrete_kind == idx, lbl).clicked() {
                                            self.param_builder_discrete_kind = idx;
                                        }
                                    }
                                });

                                match self.param_builder_discrete_kind {
                                    0 => {
                                        // Modulo arithmetic
                                        ui.horizontal(|ui| {
                                            ui.label("Modulus n:");
                                            ui.add(egui::DragValue::new(&mut self.param_builder_modulo_n).range(2..=65536));
                                            ui.separator();
                                            ui.label("Representation:");
                                            if ui.selectable_label(self.param_builder_modulo_rep == 0, "Canonical [0, n-1]").clicked() {
                                                self.param_builder_modulo_rep = 0;
                                            }
                                            if ui.selectable_label(self.param_builder_modulo_rep == 1, "Balanced [-n/2, n/2]").clicked() {
                                                self.param_builder_modulo_rep = 1;
                                            }
                                            if ui.selectable_label(self.param_builder_modulo_rep == 2, "Units (ℤ/nℤ)ˣ").clicked() {
                                                self.param_builder_modulo_rep = 2;
                                            }
                                        });

                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut self.param_builder_has_congruence, "Congruence Condition:");
                                            if self.param_builder_has_congruence {
                                                ui.label("x ≡");
                                                ui.add(egui::DragValue::new(&mut self.param_builder_congruence_rem));
                                                ui.label("(mod");
                                                ui.add(egui::DragValue::new(&mut self.param_builder_congruence_mod).range(2..=65536));
                                                ui.label(")");
                                            }
                                        });
                                        ui.weak(format!("Residue ring ℤ/{}ℤ with modular addition and multiplication", self.param_builder_modulo_n));
                                    }
                                    1 => {
                                        // Integers & Progressions
                                        ui.horizontal(|ui| {
                                            ui.label("Step Size Δk:");
                                            ui.add(egui::DragValue::new(&mut self.param_builder_integer_step).range(1..=1000));
                                            ui.separator();
                                            ui.label("Parity / Divisibility:");
                                            if ui.selectable_label(self.param_builder_integer_parity == 0, "All ℤ").clicked() {
                                                self.param_builder_integer_parity = 0;
                                            }
                                            if ui.selectable_label(self.param_builder_integer_parity == 1, "Even (2ℤ)").clicked() {
                                                self.param_builder_integer_parity = 1;
                                            }
                                            if ui.selectable_label(self.param_builder_integer_parity == 2, "Odd (2ℤ+1)").clicked() {
                                                self.param_builder_integer_parity = 2;
                                            }
                                            if ui.selectable_label(self.param_builder_integer_parity == 3, "kℤ").clicked() {
                                                self.param_builder_integer_parity = 3;
                                            }
                                        });

                                        if self.param_builder_integer_parity == 3 {
                                            ui.horizontal(|ui| {
                                                ui.label("Multiple Factor k:");
                                                ui.add(egui::DragValue::new(&mut self.param_builder_integer_multiple).range(2..=1000));
                                            });
                                        }
                                        ui.weak("Discrete integral domain ℤ with discrete step arithmetic");
                                    }
                                    2 => {
                                        // Galois Field
                                        ui.horizontal(|ui| {
                                            ui.label("Prime Characteristic p:");
                                            ui.add(egui::DragValue::new(&mut self.param_builder_galois_prime).range(2..=257));
                                            ui.label("Power k:");
                                            ui.add(egui::DragValue::new(&mut self.param_builder_galois_power).range(1..=16));
                                        });
                                        let order = self.param_builder_galois_prime.saturating_pow(self.param_builder_galois_power);
                                        ui.weak(format!("Galois field GF({}^{}) of order q = {}", self.param_builder_galois_prime, self.param_builder_galois_power, order));
                                    }
                                    3 => {
                                        // Complex Lattices (Gaussian / Eisenstein)
                                        ui.horizontal(|ui| {
                                            ui.label("Lattice Type:");
                                            if ui.selectable_label(self.param_builder_lattice_kind == 0, "Gaussian Integers ℤ[i]").clicked() {
                                                self.param_builder_lattice_kind = 0;
                                            }
                                            if ui.selectable_label(self.param_builder_lattice_kind == 1, "Eisenstein Integers ℤ[ω]").clicked() {
                                                self.param_builder_lattice_kind = 1;
                                            }
                                        });
                                        if self.param_builder_lattice_kind == 0 {
                                            ui.weak("Square lattice ℤ[i] = { a + bi | a,b ∈ ℤ } with norm N(z) = a² + b²");
                                        } else {
                                            ui.weak("Triangular lattice ℤ[ω] = { a + bω | a,b ∈ ℤ, ω = e^(2πi/3) } with norm N(z) = a² - ab + b²");
                                        }
                                    }
                                    _ => {
                                        // Boolean & Bit-Vectors
                                        ui.horizontal(|ui| {
                                            ui.label("Bit Width:");
                                            let widths = [1, 8, 16, 32, 64];
                                            for w in widths {
                                                if ui.selectable_label(self.param_builder_bit_width == w, format!("{}-bit", w)).clicked() {
                                                    self.param_builder_bit_width = w;
                                                }
                                            }
                                            if self.param_builder_bit_width > 1 {
                                                ui.checkbox(&mut self.param_builder_bit_signed, "Signed (Two's Complement)");
                                            }
                                        });
                                        if self.param_builder_bit_width == 1 {
                                            ui.weak("Boolean 2-element lattice 𝔹 = {0, 1}");
                                        } else {
                                            ui.weak(format!("Word domain 𝔹^{} with bitwise logic and modular wrapping arithmetic", self.param_builder_bit_width));
                                        }
                                    }
                                }
                            });
                        }
                        _ => {
                            // Matrix & Tensor Domains
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Matrix & Multilinear Tensor Domains:").strong());
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.param_builder_standard_kind == 0, "Matrix Space Mₘₓₙ(D)").clicked() {
                                        self.param_builder_standard_kind = 0;
                                    }
                                    if ui.selectable_label(self.param_builder_standard_kind == 1, "Tensor Algebra Tʳ(D)").clicked() {
                                        self.param_builder_standard_kind = 1;
                                    }
                                });

                                if self.param_builder_standard_kind == 0 {
                                    ui.horizontal(|ui| {
                                        ui.label("Rows m:");
                                        ui.add(egui::DragValue::new(&mut self.param_builder_matrix_rows).range(1..=16));
                                        ui.label("Cols n:");
                                        ui.add(egui::DragValue::new(&mut self.param_builder_matrix_cols).range(1..=16));
                                        ui.label("Element Domain:");
                                        ui.text_edit_singleline(&mut self.param_builder_matrix_domain);
                                    });
                                    ui.weak(format!("Matrix ring M_{{ {} x {} }}({})", self.param_builder_matrix_rows, self.param_builder_matrix_cols, self.param_builder_matrix_domain));
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label("Tensor Rank r:");
                                        ui.add(egui::Slider::new(&mut self.param_builder_tensor_rank, 1..=8));
                                        ui.label("Base Domain:");
                                        ui.text_edit_singleline(&mut self.param_builder_matrix_domain);
                                    });
                                    ui.weak(format!("Tensor algebra of rank {} over {}", self.param_builder_tensor_rank, self.param_builder_matrix_domain));
                                }
                            });
                        }
                    }

                    ui.add_space(4.0);

                    // 4. Coordinate-Relative Symbolic Bounds
                    ui.subheading("Coordinate-Relative Interval & Symbolic Bounds");
                    ui.group(|ui| {
                        // Ensure coords vector has at least 4 items
                        while self.param_builder_coords.len() < 4 {
                            self.param_builder_coords.push(("Coord".to_string(), -5.0, 5.0, false));
                        }

                        // Configure coordinate labels according to active category
                        let active_count = match self.param_builder_category {
                            0 => {
                                if self.param_builder_standard_kind == 6 {
                                    self.param_builder_coords[0].0 = "Re".to_string();
                                    self.param_builder_coords[1].0 = "Im".to_string();
                                    2
                                } else {
                                    self.param_builder_coords[0].0 = "Bound".to_string();
                                    1
                                }
                            }
                            1 => {
                                match self.param_builder_cayley_depth {
                                    0 => {
                                        self.param_builder_coords[0].0 = "x0".to_string();
                                        1
                                    }
                                    1 => {
                                        self.param_builder_coords[0].0 = "Re".to_string();
                                        self.param_builder_coords[1].0 = "Im".to_string();
                                        2
                                    }
                                    2 => {
                                        self.param_builder_coords[0].0 = "Scalar".to_string();
                                        self.param_builder_coords[1].0 = "Vector".to_string();
                                        self.param_builder_coords[2].0 = "j_part".to_string();
                                        self.param_builder_coords[3].0 = "k_part".to_string();
                                        4
                                    }
                                    _ => {
                                        self.param_builder_coords[0].0 = "Scalar".to_string();
                                        self.param_builder_coords[1].0 = "NonScalar".to_string();
                                        2
                                    }
                                }
                            }
                            2 => {
                                self.param_builder_coords[0].0 = "Re".to_string();
                                self.param_builder_coords[1].0 = if self.param_builder_adjoin_eps {
                                    "Dual".to_string()
                                } else if self.param_builder_adjoin_j {
                                    "Split".to_string()
                                } else {
                                    "Im".to_string()
                                };
                                self.param_builder_coords[2].0 = "Part2".to_string();
                                self.param_builder_coords[3].0 = "Part3".to_string();
                                if self.param_builder_adjoin_i && self.param_builder_adjoin_eps { 3 }
                                else if self.param_builder_adjoin_i || self.param_builder_adjoin_eps || self.param_builder_adjoin_j { 2 }
                                else { 1 }
                            }
                            3 => {
                                self.param_builder_coords[0].0 = "Interval".to_string();
                                1
                            }
                            4 => {
                                if self.param_builder_discrete_kind == 3 {
                                    self.param_builder_coords[0].0 = "Re".to_string();
                                    self.param_builder_coords[1].0 = "Im".to_string();
                                    2
                                } else {
                                    self.param_builder_coords[0].0 = "Integer Range".to_string();
                                    1
                                }
                            }
                            _ => {
                                self.param_builder_coords[0].0 = "Elements".to_string();
                                1
                            }
                        };

                        for i in 0..active_count.min(self.param_builder_coords.len()) {
                            let (label, min_v, max_v, enabled) = &mut self.param_builder_coords[i];
                            ui.horizontal(|ui| {
                                ui.checkbox(enabled, format!("{}:", label));
                                if *enabled {
                                    ui.label("Range [");
                                    ui.add(egui::DragValue::new(min_v).speed(0.1));
                                    ui.label(",");
                                    ui.add(egui::DragValue::new(max_v).speed(0.1));
                                    ui.label("]");
                                }
                            });
                        }
                    });

                    ui.separator();

                    // 5. Live Universal Syntax Preview
                    let params = ParameterBuilderParams {
                        var: &self.param_builder_var,
                        is_param: self.param_builder_is_param,
                        category: self.param_builder_category,
                        standard_kind: self.param_builder_standard_kind,
                        cayley_depth: self.param_builder_cayley_depth,
                        adjoin_i: self.param_builder_adjoin_i,
                        adjoin_eps: self.param_builder_adjoin_eps,
                        adjoin_j: self.param_builder_adjoin_j,
                        adjoin_clifford: self.param_builder_adjoin_clifford,
                        padic_prime: self.param_builder_padic_prime,
                        padic_valuation: self.param_builder_padic_valuation,
                        surreal_generation: self.param_builder_surreal_generation,
                        modulo_n: self.param_builder_modulo_n,
                        galois_prime: self.param_builder_galois_prime,
                        galois_power: self.param_builder_galois_power,
                        matrix_rows: self.param_builder_matrix_rows,
                        matrix_cols: self.param_builder_matrix_cols,
                        matrix_domain: &self.param_builder_matrix_domain,
                        tensor_rank: self.param_builder_tensor_rank,
                        discrete_kind: self.param_builder_discrete_kind,
                        modulo_rep: self.param_builder_modulo_rep,
                        congruence_rem: self.param_builder_congruence_rem,
                        congruence_mod: self.param_builder_congruence_mod,
                        has_congruence: self.param_builder_has_congruence,
                        integer_step: self.param_builder_integer_step,
                        integer_parity: self.param_builder_integer_parity,
                        integer_multiple: self.param_builder_integer_multiple,
                        lattice_kind: self.param_builder_lattice_kind,
                        bit_width: self.param_builder_bit_width,
                        bit_signed: self.param_builder_bit_signed,
                        coords: &self.param_builder_coords,
                    };
                    let syntax = generate_universal_parameter_builder_syntax(&params);

                    ui.label("Live Syntax Preview:");
                    ui.monospace(
                        egui::RichText::new(&syntax)
                            .strong()
                            .color(egui::Color32::from_rgb(180, 140, 255)),
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("➕ Insert at Active Line").clicked() {
                            self.state.insert_text_at_active_line(&syntax);
                            self.builder_snapshot = None;
                            self.show_domain_picker = false;
                        }
                        if ui.button("Cancel").clicked() {
                            if let Some(orig) = self.builder_snapshot.take() {
                                self.state.session.raw_document_text = orig;
                                self.state.evaluate_all();
                            }
                            self.show_domain_picker = false;
                        }
                    });
                });
            self.show_domain_picker = is_open;
        }
    }

    /// Render Riemann Branch Cut Configurator Modal
    fn render_branch_cut_modal(&mut self, ctx: &egui::Context) {
        if self.show_branch_cut_config {
            let mut is_open = self.show_branch_cut_config;
            egui::Window::new("Riemann Branch Cut Configurator")
                .open(&mut is_open)
                .default_size([420.0, 260.0])
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.subheading("Multi-Valued Function & Cut Geometry");
                    ui.horizontal(|ui| {
                        ui.label("Function:");
                        egui::ComboBox::from_id_salt("branch_fn_combo")
                            .selected_text(&self.branch_cut_fn)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.branch_cut_fn,
                                    "log".to_string(),
                                    "log(z)",
                                );
                                ui.selectable_value(
                                    &mut self.branch_cut_fn,
                                    "sqrt".to_string(),
                                    "sqrt(z)",
                                );
                                ui.selectable_value(
                                    &mut self.branch_cut_fn,
                                    "asin".to_string(),
                                    "arcsin(z)",
                                );
                                ui.selectable_value(
                                    &mut self.branch_cut_fn,
                                    "atan".to_string(),
                                    "arctan(z)",
                                );
                                ui.selectable_value(
                                    &mut self.branch_cut_fn,
                                    "power".to_string(),
                                    "z^a (general power)",
                                );
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Riemann Sheet Index (k ∈ ℤ):");
                        ui.add(egui::Slider::new(&mut self.branch_cut_sheet, -5..=5));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Branch Cut Position:");
                        egui::ComboBox::from_id_salt("branch_cut_pos_combo")
                            .selected_text(&self.branch_cut_pos)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.branch_cut_pos,
                                    "(-infinity, 0]".to_string(),
                                    "Negative Real Axis: (-∞, 0]",
                                );
                                ui.selectable_value(
                                    &mut self.branch_cut_pos,
                                    "[1, +infinity)".to_string(),
                                    "Positive Real Axis: [1, +∞)",
                                );
                                ui.selectable_value(
                                    &mut self.branch_cut_pos,
                                    "[-1, 1]".to_string(),
                                    "Unit Segment: [-1, 1]",
                                );
                                ui.selectable_value(
                                    &mut self.branch_cut_pos,
                                    "(-i*infinity, -i] U [i, +i*infinity)".to_string(),
                                    "Imaginary Axis Slits",
                                );
                            });
                    });

                    ui.separator();
                    let syntax = generate_branch_cut_syntax(
                        &self.branch_cut_fn,
                        self.branch_cut_sheet,
                        &self.branch_cut_pos,
                    );

                    ui.label("Live Syntax Preview:");
                    ui.monospace(
                        egui::RichText::new(&syntax)
                            .strong()
                            .color(egui::Color32::from_rgb(140, 220, 180)),
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Insert at Active Line").clicked() {
                            self.state.insert_text_at_active_line(&syntax);
                            self.show_branch_cut_config = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_branch_cut_config = false;
                        }
                    });
                });
            self.show_branch_cut_config = is_open;
        }
    }

    /// Render ODE/PDE Boundary Condition Wizard Modal
    fn render_ode_wizard_modal(&mut self, ctx: &egui::Context) {
        if self.show_ode_wizard {
            if self.builder_snapshot.is_none() {
                self.builder_snapshot = Some(self.state.session.raw_document_text.clone());
            }
            let mut is_open = self.show_ode_wizard;
            egui::Window::new("ODE/PDE Boundary Wizard")
                .open(&mut is_open)
                .default_size([420.0, 260.0])
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.subheading("Initial & Boundary Conditions");
                    ui.horizontal(|ui| {
                        ui.label("Dependent Var:");
                        ui.text_edit_singleline(&mut self.ode_var);
                        ui.label("Independent Var:");
                        ui.text_edit_singleline(&mut self.ode_indep);
                    });

                    ui.horizontal(|ui| {
                        ui.label(format!("{}(0) =", self.ode_var));
                        ui.add(egui::DragValue::new(&mut self.ode_y0).speed(0.1));
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.ode_has_deriv,
                            format!("Include {}'(0)", self.ode_var),
                        );
                        if self.ode_has_deriv {
                            let mut dy = self.ode_dy0.unwrap_or(0.0);
                            if ui.add(egui::DragValue::new(&mut dy).speed(0.1)).changed() {
                                self.ode_dy0 = Some(dy);
                            }
                        }
                    });

                    ui.separator();
                    let syntax = generate_ode_bc_syntax(
                        &self.ode_var,
                        &self.ode_indep,
                        self.ode_y0,
                        if self.ode_has_deriv {
                            self.ode_dy0
                        } else {
                            None
                        },
                    );

                    ui.label("Live Syntax Preview:");
                    ui.monospace(
                        egui::RichText::new(&syntax)
                            .strong()
                            .color(egui::Color32::from_rgb(240, 180, 100)),
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Insert at Active Line").clicked() {
                            self.state.insert_text_at_active_line(&syntax);
                            self.builder_snapshot = None;
                            self.show_ode_wizard = false;
                        }
                        if ui.button("Cancel").clicked() {
                            if let Some(orig) = self.builder_snapshot.take() {
                                self.state.session.raw_document_text = orig;
                                self.state.evaluate_all();
                            }
                            self.show_ode_wizard = false;
                        }
                    });
                });
            self.show_ode_wizard = is_open;
        }
    }

    /// Render Physical Units & Dimensional Palette Modal
    fn render_units_palette_modal(&mut self, ctx: &egui::Context) {
        if self.show_units_palette {
            if self.builder_snapshot.is_none() {
                self.builder_snapshot = Some(self.state.session.raw_document_text.clone());
            }
            let mut is_open = self.show_units_palette;
            egui::Window::new("Physical Units & Dimensions")
                .open(&mut is_open)
                .default_size([440.0, 300.0])
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.subheading("Physical Variable & SI Dimensions");
                    ui.horizontal(|ui| {
                        ui.label("Variable Name:");
                        ui.text_edit_singleline(&mut self.units_palette_var);
                        ui.label("Scalar Value:");
                        ui.add(egui::DragValue::new(&mut self.units_palette_val).speed(0.1));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Unit String:");
                        ui.text_edit_singleline(&mut self.units_palette_unit);
                    });

                    ui.separator();
                    ui.label("Common Dimension Presets:");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Velocity [m/s]").clicked() {
                            self.units_palette_unit = "m/s".to_string();
                        }
                        if ui.button("Accel [m/s^2]").clicked() {
                            self.units_palette_unit = "m/s^2".to_string();
                        }
                        if ui.button("Force [N]").clicked() {
                            self.units_palette_unit = "N".to_string();
                        }
                        if ui.button("Energy [J]").clicked() {
                            self.units_palette_unit = "J".to_string();
                        }
                        if ui.button("Power [W]").clicked() {
                            self.units_palette_unit = "W".to_string();
                        }
                        if ui.button("Pressure [Pa]").clicked() {
                            self.units_palette_unit = "Pa".to_string();
                        }
                        if ui.button("Voltage [V]").clicked() {
                            self.units_palette_unit = "V".to_string();
                        }
                    });

                    ui.separator();
                    let syntax = generate_physical_unit_syntax(
                        &self.units_palette_var,
                        self.units_palette_val,
                        &self.units_palette_unit,
                    );

                    ui.label("Live Syntax Preview:");
                    ui.monospace(
                        egui::RichText::new(&syntax)
                            .strong()
                            .color(egui::Color32::from_rgb(120, 220, 180)),
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Insert at Active Line").clicked() {
                            self.state.insert_text_at_active_line(&syntax);
                            self.builder_snapshot = None;
                            self.show_units_palette = false;
                        }
                        if ui.button("Cancel").clicked() {
                            if let Some(orig) = self.builder_snapshot.take() {
                                self.state.session.raw_document_text = orig;
                                self.state.evaluate_all();
                            }
                            self.show_units_palette = false;
                        }
                    });
                });
            self.show_units_palette = is_open;
        }
    }

    /// Render Example Notebook Gallery & Interactive Tutorials Modal
    fn render_example_gallery_modal(&mut self, ctx: &egui::Context) {
        if self.show_example_gallery {
            let mut is_open = self.show_example_gallery;
            egui::Window::new("📚 URAE Example Notebook Gallery & Tutorials")
                .open(&mut is_open)
                .default_size([880.0, 560.0])
                .resizable(true)
                .collapsible(true)
                .show(&ctx, |ui| {
                    ui.subheading("Interactive Mathematical Notebook Examples");
                    ui.weak("Browse, search, and load pre-built production-ready notebooks covering all 31 phases of URAE.");
                    ui.separator();

                    // Search and Category Filter Header
                    ui.horizontal(|ui| {
                        ui.label("🔍 Search:");
                        ui.text_edit_singleline(&mut self.example_search_query);
                        if !self.example_search_query.is_empty() && ui.small_button("✕").clicked() {
                            self.example_search_query.clear();
                        }

                        ui.separator();
                        let all_selected = self.selected_example_category.is_none();
                        if ui.selectable_label(all_selected, "All").clicked() {
                            self.selected_example_category = None;
                        }
                        for cat in crate::examples::ExampleCategory::all() {
                            let selected = self.selected_example_category == Some(*cat);
                            let label = format!("{} {}", cat.icon(), cat.display_name().split('&').next().unwrap_or("").trim());
                            if ui.selectable_label(selected, label).clicked() {
                                self.selected_example_category = if selected { None } else { Some(*cat) };
                            }
                        }
                    });

                    ui.separator();

                    // Two-pane Layout: Left list of cards, Right detailed preview and load button
                    let filtered_examples: Vec<&'static crate::examples::ExampleNotebook> = match self.selected_example_category {
                        Some(cat) => crate::examples::ExampleRegistry::filter_by_category(cat)
                            .into_iter()
                            .filter(|ex| {
                                self.example_search_query.is_empty()
                                    || ex.title.to_lowercase().contains(&self.example_search_query.to_lowercase())
                                    || ex.tags.iter().any(|t| t.to_lowercase().contains(&self.example_search_query.to_lowercase()))
                            })
                            .collect(),
                        None => crate::examples::ExampleRegistry::search(&self.example_search_query),
                    };

                    ui.columns(2, |cols| {
                        // Left Column: Scrollable list of example cards
                        cols[0].vertical(|ui| {
                            ui.label(egui::RichText::new(format!("Available Notebooks ({})", filtered_examples.len())).strong());
                            egui::ScrollArea::vertical().id_salt("examples_list_scroll").max_height(420.0).show(ui, |ui| {
                                for ex in &filtered_examples {
                                    let is_selected = self.selected_example_id == ex.id;
                                    let card_frame = if is_selected {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgba_premultiplied(40, 90, 160, 45))
                                            .stroke(egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(100, 180, 255)))
                                            .corner_radius(6)
                                            .inner_margin(6)
                                    } else {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgba_premultiplied(30, 30, 30, 30))
                                            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(70, 70, 70)))
                                            .corner_radius(6)
                                            .inner_margin(6)
                                    };

                                    card_frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(ex.category.icon()).strong());
                                            if ui.selectable_label(is_selected, egui::RichText::new(ex.title).strong()).clicked() {
                                                self.selected_example_id = ex.id.to_string();
                                            }
                                        });
                                        ui.small(ex.description);
                                    });
                                    ui.add_space(3.0);
                                }
                            });
                        });

                        // Right Column: Detailed preview and Load Action
                        cols[1].vertical(|ui| {
                            if let Some(selected_ex) = crate::examples::ExampleRegistry::find_by_id(&self.selected_example_id) {
                                ui.heading(format!("{} {}", selected_ex.category.icon(), selected_ex.title));
                                ui.colored_label(egui::Color32::from_rgb(120, 190, 255), selected_ex.category.display_name());
                                ui.add_space(4.0);
                                ui.label(selected_ex.description);

                                ui.separator();
                                ui.label(egui::RichText::new("Formula / Mathematical Core:").strong());
                                ui.monospace(egui::RichText::new(selected_ex.latex_formula).color(egui::Color32::from_rgb(240, 200, 100)));

                                ui.separator();
                                ui.label(egui::RichText::new("Notebook Script Preview:").strong());
                                egui::ScrollArea::vertical().id_salt("example_preview_scroll").max_height(200.0).show(ui, |ui| {
                                    let mut preview_text = selected_ex.content.to_string();
                                    ui.add(
                                        egui::TextEdit::multiline(&mut preview_text)
                                            .desired_rows(10)
                                            .desired_width(f32::INFINITY)
                                            .font(egui::TextStyle::Monospace)
                                            .interactive(false),
                                    );
                                });

                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if ui.button(egui::RichText::new("🚀 Load Example Notebook into Editor").strong().size(15.0)).clicked() {
                                        self.state.record_snapshot(format!("Load example '{}'", selected_ex.title));
                                        self.state.session.raw_document_text = selected_ex.content.to_string();
                                        self.state.evaluate_all();
                                        self.active_notebook = format!("{}.math", selected_ex.id);
                                        self.export_notice = Some(format!("Loaded example notebook: '{}'", selected_ex.title));
                                        self.show_example_gallery = false;
                                    }
                                    if ui.button("Close").clicked() {
                                        self.show_example_gallery = false;
                                    }
                                });
                            } else {
                                ui.weak("Select an example notebook on the left to preview.");
                            }
                        });
                    });
                });
            self.show_example_gallery = is_open;
        }
    }

    /// Render Unique Open Object & Symbol Inspectors (Floating Tooltips without header)
    fn render_open_inspectors(&mut self, ctx: &egui::Context) {
        let palette = self.theme.palette();
        let mut symbol_to_open = None;
        let mut to_close = Vec::new();

        if let Some(focus_sym) = self.inspector_to_focus.take() {
            let window_id = egui::Id::new(format!("obj_inspector_window_{}", focus_sym));
            ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, window_id));
        }

        for sym_name in &self.active_open_inspectors {
            let mut is_open = true;
            let window_id = egui::Id::new(format!("obj_inspector_window_{}", sym_name));

            // Determine default position: right below the highlighted line or origin line
            let mut default_pos = None;
            if let Some(num_str) = sym_name.strip_prefix('$') {
                if let Ok(line_num) = num_str.parse::<usize>() {
                    if line_num > 0 {
                        default_pos = self.line_screen_positions.get(&(line_num - 1)).copied();
                    }
                }
            }
            if default_pos.is_none() {
                if let Some(h_idx) = self.hovered_line_idx {
                    default_pos = self.line_screen_positions.get(&h_idx).copied();
                }
            }

            let mut win = egui::Window::new(format!("inspector_{}", sym_name))
                .id(window_id)
                .title_bar(false) // NO HEADER
                .resizable(true)
                .default_size([280.0, 280.0])
                .frame(
                    egui::Frame::window(&ctx.style_of(ctx.theme()))
                        .fill(palette.bg_card)
                        .stroke(egui::Stroke::new(1.0_f32, palette.accent_primary))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(10, 8)),
                );

            if let Some(pos) = default_pos {
                win = win.default_pos(egui::pos2(pos.x.max(40.0), pos.y + 4.0));
            }

            win.show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(palette.accent_primary, format!("🔍 {}", sym_name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("✕")
                            .on_hover_text("Close inspector")
                            .clicked()
                        {
                            is_open = false;
                        }
                    });
                });
                ui.separator();

                if let Some(info) = self.state.get_symbol_info_card(sym_name) {
                    render_symbol_info_card_content(ui, &info, &palette, &mut symbol_to_open);
                } else {
                    ui.weak(format!("No symbol metadata for '{}'", sym_name));
                }
            });

            if !is_open {
                to_close.push(sym_name.clone());
            }
        }

        for sym in to_close {
            self.active_open_inspectors.retain(|s| s != &sym);
        }

        if let Some(next_sym) = symbol_to_open {
            self.open_or_focus_inspector(&next_sym);
        }
    }

    /// Render Interactive Dependency Graph View (DAG of symbols, equations, and expressions)
    pub fn render_dependency_graph_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let palette = self.theme.palette();
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🕸 Symbol & Equation Dependency Graph")
                    .color(palette.accent_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Back to Editor (Smart Stream)").clicked() {
                    self.view_mode = ViewMode::SmartStream;
                }
            });
        });
        ui.separator();

        ui.label("Interactive visualization of upstream and downstream mathematical dependencies across your document:");
        ui.add_space(4.0);

        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            let parsed = &self.state.parsed_lines;
            if parsed.is_empty() {
                ui.weak("Document is currently empty. Add math formulas or variables to visualize the dependency graph.");
                return;
            }

            ui.horizontal(|ui| {
                // Left column: Symbol nodes
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Declared Symbols & Constants").strong().size(15.0));
                    for (sym_name, meta) in &self.state.session.symbol_metadata {
                        let is_sel = self.inspected_symbol.as_deref() == Some(sym_name);
                        let role_color = match meta.role {
                            crate::notebook::SymbolRole::Constant => palette.accent_secondary,
                            crate::notebook::SymbolRole::Parameter => egui::Color32::from_rgb(250, 204, 21),
                            crate::notebook::SymbolRole::Variable => palette.accent_primary,
                        };

                        egui::Frame::NONE
                            .fill(if is_sel { palette.accent_primary.gamma_multiply(0.2) } else { palette.bg_card })
                            .stroke(egui::Stroke::new(1.0, role_color))
                            .corner_radius(6)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.colored_label(role_color, format!("{:?}", meta.role));
                                    ui.label(egui::RichText::new(sym_name).strong().size(15.0));
                                    ui.weak(format!("= {:.4}", meta.cur_val));
                                    if let Some(u) = &meta.unit_str {
                                        ui.weak(format!("[{}]", u));
                                    }
                                });
                                if !meta.compound_tags().is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        for tag in meta.compound_tags() {
                                            ui.weak(format!("[{}]", tag));
                                        }
                                    });
                                }
                            });
                        ui.add_space(4.0);
                    }
                });

                ui.separator();

                // Right column: Formula dependency lines
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Formula Execution Nodes & Dataflow").strong().size(15.0));
                    for pl in parsed {
                        if pl.kind == LineKind::Markdown {
                            continue;
                        }
                        let upstream = compute_line_upstream_dependencies(pl.line_idx, parsed);
                        egui::Frame::NONE
                            .fill(palette.bg_card)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                            .corner_radius(6)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("L{}", pl.line_idx + 1)).strong().color(palette.text_muted));
                                    ui.monospace(&pl.raw_text);
                                    if !pl.output_unicode.is_empty() {
                                        ui.weak("➔");
                                        ui.colored_label(palette.math_result, &pl.output_unicode);
                                    }
                                });
                                if !upstream.is_empty() {
                                    let up_str = upstream.iter().map(|idx| format!("L{}", idx + 1)).collect::<Vec<_>>().join(", ");
                                    ui.weak(format!("Upstream dependencies: {}", up_str));
                                }
                            });
                        ui.add_space(4.0);
                    }
                });
            });
        });
    }

    /// Render clean, publishable presentation / report view
    pub fn render_presentation_report_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let palette = self.theme.palette();
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("📰 Presentation & Publication Report")
                    .color(palette.accent_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Back to Editor").clicked() {
                    self.view_mode = ViewMode::SmartStream;
                }
                if ui.button("🖨 Print / Export").clicked() {
                    self.export_notice = Some("Exporting presentation document...".to_string());
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);
                for pl in &self.state.parsed_lines {
                    match &pl.kind {
                        LineKind::Markdown => {
                            let trimmed = pl.raw_text.trim();
                            if trimmed.starts_with('#') {
                                let level = trimmed.chars().take_while(|c| *c == '#').count();
                                let txt = trimmed.trim_start_matches('#').trim();
                                if level == 1 {
                                    ui.heading(
                                        egui::RichText::new(txt)
                                            .size(22.0)
                                            .color(palette.accent_primary)
                                            .strong(),
                                    );
                                } else {
                                    ui.heading(
                                        egui::RichText::new(txt)
                                            .size(16.0)
                                            .color(palette.accent_secondary)
                                            .strong(),
                                    );
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(trimmed.trim_start_matches("//").trim())
                                        .size(14.0),
                                );
                            }
                        }
                        LineKind::Formula => {
                            egui::Frame::NONE
                                .fill(palette.bg_card)
                                .stroke(egui::Stroke::new(1.0, palette.border))
                                .corner_radius(6)
                                .inner_margin(12)
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new(&pl.output_unicode)
                                                .size(15.0)
                                                .strong()
                                                .color(palette.math_result),
                                        );
                                        if let Some(u) = &pl.physical_unit {
                                            if !pl.output_unicode.contains(&format!("[{}]", u)) {
                                                ui.weak(format!("[{}]", u));
                                            }
                                        }
                                    });
                                    if pl.is_surface_3d {
                                        ui.add_space(4.0);
                                        crate::ui::Viewport3D::show_with_height(
                                            ui,
                                            &mut self.viewport_3d_state,
                                            None,
                                            240.0,
                                        );
                                    }
                                });
                        }
                        LineKind::CadMesh {
                            model_kind,
                            summary,
                        } => {
                            egui::Frame::NONE
                                .fill(palette.bg_card)
                                .stroke(egui::Stroke::new(1.0, palette.border))
                                .corner_radius(6)
                                .inner_margin(12)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Engineering Model: {}",
                                            model_kind
                                        ))
                                        .heading()
                                        .color(palette.accent_primary),
                                    );
                                    ui.label(summary);
                                    ui.add_space(6.0);
                                    crate::ui::Viewport3D::show_with_height(
                                        ui,
                                        &mut self.viewport_3d_state,
                                        None,
                                        280.0,
                                    );
                                });
                        }
                        LineKind::Plot3D {
                            expr_str,
                            x_var,
                            y_var,
                        } => {
                            egui::Frame::NONE
                                .fill(palette.bg_card)
                                .stroke(egui::Stroke::new(1.0, palette.border))
                                .corner_radius(6)
                                .inner_margin(12)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "3D Surface: z = f({}, {}) = {}",
                                            x_var, y_var, expr_str
                                        ))
                                        .heading()
                                        .color(palette.accent_secondary),
                                    );
                                    ui.add_space(6.0);
                                    crate::ui::Viewport3D::show_with_height(
                                        ui,
                                        &mut self.viewport_3d_state,
                                        None,
                                        280.0,
                                    );
                                });
                        }
                        _ => {}
                    }
                    ui.add_space(8.0);
                }
            });
    }
}

/// Returns true if `raw_text` is a variable, parameter, or constant assignment / definition.
pub fn is_variable_or_param_assignment(raw_text: &str) -> bool {
    let text = raw_text.trim();
    if text.contains("=?") || text.contains("==") || text.starts_with("solve") {
        return false;
    }
    if let Some((lhs, _rhs)) = text.split_once('=') {
        let mut lhs = lhs.trim();
        for prefix in &["var ", "let ", "const ", "param ", "slider "] {
            if let Some(rest) = lhs.strip_prefix(prefix) {
                lhs = rest.trim();
                break;
            }
        }
        !lhs.is_empty()
            && !lhs.contains('(')
            && !lhs.contains(')')
            && !lhs.contains('[')
            && !lhs.contains(']')
            && !lhs.contains('{')
            && !lhs.contains('}')
            && !lhs.contains(' ')
            && lhs.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !lhs.chars().next().unwrap().is_ascii_digit()
    } else {
        false
    }
}

/// Finds numeric literal around `char_idx` in `text`.
/// Returns `Some((byte_start, byte_end, initial_val, has_decimal, decimal_places))`.
pub fn find_numeric_literal_at(
    text: &str,
    char_idx: usize,
) -> Option<(usize, usize, f64, bool, usize)> {
    if text.is_empty() {
        return None;
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    if chars.is_empty() {
        return None;
    }
    let clamped_idx = char_idx.min(chars.len() - 1);

    // Look around clamped_idx for digit or decimal point
    let mut target_idx = clamped_idx;
    if !chars[target_idx].1.is_ascii_digit() && chars[target_idx].1 != '.' {
        if target_idx > 0
            && (chars[target_idx - 1].1.is_ascii_digit() || chars[target_idx - 1].1 == '.')
        {
            target_idx -= 1;
        } else if target_idx + 1 < chars.len()
            && (chars[target_idx + 1].1.is_ascii_digit() || chars[target_idx + 1].1 == '.')
        {
            target_idx += 1;
        } else {
            return None;
        }
    }

    if !chars[target_idx].1.is_ascii_digit() && chars[target_idx].1 != '.' {
        return None;
    }

    // Expand backwards
    let mut start_idx = target_idx;
    while start_idx > 0 {
        let prev_c = chars[start_idx - 1].1;
        if prev_c.is_ascii_digit() || prev_c == '.' {
            start_idx -= 1;
        } else if prev_c == '-' {
            if start_idx == 1 {
                start_idx -= 1;
            } else {
                let before_minus = chars[start_idx - 2].1;
                if before_minus.is_whitespace() || "+-*/=,(^".contains(before_minus) {
                    start_idx -= 1;
                }
            }
            break;
        } else {
            break;
        }
    }

    // Expand forwards
    let mut end_idx = target_idx;
    while end_idx + 1 < chars.len() {
        let next_c = chars[end_idx + 1].1;
        if next_c.is_ascii_digit() || next_c == '.' {
            end_idx += 1;
        } else {
            break;
        }
    }

    let byte_start = chars[start_idx].0;
    let byte_end = if end_idx + 1 < chars.len() {
        chars[end_idx + 1].0
    } else {
        text.len()
    };

    let slice = &text[byte_start..byte_end];
    if let Ok(val) = slice.parse::<f64>() {
        let has_decimal = slice.contains('.');
        let decimal_places = if let Some(dot_pos) = slice.find('.') {
            slice.len().saturating_sub(dot_pos + 1)
        } else {
            0
        };
        Some((byte_start, byte_end, val, has_decimal, decimal_places))
    } else {
        None
    }
}

/// Compute all upstream and direct dependency line indices for `target_line_idx`.
/// Lines are never marked as their own dependencies.
pub fn compute_line_upstream_dependencies(
    target_line_idx: usize,
    parsed_lines: &[crate::notebook::ParsedLine],
) -> HashSet<usize> {
    let mut deps = HashSet::new();
    let mut visited = HashSet::new();
    let mut queue = vec![target_line_idx];
    visited.insert(target_line_idx);

    while let Some(curr_idx) = queue.pop() {
        if curr_idx >= parsed_lines.len() {
            continue;
        }
        let pl = &parsed_lines[curr_idx];
        let text = &pl.raw_text;

        // 1. Check for $N references (e.g. $1 -> line index 0)
        for tok in text.split(|c: char| !c.is_alphanumeric() && c != '$' && c != '_') {
            if let Some(num_str) = tok.strip_prefix('$') {
                if let Ok(num) = num_str.parse::<usize>() {
                    if num > 0 && num - 1 < parsed_lines.len() && num - 1 != curr_idx {
                        let dep_line = num - 1;
                        if dep_line != target_line_idx {
                            deps.insert(dep_line);
                        }
                        if visited.insert(dep_line) {
                            queue.push(dep_line);
                        }
                    }
                }
            }
        }

        // 2. Check for 'ans' reference (depends on immediately preceding line)
        if text.contains("ans") && curr_idx > 0 {
            let dep_line = curr_idx - 1;
            if dep_line != target_line_idx {
                deps.insert(dep_line);
            }
            if visited.insert(dep_line) {
                queue.push(dep_line);
            }
        }

        // 3. Check for identifier references defined on earlier lines
        let tokens: Vec<&str> = text
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|t| !t.is_empty() && !t.chars().next().unwrap().is_ascii_digit())
            .collect();

        for &tok in &tokens {
            for (j, prev_pl) in parsed_lines.iter().enumerate().take(curr_idx) {
                let defines_tok = match &prev_pl.kind {
                    LineKind::SliderDef { name, .. } => name == tok,
                    LineKind::DomainRestriction { name, .. } => name == tok,
                    LineKind::SetBuilder { var_name, .. } => var_name == tok,
                    LineKind::RoleDeclaration { name, .. } => name == tok,
                    LineKind::Formula => {
                        let raw = prev_pl.raw_text.trim();
                        if let Some((lhs, _)) = raw.split_once('=') {
                            let lhs_trimmed = lhs.trim();
                            lhs_trimmed == tok || lhs_trimmed.starts_with(&format!("{}(", tok))
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                if defines_tok {
                    if j != target_line_idx {
                        deps.insert(j);
                    }
                    if visited.insert(j) {
                        queue.push(j);
                    }
                }
            }
        }
    }

    deps
}

/// Render header-less rich object / symbol inspector preview UI (used inside hover tooltips and floating cards).
pub fn render_symbol_info_card_content(
    ui: &mut egui::Ui,
    info: &crate::notebook::SymbolInfoCard,
    palette: &crate::ui::ThemePalette,
    on_select_symbol: &mut Option<String>,
) {
    if !info.compound_tags.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for tag in &info.compound_tags {
                ui.label(
                    egui::RichText::new(format!("[{}]", tag))
                        .size(11.0)
                        .color(palette.accent_secondary)
                        .strong(),
                );
            }
        });
        ui.separator();
    }

    if let Some(kind) = &info.object_kind {
        match kind {
            crate::notebook::ObjectKind::Constant {
                symbol_name,
                exact_desc,
                approx_val,
            } => {
                ui.colored_label(
                    palette.accent_secondary,
                    format!("🏛 Constant: {} ≈ {:.8}", symbol_name, approx_val),
                );
                ui.label(exact_desc);
            }
            crate::notebook::ObjectKind::Scalar {
                type_name,
                value_str,
                approx_f64,
            } => {
                ui.colored_label(
                    palette.text_success,
                    format!("🔢 Scalar ({}) = {}", type_name, value_str),
                );
                if let Some(f) = approx_f64 {
                    ui.weak(format!("• Float: {:.6}", f));
                }
            }
            crate::notebook::ObjectKind::Complex { real, imag } => {
                ui.colored_label(
                    palette.accent_primary,
                    format!("ℂ Complex: {:.4} + {:.4}i", real, imag),
                );
                let mag = (real * real + imag * imag).sqrt();
                let phase = imag.atan2(*real);
                ui.weak(format!("• |z| = {:.4}, Arg(z) = {:.4} rad", mag, phase));
            }
            crate::notebook::ObjectKind::Matrix {
                rows,
                cols,
                is_square,
                shape_desc,
            } => {
                ui.colored_label(
                    palette.accent_secondary,
                    format!("📊 Matrix {}x{} ({})", rows, cols, shape_desc),
                );
                ui.weak(format!(
                    "• Dimensions: [{} × {}] (Square: {})",
                    rows,
                    cols,
                    if *is_square { "Yes" } else { "No" }
                ));
            }
            crate::notebook::ObjectKind::Function {
                arity,
                domain,
                codomain,
                range,
                is_linear,
                roots,
                is_roots_symbolic,
                critical_points,
                is_critical_points_symbolic,
                parity,
                period,
            } => {
                ui.colored_label(
                    palette.accent_primary,
                    format!("📈 Function Mapping: {} → {}", domain, codomain),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.weak(format!(
                        "• Arity: {} | Linearity: {}",
                        arity,
                        if *is_linear { "Linear" } else { "Non-Linear" }
                    ));
                    if let Some(r) = range {
                        ui.weak(format!("| Range: {}", r));
                    }
                });

                if let Some(p) = parity {
                    ui.weak(format!("• Symmetry / Parity: {}", p));
                }
                if let Some(t) = period {
                    ui.weak(format!(
                        "• Periodic Fundamental Period: T ≈ {:.4} ({:.2}π)",
                        t,
                        t / std::f64::consts::PI
                    ));
                }

                // Roots / X-Intercepts
                if !roots.is_empty() {
                    ui.add_space(2.0);
                    let title = if *is_roots_symbolic {
                        "✓ Exact Symbolic Roots (x-intercepts):"
                    } else {
                        "⚡ VM Discovered Roots (x-intercepts):"
                    };
                    let color = if *is_roots_symbolic {
                        palette.text_success
                    } else {
                        palette.accent_secondary
                    };
                    ui.colored_label(color, title);
                    for r in roots {
                        ui.monospace(format!("  • {}", r));
                    }
                } else {
                    ui.weak("• Roots (x-intercepts): None found in primary domain");
                }

                // Critical Points (Extrema / Inflection)
                if !critical_points.is_empty() {
                    ui.add_space(2.0);
                    let title = if *is_critical_points_symbolic {
                        "✓ Exact Symbolic Critical Points (f'(x) = 0):"
                    } else {
                        "⚡ VM Discovered Critical Points (f'(x) = 0):"
                    };
                    let color = if *is_critical_points_symbolic {
                        palette.text_success
                    } else {
                        palette.accent_secondary
                    };
                    ui.colored_label(color, title);
                    for c in critical_points {
                        ui.monospace(format!("  • {}", c));
                    }
                }
            }
            crate::notebook::ObjectKind::PhysicalQuantity {
                dimension,
                unit,
                magnitude,
            } => {
                ui.colored_label(
                    palette.text_success,
                    format!("📐 Physical Quantity: {:.4} [{}]", magnitude, unit),
                );
                ui.weak(format!("• Dimension: {}", dimension));
            }
            crate::notebook::ObjectKind::SetOrInterval {
                set_type,
                bounds_str,
            } => {
                ui.colored_label(
                    palette.accent_primary,
                    format!("📦 Set / Interval: {} ∈ {}", set_type, bounds_str),
                );
            }
            crate::notebook::ObjectKind::Polynomial {
                indeterminate,
                degree,
            } => {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        palette.accent_secondary,
                        format!("Polynomial (Degree {}) in", degree),
                    );
                    if ui
                        .small_button(indeterminate)
                        .on_hover_text(format!("Inspect symbol '{}'", indeterminate))
                        .clicked()
                    {
                        *on_select_symbol = Some(indeterminate.clone());
                    }
                });
            }
            crate::notebook::ObjectKind::LineResult { line_idx, summary } => {
                ui.colored_label(
                    palette.text_success,
                    format!("📝 Line #{} Output: {}", line_idx, summary),
                );
            }
            crate::notebook::ObjectKind::Expression {
                free_variables,
                structure,
            } => {
                ui.colored_label(palette.text_primary, format!("Expression: {}", structure));
                if !free_variables.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak("Free variables:");
                        for v in free_variables {
                            if ui
                                .small_button(v)
                                .on_hover_text(format!("Inspect object '{}'", v))
                                .clicked()
                            {
                                *on_select_symbol = Some(v.clone());
                            }
                        }
                    });
                }
            }
            crate::notebook::ObjectKind::TransferFunction {
                numerator_degree,
                denominator_degree,
                poles,
                zeros,
                is_stable,
            } => {
                let status = if *is_stable {
                    "✓ Asymptotically Stable"
                } else {
                    "⚠ Unstable / Marginally Stable"
                };
                let color = if *is_stable {
                    palette.text_success
                } else {
                    palette.accent_secondary
                };
                ui.colored_label(color, format!("📡 Transfer Function H(s) [{}]", status));
                ui.weak(format!(
                    "• Order: Denom s^{}, Num s^{} (Proper: {})",
                    denominator_degree,
                    numerator_degree,
                    denominator_degree >= numerator_degree
                ));
                if !poles.is_empty() {
                    ui.weak(format!("• Poles: {}", poles.join(", ")));
                }
                if !zeros.is_empty() {
                    ui.weak(format!("• Zeros: {}", zeros.join(", ")));
                }
            }
            crate::notebook::ObjectKind::StochasticProcess {
                drift_term,
                diffusion_term,
                is_martingale,
            } => {
                ui.colored_label(
                    palette.accent_primary,
                    "🎲 Itô Stochastic Differential dX_t",
                );
                ui.monospace(format!(
                    "  dX_t = ({}) dt + ({}) dW_t",
                    drift_term, diffusion_term
                ));
                ui.weak(format!(
                    "• Martingale Property: {}",
                    if *is_martingale {
                        "True (Drift μ = 0)"
                    } else {
                        "False (Drift present)"
                    }
                ));
            }
            crate::notebook::ObjectKind::FEAResult {
                nodes,
                elements,
                max_stress,
                deformation_scale,
                solution_type,
            } => {
                ui.colored_label(
                    palette.accent_secondary,
                    format!("🏗 FEA / IGA Geometry: {}", solution_type),
                );
                ui.weak(format!(
                    "• Mesh / Control Net: {} nodes, {} elements",
                    nodes, elements
                ));
                ui.weak(format!(
                    "• Peak Stress / Measure: {:.2} | Scale: {:.1}x",
                    max_stress, deformation_scale
                ));
            }
            crate::notebook::ObjectKind::ThermodynamicState {
                system_type,
                equation_of_state,
                properties,
            } => {
                ui.colored_label(
                    palette.text_success,
                    format!("🔥 Thermodynamics: {}", system_type),
                );
                ui.monospace(format!("  EoS: {}", equation_of_state));
                for (k, v) in properties {
                    ui.weak(format!("• {}: {:.2}", k, v));
                }
            }
            crate::notebook::ObjectKind::AlgebraicForm {
                form_type,
                complexity_score,
            } => {
                ui.colored_label(
                    palette.accent_primary,
                    format!("📐 Algebraic Form: {}", form_type),
                );
                ui.weak(format!(
                    "• Structural Complexity / Depth: {}",
                    complexity_score
                ));
            }
            crate::notebook::ObjectKind::LogicSystem {
                system_name,
                truth_values_count,
                is_paraconsistent,
                is_intuitionistic,
                tautologies_summary,
            } => {
                ui.colored_label(
                    palette.accent_secondary,
                    format!("⚖ Logic System: {}", system_name),
                );
                ui.weak(format!(
                    "• Values: {}-Valued | Intuitionistic: {}",
                    truth_values_count,
                    if *is_intuitionistic { "Yes" } else { "No" }
                ));
                ui.weak(format!(
                    "• Paraconsistent: {}",
                    if *is_paraconsistent { "Yes" } else { "No" }
                ));
                ui.weak(format!("• Properties: {}", tautologies_summary));
            }
            crate::notebook::ObjectKind::CryptographicCurve {
                curve_name,
                field_order,
                equation,
            } => {
                ui.colored_label(
                    palette.accent_primary,
                    format!("🔐 Elliptic Curve: {}", curve_name),
                );
                ui.monospace(format!("  E: {}", equation));
                ui.weak(format!("• Finite Field: GF({})", field_order));
            }
            crate::notebook::ObjectKind::GraphStructure {
                vertices,
                edges,
                is_directed,
                spectral_gap,
            } => {
                ui.colored_label(
                    palette.accent_secondary,
                    format!("🕸 Graph: {} Nodes, {} Edges", vertices, edges),
                );
                ui.weak(format!(
                    "• Topology: {}",
                    if *is_directed {
                        "Directed"
                    } else {
                        "Undirected"
                    }
                ));
                if let Some(gap) = spectral_gap {
                    ui.weak(format!("• Algebraic Connectivity λ₂: {:.4}", gap));
                }
            }
            crate::notebook::ObjectKind::DifferentialManifold {
                dimension,
                metric_name,
                curvature_scalar,
            } => {
                ui.colored_label(
                    palette.accent_primary,
                    format!("🌌 Manifold: {}-Dimensional", dimension),
                );
                ui.weak(format!("• Metric: {}", metric_name));
                if let Some(r) = curvature_scalar {
                    ui.weak(format!("• Ricci Curvature Scalar R: {:.4}", r));
                }
            }
            crate::notebook::ObjectKind::ProbabilityDistribution {
                dist_type,
                mean,
                variance,
                is_discrete,
            } => {
                ui.colored_label(
                    palette.text_success,
                    format!("📊 Distribution: {}", dist_type),
                );
                ui.weak(format!(
                    "• Type: {}",
                    if *is_discrete {
                        "Discrete PMF"
                    } else {
                        "Continuous PDF"
                    }
                ));
                ui.weak(format!("• Expectation E[X]: {:.4}", mean));
                ui.weak(format!(
                    "• Variance Var(X): {:.4} (σ = {:.4})",
                    variance,
                    variance.sqrt()
                ));
            }
        }
    } else {
        ui.weak(format!(
            "• Value: {:.4} | Domain: {}",
            info.cur_val, info.domain_type
        ));
        if let Some(u) = &info.unit_str {
            ui.weak(format!("• Unit: [{}]", u));
        }
    }

    if let Some(t_ms) = info.computation_time_ms {
        let tag = if t_ms < 50.0 { "⚡" } else { "⏳" };
        ui.colored_label(
            palette.accent_primary,
            format!("{} Solved in {:.2} ms", tag, t_ms),
        );
    }

    if info.ast_depth.is_some() || info.neural_embedding.is_some() {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if let Some(d) = info.ast_depth {
                ui.label(
                    egui::RichText::new(format!("AST Depth: {}", d))
                        .size(11.0)
                        .color(palette.accent_primary)
                        .strong(),
                );
            }
            if let Some(pd) = info.proof_search_depth {
                ui.label(
                    egui::RichText::new(format!("Proof-Search Depth: {}", pd))
                        .size(11.0)
                        .color(palette.text_success)
                        .strong(),
                );
            }
        });
        if let Some(emb) = &info.neural_embedding {
            ui.horizontal_wrapped(|ui| {
                ui.weak(format!("Neural Embedding ({}d):", emb.len()));
                let preview = emb
                    .iter()
                    .take(4)
                    .map(|v| format!("{:+.3}", v))
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.monospace(format!("[{}, ...]", preview));
            });
        }
    }

    if !info.dependent_lines.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.weak("Used in:");
            for idx in &info.dependent_lines {
                let line_tok = format!("${}", idx + 1);
                if ui
                    .small_button(&line_tok)
                    .on_hover_text(format!("Inspect line {}", idx + 1))
                    .clicked()
                {
                    *on_select_symbol = Some(line_tok);
                }
            }
        });
    }
    if !info.formula_references.is_empty() {
        ui.collapsing("Referencing formulas", |ui| {
            for f in &info.formula_references {
                ui.monospace(f);
            }
        });
    }
}

trait HeadingExt {
    fn subheading(&mut self, text: &str);
}

impl HeadingExt for egui::Ui {
    fn subheading(&mut self, text: &str) {
        self.label(egui::RichText::new(text).heading().size(16.0));
    }
}
