//! # `urae_notebook::ui::command_palette`
//!
//! Keyboard-Driven Command Palette (`Cmd+K` / `Ctrl+K`) for Searchable Math Operations, Greek Symbols & CAD Generators.

use eframe::egui;

/// Item in the command palette.
#[derive(Debug, Clone)]
pub struct CommandItem {
    pub category: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub syntax_template: &'static str,
    pub shortcut: Option<&'static str>,
}

/// Command Palette State.
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub is_open: bool,
    pub search_query: String,
    pub selected_index: usize,
}

pub struct CommandPalette;

impl CommandPalette {
    /// List of all built-in searchable command templates.
    pub fn all_commands() -> Vec<CommandItem> {
        vec![
            // 1. Calculus & Solvers
            CommandItem {
                category: "Calculus",
                title: "Differentiate: d/dx f(x)",
                description: "Symbolic differentiation with respect to variable",
                syntax_template: "diff(x^3 + 2*x, x)",
                shortcut: Some("diff"),
            },
            CommandItem {
                category: "Calculus",
                title: "Integrate: ∫ f(x) dx",
                description: "Symbolic Risch integration",
                syntax_template: "integrate(x * exp(x), x)",
                shortcut: Some("int"),
            },
            CommandItem {
                category: "Calculus",
                title: "Taylor / Laurent Series",
                description: "Asymptotic expansion around x0",
                syntax_template: "series(sin(x), x, 0, 6)",
                shortcut: Some("series"),
            },
            CommandItem {
                category: "Solvers",
                title: "Natural Language Equation Solve",
                description: "Solves algebraic or transcendental equations",
                syntax_template: "solve x^2 - 4 = 0 for x",
                shortcut: Some("solve"),
            },
            CommandItem {
                category: "Solvers",
                title: "Initial Value Problem ODE",
                description: "Solves 1st/2nd order ODEs with initial conditions",
                syntax_template: "solve_ode(y'' + y = 0, y(0)=1, y'(0)=0)",
                shortcut: Some("ode"),
            },
            // 2. CAD & Machinery Generators
            CommandItem {
                category: "CAD Machinery",
                title: "Involute Spur / Helical Gear",
                description: "Parametric 3D involute gear mesh generator",
                syntax_template: "gear!(teeth = 16, module = 2.0, pressure_angle = 20.0)",
                shortcut: Some("gear"),
            },
            CommandItem {
                category: "CAD Machinery",
                title: "Threaded Fastener / ISO Bolt",
                description: "Parametric ISO 60° metric threaded screw with hex head",
                syntax_template: "screw!(dia = 8.0, pitch = 1.25, len = 20.0, head = Hex)",
                shortcut: Some("screw"),
            },
            CommandItem {
                category: "CAD Machinery",
                title: "NACA 4-Digit Airfoil & Wing",
                description: "NACA 2412 cambered aerodynamic wing lofting",
                syntax_template: "airfoil!(code = \"2412\", chord = 10.0, span = 25.0)",
                shortcut: Some("airfoil"),
            },
            CommandItem {
                category: "CAD Machinery",
                title: "Helical Coil Spring",
                description: "Compression spring with wire diameter & pitch",
                syntax_template: "spring!(mean_dia = 8.0, wire_dia = 1.2, pitch = 3.0, coils = 6)",
                shortcut: Some("spring"),
            },
            // 3. FEA Multiphysics & Simulation
            CommandItem {
                category: "Simulation",
                title: "Linear Elasticity FEA",
                description: "2D/3D Stress & displacement variational solver",
                syntax_template: "fea_elasticity!(mesh, E = 210000.0, nu = 0.3, bcs = &bcs)",
                shortcut: Some("fea"),
            },
            CommandItem {
                category: "Simulation",
                title: "Heat Conduction FEA",
                description: "Steady-state Poisson heat transfer with convection",
                syntax_template: "fea_heat!(mesh, k = 45.0, Q = 0.0, bcs = &heat_bcs)",
                shortcut: Some("heat"),
            },
            CommandItem {
                category: "Simulation",
                title: "End-to-End CAD-to-FEA Pipeline",
                description: "Simulate mechanical deformation directly on CAD mesh",
                syntax_template: "simulate_cad!(gear_mesh, physics, bcs)",
                shortcut: Some("sim"),
            },
            // 4. Mathematical & Greek Symbols
            CommandItem {
                category: "Symbols",
                title: "Greek: α (Alpha)",
                description: "Greek lowercase alpha",
                syntax_template: "α",
                shortcut: Some("alpha"),
            },
            CommandItem {
                category: "Symbols",
                title: "Greek: β (Beta)",
                description: "Greek lowercase beta",
                syntax_template: "β",
                shortcut: Some("beta"),
            },
            CommandItem {
                category: "Symbols",
                title: "Greek: γ, Γ (Gamma)",
                description: "Greek gamma / Gamma function",
                syntax_template: "γ",
                shortcut: Some("gamma"),
            },
            CommandItem {
                category: "Symbols",
                title: "Greek: θ (Theta)",
                description: "Angle / polar coordinate theta",
                syntax_template: "θ",
                shortcut: Some("theta"),
            },
            CommandItem {
                category: "Symbols",
                title: "Operator: ∇ (Nabla / Gradient)",
                description: "Vector differential del/nabla operator",
                syntax_template: "∇",
                shortcut: Some("grad"),
            },
            CommandItem {
                category: "Symbols",
                title: "Operator: ∂ (Partial Derivative)",
                description: "Partial differentiation symbol",
                syntax_template: "∂",
                shortcut: Some("partial"),
            },
            CommandItem {
                category: "Symbols",
                title: "Number Sets: ℝ, ℂ, ℤ, ℍ",
                description: "Real, Complex, Integer, Quaternion domains",
                syntax_template: "{ x in Reals | x >= 0 }",
                shortcut: Some("reals"),
            },
            // 5. Example Notebooks & Tutorials
            CommandItem {
                category: "Examples",
                title: "Example: Advanced Calculus & ODEs",
                description: "Symbolic differentiation, Risch integration, limits, damped oscillator",
                syntax_template: "# Calculus Example\nf(x) = 2.5 * exp(-1.2 * x) * cos(3.14 * x)\ndiff(f(x), x)\nintegrate(x^3 * exp(x), x)\n",
                shortcut: Some("ex:calc"),
            },
            CommandItem {
                category: "Examples",
                title: "Example: Quantum Field Theory & Spinors",
                description: "Dirac gamma traces, Weyl spinor helicity, Parke-Taylor amplitudes",
                syntax_template: "# QFT Example\ndirac_trace([0, 1, 0, 1])\nspinor_bracket_angle(1, 2)\nparke_taylor_mhv(4, [1, 2])\n",
                shortcut: Some("ex:qft"),
            },
            CommandItem {
                category: "Examples",
                title: "Example: Parametric CAD & IGA",
                description: "Involute gears, metric screws, NACA airfoils, B-spline IGA stiffness",
                syntax_template: "# CAD & IGA Example\ngear!(module = 2.0, teeth = 18, width = 10.0)\nscrew!(dia = 10.0, pitch = 1.5, length = 25.0)\nairfoil!(code = \"2412\", chord = 10.0, span = 25.0)\n",
                shortcut: Some("ex:cad"),
            },
            CommandItem {
                category: "Examples",
                title: "Example: Exceptional Lie & Octonions",
                description: "8D non-associative Octonions, Albert Jordan algebra, E8 roots",
                syntax_template: "# Octonions Example\nx = octonion!(1, 0, 1, 0, 0, 0, 0, 0)\ny = octonion!(0, 1, 0, 1, 0, 0, 0, 0)\nz = octonion!(0, 0, 1, 0, 1, 0, 0, 0)\noctonion_associator!(x, y, z)\n",
                shortcut: Some("ex:oct"),
            },
            CommandItem {
                category: "Examples",
                title: "Example: Holonomic D-Modules & Zeilberger",
                description: "Weyl algebra A_n, [d, x] = 1, automated binomial summation proofs",
                syntax_template: "# Weyl D-Modules Example\nx = weyl_x!(1, 0, 1)\nd = weyl_d!(1, 0, 1)\nweyl_commute!(d, x)\nzeilberger_binomial_proof!()\n",
                shortcut: Some("ex:weyl"),
            },
            // 6. Document Actions & History
            CommandItem {
                category: "Edit",
                title: "Undo: Revert Last Action",
                description: "Reverts the most recent document, parameter, or palette modification",
                syntax_template: "__URAE_CMD_UNDO__",
                shortcut: Some("undo"),
            },
            CommandItem {
                category: "Edit",
                title: "Redo: Reapply Undone Action",
                description: "Re-applies the next action from the forward history stack",
                syntax_template: "__URAE_CMD_REDO__",
                shortcut: Some("redo"),
            },
            // 7. Themes & Appearance
            CommandItem {
                category: "Themes",
                title: "Theme: Charcoal Dark",
                description: "Deep charcoal background with vibrant math syntax colors",
                syntax_template: "__URAE_CMD_THEME_DARK__",
                shortcut: Some("theme:dark"),
            },
            CommandItem {
                category: "Themes",
                title: "Theme: Slate Paper Light",
                description: "Clean bright paper background with high-contrast text",
                syntax_template: "__URAE_CMD_THEME_LIGHT__",
                shortcut: Some("theme:light"),
            },
            CommandItem {
                category: "Themes",
                title: "Theme: Catppuccin Mocha",
                description: "Soothing warm pastel dark theme",
                syntax_template: "__URAE_CMD_THEME_CATPPUCCIN__",
                shortcut: Some("theme:catppuccin"),
            },
            CommandItem {
                category: "Themes",
                title: "Theme: Nord Arctic",
                description: "Arctic slate and ice-blue mathematical theme",
                syntax_template: "__URAE_CMD_THEME_NORD__",
                shortcut: Some("theme:nord"),
            },
            CommandItem {
                category: "Themes",
                title: "Theme: Dracula Neon",
                description: "High-contrast gothic neon color palette",
                syntax_template: "__URAE_CMD_THEME_DRACULA__",
                shortcut: Some("theme:dracula"),
            },
            CommandItem {
                category: "Themes",
                title: "Theme: Solarized Dark",
                description: "Mathematically balanced low-strain dark palette",
                syntax_template: "__URAE_CMD_THEME_SOLARIZED_DARK__",
                shortcut: Some("theme:solarized-dark"),
            },
            CommandItem {
                category: "Themes",
                title: "Theme: Solarized Light",
                description: "Balanced warm paper low-strain light palette",
                syntax_template: "__URAE_CMD_THEME_SOLARIZED_LIGHT__",
                shortcut: Some("theme:solarized-light"),
            },
            CommandItem {
                category: "Themes",
                title: "Theme: Cyberpunk Matrix",
                description: "Neon emerald and electric magenta high-tech terminal theme",
                syntax_template: "__URAE_CMD_THEME_CYBERPUNK__",
                shortcut: Some("theme:cyberpunk"),
            },
            // 8. Workspace Layouts
            CommandItem {
                category: "Layout",
                title: "Layout: Fluid Inline Notepad",
                description: "Single unified document with inline plots and parameter scrubbers",
                syntax_template: "__URAE_CMD_LAYOUT_FLUID__",
                shortcut: Some("layout:fluid"),
            },
            CommandItem {
                category: "Layout",
                title: "Layout: Dual Split View",
                description: "Split editor on left, 2D plots and 3D visualizers on right",
                syntax_template: "__URAE_CMD_LAYOUT_DUAL__",
                shortcut: Some("layout:dual"),
            },
            CommandItem {
                category: "Layout",
                title: "Layout: Triple IDE",
                description: "IDE layout with editor, plots visualizer, and bottom CLI terminal",
                syntax_template: "__URAE_CMD_LAYOUT_TRIPLE__",
                shortcut: Some("layout:triple"),
            },
            CommandItem {
                category: "Layout",
                title: "Layout: Presentation Report",
                description: "Full-page clean report view with print / HTML export",
                syntax_template: "__URAE_CMD_LAYOUT_PRESENTATION__",
                shortcut: Some("layout:presentation"),
            },
            CommandItem {
                category: "Layout",
                title: "Layout: Dependency Graph",
                description: "Full-page view of how the document's lines depend on each other",
                syntax_template: "__URAE_CMD_LAYOUT_GRAPH__",
                shortcut: Some("layout:graph"),
            },
            CommandItem {
                category: "Layout",
                title: "Layout: Zen Mode",
                description: "Distraction-free fullscreen mathematical workspace",
                syntax_template: "__URAE_CMD_LAYOUT_ZEN__",
                shortcut: Some("layout:zen"),
            },
        ]
    }

