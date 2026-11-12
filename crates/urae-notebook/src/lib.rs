//! # `urae-notebook` (URAE Reactive Mathematics Notebook)
//!
//! Cross-platform `egui` / `eframe` interactive mathematics notepad application for the Universal Rust Algebra Engine (URAE).
//!
//! ---
//!
//! # PROMINENT HOW-TO GUIDE & MASTER TUTORIAL
//!
//! ## 1. Launching the Application
//!
//! - **Native Desktop Window**:
//!   ```bash
//!   cargo run -p urae-notebook
//!   ```
//! - **WebAssembly Browser App (Cloudflare Pages / Workers)**:
//!   ```javascript
//!   import init, { start_notebook_web } from './pkg/urae_wasm.js';
//!   await init();
//!   await start_notebook_web('urae_canvas');
//!   ```
//!
//! ## 2. Core Notepad Syntax Cheat Sheet
//!
//! Write natural mathematical expressions directly in the notepad editor:
//!
//! | Mathematical Concept | Notepad Syntax Example | Live Reactive Output |
//! | :--- | :--- | :--- |
//! | **Parameter Sliders** | `a = 2.00 [m]`, `c = -3.00` | Instantiates interactive component sliders |
//! | **Role Declarations** | `a: Parameter = 2.0`, `x: Variable` | Declares symbol role with 100% metadata retention |
//! | **Set-Builder Bounds** | `{ x in Reals \| -5 <= x <= 5 }` | Enforces interval bounds and domain restrictions |
//! | **Scoped Math Context** | `{ q in Quaternion \| norm(q) == 1 }` | Dynamically switches scoped algebra to $\mathbb{H}$ |
//! | **Functions & Plots** | `f(x) = a * x^2 + c` | Renders Unicode math + auto-updating function plot |
//! | **Background Simplification**| `(x + 0) * 1 + 0` | Automatically simplifies via E-Graphs to `x` |
//! | **Permissive Equation Solving**| `given g(x) = 3 find x`, `solve g(x) = 3 for x`, `find x where g(x) = 3` | Solves equation analytically across multiple notations |
//! | **Probability Distributions** | `X ~ Normal(0, 1)` | Renders distribution model $X \sim \text{Normal}(0, 1)$ |
//! | **Equivalence Comparisons** | `are f(x) and g(x) equal?`, `is Quaternion isomorphic to Matrix2x2Complex?` | Evaluates structural isomorphism and zero-equivalence |
//! | **Substitutions & Alterations** | `evaluate x^2 + 3 at x = 2`, `substitute x = 2 in f(x)` | Substitutes variable values and simplifies output |
//! | **Physical Units** | `v = 10.0 [m/s]`, `g = 9.81 [m/s^2]` | Enforces dimensional consistency (Length, Time) |
//! | **Curvilinear Accessors** | `v.curvilinear("spherical").r` | Coordinates in Spherical, Polar, Cylindrical systems |
//!
//! ## 3. Interactive UI Controls
//!
//! - **Help Button**: Click `Help` in the top header bar to toggle the in-app interactive tutorial & cheat sheet modal.
//! - **Details Dropdown**: Click `[Details]` on any formula or declaration line to inspect scoped `MathContext`, raw vs. background simplified forms, $f'(x)$, $\int f(x)dx$, $f64$ numerical roots, linearized Taylor estimates, and evaluation latency.
//! - **Plot Pan & Zoom**: Click and drag on plot cards to pan, or use mouse scroll wheel to zoom the X and Y axes smoothly in real time.
//! - **Embedded CLI Terminal**: Click `Terminal CLI` in the header bar or press `F12` in browser to run interactive commands (`vars`, `eval a * x^2`, `diff`, `solve`, `proof`).
//! - **Multi-Provider AI Copilot**: Configure Google AI, Anthropic, LM Studio, OpenCode, or OpenRouter in Settings modal window.

pub mod app;
pub mod examples;
pub mod notebook;
pub mod ui;

