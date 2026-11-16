//! # `urae_notebook::ui::workbench_ui`
//!
//! Interactive Q&A Mathematical Solver View and Systems & Differential Equations Workbench
//! built on top of [`urae::QueryEngine`] and [`urae::WorkbenchEngine`].

use crate::notebook::NotebookState;
use crate::ui::ThemePalette;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};
use urae::prelude::*;

/// Active sub-mode inside the Systems & Differential Equations Workbench.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkbenchSubMode {
    #[default]
    Equations,
    Differentials,
}

/// Persistent UI state backing both Q&A Solver Mode and the Systems & DE Workbench.
pub struct WorkbenchUiState {
    // Q&A Mode state
    pub qa_input: String,
    pub qa_history: Vec<(String, QueryResponse)>,
    pub qa_use_notebook_params: bool,

    // Workbench Mode state
    pub sub_mode: WorkbenchSubMode,
    pub eq_spec: EquationSystemSpec,
    pub eq_analysis: Option<EquationSystemAnalysis>,
    pub eq_new_param_name: String,
    pub eq_new_param_val: f64,
    pub eq_selected_preset: usize,

    pub de_spec: DifferentialSystemSpec,
    pub de_analysis: Option<DifferentialSystemAnalysis>,
    pub de_new_param_name: String,
    pub de_new_param_val: f64,
    pub de_selected_preset: usize,
    pub show_time_series: bool,
    pub show_phase_portrait: bool,
}

impl Default for WorkbenchUiState {
    fn default() -> Self {
        let eq_presets = WorkbenchPresets::equation_presets();
        let de_presets = WorkbenchPresets::differential_presets();
        let eq_spec = eq_presets.into_iter().next().unwrap_or_default();
        let de_spec = de_presets.into_iter().next().unwrap_or_default();

        Self {
            qa_input: "integrate x * sin(x), x".to_string(),
            qa_history: Vec::new(),
            qa_use_notebook_params: true,
            sub_mode: WorkbenchSubMode::Equations,
            eq_spec,
            eq_analysis: None,
            eq_new_param_name: "k".to_string(),
            eq_new_param_val: 1.0,
            eq_selected_preset: 0,
            de_spec,
            de_analysis: None,
            de_new_param_name: "c".to_string(),
            de_new_param_val: 1.0,
            de_selected_preset: 0,
            show_time_series: true,
            show_phase_portrait: true,
        }
    }
}

/// Action requested from Q&A or Workbench UI that updates parent notebook state.
pub enum WorkbenchAction {
    None,
    BackToNotepad,
    InsertToNotepad(String),
    SwitchToEquations(Vec<String>),
}

