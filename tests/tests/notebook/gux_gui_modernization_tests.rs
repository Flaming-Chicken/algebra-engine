//! Tests for GUX & GUI Modernization: ThemeKind, WorkspaceState, CellBlock, and SymbolInfoCard.

use urae_notebook::notebook::{CellBlock, CellBlockKind, NotebookState, SymbolInfoCard};
use urae_notebook::ui::{ThemeKind, WorkspaceLayoutPreset, WorkspaceState, WorkspaceTab};

// ─── ThemeKind Tests ────────────────────────────────────────────────

#[test]
fn test_theme_kind_all_returns_all_variants() {
    let all = ThemeKind::all();
    assert!(
        all.len() >= 8,
        "Expected at least 8 theme variants, got {}",
        all.len()
    );

    // Verify unique names
    let names: Vec<&str> = all.iter().map(|t| t.name()).collect();
    let mut deduped = names.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(names.len(), deduped.len(), "Theme names must be unique");
}

#[test]
fn test_theme_kind_name_and_round_trip() {
    let dark = ThemeKind::Dark;
    assert!(
        dark.name().contains("Charcoal Dark"),
        "Dark theme name should contain 'Charcoal Dark', got: {}",
        dark.name()
    );

    let light = ThemeKind::Light;
    assert!(
        light.name().contains("Slate Paper"),
        "Light theme name should contain 'Slate Paper', got: {}",
        light.name()
    );

    let catppuccin = ThemeKind::CatppuccinMocha;
    assert!(
        catppuccin.name().contains("Catppuccin Mocha"),
        "Catppuccin theme name should contain 'Catppuccin Mocha', got: {}",
        catppuccin.name()
    );
}

#[test]
fn test_theme_kind_palette_has_valid_colors() {
    for theme in ThemeKind::all() {
        let palette = theme.palette();
        assert!(
            !palette.plot_palette.is_empty(),
            "Theme {:?} should have plot colors",
            theme
        );
        // Verify variable_color is not fully transparent (alpha > 0)
        assert!(
            palette.math_var.a() > 0,
            "math_var should not be transparent"
        );
    }
}

#[test]
fn test_theme_kind_to_style_produces_valid_egui_style() {
    for theme in ThemeKind::all() {
        let style = theme.to_style();
        // Rounded corners should be applied
        assert!(
            style.visuals.widgets.noninteractive.corner_radius.nw >= 4,
            "Theme {:?} should have >= 4px rounding",
            theme
        );
    }
}

#[test]
fn test_theme_kind_equality_and_copy() {
    let a = ThemeKind::Nord;
    let b = a; // Copy
    assert_eq!(a, b);
    assert_ne!(ThemeKind::Dark, ThemeKind::Light);
}

// ─── WorkspaceState Tests ───────────────────────────────────────────

#[test]
fn test_workspace_state_default() {
    let ws = WorkspaceState::default();
    assert_eq!(ws.layout_preset, WorkspaceLayoutPreset::DualHalves);
    assert_eq!(ws.active_tab, WorkspaceTab::MathDocument);
    assert!(
        !ws.pinned_tabs.is_empty(),
        "Default workspace should have pinned tabs"
    );
    assert!(ws.pinned_tabs.contains(&WorkspaceTab::MathDocument));
}

#[test]
fn test_workspace_set_layout_dual_halves() {
    let mut ws = WorkspaceState::default();
    ws.set_layout(WorkspaceLayoutPreset::DualHalves);
    assert_eq!(ws.layout_preset, WorkspaceLayoutPreset::DualHalves);
    assert_eq!(ws.split_ratio, 0.50);
}

#[test]
fn test_workspace_set_layout_fluid() {
    let mut ws = WorkspaceState::default();
    ws.set_layout(WorkspaceLayoutPreset::FluidNotepad);
    assert_eq!(ws.layout_preset, WorkspaceLayoutPreset::FluidNotepad);
    assert!(ws.show_sidebar);
    assert!(!ws.show_bottom_panel);
}

#[test]
fn test_workspace_set_layout_split_dual() {
    let mut ws = WorkspaceState::default();
    ws.set_layout(WorkspaceLayoutPreset::SplitDual);
    assert_eq!(ws.layout_preset, WorkspaceLayoutPreset::SplitDual);
    assert!(ws.show_sidebar);
    assert_eq!(ws.split_ratio, 0.50);
}

