use regex::Regex;

use crate::error::StructuredError;
use crate::recipe::{Anchor, ScopeType};
use crate::scope;

use super::{ExecutionContext, OpResult};

/// Execute a patch operation: anchor→scope→find→position→insert.
pub fn execute(
    rendered_path: &str,
    rendered_content: &str,
    rendered_skip_if: Option<&str>,
    anchor: &Anchor,
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
                        hint: "ensure the target file exists, or add a create operation before this patch".into(),
                    },
                    rendered_content: rendered_content.to_string(),
                };
            }
        }
    };

    // Check skip_if: search entire file content.
    if let Some(skip_str) = rendered_skip_if
        && file_content.contains(skip_str)
    {
        return OpResult::Skip {
            path: target,
            reason: format!("skip_if matched: {skip_str}"),
            rendered_content: content_for_verbose,
        };
    }

    let lines: Vec<&str> = file_content.lines().collect();

    // Find anchor line.
    let anchor_re = Regex::new(&anchor.pattern).expect("regex validated during preparation");
    let anchor_line = match lines.iter().position(|line| anchor_re.is_match(line)) {
        Some(idx) => idx,
        None => {
            return OpResult::Error {
                path: target.clone(),
                error: StructuredError {
                    what: format!("anchor pattern '{}' not found", anchor.pattern),
                    where_: target.display().to_string(),
                    why: format!(
                        "pattern '{}' did not match any line in '{}'",
                        anchor.pattern, rendered_path
                    ),
                    hint: "check the anchor pattern against the file contents".into(),
                },
                rendered_content: rendered_content.to_string(),
            };
        }
    };

    // scope: line degenerate case — just insert after anchor.
    if anchor.scope == ScopeType::Line {
        let mut result_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let indent = detect_anchor_indent(&lines, anchor_line);
        let adjusted = adjust_indentation(rendered_content, &indent);
        let insert_at = anchor_line + 1;
        for (j, ins_line) in adjusted.lines().enumerate() {
            result_lines.insert(insert_at + j, ins_line.to_string());
        }
        let new_content = join_owned_lines(&result_lines, file_content.ends_with('\n'));
        if let Some(err) = write_back(&target, &new_content, rendered_content, ctx) {
            return err;
        }

        return OpResult::Success {
            action: "patch",
            path: target,
            lines: rendered_content.lines().count(),
            location: Some(format!("line({}):after", anchor.pattern)),
            rendered_content: content_for_verbose,
            scope_diagnostics: None,
        };
    }

    // Detect scope.
    let scope_result = match scope::detect_scope(&lines, anchor_line, &anchor.scope) {
        Ok(s) => s,
        Err(e) => {
            return OpResult::Error {
                path: target,
                error: StructuredError {
                    what: format!("scope detection failed: {}", e.what),
                    where_: e.where_,
                    why: e.why,
                    hint: "try a simpler scope type (e.g., 'line') or use inject instead".into(),
                },
                rendered_content: rendered_content.to_string(),
            };
        }
    };

    // Apply find narrowing (if specified).
    let mut find_match_line = None;
    let effective_scope = if let Some(ref find_str) = anchor.find {
        match scope::find_within_scope(&lines, &scope_result, find_str) {
            Ok(find_result) => {
                find_match_line = Some(find_result.found_line);
                if let Some(sub) = find_result.sub_scope {
                    sub
                } else {
                    scope_result
                }
            }
            Err(e) => {
                return OpResult::Error {
                    path: target,
                    error: e,
                    rendered_content: rendered_content.to_string(),
                };
            }
        }
    } else {
        scope_result
    };

    // Resolve position.
    let pos_result = match scope::position::resolve_position(
        &lines,
        &effective_scope,
        &anchor.position,
        Some(rendered_content),
    ) {
        Ok(p) => p,
        Err(e) => {
            return OpResult::Error {
                path: target,
                error: e,
                rendered_content: rendered_content.to_string(),
            };
        }
    };

    // Adjust indentation and insert.
    let adjusted = adjust_indentation(rendered_content, &pos_result.indent);
    let mut result_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    let insert_at = pos_result.insertion_line.min(result_lines.len());
    for (j, ins_line) in adjusted.lines().enumerate() {
        result_lines.insert(insert_at + j, ins_line.to_string());
    }

    let new_content = join_owned_lines(&result_lines, file_content.ends_with('\n'));
    if let Some(err) = write_back(&target, &new_content, rendered_content, ctx) {
        return err;
    }

    let location = format!("{}({}):{}", anchor.scope, anchor.pattern, anchor.position);

    let diagnostics = if verbose {
        Some(super::ScopeDiagnostics {
            anchor_line: anchor_line + 1,
            scope_start: effective_scope.start_line + 1,
            scope_end: effective_scope.end_line + 1,
            insertion_line: insert_at + 1,
            find_match_line: find_match_line.map(|l| l + 1),
            position_fallback: pos_result.fallback,
        })
    } else {
        None
    };

    OpResult::Success {
        action: "patch",
        path: target,
        lines: rendered_content.lines().count(),
        location: Some(location),
        rendered_content: content_for_verbose,
        scope_diagnostics: diagnostics,
    }
}

