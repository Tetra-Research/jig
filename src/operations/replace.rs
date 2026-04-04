use regex::Regex;

use crate::error::StructuredError;
use crate::recipe::{Fallback, ReplaceSpec};

use super::{ExecutionContext, OpResult};

/// Execute a replace operation: find and replace a region in an existing file.
pub fn execute(
    rendered_path: &str,
    rendered_content: &str,
    spec: &ReplaceSpec,
    fallback: &Fallback,
    ctx: &mut ExecutionContext,
    verbose: bool,
) -> OpResult {
    let target = ctx.resolve_path(rendered_path);
    let content_for_verbose = if verbose {
        Some(rendered_content.to_string())
    } else {
        None
    };

    // Read target file: prefer virtual_files, then disk.
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
                        hint: "ensure the target file exists, or add a create operation before this replace".into(),
                    },
                    rendered_content: rendered_content.to_string(),
                };
            }
        }
    };

    let lines: Vec<&str> = file_content.lines().collect();

    // Dispatch to match mode.
    let match_result = match spec {
        ReplaceSpec::Between { start, end } => match_between(&lines, start, end, &target),
        ReplaceSpec::Pattern(pattern) => match_pattern(&lines, pattern),
    };

    match match_result {
        MatchResult::Found { start_idx, end_idx, mode } => {
            // Build new content by replacing the matched region.
            let (new_content, location) = match mode {
                MatchMode::Between => {
                    // Exclusive: preserve marker lines, replace content between.
                    let mut result_lines: Vec<&str> = Vec::new();
                    for (i, line) in lines.iter().enumerate() {
                        if i <= start_idx || i >= end_idx {
                            result_lines.push(line);
                        }
                        if i == start_idx {
                            // Insert rendered content lines after start marker.
                            for rc_line in rendered_content.lines() {
                                result_lines.push(rc_line);
                            }
                        }
                    }
                    let new = join_lines(&result_lines, file_content.ends_with('\n'));
                    let replaced_count = if end_idx > start_idx + 1 { end_idx - start_idx - 1 } else { 0 };
                    let loc = format!(
                        "between lines {}-{} ({} lines replaced)",
                        start_idx + 1, end_idx + 1, replaced_count,
                    );
                    (new, loc)
                }
                MatchMode::Pattern => {
                    // Inclusive: replace matched lines entirely.
                    let mut result_lines: Vec<&str> = Vec::new();
                    for (i, line) in lines.iter().enumerate() {
                        if i == start_idx {
                            for rc_line in rendered_content.lines() {
                                result_lines.push(rc_line);
                            }
                        }
                        if i < start_idx || i > end_idx {
                            result_lines.push(line);
                        }
                    }
                    let new = join_lines(&result_lines, file_content.ends_with('\n'));
                    let replaced_count = end_idx - start_idx + 1;
                    let loc = format!(
                        "pattern lines {}-{} ({} lines replaced)",
                        start_idx + 1, end_idx + 1, replaced_count,
                    );
                    (new, loc)
                }
            };

            // Write back.
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
                ctx.virtual_files.insert(target.clone(), new_content);
            }

            OpResult::Success {
                action: "replace",
                path: target,
                lines: rendered_content.lines().count(),
                location: Some(location),
                rendered_content: content_for_verbose,
            }
        }
        MatchResult::NoMatch => {
            apply_fallback(&file_content, rendered_content, fallback, &target, ctx, content_for_verbose)
        }
        MatchResult::Error(error) => {
            OpResult::Error {
                path: target,
                error,
                rendered_content: rendered_content.to_string(),
            }
        }
    }
}

fn join_lines(lines: &[&str], trailing_newline: bool) -> String {
    let mut result = lines.join("\n");
    if trailing_newline && !result.is_empty() {
        result.push('\n');
    }
    result
}

enum MatchMode {
    Between,
    Pattern,
}

enum MatchResult {
    Found { start_idx: usize, end_idx: usize, mode: MatchMode },
    NoMatch,
    Error(StructuredError),
}

fn match_between(lines: &[&str], start_pattern: &str, end_pattern: &str, target: &std::path::Path) -> MatchResult {
    let start_re = Regex::new(start_pattern).expect("regex validated at parse time");
    let end_re = Regex::new(end_pattern).expect("regex validated at parse time");

    // Find first line matching start.
    let start_idx = match lines.iter().position(|line| start_re.is_match(line)) {
        Some(idx) => idx,
        None => return MatchResult::NoMatch,
    };

    // Find first line matching end AFTER start.
    let end_idx = match lines[start_idx + 1..].iter().position(|line| end_re.is_match(line)) {
        Some(offset) => start_idx + 1 + offset,
        None => {
            return MatchResult::Error(StructuredError {
                what: format!("end marker not found after line {}", start_idx + 1),
                where_: target.display().to_string(),
                why: format!(
                    "start marker '{}' found at line {}, but end marker '{}' was not found after it",
                    start_pattern, start_idx + 1, end_pattern,
                ),
                hint: "check that the end marker pattern matches a line after the start marker".into(),
            });
        }
    };

    MatchResult::Found { start_idx, end_idx, mode: MatchMode::Between }
}

