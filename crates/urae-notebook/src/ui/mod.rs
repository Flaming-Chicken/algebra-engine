//! # `urae_notebook::ui`
//!
//! Modular UI Subsystem for the URAE Continuous Mathematics & CAD Notepad.

pub mod command_palette;
pub mod palettes;
pub mod theme;
pub mod viewport3d;
pub mod workbench_ui;
pub mod workspace;

pub use command_palette::{CommandItem, CommandPalette, CommandPaletteState};
pub use palettes::PalettesUi;
pub use theme::{ThemeKind, ThemePalette};
pub use viewport3d::{Viewport3D, Viewport3DState, ViewportColorMode, ViewportModelPreset};
pub use workbench_ui::{WorkbenchAction, WorkbenchSubMode, WorkbenchUiState};
pub use workspace::{TabRoute, WorkspaceLayoutPreset, WorkspaceState, WorkspaceTab};
