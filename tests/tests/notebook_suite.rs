//! Workspace Integration Test Suite for `urae-notebook`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "notebook/notebook_tests.rs"]
mod notebook_tests;

#[path = "notebook/notebook_ui_tests.rs"]
mod notebook_ui_tests;

#[path = "notebook/serde_and_format_tests.rs"]
mod serde_and_format_tests;

#[path = "notebook/phase9_notepad_canvas_tests.rs"]
mod phase9_notepad_canvas_tests;

#[path = "notebook/phase17_frontend_ux_tests.rs"]
mod phase17_frontend_ux_tests;

#[path = "notebook/example_gallery_tests.rs"]
mod example_gallery_tests;

#[path = "notebook/undo_redo_tests.rs"]
mod undo_redo_tests;

#[path = "notebook/gux_gui_modernization_tests.rs"]
mod gux_gui_modernization_tests;

#[path = "notebook/smart_stream_tests.rs"]
mod smart_stream_tests;

#[path = "notebook/export_tests.rs"]
mod export_tests;

#[path = "notebook/workspace_routing_tests.rs"]
mod workspace_routing_tests;