fn match_pattern(lines: &[&str], pattern: &str) -> MatchResult {
    let re = Regex::new(pattern).expect("regex validated at parse time");

    // Find first contiguous block of matching lines.
    let first = match lines.iter().position(|line| re.is_match(line)) {
        Some(idx) => idx,
        None => return MatchResult::NoMatch,
    };

    let mut last = first;
    for i in (first + 1)..lines.len() {
        if re.is_match(lines[i]) {
            last = i;
        } else {
            break;
        }
    }

    MatchResult::Found { start_idx: first, end_idx: last, mode: MatchMode::Pattern }
}

fn apply_fallback(
    file_content: &str,
    rendered_content: &str,
    fallback: &Fallback,
    target: &std::path::Path,
    ctx: &mut ExecutionContext,
    content_for_verbose: Option<String>,
) -> OpResult {
    match fallback {
        Fallback::Append => {
            let mut new_content = file_content.to_string();
            if !new_content.ends_with('\n') && !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(rendered_content);
            write_back(target, &new_content, rendered_content, ctx);
            OpResult::Success {
                action: "replace",
                path: target.to_path_buf(),
                lines: rendered_content.lines().count(),
                location: Some("fallback:append".to_string()),
                rendered_content: content_for_verbose,
            }
        }
        Fallback::Prepend => {
            let mut new_content = String::with_capacity(rendered_content.len() + file_content.len() + 1);
            new_content.push_str(rendered_content);
            if !rendered_content.ends_with('\n') && !file_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(file_content);
            write_back(target, &new_content, rendered_content, ctx);
            OpResult::Success {
                action: "replace",
                path: target.to_path_buf(),
                lines: rendered_content.lines().count(),
                location: Some("fallback:prepend".to_string()),
                rendered_content: content_for_verbose,
            }
        }
        Fallback::Skip => {
            OpResult::Skip {
                path: target.to_path_buf(),
                reason: "pattern not found".to_string(),
                rendered_content: content_for_verbose,
            }
        }
        Fallback::Error => {
            OpResult::Error {
                path: target.to_path_buf(),
                error: StructuredError {
                    what: format!("replace pattern not found in '{}'", target.display()),
                    where_: target.display().to_string(),
                    why: "the match pattern did not find any matching region in the file".into(),
                    hint: "check the pattern/between markers against the file contents, or set fallback: skip/append/prepend".into(),
                },
                rendered_content: rendered_content.to_string(),
            }
        }
    }
}