#[test]
fn test_workspace_set_layout_triple_ide() {
    let mut ws = WorkspaceState::default();
    ws.set_layout(WorkspaceLayoutPreset::TripleIDE);
    assert_eq!(ws.layout_preset, WorkspaceLayoutPreset::TripleIDE);
    assert!(ws.show_sidebar);
    assert!(ws.show_bottom_panel);
}

#[test]
fn test_workspace_set_layout_zen_mode() {
    let mut ws = WorkspaceState::default();
    ws.set_layout(WorkspaceLayoutPreset::ZenMode);
    assert_eq!(ws.layout_preset, WorkspaceLayoutPreset::ZenMode);
    assert!(!ws.show_sidebar, "Zen mode hides sidebar");
    assert!(!ws.show_bottom_panel, "Zen mode hides bottom panel");
    assert_eq!(ws.split_ratio, 1.0, "Zen mode uses full canvas");
}

#[test]
fn test_workspace_layout_preset_name() {
    assert!(WorkspaceLayoutPreset::FluidNotepad.name().contains("Fluid"));
    assert!(WorkspaceLayoutPreset::SplitDual.name().contains("Dual"));
    assert!(WorkspaceLayoutPreset::TripleIDE.name().contains("Triple"));
    assert!(WorkspaceLayoutPreset::ZenMode.name().contains("Zen"));
}

#[test]
fn test_workspace_select_tab() {
    let mut ws = WorkspaceState::default();
    ws.select_tab(WorkspaceTab::Viewport3D);
    assert_eq!(ws.active_tab, WorkspaceTab::Viewport3D);
}

#[test]
fn test_workspace_toggle_sidebar() {
    let mut ws = WorkspaceState::default();
    let initial = ws.show_sidebar;
    ws.toggle_sidebar();
    assert_ne!(ws.show_sidebar, initial);
    ws.toggle_sidebar();
    assert_eq!(ws.show_sidebar, initial);
}

// ─── CellBlock Tests ────────────────────────────────────────────────

#[test]
fn test_cell_block_math_creation() {
    let block = CellBlock {
        id: "cell_0".to_string(),
        content: "f(x) = x^2 + 1".to_string(),
        line_idx: 0,
        kind: CellBlockKind::Math,
    };
    assert_eq!(block.line_idx, 0);
    assert!(matches!(block.kind, CellBlockKind::Math));
    assert_eq!(block.content, "f(x) = x^2 + 1");
}

#[test]
fn test_cell_block_markdown_creation() {
    let block = CellBlock {
        id: "cell_1".to_string(),
        content: "# Section Title".to_string(),
        line_idx: 1,
        kind: CellBlockKind::Markdown,
    };
    assert!(matches!(block.kind, CellBlockKind::Markdown));
}

#[test]
fn test_cell_block_section_header_creation() {
    let block = CellBlock {
        id: "cell_2".to_string(),
        content: "## Quadratic Analysis".to_string(),
        line_idx: 2,
        kind: CellBlockKind::SectionHeader {
            level: 2,
            collapsed: false,
        },
    };
    if let CellBlockKind::SectionHeader { level, collapsed } = block.kind {
        assert_eq!(level, 2);
        assert!(!collapsed);
    } else {
        panic!("Expected SectionHeader");
    }
}

// ─── NotebookState Cell Block Methods Tests ─────────────────────────

#[test]
fn test_notebook_get_cell_blocks() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "f(x) = x^2\n# Note\ng(x) = sin(x)".to_string();
    state.evaluate_all();
    let blocks = state.get_cell_blocks();
    assert_eq!(blocks.len(), 3);
}

#[test]
fn test_notebook_move_line_up() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "a = 1\nb = 2\nc = 3".to_string();
    state.evaluate_all();

    assert!(state.move_line_up(1));
    let lines: Vec<&str> = state.session.raw_document_text.lines().collect();
    assert_eq!(lines[0], "b = 2");
    assert_eq!(lines[1], "a = 1");
    assert_eq!(lines[2], "c = 3");
}

#[test]
fn test_notebook_move_line_up_at_zero_is_noop() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "a = 1\nb = 2".to_string();
    state.evaluate_all();
    assert!(!state.move_line_up(0));
}

