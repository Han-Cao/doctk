//! `doctk-core` contains the UI-agnostic document-processing logic shared by
//! the doctk CLI and GUI applications.
//!
//! The crate is organized around the feature registry pattern described in
//! `docs/tool-framework.md`: each tool lives under [`features`] and publishes
//! a [`registry::ToolManifest`] so shells can discover it.

pub mod error;
pub mod features;
pub mod registry;
pub mod units;

pub use error::{DoctkError, Result};