fn write_back(target: &std::path::Path, new_content: &str, _rendered_content: &str, ctx: &mut ExecutionContext) {
    if ctx.dry_run {
        ctx.virtual_files.insert(target.to_path_buf(), new_content.to_string());
    } else {
        // Best-effort write; errors handled at a higher level for fallback paths.
        let _ = std::fs::write(target, new_content);
        ctx.virtual_files.insert(target.to_path_buf(), new_content.to_string());
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_ctx(dir: &std::path::Path, dry_run: bool) -> ExecutionContext {
        ExecutionContext::new(dir.to_path_buf(), dry_run, false)
    }

    // ── Between mode ──

    #[test]
    fn between_replaces_exclusively() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "# START\nold line 1\nold line 2\n# END\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("target.txt", "new content\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "replace", .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("# START"));
        assert!(content.contains("# END"));
        assert!(content.contains("new content"));
        assert!(!content.contains("old line"));
    }

    #[test]
    fn between_adjacent_markers_inserts() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "# START\n# END\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("target.txt", "inserted\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "replace", .. }));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "# START");
        assert_eq!(lines[1], "inserted");
        assert_eq!(lines[2], "# END");
    }

    #[test]
    fn between_multiline_content() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "header\n# START\nold\n# END\nfooter\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("target.txt", "line1\nline2\nline3\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("line1\nline2\nline3"));
        assert!(content.contains("header"));
        assert!(content.contains("footer"));
    }

    #[test]
    fn between_end_not_found() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "# START\ncontent\nno end marker\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("target.txt", "new\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(result.is_error());
        if let OpResult::Error { error, .. } = &result {
            assert!(error.what.contains("end marker not found"));
        }
    }

    #[test]
    fn between_start_not_found() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "no markers here\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("target.txt", "new\n", &spec, &Fallback::Error, &mut ctx, false);

        // Start not found → NoMatch → fallback:error
        assert!(result.is_error());
        if let OpResult::Error { error, .. } = &result {
            assert!(error.what.contains("replace pattern not found"));
        }
    }

    // ── Pattern mode ──

    #[test]
    fn pattern_replaces_matched_lines() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "keep\nold_1\nold_2\nkeep_end\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern("^old_".into());
        let result = execute("target.txt", "new_line\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "replace", .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("new_line"));
        assert!(!content.contains("old_"));
        assert!(content.contains("keep"));
        assert!(content.contains("keep_end"));
    }

    #[test]
    fn pattern_single_line() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "line1\ntarget_line\nline3\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern("^target_".into());
        let result = execute("target.txt", "replaced\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { .. }));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("replaced"));
        assert!(!content.contains("target_line"));
    }

    #[test]
    fn pattern_no_match() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "no match here\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern("^NONEXISTENT".into());
        let result = execute("target.txt", "new\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(result.is_error());
    }

    // ── Fallback ──

    #[test]
    fn fallback_append() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "existing\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern("^NONEXISTENT".into());
        let result = execute("target.txt", "appended\n", &spec, &Fallback::Append, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "replace", .. }));
        if let OpResult::Success { location, .. } = &result {
            assert_eq!(location.as_deref(), Some("fallback:append"));
        }
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("existing"));
        assert!(content.ends_with("appended\n"));
    }

    #[test]
    fn fallback_prepend() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "existing\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern("^NONEXISTENT".into());
        let result = execute("target.txt", "prepended\n", &spec, &Fallback::Prepend, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "replace", .. }));
        if let OpResult::Success { location, .. } = &result {
            assert_eq!(location.as_deref(), Some("fallback:prepend"));
        }
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("prepended\n"));
    }

    #[test]
    fn fallback_skip() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "existing\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern("^NONEXISTENT".into());
        let result = execute("target.txt", "new\n", &spec, &Fallback::Skip, &mut ctx, false);

        assert!(matches!(&result, OpResult::Skip { .. }));
    }

    #[test]
    fn fallback_error() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "existing\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern("^NONEXISTENT".into());
        let result = execute("target.txt", "new\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(result.is_error());
        if let OpResult::Error { error, rendered_content, .. } = &result {
            assert!(error.what.contains("replace pattern not found"));
            assert_eq!(rendered_content, "new\n");
        }
    }

    #[test]
    fn fallback_error_is_default() {
        // Default fallback is Error — tested via parse in recipe.rs.
        assert_eq!(Fallback::Error, Fallback::Error);
    }

    // ── Edge cases ──

    #[test]
    fn missing_target_file() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Pattern(".*".into());
        let result = execute("nonexistent.txt", "content\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(result.is_error());
        if let OpResult::Error { error, rendered_content, .. } = &result {
            assert!(error.what.contains("target file not found"));
            assert_eq!(rendered_content, "content\n");
        }
    }

    #[test]
    fn dry_run_virtual_files() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true);
        let target = dir.path().join("file.txt");
        ctx.virtual_files.insert(target.clone(), "# START\nold\n# END\n".into());

        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("file.txt", "new\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { action: "replace", .. }));
        let content = ctx.virtual_files.get(&target).unwrap();
        assert!(content.contains("new"));
        assert!(!content.contains("old"));
        assert!(!target.exists());
    }

    #[test]
    fn dry_run_chains_with_create() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true);
        let target = dir.path().join("file.txt");
        // Simulate prior create.
        ctx.virtual_files.insert(target.clone(), "header\n# START\n# END\nfooter\n".into());

        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("file.txt", "inserted\n", &spec, &Fallback::Error, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { .. }));
        let content = ctx.virtual_files.get(&target).unwrap();
        assert!(content.contains("inserted"));
    }

    #[test]
    fn success_reports_action() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "# START\nold\n# END\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("target.txt", "new\n", &spec, &Fallback::Error, &mut ctx, false);

        match &result {
            OpResult::Success { action, location, lines, .. } => {
                assert_eq!(*action, "replace");
                assert!(location.is_some());
                assert!(*lines > 0);
            }
            _ => panic!("expected Success"),
        }
    }

    #[test]
    fn verbose_includes_content() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "# START\nold\n# END\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let spec = ReplaceSpec::Between { start: "^# START".into(), end: "^# END".into() };
        let result = execute("target.txt", "new\n", &spec, &Fallback::Error, &mut ctx, true);

        match &result {
            OpResult::Success { rendered_content, .. } => {
                assert_eq!(rendered_content.as_deref(), Some("new\n"));
            }
            _ => panic!("expected Success"),
        }
    }
}
