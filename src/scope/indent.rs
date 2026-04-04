use crate::error::StructuredError;
use crate::recipe::ScopeType;

use super::ScopeResult;

/// Detect scope via indentation (for Python-style, YAML, etc.).
pub fn detect_indent_scope(
    lines: &[&str],
    anchor_line: usize,
    scope_type: &ScopeType,
) -> Result<ScopeResult, StructuredError> {
    if anchor_line >= lines.len() {
        return Err(StructuredError {
            what: format!("anchor line {} is out of range", anchor_line + 1),
            where_: format!("line {}", anchor_line + 1),
            why: format!("file has only {} lines", lines.len()),
            hint: "check the anchor pattern matches a valid line".into(),
        });
    }

    let anchor_indent = measure_indent(lines[anchor_line]);

    // For class_body/function_body, find the start of the body.
    let body_start = match scope_type {
        ScopeType::ClassBody | ScopeType::FunctionBody => {
            find_body_start(lines, anchor_line)
        }
        _ => anchor_line + 1,
    };

    if body_start >= lines.len() {
        return Ok(ScopeResult {
            start_line: body_start.min(lines.len().saturating_sub(1)),
            end_line: body_start.min(lines.len().saturating_sub(1)),
            closing_line: None,
            is_empty: true,
        });
    }

    // Walk forward: include deeper-indented lines, skip blank lines, stop at same/shallower.
    let mut end_line = None;
    let mut i = body_start;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            // Blank line: include if followed by deeper-indented content.
            i += 1;
            continue;
        }
        let line_indent = measure_indent(line);
        if line_indent > anchor_indent {
            end_line = Some(i);
            i += 1;
        } else {
            break;
        }
    }

    match end_line {
        Some(end) => {
            // Trim trailing blank lines from scope.
            let mut actual_end = end;
            while actual_end > body_start && lines[actual_end].trim().is_empty() {
                actual_end -= 1;
            }
            Ok(ScopeResult {
                start_line: body_start,
                end_line: actual_end,
                closing_line: None,
                is_empty: false,
            })
        }
        None => {
            Ok(ScopeResult {
                start_line: body_start.min(lines.len().saturating_sub(1)),
                end_line: body_start.min(lines.len().saturating_sub(1)),
                closing_line: None,
                is_empty: true,
            })
        }
    }
}

/// Find the start of the body for class/function declarations.
/// Handles multi-line declarations by scanning forward for the colon.
fn find_body_start(lines: &[&str], anchor_line: usize) -> usize {
    for i in anchor_line..lines.len() {
        if lines[i].contains(':') {
            return i + 1;
        }
        // Stop at blank lines or non-continuation lines after anchor.
        if i > anchor_line && lines[i].trim().is_empty() {
            break;
        }
    }
    anchor_line + 1
}

/// Measure raw whitespace prefix length (tab = 1 char, space = 1 char).
fn measure_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::ScopeType;

    #[test]
    fn python_class_body() {
        let lines: Vec<&str> = vec![
            "class User:",
            "    name = ''",
            "    age = 0",
            "",
            "other = True",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::ClassBody).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert!(!scope.is_empty);
    }

    #[test]
    fn python_function_body() {
        let lines: Vec<&str> = vec![
            "def hello():",
            "    print('hi')",
            "    return True",
            "",
            "x = 1",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::FunctionBody).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert!(!scope.is_empty);
    }

    #[test]
    fn blank_lines_within_scope() {
        let lines: Vec<&str> = vec![
            "class Foo:",
            "    a = 1",
            "",
            "    b = 2",
            "",
            "outside",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::ClassBody).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 3);
        assert!(!scope.is_empty);
    }

    #[test]
    fn empty_scope() {
        let lines: Vec<&str> = vec![
            "class Empty:",
            "other",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::ClassBody).unwrap();
        assert!(scope.is_empty);
    }

    #[test]
    fn nested_class() {
        let lines: Vec<&str> = vec![
            "class Outer:",
            "    class Inner:",
            "        x = 1",
            "    y = 2",
            "",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::ClassBody).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 3);
    }

    #[test]
    fn nested_function() {
        let lines: Vec<&str> = vec![
            "def outer():",
            "    def inner():",
            "        return 1",
            "    return inner()",
            "",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::FunctionBody).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 3);
    }

    #[test]
    fn mixed_tabs_spaces() {
        // Tab-indented body (1 tab > 0 spaces anchor).
        let lines: Vec<&str> = vec![
            "class Foo:",
            "\tname = ''",
            "\tage = 0",
            "",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::ClassBody).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
    }

    #[test]
    fn multiline_class_declaration() {
        let lines: Vec<&str> = vec![
            "class MyModel(",
            "        BaseModel):",
            "    name = ''",
            "    age = 0",
            "",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::ClassBody).unwrap();
        // Body starts after the colon line.
        assert_eq!(scope.start_line, 2);
        assert_eq!(scope.end_line, 3);
    }

    #[test]
    fn decorator_before_class() {
        // Decorator is before the class, anchor is on the class line.
        let lines: Vec<&str> = vec![
            "@decorator",
            "class Foo:",
            "    x = 1",
            "",
        ];
        let scope = detect_indent_scope(&lines, 1, &ScopeType::ClassBody).unwrap();
        assert_eq!(scope.start_line, 2);
        assert_eq!(scope.end_line, 2);
    }

    #[test]
    fn yaml_nested_block() {
        let lines: Vec<&str> = vec![
            "server:",
            "  host: localhost",
            "  port: 8080",
            "database:",
            "  host: db",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::Block).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
    }

    #[test]
    fn deeply_nested_indent() {
        let lines: Vec<&str> = vec![
            "level0:",
            "  level1:",
            "    level2a: a",
            "    level2b: b",
            "  level1b: c",
            "other:",
        ];
        let scope = detect_indent_scope(&lines, 0, &ScopeType::Block).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 4);
    }
}
