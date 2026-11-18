//! # Workspace Management & Docking Layout Subsystem
//!
//! Provides a flexible multi-panel workspace supporting tabbed navigation, split views,
//! floating vs docked inspectors, and IDE-grade layout presets.

use serde::{Deserialize, Serialize};

/// Available tabs in the URAE workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceTab {
    MathDocument,
    Plot2D,
    Viewport3D,
    TerminalCLI,
    PalettesBuilder,
    ExampleGallery,
    StorageDiagnostics,
    DependencyGraph,
    PresentationReport,
    Settings,
}

impl WorkspaceTab {
    pub fn title(&self) -> &'static str {
        match self {
            Self::MathDocument => "📝 Math Notepad",
            Self::Plot2D => "📈 2D Plots",
            Self::Viewport3D => "🧊 3D Viewport",
            Self::TerminalCLI => "💻 Terminal CLI",
            Self::PalettesBuilder => "🧮 Builder Wizards",
            Self::ExampleGallery => "📚 Example Gallery",
            Self::StorageDiagnostics => "💾 Storage & Sync",
            Self::DependencyGraph => "🕸️ Dependency Graph",
            Self::PresentationReport => "📑 Presentation Report",
            Self::Settings => "⚙️ Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::MathDocument => "📝",
            Self::Plot2D => "📈",
            Self::Viewport3D => "🧊",
            Self::TerminalCLI => "💻",
            Self::PalettesBuilder => "🧮",
            Self::ExampleGallery => "📚",
            Self::StorageDiagnostics => "💾",
            Self::DependencyGraph => "🕸️",
            Self::PresentationReport => "📑",
            Self::Settings => "⚙️",
        }
    }
}

/// What a workspace tab shows in the central panel and which side panels it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabRoute {
    /// The central view that renders this tab.
    pub view: crate::app::ViewMode,
    /// The tab needs the 3-D viewport panel.
    pub viewport_3d: bool,
    /// The tab needs the CLI terminal panel.
    pub terminal: bool,
}

impl WorkspaceTab {
    /// Where the tab is rendered. Tabs without a dedicated full-page view (plots, the 3-D
    /// viewport, the terminal, builders, gallery, storage, settings) live around the notepad: the
    /// central view stays the document and the matching panel is switched on.
    pub fn route(&self) -> TabRoute {
        use crate::app::ViewMode;
        let (view, viewport_3d, terminal) = match self {
            Self::MathDocument | Self::Plot2D => (ViewMode::SmartStream, false, false),
            Self::Viewport3D => (ViewMode::SmartStream, true, false),
            Self::TerminalCLI => (ViewMode::SmartStream, false, true),
            Self::PalettesBuilder => (ViewMode::SystemsWorkbench, false, false),
            Self::ExampleGallery | Self::StorageDiagnostics | Self::Settings => {
                (ViewMode::SmartStream, false, false)
            }
            Self::DependencyGraph => (ViewMode::DependencyGraph, false, false),
            Self::PresentationReport => (ViewMode::PresentationReport, false, false),
        };
        TabRoute {
            view,
            viewport_3d,
            terminal,
        }
    }
}

/// Workspace layout arrangement presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkspaceLayoutPreset {
    #[default]
    DualHalves,
    FluidNotepad,
    PresentationReport,
    DependencyGraph,
    SplitDual,
    TripleIDE,
    ZenMode,
    SimplifiedSingleTab,
}

impl WorkspaceLayoutPreset {
    /// The tab a preset opens on: the full-page presets open their page, every other preset the
    /// math notepad.
    pub fn primary_tab(&self) -> WorkspaceTab {
        match self {
            Self::PresentationReport => WorkspaceTab::PresentationReport,
            Self::DependencyGraph => WorkspaceTab::DependencyGraph,
            _ => WorkspaceTab::MathDocument,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::DualHalves => "Dual Halves (Left Inputs & Sliders / Right Outputs & 3D)",
            Self::FluidNotepad => "Fluid Inline Notepad",
            Self::PresentationReport => "Presentation & Reporting Mode",
            Self::DependencyGraph => "Dependency Graph View",
            Self::SplitDual => "Dual Split (Editor + Visualizer)",
            Self::TripleIDE => "Triple IDE (Editor + Plots + Terminal)",
            Self::ZenMode => "Zen Mode (Full Screen Canvas)",
            Self::SimplifiedSingleTab => "Single Tab (Simplified Canvas)",
        }
    }
}