#[test]
fn test_notebook_move_line_down() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "a = 1\nb = 2\nc = 3".to_string();
    state.evaluate_all();

    assert!(state.move_line_down(0));
    let lines: Vec<&str> = state.session.raw_document_text.lines().collect();
    assert_eq!(lines[0], "b = 2");
    assert_eq!(lines[1], "a = 1");
    assert_eq!(lines[2], "c = 3");
}

#[test]
fn test_notebook_move_line_down_at_last_is_noop() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "a = 1\nb = 2".to_string();
    state.evaluate_all();
    assert!(!state.move_line_down(1));
}

#[test]
fn test_notebook_delete_line() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "a = 1\nb = 2\nc = 3".to_string();
    state.evaluate_all();

    assert!(state.delete_line(1));
    let lines: Vec<&str> = state.session.raw_document_text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "a = 1");
    assert_eq!(lines[1], "c = 3");
}

#[test]
fn test_notebook_duplicate_line() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "a = 1\nb = 2".to_string();
    state.evaluate_all();

    assert!(state.duplicate_line(0));
    let lines: Vec<&str> = state.session.raw_document_text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "a = 1");
    assert_eq!(lines[1], "a = 1");
    assert_eq!(lines[2], "b = 2");
}

// ─── SymbolInfoCard Tests ───────────────────────────────────────────

#[test]
fn test_symbol_info_card_structure() {
    let card = SymbolInfoCard {
        name: "x".to_string(),
        role: urae_notebook::notebook::SymbolRole::Variable,
        compound_tags: vec!["Variable".to_string(), "Scalar".to_string()],
        cur_val: std::f64::consts::PI,
        domain_type: "Real".to_string(),
        unit_str: Some("m/s".to_string()),
        dependent_lines: vec![0, 2, 5],
        formula_references: vec!["f(x) = x^2".to_string()],
        object_kind: Some(urae_notebook::notebook::ObjectKind::PhysicalQuantity {
            dimension: "Velocity".to_string(),
            unit: "m/s".to_string(),
            magnitude: std::f64::consts::PI,
        }),
        computation_time_ms: Some(1.23),
        ast_depth: Some(3),
        proof_search_depth: Some(2),
        neural_embedding: Some(vec![0.1, 0.2, -0.3]),
    };
    assert_eq!(card.name, "x");
    assert_eq!(card.compound_tags.len(), 2);
    assert_eq!(card.dependent_lines.len(), 3);
    assert_eq!(card.formula_references.len(), 1);
    assert_eq!(card.ast_depth, Some(3));
    assert_eq!(card.proof_search_depth, Some(2));
    assert!(card.neural_embedding.is_some());
    assert!(card.unit_str.is_some());
    assert!(matches!(
        card.object_kind,
        Some(urae_notebook::notebook::ObjectKind::PhysicalQuantity { .. })
    ));
}

#[test]
fn test_get_symbol_info_card_returns_none_for_unknown() {
    let state = NotebookState::default();
    assert!(state.get_symbol_info_card("nonexistent_var").is_none());
}

#[test]
fn test_symbol_inspector_classifies_constants_and_line_results() {
    let mut state = NotebookState::default();
    state.session.raw_document_text = "# Header\n// Note comment\nx = 10\ny = x + 5".to_string();
    state.evaluate_all();

    // 1. Check pi constant classification
    let pi_card = state
        .get_symbol_info_card("pi")
        .expect("pi should be classified");
    assert_eq!(pi_card.name, "π");
    assert!(matches!(
        pi_card.object_kind,
        Some(urae_notebook::notebook::ObjectKind::Constant { .. })
    ));

    // 2. Check e constant classification
    let e_card = state
        .get_symbol_info_card("e")
        .expect("e should be classified");
    assert_eq!(e_card.name, "e");
    assert!(matches!(
        e_card.object_kind,
        Some(urae_notebook::notebook::ObjectKind::Constant { .. })
    ));

    // 3. Check i imaginary unit classification
    let i_card = state
        .get_symbol_info_card("i")
        .expect("i should be classified");
    assert_eq!(i_card.name, "i");
    assert!(matches!(
        i_card.object_kind,
        Some(urae_notebook::notebook::ObjectKind::Constant { .. })
    ));

    // 4. Check line reference classification ($3)
    let line_card = state
        .get_symbol_info_card("$3")
        .expect("$3 should be classified");
    assert!(matches!(
        line_card.object_kind,
        Some(urae_notebook::notebook::ObjectKind::LineResult { line_idx: 3, .. })
    ));
}
