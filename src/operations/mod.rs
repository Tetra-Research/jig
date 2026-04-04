pub mod create;
pub mod inject;
pub mod patch;
pub mod replace;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{JigError, StructuredError};
use crate::recipe::FileOp;

// ── Execution context ─────────────────────────────────────────────

/// Shared state for operation execution.
pub struct ExecutionContext {
    /// Base directory for resolving output paths (default: cwd).
    pub base_dir: PathBuf,
    /// If true, produce output but write nothing to disk.
    pub dry_run: bool,
    /// If true, overwrite existing files on create.
    pub force: bool,
    /// Virtual file state for dry-run mode: path -> content.
    /// Create ops populate this instead of writing to disk.
    /// Inject ops read from here if the target was created in the same run.
    pub virtual_files: HashMap<PathBuf, String>,
}

impl ExecutionContext {
    pub fn new(base_dir: PathBuf, dry_run: bool, force: bool) -> Self {
        Self {
            base_dir,
            dry_run,
            force,
            virtual_files: HashMap::new(),
        }
    }

    /// Resolve an output path relative to the base directory.
    pub fn resolve_path(&self, relative: &str) -> PathBuf {
        self.base_dir.join(relative)
    }
}

// ── Operation result ──────────────────────────────────────────────

/// Result of a single file operation.
#[derive(Debug)]
pub enum OpResult {
    Success {
        action: &'static str,
        path: PathBuf,
        lines: usize,
        location: Option<String>,
        rendered_content: Option<String>,
    },
    Skip {
        path: PathBuf,
        reason: String,
        rendered_content: Option<String>,
    },
    Error {
        path: PathBuf,
        error: StructuredError,
        rendered_content: String,
    },
}

impl OpResult {
    pub fn path(&self) -> &Path {
        match self {
            OpResult::Success { path, .. }
            | OpResult::Skip { path, .. }
            | OpResult::Error { path, .. } => path,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, OpResult::Error { .. })
    }

    pub fn is_write(&self) -> bool {
        matches!(self, OpResult::Success { .. })
    }
}

// ── Dispatch ──────────────────────────────────────────────────────

/// Pre-rendered operation ready for execution.
pub struct PreparedOp {
    pub file_op: FileOp,
    pub rendered_content: String,
    pub rendered_path: String,
    /// For inject: rendered skip_if string (if any).
    pub rendered_skip_if: Option<String>,
}

/// Execute a single prepared operation.
pub fn execute_operation(
    prepared: &PreparedOp,
    ctx: &mut ExecutionContext,
    verbose: bool,
) -> OpResult {
    match &prepared.file_op {
        FileOp::Create { skip_if_exists, .. } => {
            create::execute(
                &prepared.rendered_path,
                &prepared.rendered_content,
                *skip_if_exists,
                ctx,
                verbose,
            )
        }
        FileOp::Inject { mode, .. } => {
            inject::execute(
                &prepared.rendered_path,
                &prepared.rendered_content,
                prepared.rendered_skip_if.as_deref(),
                mode,
                ctx,
                verbose,
            )
        }
        FileOp::Replace { spec, fallback, .. } => {
            replace::execute(
                &prepared.rendered_path,
                &prepared.rendered_content,
                spec,
                fallback,
                ctx,
                verbose,
            )
        }
        FileOp::Patch { anchor, .. } => {
            patch::execute(
                &prepared.rendered_path,
                &prepared.rendered_content,
                prepared.rendered_skip_if.as_deref(),
                anchor,
                ctx,
                verbose,
            )
        }
    }
}

/// Convert an OpResult::Error into a JigError for fail-fast propagation.
pub fn op_error_to_jig_error(result: &OpResult) -> Option<JigError> {
    match result {
        OpResult::Error { error, .. } => Some(JigError::FileOperation(error.clone())),
        _ => None,
    }
}
