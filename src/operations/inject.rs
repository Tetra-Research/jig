use regex::Regex;

use crate::error::StructuredError;
use crate::recipe::{InjectMode, MatchPosition};

use super::{ExecutionContext, OpResult};

/// Execute an inject operation: insert rendered content into an existing file.
///
/// - `rendered_path` is the already-rendered target file path.
/// - `rendered_content` is the template output to inject.
/// - `rendered_skip_if` is the rendered skip_if string (if any).
/// - `mode` determines where to inject (after/before regex, prepend, append).
/// - Reads from virtual_files if target was created in same run, else from disk.
/// - Updates virtual_files with post-injection content for subsequent operations.
pub fn execute(
    rendered_path: &str,
    rendered_content: &str,
    rendered_skip_if: Option<&str>,
    mode: &InjectMode,
    ctx: &mut ExecutionContext,
    verbose: bool,
) -> OpResult {
    let target = ctx.resolve_path(rendered_path);
    let injected_lines = rendered_content.lines().count();
    let content_for_verbose = if verbose {
        Some(rendered_content.to_string())
    } else {
        None
    };

    // Read target file content: prefer virtual_files, then disk.
    let file_content = if let Some(content) = ctx.virtual_files.get(&target) {
        content.clone()
    } else {
        match std::fs::read_to_string(&target) {
            Ok(content) => content,
            Err(_) => {
                return OpResult::Error {
                    path: target.clone(),
                    error: StructuredError {
                        what: format!("target file not found: '{}'", target.display()),
                        where_: target.display().to_string(),
                        why: "the file does not exist and was not created by a prior operation in this run".into(),
                        hint: "ensure the target file exists, or add a create operation before this inject".into(),
                    },
                    rendered_content: rendered_content.to_string(),
                };
            }
        }
    };

    // Check skip_if: if the rendered string is found in the file, skip.
    if let Some(skip_str) = rendered_skip_if
        && file_content.contains(skip_str)
    {
        return OpResult::Skip {
            path: target,
            reason: format!("skip_if matched: {skip_str}"),
            rendered_content: content_for_verbose,
        };
    }

    // Perform injection based on mode.
    let (new_content, location) = match mode {
        InjectMode::After { pattern, at } => {
            match inject_after_before(&file_content, rendered_content, pattern, at, true, &target) {
                Ok(v) => v,
                Err(op_result) => return op_result,
            }
        }
        InjectMode::Before { pattern, at } => {
            match inject_after_before(&file_content, rendered_content, pattern, at, false, &target) {
                Ok(v) => v,
                Err(op_result) => return op_result,
            }
        }
        InjectMode::Prepend => {
            let mut result = String::with_capacity(rendered_content.len() + file_content.len() + 1);
            result.push_str(rendered_content);
            if !rendered_content.ends_with('\n') && !file_content.is_empty() {
                result.push('\n');
            }
            result.push_str(&file_content);
            (result, "prepend".to_string())
        }
        InjectMode::Append => {
            let mut result = String::with_capacity(file_content.len() + rendered_content.len() + 1);
            result.push_str(&file_content);
            if !file_content.ends_with('\n') && !file_content.is_empty() {
                result.push('\n');
            }
            result.push_str(rendered_content);
            (result, "append".to_string())
        }
    };

    // Write the modified content.
    if ctx.dry_run {
        ctx.virtual_files.insert(target.clone(), new_content);
    } else {
        if let Err(e) = std::fs::write(&target, &new_content) {
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
        // Also update virtual_files so subsequent ops in this run see the new content.
        ctx.virtual_files.insert(target.clone(), new_content);
    }

    OpResult::Success {
        action: "inject",
        path: target,
        lines: injected_lines,
        location: Some(location),
        rendered_content: content_for_verbose,
    }
}

/// Inject content after or before a regex match.
/// Returns (new_content, location_description) or an OpResult::Error.
#[allow(clippy::result_large_err)]
fn inject_after_before(
    file_content: &str,
    rendered_content: &str,
    pattern: &str,
    at: &MatchPosition,
    is_after: bool,
    target: &std::path::Path,
) -> Result<(String, String), OpResult> {
    let re = Regex::new(pattern).expect("regex was validated at parse time");

    let lines: Vec<&str> = file_content.lines().collect();
    let matching_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| re.is_match(line))
        .map(|(i, _)| i)
        .collect();

    if matching_indices.is_empty() {
        return Err(OpResult::Error {
            path: target.to_path_buf(),
            error: StructuredError {
                what: format!("regex pattern matched no lines in '{}'", target.display()),
                where_: target.display().to_string(),
                why: format!("pattern '{}' did not match any line in the file", pattern),
                hint: "check the regex pattern against the file contents".into(),
            },
            rendered_content: rendered_content.to_string(),
        });
    }

    let match_idx = match at {
        MatchPosition::First => matching_indices[0],
        MatchPosition::Last => *matching_indices.last().unwrap(),
    };

    // Build new content by inserting at the right position.
    let direction = if is_after { "after" } else { "before" };
    let at_str = match at {
        MatchPosition::First => "first",
        MatchPosition::Last => "last",
    };
    // Line numbers are 1-based for human display.
    let location = format!("{direction}:{pattern} ({at_str} match, line {})", match_idx + 1);

    let mut result = String::new();
    let has_trailing_newline = file_content.ends_with('\n');

    if is_after {
        // Insert after match_idx line.
        for (i, line) in lines.iter().enumerate() {
            result.push_str(line);
            result.push('\n');
            if i == match_idx {
                result.push_str(rendered_content);
                if !rendered_content.ends_with('\n') {
                    result.push('\n');
                }
            }
        }
    } else {
        // Insert before match_idx line.
        for (i, line) in lines.iter().enumerate() {
            if i == match_idx {
                result.push_str(rendered_content);
                if !rendered_content.ends_with('\n') {
                    result.push('\n');
                }
            }
            result.push_str(line);
            result.push('\n');
        }
    }

    // Preserve original trailing newline behavior.
    if !has_trailing_newline && result.ends_with('\n') {
        result.pop();
    }

    Ok((result, location))
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

    // ── AC-5.1: after inserts content on line after first match ──

    #[test]
    fn ac_5_1_after_first_match() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "# fixtures\nfixture_a = 1\nfixture_b = 2\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::After {
            pattern: "^# fixtures".into(),
            at: MatchPosition::First,
        };
        let result = execute("target.py", "fixture_c = 3\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "# fixtures");
        assert_eq!(lines[1], "fixture_c = 3");
        assert_eq!(lines[2], "fixture_a = 1");
    }

    // ── AC-5.2: before inserts content on line before first match ──

    #[test]
    fn ac_5_2_before_first_match() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "import os\nimport sys\nclass Foo:\n    pass\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Before {
            pattern: "^class ".into(),
            at: MatchPosition::First,
        };
        let result = execute("target.py", "import json\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[2], "import json");
        assert_eq!(lines[3], "class Foo:");
    }

    // ── AC-5.3: prepend inserts content at start of file ──

    #[test]
    fn ac_5_3_prepend() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.rs");
        fs::write(&target, "fn main() {}\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("target.rs", "// header\n", None, &InjectMode::Prepend, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("// header\n"));
        assert!(content.contains("fn main() {}"));
    }

    // ── AC-5.4: append inserts content at end of file ──

    #[test]
    fn ac_5_4_append() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.rs");
        fs::write(&target, "fn main() {}\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("target.rs", "// footer\n", None, &InjectMode::Append, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.ends_with("// footer\n"));
    }

    // ── AC-5.5: at:last uses last regex match ──

    #[test]
    fn ac_5_5_after_last_match() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "# section\nfoo\n# section\nbar\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::After {
            pattern: "^# section".into(),
            at: MatchPosition::Last,
        };
        let result = execute("target.py", "baz\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        // Second "# section" is at index 2, so "baz" should be at index 3.
        assert_eq!(lines[2], "# section");
        assert_eq!(lines[3], "baz");
        assert_eq!(lines[4], "bar");
    }

    // ── AC-5.6: at:first (default) uses first regex match ──

    #[test]
    fn ac_5_6_after_first_match_default() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "# section\nfoo\n# section\nbar\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::After {
            pattern: "^# section".into(),
            at: MatchPosition::First,
        };
        let result = execute("target.py", "baz\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "# section");
        assert_eq!(lines[1], "baz");
        assert_eq!(lines[2], "foo");
    }

    // ── AC-5.7: skip_if skips when string found in file ──

    #[test]
    fn ac_5_7_skip_if_matched() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "import os\nimport BookingService\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Append;
        let result = execute("target.py", "new line\n", Some("BookingService"), &mode, &mut ctx, false);

        match &result {
            OpResult::Skip { reason, .. } => {
                assert!(reason.contains("BookingService"));
            }
            _ => panic!("expected Skip, got {:?}", result),
        }
        // File unchanged.
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "import os\nimport BookingService\n");
    }

    // ── AC-5.7: skip_if does not skip when string not found ──

    #[test]
    fn ac_5_7_skip_if_not_matched() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "import os\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Append;
        let result = execute("target.py", "import json\n", Some("BookingService"), &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
    }

    // ── AC-5.8: Regex no-match exits 3 with pattern, path, hint ──

    #[test]
    fn ac_5_8_regex_no_match() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "import os\nimport sys\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::After {
            pattern: "^class NonExistent".into(),
            at: MatchPosition::First,
        };
        let result = execute("target.py", "new line\n", None, &mode, &mut ctx, false);

        assert!(result.is_error());
        if let OpResult::Error { error, rendered_content, .. } = &result {
            assert!(error.what.contains("matched no lines"));
            assert!(error.why.contains("NonExistent"));
            assert!(!error.hint.is_empty());
            assert_eq!(rendered_content, "new line\n");
        }
    }

    // ── AC-5.9: Missing target file exits 3 ──

    #[test]
    fn ac_5_9_missing_target_file() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Append;
        let result = execute("nonexistent.py", "content\n", None, &mode, &mut ctx, false);

        assert!(result.is_error());
        if let OpResult::Error { error, rendered_content, .. } = &result {
            assert!(error.what.contains("target file not found"));
            assert_eq!(rendered_content, "content\n");
        }
    }

    // ── AC-5.10: Inject success reports action:"inject" with path, location, line count ──

    #[test]
    fn ac_5_10_success_reports_inject() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "# header\ncode\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::After {
            pattern: "^# header".into(),
            at: MatchPosition::First,
        };
        let result = execute("target.py", "line1\nline2\n", None, &mode, &mut ctx, false);

        match &result {
            OpResult::Success { action, lines, location, .. } => {
                assert_eq!(*action, "inject");
                assert_eq!(*lines, 2);
                assert!(location.is_some());
                let loc = location.as_ref().unwrap();
                assert!(loc.contains("after"), "location should contain 'after': {loc}");
            }
            _ => panic!("expected Success, got {:?}", result),
        }
    }

    // ── AC-5.11: Inject path renders as template (caller renders; we verify resolved path works) ──

    #[test]
    fn ac_5_11_templated_inject_path() {
        let dir = TempDir::new().unwrap();
        // Caller already rendered the path.
        let target = dir.path().join("tests/test_booking.py");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "# tests\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Append;
        let result = execute("tests/test_booking.py", "test_case\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("test_case"));
    }

    // ── AC-5.12: at field ignored when prepend/append ──

    #[test]
    fn ac_5_12_at_ignored_for_prepend_append() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "existing\n").unwrap();

        // Prepend mode — at field doesn't exist in the enum variant, so it's inherently ignored.
        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("target.txt", "prepended\n", None, &InjectMode::Prepend, &mut ctx, false);
        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("prepended\n"));
    }

    // ── AC-5.16: --force has no effect on inject operations ──

    #[test]
    fn ac_5_16_force_no_effect_on_inject() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "line1\nline2\n").unwrap();

        // With force=true, inject should behave identically.
        let mut ctx = make_ctx(dir.path(), false, true);
        let mode = InjectMode::Append;
        let result = execute("target.py", "appended\n", None, &mode, &mut ctx, false);
        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("appended"));
    }

    // ── AC-N2.2: No duplicate content with skip_if ──

    #[test]
    fn ac_n2_2_no_duplicate_with_skip_if() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "# header\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Append;

        // First inject.
        let r1 = execute("target.py", "new_line\n", Some("new_line"), &mode, &mut ctx, false);
        assert!(matches!(&r1, OpResult::Success { action: "inject", .. }));

        // Second inject — should skip because skip_if matches.
        let r2 = execute("target.py", "new_line\n", Some("new_line"), &mode, &mut ctx, false);
        assert!(matches!(&r2, OpResult::Skip { .. }));

        // Verify no duplicate.
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content.matches("new_line").count(), 1);
    }

    // ── AC-N4.2: Error includes rendered content for fallback ──

    #[test]
    fn ac_n4_2_error_includes_rendered_content() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Append;
        let result = execute("nonexistent.py", "my rendered content", None, &mode, &mut ctx, false);

        if let OpResult::Error { rendered_content, error, .. } = &result {
            assert_eq!(rendered_content, "my rendered content");
            assert!(!error.what.is_empty());
            assert!(!error.where_.is_empty());
            assert!(!error.why.is_empty());
            assert!(!error.hint.is_empty());
        } else {
            panic!("expected Error");
        }
    }

    // ── Dry-run: reads from virtual_files when target was created in same run ──

    #[test]
    fn dry_run_reads_virtual_files() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true, false);

        // Simulate a prior create operation by populating virtual_files.
        let target = dir.path().join("new_file.py");
        ctx.virtual_files.insert(target.clone(), "# created\ncode\n".into());

        let mode = InjectMode::Append;
        let result = execute("new_file.py", "injected\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        // Virtual files should have the updated content.
        let virtual_content = ctx.virtual_files.get(&target).unwrap();
        assert!(virtual_content.contains("# created"));
        assert!(virtual_content.contains("injected"));
        // No file on disk.
        assert!(!target.exists());
    }

    // ── Dry-run: skip_if checks virtual_files content ──

    #[test]
    fn dry_run_skip_if_checks_virtual_files() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true, false);

        // Virtual file already has the content.
        let target = dir.path().join("file.py");
        ctx.virtual_files.insert(target.clone(), "BookingService\n".into());

        let mode = InjectMode::Append;
        let result = execute("file.py", "new stuff\n", Some("BookingService"), &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Skip { .. }));
    }

    // ── Dry-run: inject updates virtual_files for subsequent operations ──

    #[test]
    fn dry_run_inject_updates_virtual_files() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true, false);

        let target = dir.path().join("file.py");
        ctx.virtual_files.insert(target.clone(), "# start\n".into());

        let mode = InjectMode::Append;
        let r1 = execute("file.py", "first\n", None, &mode, &mut ctx, false);
        assert!(matches!(&r1, OpResult::Success { .. }));

        let r2 = execute("file.py", "second\n", None, &mode, &mut ctx, false);
        assert!(matches!(&r2, OpResult::Success { .. }));

        let content = ctx.virtual_files.get(&target).unwrap();
        assert!(content.contains("first"));
        assert!(content.contains("second"));
    }

    // ── Verbose includes rendered content ──

    #[test]
    fn verbose_includes_content() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "existing\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("target.txt", "injected", None, &InjectMode::Append, &mut ctx, true);

        match result {
            OpResult::Success { rendered_content, .. } => {
                assert_eq!(rendered_content.as_deref(), Some("injected"));
            }
            _ => panic!("expected Success"),
        }
    }

    // ── Before with at:last ──

    #[test]
    fn before_last_match() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class A:\n    pass\nclass B:\n    pass\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::Before {
            pattern: "^class ".into(),
            at: MatchPosition::Last,
        };
        let result = execute("target.py", "# before last class\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "inject", .. }));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[2], "# before last class");
        assert_eq!(lines[3], "class B:");
    }

    // ── After regex with special characters ──

    #[test]
    fn after_regex_with_special_chars() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.rs");
        fs::write(&target, "use std::io;\nuse std::fs;\n\nfn main() {}\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let mode = InjectMode::After {
            pattern: r"^use std::fs;".into(),
            at: MatchPosition::First,
        };
        let result = execute("target.rs", "use std::path::Path;\n", None, &mode, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { .. }));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[1], "use std::fs;");
        assert_eq!(lines[2], "use std::path::Path;");
    }

    // ── Non-dry-run also updates virtual_files for subsequent ops ──

    #[test]
    fn non_dry_run_updates_virtual_files() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("file.txt");
        fs::write(&target, "original\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false, false);
        let result = execute("file.txt", "added\n", None, &InjectMode::Append, &mut ctx, false);
        assert!(matches!(&result, OpResult::Success { .. }));

        // virtual_files should be updated.
        assert!(ctx.virtual_files.get(&target).unwrap().contains("added"));
    }
}