    /// Render the searchable Command Palette modal. Returns `Some(inserted_syntax)` if a command is selected.
    pub fn show(ctx: &egui::Context, state: &mut CommandPaletteState) -> Option<String> {
        if !state.is_open {
            return None;
        }

        let mut selected_syntax = None;
        let mut close_requested = false;

        egui::Window::new("Command Palette (Cmd+K)")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .collapsible(false)
            .resizable(false)
            .fixed_size([540.0, 380.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔍").size(18.0));
                    let text_response = ui.add(
                        egui::TextEdit::singleline(&mut state.search_query)
                            .hint_text("Type a math command, symbol, CAD generator, or formula...")
                            .desired_width(f32::INFINITY),
                    );
                    if text_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        close_requested = true;
                    }
                    text_response.request_focus();
                });

                ui.separator();

                let all_cmds = Self::all_commands();
                let query = state.search_query.to_lowercase();
                let filtered: Vec<&CommandItem> = all_cmds
                    .iter()
                    .filter(|cmd| {
                        query.is_empty()
                            || cmd.title.to_lowercase().contains(&query)
                            || cmd.category.to_lowercase().contains(&query)
                            || cmd.description.to_lowercase().contains(&query)
                            || cmd
                                .shortcut
                                .is_some_and(|s| s.to_lowercase().contains(&query))
                    })
                    .collect();

                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.weak("No matching commands found. Type another keyword.");
                        } else {
                            for (idx, cmd) in filtered.iter().enumerate() {
                                let is_selected = idx == state.selected_index;
                                let text_color = if is_selected {
                                    egui::Color32::from_rgb(56, 189, 248)
                                } else {
                                    egui::Color32::from_rgb(226, 232, 240)
                                };

                                let mut frame = egui::Frame::NONE.inner_margin(6);
                                if is_selected {
                                    frame = frame
                                        .fill(egui::Color32::from_rgb(30, 41, 59))
                                        .corner_radius(4);
                                }

                                let resp = frame
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(148, 163, 184),
                                                format!("[{}]", cmd.category),
                                            );
                                            ui.label(
                                                egui::RichText::new(cmd.title)
                                                    .strong()
                                                    .color(text_color),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if let Some(sc) = cmd.shortcut {
                                                        ui.weak(format!(":{sc}"));
                                                    }
                                                },
                                            );
                                        });
                                        ui.weak(cmd.description);
                                    })
                                    .response;

                                if resp.clicked() {
                                    selected_syntax = Some(cmd.syntax_template.to_string());
                                    close_requested = true;
                                }
                            }
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.weak("Press Enter to insert, Esc to close");
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });

        if close_requested {
            state.is_open = false;
            state.search_query.clear();
        }

        selected_syntax
    }
}
