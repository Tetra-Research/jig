use crate::error::StructuredError;

use super::{ExecutionContext, OpResult};

/// Execute a create operation: write rendered content to a file.
///
/// - Renders `to` path already resolved by caller.
/// - Creates parent directories as needed.
/// - Respects skip_if_exists, --force, --dry-run, --base-dir.
pub fn execute(
    rendered_path: &str,
    rendered_content: &str,
    skip_if_exists: bool,
    ctx: &mut ExecutionContext,
    verbose: bool,
) -> OpResult {
    let target = ctx.resolve_path(rendered_path);
    let lines = rendered_content.lines().count();
    let content_for_verbose = if verbose {
        Some(rendered_content.to_string())
    } else {
        None
    };

    // Check if file already exists (on disk or in virtual state).
    let exists = target.exists() || ctx.virtual_files.contains_key(&target);

    if exists && !ctx.force {
        if skip_if_exists {
            return OpResult::Skip {
                path: target,
                reason: "file already exists (skip_if_exists: true)".into(),
                rendered_content: content_for_verbose,
            };
        } else {
            return OpResult::Error {
                path: target.clone(),
                error: StructuredError {
                    what: format!("file already exists: '{}'", target.display()),
                    where_: target.display().to_string(),
                    why: "skip_if_exists is false and --force was not specified".into(),
                    hint: "use --force to overwrite, or set skip_if_exists: true in the recipe".into(),
                },
                rendered_content: rendered_content.to_string(),
            };
        }
    }

    if ctx.dry_run {
        // Record in virtual file state but don't write to disk.
        ctx.virtual_files.insert(target.clone(), rendered_content.to_string());
        return OpResult::Success {
            action: "create",
            path: target,
            lines,
            location: None,
            rendered_content: content_for_verbose,
        };
    }

    // Create parent directories.
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty())
        && let Err(e) = std::fs::create_dir_all(parent) {
            let parent_display = parent.display().to_string();
            return OpResult::Error {
                path: target,
                error: StructuredError {
                    what: format!("cannot create parent directory '{}'", parent_display),
                    where_: parent_display,
                    why: e.to_string(),
                    hint: "check directory permissions".into(),
                },
                rendered_content: rendered_content.to_string(),
            };
        }

    // Write the file.
    if let Err(e) = std::fs::write(&target, rendered_content) {
        return OpResult::Error {
            path: target.clone(),
            error: StructuredError {
                what: format!("cannot write file '{}'", target.display()),
                where_: target.display().to_string(),
                why: e.to_string(),
                hint: "check file permissions".into(),
            },
            rendered_content: rendered_content.to_string(),
        };
    }

    OpResult::Success {
        action: "create",
        path: target,
        lines,
        location: None,
        rendered_content: content_for_verbose,
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_ctx(dir: &std::path::Path, dry_run: bool, force: bool) -> ExecutionContext {
        ExecutionContext::new(dir.to_path_buf(), dry_run, force)
    }

    // ── AC-4.1: Create writes rendered content to target path ──

    #[test]
    fn ac_4_1_create_writes_file() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("output.rs", "fn main() {}\n", false, &mut ctx, false);
        assert!(matches!(result, OpResult::Success { action: "create", .. }));
        let content = fs::read_to_string(dir.path().join("output.rs")).unwrap();
        assert_eq!(content, "fn main() {}\n");
    }

    // ── AC-4.2: Templated paths (caller renders the path; we verify the resolved path works) ──

    #[test]
    fn ac_4_2_templated_path() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        // Caller already rendered the path template.
        let result = execute(
            "src/services/booking_service.rs",
            "pub struct BookingService;",
            false,
            &mut ctx,
            false,
        );
        assert!(matches!(result, OpResult::Success { action: "create", .. }));
        let path = dir.path().join("src/services/booking_service.rs");
        assert!(path.exists());
        assert_eq!(fs::read_to_string(path).unwrap(), "pub struct BookingService;");
    }

    // ── AC-4.3: Parent directories created automatically ──

    #[test]
    fn ac_4_3_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("deep/nested/dir/file.txt", "content", false, &mut ctx, false);
        assert!(matches!(result, OpResult::Success { .. }));
        assert!(dir.path().join("deep/nested/dir/file.txt").exists());
    }

    // ── AC-4.4: skip_if_exists: true skips existing file ──

    #[test]
    fn ac_4_4_skip_if_exists() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.rs"), "old content").unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("existing.rs", "new content", true, &mut ctx, false);
        match &result {
            OpResult::Skip { reason, .. } => {
                assert!(reason.contains("skip_if_exists"));
            }
            _ => panic!("expected Skip, got {:?}", result),
        }
        // File content unchanged.
        assert_eq!(fs::read_to_string(dir.path().join("existing.rs")).unwrap(), "old content");
    }

    // ── AC-4.5: File exists without force → error ──

    #[test]
    fn ac_4_5_file_exists_error() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.rs"), "old").unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("existing.rs", "new", false, &mut ctx, false);
        assert!(result.is_error());
        if let OpResult::Error { error, rendered_content, .. } = &result {
            assert!(error.what.contains("already exists"));
            assert!(error.hint.contains("--force"));
            assert_eq!(rendered_content, "new");
        }
    }

    // ── AC-4.6: --force overwrites existing file ──

    #[test]
    fn ac_4_6_force_overwrite() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.rs"), "old").unwrap();
        let mut ctx = make_ctx(dir.path(), false, true);
        let result = execute("existing.rs", "new content", false, &mut ctx, false);
        assert!(matches!(result, OpResult::Success { action: "create", .. }));
        assert_eq!(fs::read_to_string(dir.path().join("existing.rs")).unwrap(), "new content");
    }

    // ── AC-4.7: --base-dir changes output root ──

    #[test]
    fn ac_4_7_base_dir() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("custom_base");
        fs::create_dir_all(&base).unwrap();
        let mut ctx = make_ctx(&base, false, false);
        let result = execute("output.txt", "hello", false, &mut ctx, false);
        assert!(matches!(result, OpResult::Success { .. }));
        assert!(base.join("output.txt").exists());
        assert!(!dir.path().join("output.txt").exists());
    }

    // ── AC-4.8: Success reports action:"create" with path and line count ──

    #[test]
    fn ac_4_8_success_reports_lines() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let content = "line1\nline2\nline3\n";
        let result = execute("file.txt", content, false, &mut ctx, false);
        match result {
            OpResult::Success { action, lines, .. } => {
                assert_eq!(action, "create");
                assert_eq!(lines, 3);
            }
            _ => panic!("expected Success"),
        }
    }

    // ── AC-4.9: Permission error (tested by writing to read-only dir) ──

    #[test]
    fn ac_4_9_permission_error() {
        // Use a path we definitely can't write to.
        let mut ctx = make_ctx(std::path::Path::new("/proc/nonexistent"), false, false);
        let result = execute("file.txt", "content", false, &mut ctx, false);
        assert!(result.is_error());
        if let OpResult::Error { error, rendered_content, .. } = &result {
            assert!(!error.what.is_empty());
            assert_eq!(rendered_content, "content");
        }
    }

    // ── AC-6.8: --dry-run produces output but writes nothing ──

    #[test]
    fn ac_6_8_dry_run_no_write() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true, false);
        let result = execute("output.rs", "fn main() {}\n", false, &mut ctx, false);
        assert!(matches!(result, OpResult::Success { action: "create", .. }));
        // File should NOT exist on disk.
        assert!(!dir.path().join("output.rs").exists());
        // But should be in virtual_files.
        assert!(ctx.virtual_files.contains_key(&dir.path().join("output.rs")));
    }

    // ── AC-6.8: dry-run + force reports create for existing files ──

    #[test]
    fn ac_6_8_dry_run_force_existing() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.rs"), "old").unwrap();
        let mut ctx = make_ctx(dir.path(), true, true);
        let result = execute("existing.rs", "new", false, &mut ctx, false);
        assert!(matches!(result, OpResult::Success { action: "create", .. }));
        // Original file untouched.
        assert_eq!(fs::read_to_string(dir.path().join("existing.rs")).unwrap(), "old");
    }

    // ── Verbose includes rendered content ──

    #[test]
    fn verbose_includes_content() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("file.txt", "hello world", false, &mut ctx, true);
        match result {
            OpResult::Success { rendered_content, .. } => {
                assert_eq!(rendered_content.as_deref(), Some("hello world"));
            }
            _ => panic!("expected Success"),
        }
    }

    // ── AC-N4.2: Error includes rendered content for fallback ──

    #[test]
    fn ac_n4_2_error_includes_rendered_content() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.rs"), "old").unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("existing.rs", "new content here", false, &mut ctx, false);
        match result {
            OpResult::Error { rendered_content, .. } => {
                assert_eq!(rendered_content, "new content here");
            }
            _ => panic!("expected Error"),
        }
    }

    // ── Dry-run skip_if_exists with existing file on disk ──

    #[test]
    fn dry_run_skip_if_exists() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.rs"), "old").unwrap();
        let mut ctx = make_ctx(dir.path(), true, false);
        let result = execute("existing.rs", "new", true, &mut ctx, false);
        assert!(matches!(result, OpResult::Skip { .. }));
    }

    // ── Dry-run virtual file collision detection ──

    #[test]
    fn dry_run_virtual_file_collision() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true, false);
        // First create succeeds.
        let r1 = execute("file.rs", "first", false, &mut ctx, false);
        assert!(matches!(r1, OpResult::Success { .. }));
        // Second create to same path without force → error.
        let r2 = execute("file.rs", "second", false, &mut ctx, false);
        assert!(r2.is_error());
    }
}
