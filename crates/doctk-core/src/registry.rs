//! Compile-time feature registry.
//!
//! Both the CLI and the GUI shell use [`TOOL_REGISTRY`] to discover which
//! tools exist.  A new feature must be registered here in addition to the
//! layer-specific registries (CLI subcommands, Tauri commands, frontend tool
//! definitions).

use serde::Serialize;

/// Stable metadata describing one tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ToolManifest {
    /// Stable machine id (also used for CLI subcommand routing and GUI routes).
    pub id: &'static str,
    /// Human readable display name.
    pub name: &'static str,
    /// One-line description for tooltips and menus.
    pub description: &'static str,
    /// Search keywords for future GUI filtering.
    pub keywords: &'static [&'static str],
    /// Feature version string.
    pub version: &'static str,
}

/// All features shipped with this build.
pub const TOOL_REGISTRY: &[ToolManifest] = &[
    crate::features::markdown_tsv::MANIFEST,
    crate::features::diff_checker::MANIFEST,
    crate::features::pdf_checker::MANIFEST,
];

/// Look up a manifest by tool id.
pub fn find_tool(id: &str) -> Option<&'static ToolManifest> {
    TOOL_REGISTRY.iter().find(|m| m.id == id)
}
