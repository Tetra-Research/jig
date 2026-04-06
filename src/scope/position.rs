use regex::Regex;

use crate::error::StructuredError;
use crate::recipe::Position;

use super::PositionResult;
use super::ScopeResult;

/// Resolve the insertion position within a scope.
/// `rendered_content` is needed for `Position::Sorted` to determine alphabetical placement.
pub fn resolve_position(
    lines: &[&str],
    scope: &ScopeResult,
    position: &Position,
    rendered_content: Option<&str>,
) -> Result<PositionResult, StructuredError> {
    let indent = detect_scope_indent(lines, scope);

    match position {
        Position::Before => Ok(PositionResult {
            insertion_line: scope.start_line,
            indent,
            fallback: None,
        }),
        Position::After => Ok(PositionResult {
            insertion_line: scope.end_line + 1,
            indent,
            fallback: None,
        }),
        Position::BeforeClose => {
            let line = scope.closing_line.unwrap_or(scope.end_line + 1);
            Ok(PositionResult {
                insertion_line: line,
                indent,
                fallback: None,
            })
        }
        Position::AfterLastField => {
            let field_re = Regex::new(r"^\s*\w+\s*[:=]").unwrap();
            match find_last_matching(lines, scope, &field_re) {
                Some(line_idx) => Ok(PositionResult {
                    insertion_line: line_idx + 1,
                    indent,
                    fallback: None,
                }),
                None => {
                    // Fallback to before_close.
                    let line = scope.closing_line.unwrap_or(scope.end_line + 1);
                    Ok(PositionResult {
                        insertion_line: line,
                        indent,
                        fallback: Some(("after_last_field".into(), "before_close".into())),
                    })
                }
            }
        }
        Position::AfterLastMethod => {
            let method_re = Regex::new(r#"^\s*(pub(\s*\([^)]*\))?\s+)?(const\s+)?(unsafe\s+)?(async\s+)?(extern\s+"[^"]*"\s+)?(fn|def)\s+\w+"#).unwrap();
            match find_last_matching(lines, scope, &method_re) {
                Some(method_line) => {
                    // Find the end of this method's body.
                    let method_end = find_method_body_end(lines, method_line);
                    Ok(PositionResult {
                        insertion_line: method_end + 1,
                        indent,
                        fallback: None,
                    })
                }
                None => {
                    let line = scope.closing_line.unwrap_or(scope.end_line + 1);
                    Ok(PositionResult {
                        insertion_line: line,
                        indent,
                        fallback: Some(("after_last_method".into(), "before_close".into())),
                    })
                }
            }
        }
        Position::AfterLastImport => {
            let import_re = Regex::new(r"^\s*(from|import)\s+").unwrap();
            match find_last_matching(lines, scope, &import_re) {
                Some(line_idx) => Ok(PositionResult {
                    insertion_line: line_idx + 1,
                    indent,
                    fallback: None,
                }),
                None => Ok(PositionResult {
                    insertion_line: scope.start_line,
                    indent,
                    fallback: Some(("after_last_import".into(), "before".into())),
                }),
            }
        }
        Position::Sorted => resolve_sorted(lines, scope, &indent, rendered_content),
    }
}

/// Find the last line matching a regex within scope bounds.
fn find_last_matching(lines: &[&str], scope: &ScopeResult, re: &Regex) -> Option<usize> {
    let mut last = None;
    for i in scope.start_line..=scope.end_line {
        if i < lines.len() && re.is_match(lines[i]) {
            last = Some(i);
        }
    }
    last
}

/// Find the end of a method body starting from the method declaration line.
fn find_method_body_end(lines: &[&str], method_line: usize) -> usize {
    if method_line >= lines.len() {
        return method_line;
    }
    let method_indent = measure_indent(lines[method_line]);

    // Walk forward past the method body.
    let mut end = method_line;
    for (i, line) in lines.iter().enumerate().skip(method_line + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let line_indent = measure_indent(line);
        if line_indent > method_indent {
            end = i;
        } else if line_indent == method_indent && line.trim().starts_with('}') {
            // Closing brace at method indent level — include it.
            end = i;
            break;
        } else {
            break;
        }
    }
    end
}

/// Detect the indentation level used within a scope.
fn detect_scope_indent(lines: &[&str], scope: &ScopeResult) -> String {
    // Find first non-blank line within scope.
    for i in scope.start_line..=scope.end_line {
        if i < lines.len() && !lines[i].trim().is_empty() {
            let indent_len = measure_indent(lines[i]);
            return lines[i][..indent_len].to_string();
        }
    }

    // Empty scope: use anchor indent + 4 spaces.
    if scope.start_line > 0 && scope.start_line <= lines.len() {
        let anchor_line = scope.start_line.saturating_sub(1);
        if anchor_line < lines.len() {
            let anchor_indent = measure_indent(lines[anchor_line]);
            return " ".repeat(anchor_indent + 4);
        }
    }

    "    ".to_string()
}

/// Resolve sorted insertion position (alphabetical among siblings).
/// Finds the correct insertion point so the new content is in alphabetical
/// order among sibling lines at the same indentation level within the scope.
fn resolve_sorted(
    lines: &[&str],
    scope: &ScopeResult,
    indent: &str,
    rendered_content: Option<&str>,
) -> Result<PositionResult, StructuredError> {
    let sort_key = rendered_content
        .and_then(|c| c.lines().next())
        .map(|l| l.trim())
        .unwrap_or("");

    let indent_len = indent.len();

    // Collect sibling lines (same indentation level within scope).
    for i in scope.start_line..=scope.end_line {
        if i >= lines.len() {
            continue;
        }
        let line = lines[i];
        if line.trim().is_empty() {
            continue;
        }
        let line_indent = line.len() - line.trim_start().len();
        if line_indent != indent_len {
            continue;
        }
        // Compare trimmed content alphabetically.
        if line.trim() > sort_key {
            return Ok(PositionResult {
                insertion_line: i,
                indent: indent.to_string(),
                fallback: None,
            });
        }
    }

    // All existing siblings sort before — insert at end of scope.
    let insertion = scope.closing_line.unwrap_or(scope.end_line + 1);
    Ok(PositionResult {
        insertion_line: insertion,
        indent: indent.to_string(),
        fallback: None,
    })
}

fn measure_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Position;
    use crate::scope::ScopeResult;

    fn scope(start: usize, end: usize, closing: Option<usize>) -> ScopeResult {
        ScopeResult {
            start_line: start,
            end_line: end,
            closing_line: closing,
            is_empty: false,
        }
    }

    #[test]
    fn position_before() {
        let lines: Vec<&str> = vec!["class Foo:", "    x = 1", "    y = 2"];
        let s = scope(1, 2, None);
        let result = resolve_position(&lines, &s, &Position::Before, None).unwrap();
        assert_eq!(result.insertion_line, 1);
    }

    #[test]
    fn position_after() {
        let lines: Vec<&str> = vec!["class Foo:", "    x = 1", "    y = 2"];
        let s = scope(1, 2, None);
        let result = resolve_position(&lines, &s, &Position::After, None).unwrap();
        assert_eq!(result.insertion_line, 3);
    }

    #[test]
    fn position_before_close() {
        let lines: Vec<&str> = vec!["struct Foo {", "    x: i32,", "}"];
        let s = scope(1, 1, Some(2));
        let result = resolve_position(&lines, &s, &Position::BeforeClose, None).unwrap();
        assert_eq!(result.insertion_line, 2);
    }

    #[test]
    fn after_last_field_python() {
        let lines: Vec<&str> = vec!["class User:", "    name = ''", "    age = 0", ""];
        let s = scope(1, 2, None);
        let result = resolve_position(&lines, &s, &Position::AfterLastField, None).unwrap();
        assert_eq!(result.insertion_line, 3);
        assert!(result.fallback.is_none());
    }

    #[test]
    fn after_last_field_rust() {
        let lines: Vec<&str> = vec![
            "struct Config {",
            "    name: String,",
            "    value: i32,",
            "}",
        ];
        let s = scope(1, 2, Some(3));
        let result = resolve_position(&lines, &s, &Position::AfterLastField, None).unwrap();
        assert_eq!(result.insertion_line, 3);
    }

    #[test]
    fn after_last_method_python() {
        let lines: Vec<&str> = vec![
            "class Foo:",
            "    def bar(self):",
            "        return 1",
            "    def baz(self):",
            "        return 2",
            "",
        ];
        let s = scope(1, 4, None);
        let result = resolve_position(&lines, &s, &Position::AfterLastMethod, None).unwrap();
        assert_eq!(result.insertion_line, 5);
    }

    #[test]
    fn after_last_method_rust() {
        let lines: Vec<&str> = vec![
            "impl Foo {",
            "    pub fn bar(&self) -> i32 {",
            "        1",
            "    }",
            "    pub async fn baz(&self) {",
            "        todo!()",
            "    }",
            "}",
        ];
        let s = scope(1, 6, Some(7));
        let result = resolve_position(&lines, &s, &Position::AfterLastMethod, None).unwrap();
        assert_eq!(result.insertion_line, 7);
    }

    #[test]
    fn after_last_import() {
        let lines: Vec<&str> = vec![
            "module:",
            "    from foo import bar",
            "    import baz",
            "    x = 1",
        ];
        let s = scope(1, 3, None);
        let result = resolve_position(&lines, &s, &Position::AfterLastImport, None).unwrap();
        assert_eq!(result.insertion_line, 3);
    }

    #[test]
    fn sorted_insertion_middle() {
        let lines: Vec<&str> = vec!["struct Foo {", "    a: i32,", "    c: i32,", "}"];
        let s = scope(1, 2, Some(3));
        // "b: i32," sorts between "a:" and "c:"
        let result = resolve_position(&lines, &s, &Position::Sorted, Some("b: i32,")).unwrap();
        assert_eq!(result.insertion_line, 2); // before "c: i32,"
    }

    #[test]
    fn sorted_insertion_end() {
        let lines: Vec<&str> = vec!["struct Foo {", "    a: i32,", "    b: i32,", "}"];
        let s = scope(1, 2, Some(3));
        // "z: i32," sorts after everything
        let result = resolve_position(&lines, &s, &Position::Sorted, Some("z: i32,")).unwrap();
        assert_eq!(result.insertion_line, 3); // before closing brace
    }

    #[test]
    fn sorted_insertion_beginning() {
        let lines: Vec<&str> = vec!["struct Foo {", "    m: i32,", "    z: i32,", "}"];
        let s = scope(1, 2, Some(3));
        // "a: i32," sorts before everything
        let result = resolve_position(&lines, &s, &Position::Sorted, Some("a: i32,")).unwrap();
        assert_eq!(result.insertion_line, 1); // before "m: i32,"
    }

    #[test]
    fn after_last_field_fallback() {
        let lines: Vec<&str> = vec!["struct Empty {", "}"];
        let s = ScopeResult {
            start_line: 0,
            end_line: 1,
            closing_line: Some(1),
            is_empty: true,
        };
        let result = resolve_position(&lines, &s, &Position::AfterLastField, None).unwrap();
        assert!(result.fallback.is_some());
        let (from, to) = result.fallback.unwrap();
        assert_eq!(from, "after_last_field");
        assert_eq!(to, "before_close");
    }

    #[test]
    fn after_last_method_fallback() {
        let lines: Vec<&str> = vec!["class Foo:", "    x = 1"];
        let s = scope(1, 1, None);
        let result = resolve_position(&lines, &s, &Position::AfterLastMethod, None).unwrap();
        assert!(result.fallback.is_some());
    }

    #[test]
    fn after_last_import_fallback() {
        let lines: Vec<&str> = vec!["class Foo:", "    x = 1"];
        let s = scope(1, 1, None);
        let result = resolve_position(&lines, &s, &Position::AfterLastImport, None).unwrap();
        assert!(result.fallback.is_some());
        let (from, to) = result.fallback.unwrap();
        assert_eq!(from, "after_last_import");
        assert_eq!(to, "before");
    }

    #[test]
    fn after_last_method_const_fn() {
        let lines: Vec<&str> = vec![
            "impl Foo {",
            "    pub const fn size() -> usize {",
            "        42",
            "    }",
            "}",
        ];
        let s = scope(1, 3, Some(4));
        let result = resolve_position(&lines, &s, &Position::AfterLastMethod, None).unwrap();
        assert_eq!(result.insertion_line, 4);
    }

    #[test]
    fn after_last_method_unsafe_fn() {
        let lines: Vec<&str> = vec![
            "impl Foo {",
            "    pub unsafe fn danger(&self) {",
            "        todo!()",
            "    }",
            "}",
        ];
        let s = scope(1, 3, Some(4));
        let result = resolve_position(&lines, &s, &Position::AfterLastMethod, None).unwrap();
        assert_eq!(result.insertion_line, 4);
    }

    #[test]
    fn after_last_method_pub_crate_fn() {
        let lines: Vec<&str> = vec![
            "impl Foo {",
            "    pub(crate) fn internal(&self) {",
            "        todo!()",
            "    }",
            "}",
        ];
        let s = scope(1, 3, Some(4));
        let result = resolve_position(&lines, &s, &Position::AfterLastMethod, None).unwrap();
        assert_eq!(result.insertion_line, 4);
    }

    #[test]
    fn after_last_field_at_indent_zero() {
        let lines: Vec<&str> = vec!["CONST = 1", "OTHER = 2"];
        let s = scope(0, 1, None);
        let result = resolve_position(&lines, &s, &Position::AfterLastField, None).unwrap();
        assert_eq!(result.insertion_line, 2);
        assert!(result.fallback.is_none());
    }

    #[test]
    fn after_last_field_top_level_constant() {
        let lines: Vec<&str> = vec!["CONSTANT = 42", "OTHER = 'x'"];
        let s = scope(0, 1, None);
        let result = resolve_position(&lines, &s, &Position::AfterLastField, None).unwrap();
        assert_eq!(result.insertion_line, 2);
        assert!(result.fallback.is_none());
    }

    #[test]
    fn indent_detection() {
        let lines: Vec<&str> = vec!["class Foo:", "    x = 1", "    y = 2"];
        let s = scope(1, 2, None);
        let indent = detect_scope_indent(&lines, &s);
        assert_eq!(indent, "    ");
    }
}