pub use app::{UraeNotebookApp, ViewMode};
pub use eframe::egui;
pub use examples::{ExampleCategory, ExampleNotebook, ExampleRegistry};
pub use notebook::{
    CellBlock, CellBlockKind, HistorySnapshot, MatrixPresetKind, NotebookSettings, NotebookState,
    ParameterBuilderParams, ReactiveComputeMode, SessionData, SymbolInfoCard, SymbolMetadata,
    SymbolRole, UndoRedoHistory, generate_branch_cut_syntax, generate_interval_syntax,
    generate_matrix_syntax, generate_ode_bc_syntax, generate_parameter_builder_syntax,
    generate_physical_unit_syntax, generate_universal_parameter_builder_syntax,
};
pub use ui::{
    CommandItem, CommandPalette, CommandPaletteState, PalettesUi, ThemeKind, ThemePalette,
    Viewport3D, Viewport3DState, ViewportColorMode, ViewportModelPreset, WorkspaceLayoutPreset,
    WorkspaceState, WorkspaceTab,
};

/// Logging verbosity levels differentiating Debug and Release builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Off = 5,
}

impl LogLevel {
    pub fn current() -> Self {
        if let Ok(val) = std::env::var("URAE_LOG") {
            match val.to_lowercase().as_str() {
                "trace" => return Self::Trace,
                "debug" => return Self::Debug,
                "info" => return Self::Info,
                "warn" | "warning" => return Self::Warn,
                "error" => return Self::Error,
                "off" | "none" => return Self::Off,
                _ => {}
            }
        }

        if cfg!(debug_assertions) {
            Self::Debug
        } else {
            Self::Info
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Off => "OFF",
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_with_level(level: LogLevel, msg: &str) {
    let current_level = LogLevel::current();
    if level < current_level || current_level == LogLevel::Off {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let line = format!(
        "[URAE {:>5}] [{:>8}ms] {}",
        level.label(),
        now % 1_000_000,
        msg
    );
    eprintln!("{}", line);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("urae_launch.log")
    {
        use std::io::Write;
        let _ = writeln!(file, "{}", line);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_trace(msg: &str) {
    log_with_level(LogLevel::Trace, msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_debug(msg: &str) {
    log_with_level(LogLevel::Debug, msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_info(msg: &str) {
    log_with_level(LogLevel::Info, msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_warn(msg: &str) {
    log_with_level(LogLevel::Warn, msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_error(msg: &str) {
    log_with_level(LogLevel::Error, msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_event(msg: &str) {
    log_info(msg);
}

/// Launch native URAE Notebook GUI application window.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_gui() -> Result<(), eframe::Error> {
    log_info(&format!(
        "Starting run_gui() [Profile: {}, LogLevel: {}]...",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        LogLevel::current().label()
    ));

    spodeian_telemetry::init_default();


    log_debug("Configuring eframe::NativeOptions (inner_size: 1100x750, min: 600x400)...");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("URAE Reactive Mathematics Notebook")
            .with_active(true)
            .with_visible(true)
            .with_decorations(true)
            .with_resizable(true),
        centered: true,
        ..Default::default()
    };

    log_info("Initializing native desktop window and GPU/OpenGL context...");
    let res = eframe::run_native(
        "urae-notebook",
        options,
        Box::new(|cc| {
            log_debug(&format!(
                "eframe CreationContext callback received! (egui zoom: {}, cpu_usage: {:?})",
                cc.egui_ctx.zoom_factor(),
                cc.integration_info.cpu_usage
            ));
            let start = std::time::Instant::now();
            let app = app::UraeNotebookApp::new(cc);
            log_info(&format!(
                "UraeNotebookApp initialized in {:?}",
                start.elapsed()
            ));
            Ok(Box::new(app))
        }),
    );

    match &res {
        Ok(_) => log_info("eframe::run_native exited cleanly with Ok(())"),
        Err(err) => log_error(&format!("eframe::run_native returned Err({err:?})")),
    }

    res
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Web entry point for running URAE Notebook on a WebAssembly canvas (Cloudflare Pages / Workers).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start_web(canvas_id: &str) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    spodeian_telemetry::init_default();

    let web_options = eframe::WebOptions::default();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document object"))?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("Canvas element '{}' not found", canvas_id)))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Element is not HtmlCanvasElement"))?;

    let client_w = canvas.client_width();
    let client_h = canvas.client_height();
    if client_w > 0 && client_h > 0 {
        canvas.set_width(client_w as u32);
        canvas.set_height(client_h as u32);
    } else {
        canvas.set_width(1100);
        canvas.set_height(720);
    }

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(app::UraeNotebookApp::new(cc)))),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use eframe::NativeOptions;
    let mut options = NativeOptions::default();
    options.android_app = Some(app);
    eframe::run_native(
        "urae-notebook",
        options,
        Box::new(|cc| Ok(Box::new(app::UraeNotebookApp::new(cc)))),
    ).unwrap();
}