/// Adjust indentation of rendered content to match target context.
fn adjust_indentation(rendered_content: &str, target_indent: &str) -> String {
    let content_lines: Vec<&str> = rendered_content.lines().collect();
    if content_lines.is_empty() {
        return String::new();
    }

    // Detect base indent of rendered content (first non-empty line).
    let base_indent_len = content_lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);

    let mut result = String::new();
    for (i, line) in content_lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        if line.trim().is_empty() {
            // Blank lines stay blank.
            continue;
        }
        let line_indent = line.len() - line.trim_start().len();
        let relative = line_indent.saturating_sub(base_indent_len);
        result.push_str(target_indent);
        for _ in 0..relative {
            result.push(' ');
        }
        result.push_str(line.trim_start());
    }
    result
}

fn detect_anchor_indent(lines: &[&str], anchor_line: usize) -> String {
    if anchor_line < lines.len() {
        let line = lines[anchor_line];
        let indent_len = line.len() - line.trim_start().len();
        line[..indent_len].to_string()
    } else {
        String::new()
    }
}

fn join_owned_lines(lines: &[String], trailing_newline: bool) -> String {
    let mut result = lines.join("\n");
    if trailing_newline && !result.is_empty() {
        result.push('\n');
    }
    result
}

fn write_back(
    target: &std::path::Path,
    new_content: &str,
    rendered_content: &str,
    ctx: &mut ExecutionContext,
) -> Option<OpResult> {
    if ctx.dry_run {
        ctx.virtual_files
            .insert(target.to_path_buf(), new_content.to_string());
        None
    } else {
        if let Err(e) = std::fs::write(target, new_content) {
            return Some(OpResult::Error {
                path: target.to_path_buf(),
                error: StructuredError {
                    what: format!("cannot write file '{}'", target.display()),
                    where_: target.display().to_string(),
                    why: e.to_string(),
                    hint: "check file permissions".into(),
                },
                rendered_content: rendered_content.to_string(),
            });
        }
        ctx.virtual_files
            .insert(target.to_path_buf(), new_content.to_string());
        None
    }
}

