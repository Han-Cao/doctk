//! Shared error type for all doctk features.

use thiserror::Error;

/// Top-level error returned by core APIs.
///
/// Each feature contributes its own typed error variant.  Feature-specific
/// errors are converted into [`DoctkError`] with `?` and keep their
/// user-facing messages.
#[derive(Debug, Error)]
pub enum DoctkError {
    /// Markdown / TSV conversion errors.
    #[error(transparent)]
    Table(#[from] crate::features::markdown_tsv::TableError),

    /// Diff processing errors.
    #[error(transparent)]
    Diff(#[from] crate::features::diff_checker::DiffError),

    /// PDF / AI inspection errors.
    #[error(transparent)]
    Pdf(#[from] crate::features::pdf_checker::PdfError),

    /// Filesystem errors (CLI input/output).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Invalid command-line or API arguments.
    #[error("{0}")]
    InvalidArgument(String),
}

impl DoctkError {
    /// A short, human-readable message suitable for a toast or stderr line.
    pub fn to_user_message(&self) -> String {
        self.to_string()
    }

    /// Recommended process exit code for this error.
    pub fn exit_code(&self) -> i32 {
        1
    }
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, DoctkError>;
