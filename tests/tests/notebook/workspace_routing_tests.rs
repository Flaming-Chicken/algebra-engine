//! Workspace tabs and layout presets are routed to the views that render them.

use std::collections::HashSet;
use urae_notebook::ViewMode;
use urae_notebook::ui::command_palette::CommandPalette;
use urae_notebook::ui::{WorkspaceLayoutPreset, WorkspaceState, WorkspaceTab};

const ALL_TABS: [WorkspaceTab; 10] = [
    WorkspaceTab::MathDocument,
    WorkspaceTab::Plot2D,
    WorkspaceTab::Viewport3D,
    WorkspaceTab::TerminalCLI,
    WorkspaceTab::PalettesBuilder,
    WorkspaceTab::ExampleGallery,
    WorkspaceTab::StorageDiagnostics,
    WorkspaceTab::DependencyGraph,
    WorkspaceTab::PresentationReport,
    WorkspaceTab::Settings,
];

#[test]
fn every_tab_has_a_route_and_full_page_tabs_get_their_own_view() {
    // Full-page tabs render their dedicated view.
    assert_eq!(
        WorkspaceTab::DependencyGraph.route().view,
        ViewMode::DependencyGraph
    );
    assert_eq!(
        WorkspaceTab::PresentationReport.route().view,
        ViewMode::PresentationReport
    );
    assert_eq!(
        WorkspaceTab::PalettesBuilder.route().view,
        ViewMode::SystemsWorkbench
    );
    // Side-panel tabs keep the notepad and switch the panel on.
    let viewport = WorkspaceTab::Viewport3D.route();
    assert_eq!(viewport.view, ViewMode::SmartStream);
    assert!(viewport.viewport_3d && !viewport.terminal);
    let terminal = WorkspaceTab::TerminalCLI.route();
    assert!(terminal.terminal && !terminal.viewport_3d);
    assert_eq!(
        WorkspaceTab::MathDocument.route().view,
        ViewMode::SmartStream
    );

    // Every full-page view of the app is reachable from some tab (nothing is orphaned).
    let reachable: HashSet<_> = ALL_TABS
        .iter()
        .map(|t| format!("{:?}", t.route().view))
        .collect();
    for view in [
        "SmartStream",
        "SystemsWorkbench",
        "DependencyGraph",
        "PresentationReport",
    ] {
        assert!(
            reachable.contains(view),
            "{view} is not reachable from any tab"
        );
    }
}

#[test]
fn applying_a_layout_opens_its_primary_tab() {
    assert_eq!(
        WorkspaceLayoutPreset::PresentationReport.primary_tab(),
        WorkspaceTab::PresentationReport
    );
    assert_eq!(
        WorkspaceLayoutPreset::DependencyGraph.primary_tab(),
        WorkspaceTab::DependencyGraph
    );
    for preset in [
        WorkspaceLayoutPreset::DualHalves,
        WorkspaceLayoutPreset::FluidNotepad,
        WorkspaceLayoutPreset::SplitDual,
        WorkspaceLayoutPreset::TripleIDE,
        WorkspaceLayoutPreset::ZenMode,
        WorkspaceLayoutPreset::SimplifiedSingleTab,
    ] {
        assert_eq!(
            preset.primary_tab(),
            WorkspaceTab::MathDocument,
            "{preset:?}"
        );
    }

    let mut ws = WorkspaceState::default();
    ws.apply_layout(WorkspaceLayoutPreset::PresentationReport);
    assert_eq!(ws.active_tab, WorkspaceTab::PresentationReport);
    assert_eq!(ws.layout_preset, WorkspaceLayoutPreset::PresentationReport);
    assert!(
        !ws.show_sidebar,
        "the preset's own panel settings still apply"
    );
    // Going back to a notepad layout leaves the report tab.
    ws.apply_layout(WorkspaceLayoutPreset::SplitDual);
    assert_eq!(ws.active_tab, WorkspaceTab::MathDocument);
    assert_eq!(ws.route_view(), ViewMode::SmartStream);
}

#[test]
fn the_palette_offers_every_layout_page_exactly_once() {
    let commands = CommandPalette::all_commands();
    let templates: Vec<&str> = commands.iter().map(|c| c.syntax_template).collect();
    for needed in [
        "__URAE_CMD_LAYOUT_PRESENTATION__",
        "__URAE_CMD_LAYOUT_GRAPH__",
        "__URAE_CMD_LAYOUT_ZEN__",
    ] {
        assert_eq!(
            templates.iter().filter(|t| **t == needed).count(),
            1,
            "{needed}"
        );
    }
    let shortcuts: Vec<_> = commands.iter().filter_map(|c| c.shortcut).collect();
    let unique: HashSet<_> = shortcuts.iter().collect();
    assert_eq!(
        shortcuts.len(),
        unique.len(),
        "palette shortcuts must be unique"
    );
}