// ── Tests ───────────────────────────────────��──────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::{Anchor, Position, ScopeType};
    use std::fs;
    use tempfile::TempDir;

    fn make_ctx(dir: &std::path::Path, dry_run: bool) -> ExecutionContext {
        ExecutionContext::new(dir.to_path_buf(), dry_run, false)
    }

    fn anchor(pattern: &str, scope: ScopeType, position: Position) -> Anchor {
        Anchor {
            pattern: pattern.to_string(),
            scope,
            find: None,
            position,
        }
    }

    #[test]
    fn patch_class_body_after_last_field() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("models.py");
        fs::write(&target, "class User:\n    name = ''\n    age = 0\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^class User:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute("models.py", "email = ''", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("email = ''"));
        assert!(content.contains("age = 0"));
    }

    #[test]
    fn patch_function_body_before_close() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("main.py");
        fs::write(&target, "def hello():\n    print('hi')\n    return True\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^def hello\\(\\):",
            ScopeType::FunctionBody,
            Position::After,
        );
        let result = execute("main.py", "print('bye')", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
    }

    #[test]
    fn patch_braces_before_close() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("config.rs");
        fs::write(&target, "struct Config {\n    name: String,\n}\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor("^struct Config", ScopeType::Braces, Position::BeforeClose);
        let result = execute("config.rs", "value: i32,", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("value: i32,"));
    }

    #[test]
    fn patch_brackets_before_close() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("items.py");
        fs::write(&target, "items = [\n    'a',\n    'b',\n]\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor("^items = \\[", ScopeType::Brackets, Position::BeforeClose);
        let result = execute("items.py", "'c',", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("'c',"));
    }

    #[test]
    fn patch_function_signature() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("func.py");
        fs::write(
            &target,
            "def process(\n    arg1: str,\n    arg2: int,\n):\n    pass\n",
        )
        .unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor("^def process\\(", ScopeType::Parens, Position::BeforeClose);
        let result = execute("func.py", "arg3: bool,", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("arg3: bool,"));
    }

    #[test]
    fn patch_find_narrowing_brackets() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("admin.py");
        fs::write(
            &target,
            "class Admin:\n    list_display = [\n        'name',\n    ]\n    other = 1\n",
        )
        .unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = Anchor {
            pattern: "^class Admin:".into(),
            scope: ScopeType::ClassBody,
            find: Some("list_display".into()),
            position: Position::BeforeClose,
        };
        let result = execute("admin.py", "'email',", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("'email',"));
    }

    #[test]
    fn patch_skip_if_matched() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class User:\n    email = ''\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^class User:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute(
            "target.py",
            "email = ''",
            Some("email = ''"),
            &a,
            &mut ctx,
            false,
        );

        assert!(matches!(&result, OpResult::Skip { .. }));
    }

    #[test]
    fn patch_skip_if_not_matched() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class User:\n    name = ''\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^class User:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute(
            "target.py",
            "email = ''",
            Some("NONEXISTENT"),
            &a,
            &mut ctx,
            false,
        );

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
    }

    #[test]
    fn patch_scope_line() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "# marker\nold line\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor("^# marker", ScopeType::Line, Position::After);
        let result = execute("target.py", "new line", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
        let content = fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "# marker");
        assert_eq!(lines[1], "new line");
    }

    #[test]
    fn patch_indent_matching() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class Foo:\n    x = 1\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^class Foo:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute("target.py", "y = 2", None, &a, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { .. }));
        let content = fs::read_to_string(&target).unwrap();
        // The inserted line should have the same indentation as siblings.
        assert!(content.contains("    y = 2"));
    }

    #[test]
    fn patch_anchor_not_found() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class Foo:\n    x = 1\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor("^class NonExistent", ScopeType::ClassBody, Position::After);
        let result = execute("target.py", "new", None, &a, &mut ctx, false);

        assert!(result.is_error());
        if let OpResult::Error {
            error,
            rendered_content,
            ..
        } = &result
        {
            assert!(error.what.contains("anchor pattern"));
            assert_eq!(rendered_content, "new");
        }
    }

    #[test]
    fn patch_find_not_found() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class Foo:\n    x = 1\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = Anchor {
            pattern: "^class Foo:".into(),
            scope: ScopeType::ClassBody,
            find: Some("nonexistent".into()),
            position: Position::BeforeClose,
        };
        let result = execute("target.py", "new", None, &a, &mut ctx, false);

        assert!(result.is_error());
    }

    #[test]
    fn patch_dry_run_virtual_files() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true);
        let target = dir.path().join("target.py");
        ctx.virtual_files
            .insert(target.clone(), "class Foo:\n    x = 1\n".into());

        let a = anchor(
            "^class Foo:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute("target.py", "y = 2", None, &a, &mut ctx, false);

        assert!(matches!(
            &result,
            OpResult::Success {
                action: "patch",
                ..
            }
        ));
        assert!(!target.exists());
        let content = ctx.virtual_files.get(&target).unwrap();
        assert!(content.contains("y = 2"));
    }

    #[test]
    fn patch_chains_with_prior_create() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), true);
        let target = dir.path().join("new.py");
        ctx.virtual_files
            .insert(target.clone(), "class New:\n    x = 1\n".into());

        let a = anchor(
            "^class New:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute("new.py", "y = 2", None, &a, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { .. }));
    }

    #[test]
    fn patch_first_match_used() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class Foo:\n    x = 1\nclass Foo:\n    y = 2\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^class Foo:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute("target.py", "z = 3", None, &a, &mut ctx, false);

        assert!(matches!(&result, OpResult::Success { .. }));
        let content = fs::read_to_string(&target).unwrap();
        // z = 3 should be inserted after x = 1 (first match), not after y = 2.
        let z_pos = content.find("z = 3").unwrap();
        let x_pos = content.find("x = 1").unwrap();
        let y_pos = content.find("y = 2").unwrap();
        assert!(z_pos > x_pos && z_pos < y_pos);
    }

    #[test]
    fn patch_missing_target_file() {
        let dir = TempDir::new().unwrap();
        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor("^class", ScopeType::ClassBody, Position::After);
        let result = execute("nonexistent.py", "content", None, &a, &mut ctx, false);

        assert!(result.is_error());
        if let OpResult::Error { error, .. } = &result {
            assert!(error.what.contains("target file not found"));
        }
    }

    #[test]
    fn patch_verbose_diagnostics() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class Foo:\n    x = 1\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^class Foo:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );
        let result = execute("target.py", "y = 2", None, &a, &mut ctx, true);

        match &result {
            OpResult::Success {
                rendered_content,
                location,
                ..
            } => {
                assert_eq!(rendered_content.as_deref(), Some("y = 2"));
                assert!(location.is_some());
            }
            _ => panic!("expected Success"),
        }
    }

    #[test]
    fn patch_idempotency_with_skip_if() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.py");
        fs::write(&target, "class Foo:\n    x = 1\n").unwrap();

        let mut ctx = make_ctx(dir.path(), false);
        let a = anchor(
            "^class Foo:",
            ScopeType::ClassBody,
            Position::AfterLastField,
        );

        // First patch.
        let r1 = execute("target.py", "y = 2", Some("y = 2"), &a, &mut ctx, false);
        assert!(matches!(&r1, OpResult::Success { .. }));

        // Second patch: should skip.
        let r2 = execute("target.py", "y = 2", Some("y = 2"), &a, &mut ctx, false);
        assert!(matches!(&r2, OpResult::Skip { .. }));
    }

    #[test]
    fn adjust_indentation_preserves_relative() {
        let input = "if True:\n    print('hi')";
        let result = adjust_indentation(input, "        ");
        assert_eq!(result, "        if True:\n            print('hi')");
    }

    #[test]
    fn adjust_indentation_blank_lines() {
        let input = "a\n\nb";
        let result = adjust_indentation(input, "    ");
        assert_eq!(result, "    a\n\n    b");
    }
}