/// State tracking the active workspace layout and docked panels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub active_tab: WorkspaceTab,
    pub secondary_tab: Option<WorkspaceTab>,
    pub layout_preset: WorkspaceLayoutPreset,
    pub split_ratio: f32,
    pub show_sidebar: bool,
    pub show_bottom_panel: bool,
    pub simplified_single_tab: bool,
    pub pinned_tabs: Vec<WorkspaceTab>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            active_tab: WorkspaceTab::MathDocument,
            secondary_tab: Some(WorkspaceTab::Plot2D),
            layout_preset: WorkspaceLayoutPreset::DualHalves,
            split_ratio: 0.50,
            show_sidebar: true,
            show_bottom_panel: false,
            simplified_single_tab: false,
            pinned_tabs: vec![
                WorkspaceTab::MathDocument,
                WorkspaceTab::Plot2D,
                WorkspaceTab::Viewport3D,
                WorkspaceTab::DependencyGraph,
                WorkspaceTab::PresentationReport,
                WorkspaceTab::TerminalCLI,
            ],
        }
    }
}

impl WorkspaceState {
    pub fn select_tab(&mut self, tab: WorkspaceTab) {
        self.active_tab = tab;
    }

    /// The central view for the active tab.
    pub fn route_view(&self) -> crate::app::ViewMode {
        self.active_tab.route().view
    }

    /// Apply a layout preset and open its primary tab (see
    /// [`WorkspaceLayoutPreset::primary_tab`]), so the active tab never disagrees with the layout.
    pub fn apply_layout(&mut self, preset: WorkspaceLayoutPreset) {
        self.set_layout(preset);
        self.select_tab(preset.primary_tab());
    }

    pub fn toggle_sidebar(&mut self) {
        self.show_sidebar = !self.show_sidebar;
    }

    pub fn toggle_bottom_panel(&mut self) {
        self.show_bottom_panel = !self.show_bottom_panel;
    }

    pub fn set_layout(&mut self, preset: WorkspaceLayoutPreset) {
        self.layout_preset = preset;
        match preset {
            WorkspaceLayoutPreset::DualHalves => {
                self.show_sidebar = true;
                self.show_bottom_panel = false;
                self.simplified_single_tab = false;
                self.split_ratio = 0.50;
            }
            WorkspaceLayoutPreset::PresentationReport => {
                self.show_sidebar = false;
                self.show_bottom_panel = false;
                self.simplified_single_tab = false;
                self.split_ratio = 1.0;
            }
            WorkspaceLayoutPreset::DependencyGraph => {
                self.show_sidebar = false;
                self.show_bottom_panel = false;
                self.simplified_single_tab = false;
                self.split_ratio = 1.0;
            }
            WorkspaceLayoutPreset::FluidNotepad => {
                self.show_sidebar = true;
                self.show_bottom_panel = false;
                self.simplified_single_tab = false;
                self.split_ratio = 0.60;
            }
            WorkspaceLayoutPreset::SplitDual => {
                self.show_sidebar = true;
                self.show_bottom_panel = false;
                self.simplified_single_tab = false;
                self.split_ratio = 0.50;
            }
            WorkspaceLayoutPreset::TripleIDE => {
                self.show_sidebar = true;
                self.show_bottom_panel = true;
                self.simplified_single_tab = false;
                self.split_ratio = 0.50;
            }
            WorkspaceLayoutPreset::ZenMode => {
                self.show_sidebar = false;
                self.show_bottom_panel = false;
                self.simplified_single_tab = false;
                self.split_ratio = 1.0;
            }
            WorkspaceLayoutPreset::SimplifiedSingleTab => {
                self.show_sidebar = true;
                self.show_bottom_panel = false;
                self.simplified_single_tab = true;
                self.split_ratio = 1.0;
            }
        }
    }
}