impl WorkbenchUiState {
    /// Render the interactive Question & Answer Mathematical Solver View.
    pub fn render_qa_view(
        &mut self,
        ui: &mut egui::Ui,
        palette: &ThemePalette,
        notebook: &NotebookState,
    ) -> WorkbenchAction {
        let mut action = WorkbenchAction::None;

        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("❓ Question & Answer Mathematical Solver")
                    .color(palette.accent_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📝 Back to Notepad").clicked() {
                    action = WorkbenchAction::BackToNotepad;
                }
                ui.checkbox(
                    &mut self.qa_use_notebook_params,
                    "Bind active Notepad parameters",
                );
            });
        });
        ui.separator();

        // Query Input Bar
        let mut submit_query = false;
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.qa_input)
                    .hint_text("Ask any calculus, algebra, matrix, or system question (e.g. `solve [2*x+y=5, x-y=1], [x, y]`)")
                    .desired_width((ui.available_width() - 120.0).max(240.0)),
            );
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit_query = true;
            }
            if ui
                .button(egui::RichText::new("⚡ Solve").strong().color(palette.accent_primary))
                .clicked()
            {
                submit_query = true;
            }
            if !self.qa_history.is_empty() && ui.button("🗑 Clear").clicked() {
                self.qa_history.clear();
            }
        });

        // Quick Q&A Example Chips
        ui.horizontal_wrapped(|ui| {
            ui.weak("Examples:");
            for sample in [
                "solve x^2 - 5*x + 6 = 0, x",
                "integrate x * sin(x), x",
                "diff exp(-x^2) * cos(3*x), x",
                "eigenvalues [[1, 2], [2, 1]]",
                "rref [[1, 2, 3], [4, 5, 6]]",
                "solve [2*x + y = 5, x - y = 1], [x, y]",
                "limit sin(x)/x as x -> 0",
            ] {
                if ui.small_button(sample).clicked() {
                    self.qa_input = sample.to_string();
                    submit_query = true;
                }
            }
        });

        if submit_query {
            let q = self.qa_input.trim().to_string();
            if !q.is_empty() {
                let mut ctx = ExecutionContext::new();
                if self.qa_use_notebook_params {
                    ctx.bindings = notebook.session.slider_values.clone();
                    for (name, meta) in &notebook.session.symbol_metadata {
                        if let Some(u) = meta.uncertainty {
                            ctx.uncertainties.insert(name.clone(), u);
                        }
                    }
                }
                let resp = QueryEngine::query(&notebook.graph, &q, &ctx);
                self.qa_history.insert(0, (q, resp));
            }
        }

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.qa_history.is_empty() {
                    ui.add_space(16.0);
                    ui.weak("Enter any mathematical query above or click a quick example to inspect symbolic results, step-by-step derivations, alternate forms, and companion analysis.");
                    return;
                }

                for (q_idx, (query_text, resp)) in self.qa_history.iter().enumerate() {
                    egui::Frame::NONE
                        .fill(palette.bg_card)
                        .stroke(egui::Stroke::new(1.0, palette.border))
                        .corner_radius(8)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Q: {}", query_text))
                                        .strong()
                                        .size(15.0)
                                        .color(palette.accent_secondary),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.weak(format!("{} ({} µs)", resp.intent, resp.execution_time_us));
                                    if ui
                                        .small_button("➕ Send to Notepad")
                                        .clicked()
                                    {
                                        action = WorkbenchAction::InsertToNotepad(query_text.clone());
                                    }
                                    if query_text.contains('=')
                                        && ui.small_button("⚗ Open in Systems").clicked()
                                    {
                                        action = WorkbenchAction::SwitchToEquations(vec![
                                            query_text.trim_start_matches("solve ").to_string(),
                                        ]);
                                    }
                                });
                            });
                            ui.separator();

                            for (pod_idx, pod) in resp.pods.iter().enumerate() {
                                let title_color = if pod.title == "Error" {
                                    palette.text_error
                                } else if pod.title == "Result" {
                                    palette.math_result
                                } else {
                                    palette.accent_primary
                                };
                                ui.label(egui::RichText::new(&pod.title).strong().color(title_color));
                                if !pod.subpods.is_empty() {
                                    egui::CollapsingHeader::new(format!(
                                        "{} ({} details)",
                                        pod.title,
                                        pod.subpods.len()
                                    ))
                                    .id_salt(format!("qa_pod_{q_idx}_{pod_idx}"))
                                    .default_open(pod.title == "Step-by-step solution")
                                    .show(ui, |ui| {
                                        for step in &pod.subpods {
                                            ui.monospace(format!("  • {}", step));
                                        }
                                    });
                                } else {
                                    ui.monospace(&pod.unicode);
                                    if !pod.latex.is_empty() && pod.latex != pod.unicode {
                                        ui.weak(format!("LaTeX: {}", pod.latex));
                                    }
                                }
                                ui.add_space(4.0);
                            }
                        });
                    ui.add_space(8.0);
                }
            });

        action
    }

    /// Render the Interactive Systems of Equations & Differential Equations Workbench.
    pub fn render_systems_workbench(
        &mut self,
        ui: &mut egui::Ui,
        palette: &ThemePalette,
        graph: &ExprGraph,
    ) -> WorkbenchAction {
        let mut action = WorkbenchAction::None;

        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("⚗ Systems & Differential Equations Workbench")
                    .color(palette.accent_primary),
            );
            ui.separator();
            ui.selectable_value(
                &mut self.sub_mode,
                WorkbenchSubMode::Equations,
                "📐 Algebraic Equations Systems",
            );
            ui.selectable_value(
                &mut self.sub_mode,
                WorkbenchSubMode::Differentials,
                "🌊 Coupled Differential Equations & Phase Plane",
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📝 Back to Notepad").clicked() {
                    action = WorkbenchAction::BackToNotepad;
                }
            });
        });
        ui.separator();

        match self.sub_mode {
            WorkbenchSubMode::Equations => {
                self.render_equations_workbench(ui, palette, graph, &mut action)
            }
            WorkbenchSubMode::Differentials => {
                self.render_differentials_workbench(ui, palette, graph, &mut action);
            }
        }

        action
    }

    fn render_equations_workbench(
        &mut self,
        ui: &mut egui::Ui,
        palette: &ThemePalette,
        graph: &ExprGraph,
        action: &mut WorkbenchAction,
    ) {
        let presets = WorkbenchPresets::equation_presets();
        let mut needs_eval = self.eq_analysis.is_none();

        ui.horizontal(|ui| {
            ui.label("Preset:");
            let cur_name = presets
                .get(self.eq_selected_preset)
                .map(|p| p.name.as_str())
                .unwrap_or("Custom");
            egui::ComboBox::from_id_salt("wb_eq_preset_combo")
                .selected_text(cur_name)
                .show_ui(ui, |ui| {
                    for (idx, p) in presets.iter().enumerate() {
                        if ui
                            .selectable_label(self.eq_selected_preset == idx, &p.name)
                            .clicked()
                        {
                            self.eq_selected_preset = idx;
                            self.eq_spec = p.clone();
                            needs_eval = true;
                        }
                    }
                });
            if ui.button("⚡ Solve & Analyze").clicked() {
                needs_eval = true;
            }
            if ui.button("➕ Export System to Notepad").clicked() {
                let snippet = format!(
                    "solve [{}], [{}]",
                    self.eq_spec.equations.join(", "),
                    self.eq_spec.variables.join(", ")
                );
                *action = WorkbenchAction::InsertToNotepad(snippet);
            }
        });
        ui.separator();

        ui.columns(2, |cols| {
            // Left Column: Setup & Live Parameter Sliders
            egui::ScrollArea::vertical()
                .id_salt("wb_eq_left")
                .auto_shrink([false, false])
                .show(&mut cols[0], |ui| {
                    ui.label(egui::RichText::new("Equations (F(x) = 0 or lhs = rhs)").strong());
                    let mut remove_eq_idx = None;
                    let eq_len = self.eq_spec.equations.len();
                    for (i, eq) in self.eq_spec.equations.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("E{}:", i + 1));
                            if ui
                                .add(egui::TextEdit::singleline(eq).desired_width(240.0))
                                .changed()
                            {
                                needs_eval = true;
                            }
                            if eq_len > 1 && ui.small_button("✕").clicked() {
                                remove_eq_idx = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove_eq_idx {
                        self.eq_spec.equations.remove(i);
                        needs_eval = true;
                    }
                    if ui.small_button("+ Add Equation").clicked() {
                        self.eq_spec.equations.push("x + y = 0".to_string());
                        needs_eval = true;
                    }

                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Unknown Variables (comma-separated)").strong());
                    let mut vars_str = self.eq_spec.variables.join(", ");
                    if ui.text_edit_singleline(&mut vars_str).changed() {
                        self.eq_spec.variables = vars_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        needs_eval = true;
                    }

                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Interactive Parameters (drag to alter system)").strong());
                    let mut rm_param = None;
                    for (idx, (pname, pval)) in self.eq_spec.parameters.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{pname} ="));
                            if ui
                                .add(egui::Slider::new(pval, -10.0..=10.0).step_by(0.05))
                                .changed()
                            {
                                needs_eval = true;
                            }
                            if ui.small_button("✕").clicked() {
                                rm_param = Some(idx);
                            }
                        });
                    }
                    if let Some(i) = rm_param {
                        self.eq_spec.parameters.remove(i);
                        needs_eval = true;
                    }
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.eq_new_param_name).desired_width(50.0));
                        ui.add(egui::DragValue::new(&mut self.eq_new_param_val).speed(0.1));
                        if ui.small_button("+ Parameter").clicked() {
                            let name = self.eq_new_param_name.trim().to_string();
                            if !name.is_empty() && !self.eq_spec.parameters.iter().any(|(k, _)| k == &name) {
                                self.eq_spec.parameters.push((name, self.eq_new_param_val));
                                needs_eval = true;
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("X Range:");
                        if ui.add(egui::DragValue::new(&mut self.eq_spec.x_range[0]).speed(0.2)).changed() {
                            needs_eval = true;
                        }
                        if ui.add(egui::DragValue::new(&mut self.eq_spec.x_range[1]).speed(0.2)).changed() {
                            needs_eval = true;
                        }
                        ui.label("Y Range:");
                        if ui.add(egui::DragValue::new(&mut self.eq_spec.y_range[0]).speed(0.2)).changed() {
                            needs_eval = true;
                        }
                        if ui.add(egui::DragValue::new(&mut self.eq_spec.y_range[1]).speed(0.2)).changed() {
                            needs_eval = true;
                        }
                    });

                    if needs_eval {
                        self.eq_analysis =
                            Some(WorkbenchEngine::analyze_equation_system(graph, &self.eq_spec));
                    }

                    if let Some(analysis) = &self.eq_analysis {
                        ui.separator();
                        egui::Frame::NONE
                            .fill(palette.bg_card)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                            .corner_radius(6)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                if let Some(err) = &analysis.error_msg {
                                    ui.colored_label(palette.text_error, err);
                                }
                                ui.label(egui::RichText::new("Solutions").strong().color(palette.math_result));
                                ui.monospace(&analysis.solution_text);
                                if !analysis.solution_latex.is_empty() {
                                    ui.weak(format!("LaTeX: {}", analysis.solution_latex));
                                }
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Symbolic Jacobian ∂F/∂x").strong().color(palette.accent_secondary));
                                ui.monospace(&analysis.jacobian_unicode);
                                if let Some(rank) = analysis.rank {
                                    ui.weak(format!("System Rank: {rank}"));
                                }
                                if !analysis.residuals.is_empty() {
                                    let res_str = analysis
                                        .residuals
                                        .iter()
                                        .map(|r| format!("{:.2e}", r))
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    ui.weak(format!("Verified Residuals ‖F(x*)‖∞: [{}]", res_str));
                                }
                            });
                    }
                });

            // Right Column: Implicit Zero-Contours & Solution Intersection Markers
            cols[1].vertical(|ui| {
                ui.label(egui::RichText::new("Geometric Contours & Intersections (F_i(x, y) = 0)").strong());
                if let Some(analysis) = &self.eq_analysis {
                    if self.eq_spec.variables.len() != 2 {
                        ui.weak("2D contour plotting is displayed when exactly 2 variables are specified.");
                    } else {
                        let curve_colors = [
                            egui::Color32::from_rgb(77, 159, 255),
                            egui::Color32::from_rgb(255, 130, 67),
                            egui::Color32::from_rgb(76, 209, 132),
                            egui::Color32::from_rgb(186, 130, 255),
                        ];
                        Plot::new("wb_eq_contour_plot")
                            .legend(Legend::default())
                            .allow_drag(true)
                            .allow_zoom(true)
                            .data_aspect(1.0)
                            .show(ui, |plot_ui| {
                                for (eq_idx, segs) in analysis.contours.iter().enumerate() {
                                    let color = curve_colors[eq_idx % curve_colors.len()];
                                    let label = self
                                        .eq_spec
                                        .equations
                                        .get(eq_idx)
                                        .cloned()
                                        .unwrap_or_else(|| format!("Eq {}", eq_idx + 1));
                                    for (s_idx, seg) in segs.iter().enumerate() {
                                        let pts = PlotPoints::from(vec![
                                            [seg.x0, seg.y0],
                                            [seg.x1, seg.y1],
                                        ]);
                                        let line = if s_idx == 0 {
                                            Line::new(label.clone(), pts)
                                        } else {
                                            Line::new("", pts)
                                        };
                                        plot_ui.line(line.color(color).width(2.0));
                                    }
                                }
                                if !analysis.solution_points.is_empty() {
                                    let sol_pts = PlotPoints::from(analysis.solution_points.clone());
                                    plot_ui.points(
                                        Points::new("Solved Intersections", sol_pts)
                                            .color(egui::Color32::from_rgb(255, 220, 60))
                                            .radius(6.0),
                                    );
                                }
                            });
                    }
                }
            });
        });
    }

    fn render_differentials_workbench(
        &mut self,
        ui: &mut egui::Ui,
        palette: &ThemePalette,
        graph: &ExprGraph,
        action: &mut WorkbenchAction,
    ) {
        let presets = WorkbenchPresets::differential_presets();
        let mut needs_eval = self.de_analysis.is_none();

        ui.horizontal_wrapped(|ui| {
            ui.label("Preset:");
            let cur_name = presets
                .get(self.de_selected_preset)
                .map(|p| p.name.as_str())
                .unwrap_or("Custom");
            egui::ComboBox::from_id_salt("wb_de_preset_combo")
                .selected_text(cur_name)
                .show_ui(ui, |ui| {
                    for (idx, p) in presets.iter().enumerate() {
                        if ui
                            .selectable_label(self.de_selected_preset == idx, &p.name)
                            .clicked()
                        {
                            self.de_selected_preset = idx;
                            self.de_spec = p.clone();
                            needs_eval = true;
                        }
                    }
                });
            ui.label("Integrator:");
            egui::ComboBox::from_id_salt("wb_de_method_combo")
                .selected_text(self.de_spec.method.as_str())
                .show_ui(ui, |ui| {
                    for m in ["auto", "tsit5", "dp5", "rk4", "radau5", "gl4", "meta"] {
                        if ui.selectable_label(self.de_spec.method == m, m).clicked() {
                            self.de_spec.method = m.to_string();
                            needs_eval = true;
                        }
                    }
                });
            ui.checkbox(&mut self.show_time_series, "Time Series y(t)");
            ui.checkbox(
                &mut self.show_phase_portrait,
                "Phase Portrait + Vector Field",
            );
            if ui.button("⚡ Integrate & Analyze").clicked() {
                needs_eval = true;
            }
            if ui.button("➕ Export to Notepad").clicked() {
                let first = format!(
                    "solve_ode {}' = {}, {}, {}",
                    self.de_spec
                        .state_vars
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "y".into()),
                    self.de_spec
                        .rhs_expressions
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "0".into()),
                    self.de_spec
                        .state_vars
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "y".into()),
                    self.de_spec.t_var
                );
                *action = WorkbenchAction::InsertToNotepad(first);
            }
        });
        ui.separator();

        ui.columns(2, |cols| {
            // Left Column: Coupled System Equations, Initial Conditions & Parameter Sliders
            egui::ScrollArea::vertical()
                .id_salt("wb_de_left")
                .auto_shrink([false, false])
                .show(&mut cols[0], |ui| {
                    ui.label(egui::RichText::new("Coupled 1st-Order System dy_i / dt = f_i(...)").strong());
                    let mut rm_idx = None;
                    let dim = self.de_spec.state_vars.len();
                    for i in 0..dim {
                        ui.horizontal(|ui| {
                            ui.label("Var:");
                            if ui
                                .add(egui::TextEdit::singleline(&mut self.de_spec.state_vars[i]).desired_width(48.0))
                                .changed()
                            {
                                needs_eval = true;
                            }
                            ui.label("' =");
                            if ui
                                .add(egui::TextEdit::singleline(&mut self.de_spec.rhs_expressions[i]).desired_width(180.0))
                                .changed()
                            {
                                needs_eval = true;
                            }
                            ui.label("y₀:");
                            if ui
                                .add(egui::DragValue::new(&mut self.de_spec.y0[i]).speed(0.05))
                                .changed()
                            {
                                needs_eval = true;
                            }
                            if dim > 1 && ui.small_button("✕").clicked() {
                                rm_idx = Some(i);
                            }
                        });
                    }
                    if let Some(i) = rm_idx {
                        self.de_spec.state_vars.remove(i);
                        self.de_spec.rhs_expressions.remove(i);
                        self.de_spec.y0.remove(i);
                        needs_eval = true;
                    }
                    if ui.small_button("+ Add State Equation").clicked() {
                        let next_name = format!("y{}", self.de_spec.state_vars.len() + 1);
                        self.de_spec.state_vars.push(next_name);
                        self.de_spec.rhs_expressions.push("0".to_string());
                        self.de_spec.y0.push(0.0);
                        needs_eval = true;
                    }

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("t:");
                        if ui
                            .add(egui::TextEdit::singleline(&mut self.de_spec.t_var).desired_width(30.0))
                            .changed()
                        {
                            needs_eval = true;
                        }
                        ui.label("t₀:");
                        if ui.add(egui::DragValue::new(&mut self.de_spec.t_span[0]).speed(0.1)).changed() {
                            needs_eval = true;
                        }
                        ui.label("t₁:");
                        if ui.add(egui::DragValue::new(&mut self.de_spec.t_span[1]).speed(0.25)).changed() {
                            needs_eval = true;
                        }
                    });

                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Interactive Parameters (drag to alter dynamics)").strong());
                    let mut rm_p = None;
                    for (idx, (pname, pval)) in self.de_spec.parameters.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{pname} ="));
                            let max_lim = (pval.abs() * 2.5).max(10.0);
                            if ui
                                .add(egui::Slider::new(pval, -max_lim..=max_lim).step_by(0.02))
                                .changed()
                            {
                                needs_eval = true;
                            }
                            if ui.small_button("✕").clicked() {
                                rm_p = Some(idx);
                            }
                        });
                    }
                    if let Some(i) = rm_p {
                        self.de_spec.parameters.remove(i);
                        needs_eval = true;
                    }
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.de_new_param_name).desired_width(50.0));
                        ui.add(egui::DragValue::new(&mut self.de_new_param_val).speed(0.1));
                        if ui.small_button("+ Parameter").clicked() {
                            let name = self.de_new_param_name.trim().to_string();
                            if !name.is_empty() && !self.de_spec.parameters.iter().any(|(k, _)| k == &name) {
                                self.de_spec.parameters.push((name, self.de_new_param_val));
                                needs_eval = true;
                            }
                        }
                    });

                    if self.de_spec.state_vars.len() >= 2 {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Phase Axes:");
                            let max_ax = self.de_spec.state_vars.len().saturating_sub(1);
                            if ui.add(egui::DragValue::new(&mut self.de_spec.phase_axes[0]).range(0..=max_ax)).changed() {
                                needs_eval = true;
                            }
                            ui.label("vs");
                            if ui.add(egui::DragValue::new(&mut self.de_spec.phase_axes[1]).range(0..=max_ax)).changed() {
                                needs_eval = true;
                            }
                        });
                    }

                    if needs_eval {
                        self.de_analysis =
                            Some(WorkbenchEngine::analyze_differential_system(graph, &self.de_spec));
                    }

                    if let Some(analysis) = &self.de_analysis {
                        ui.separator();
                        egui::Frame::NONE
                            .fill(palette.bg_card)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                            .corner_radius(6)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                if let Some(err) = &analysis.error_msg {
                                    ui.colored_label(palette.text_error, err);
                                }
                                if let Some(sym_sol) = &analysis.symbolic_solution_text {
                                    ui.label(egui::RichText::new("Closed-Form ODE Solution").strong().color(palette.math_result));
                                    ui.monospace(sym_sol);
                                    ui.add_space(4.0);
                                }
                                ui.label(egui::RichText::new("Linearized Stability & Eigenvalues (at y₀)").strong().color(palette.accent_secondary));
                                ui.colored_label(palette.accent_primary, &analysis.stability_label);
                                ui.monospace(format!("λ = {}", analysis.eigenvalues_text));
                                ui.monospace(format!("J = {}", analysis.jacobian_unicode));
                                ui.weak(format!(
                                    "Solver: {} | {} accepted / {} rejected steps | Final state: [{}]",
                                    analysis.method_used,
                                    analysis.steps_accepted,
                                    analysis.steps_rejected,
                                    analysis
                                        .final_state
                                        .iter()
                                        .map(|v| format!("{:.4}", v))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ));
                            });
                    }
                });

            // Right Column: Trajectory & Phase Portrait / Vector Field Plots
            cols[1].vertical(|ui| {
                if let Some(analysis) = &self.de_analysis {
                    let colors = [
                        egui::Color32::from_rgb(77, 159, 255),
                        egui::Color32::from_rgb(255, 130, 67),
                        egui::Color32::from_rgb(76, 209, 132),
                        egui::Color32::from_rgb(240, 198, 116),
                    ];
                    let avail_h = ui.available_height();
                    let both = self.show_time_series && self.show_phase_portrait && self.de_spec.state_vars.len() >= 2;
                    let plot_h = if both { (avail_h * 0.46).max(180.0) } else { avail_h.max(260.0) };

                    if self.show_time_series {
                        ui.label(egui::RichText::new("Time-Domain Trajectories y_i(t)").strong());
                        Plot::new("wb_de_time_plot")
                            .height(plot_h)
                            .legend(Legend::default())
                            .show(ui, |plot_ui| {
                                for (idx, traj) in analysis.trajectories.iter().enumerate() {
                                    let pts: PlotPoints = analysis
                                        .t
                                        .iter()
                                        .zip(traj.iter())
                                        .map(|(&ti, &yi)| [ti, yi])
                                        .collect();
                                    let label = self
                                        .de_spec
                                        .state_vars
                                        .get(idx)
                                        .cloned()
                                        .unwrap_or_else(|| format!("y{}", idx + 1));
                                    plot_ui.line(
                                        Line::new(label, pts)
                                            .color(colors[idx % colors.len()])
                                            .width(2.0),
                                    );
                                }
                            });
                    }

                    if self.show_phase_portrait && self.de_spec.state_vars.len() >= 2 {
                        let ax_x = self.de_spec.phase_axes[0].min(analysis.trajectories.len() - 1);
                        let ax_y = self.de_spec.phase_axes[1].min(analysis.trajectories.len() - 1);
                        ui.label(egui::RichText::new(format!(
                            "Phase Portrait ({} vs {}) & Direction Field",
                            self.de_spec.state_vars[ax_x],
                            self.de_spec.state_vars[ax_y]
                        )).strong());
                        Plot::new("wb_de_phase_plot")
                            .height(plot_h)
                            .legend(Legend::default())
                            .show(ui, |plot_ui| {
                                let dx_scale = (self.de_spec.phase_x_range[1] - self.de_spec.phase_x_range[0]).abs() * 0.045;
                                let dy_scale = (self.de_spec.phase_y_range[1] - self.de_spec.phase_y_range[0]).abs() * 0.045;
                                for arrow in &analysis.quiver_field {
                                    let pts = PlotPoints::from(vec![
                                        [arrow.x, arrow.y],
                                        [arrow.x + arrow.dx * dx_scale, arrow.y + arrow.dy * dy_scale],
                                    ]);
                                    plot_ui.line(
                                        Line::new("", pts)
                                            .color(egui::Color32::from_rgba_unmultiplied(150, 175, 200, 130))
                                            .width(1.2),
                                    );
                                }
                                let orbit: PlotPoints = analysis.trajectories[ax_x]
                                    .iter()
                                    .zip(analysis.trajectories[ax_y].iter())
                                    .map(|(&x, &y)| [x, y])
                                    .collect();
                                plot_ui.line(
                                    Line::new("Phase Orbit", orbit)
                                        .color(egui::Color32::from_rgb(255, 166, 48))
                                        .width(2.3),
                                );
                                if let (Some(&x0), Some(&y0)) = (
                                    analysis.trajectories[ax_x].first(),
                                    analysis.trajectories[ax_y].first(),
                                ) {
                                    plot_ui.points(
                                        Points::new("Initial y₀", PlotPoints::from(vec![[x0, y0]]))
                                            .color(egui::Color32::from_rgb(90, 235, 140))
                                            .radius(5.0),
                                    );
                                }
                            });
                    }
                }
            });
        });
    }
}
