pub mod delimiter;
pub mod indent;
pub mod position;

use crate::error::StructuredError;
use crate::recipe::ScopeType;

// ── Scope result types ───────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScopeResult {
    /// First line of the scope body (inclusive).
    pub start_line: usize,
    /// Last line of the scope body (inclusive).
    pub end_line: usize,
    /// Line of closing delimiter (if applicable).
    pub closing_line: Option<usize>,
    /// True if scope is empty (no body lines).
    pub is_empty: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PositionResult {
    /// Line index where content should be inserted.
    pub insertion_line: usize,
    /// Indentation string for inserted content.
    pub indent: String,
    /// If a fallback was used: (original_position, fallback_position).
    pub fallback: Option<(String, String)>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FindResult {
    /// Line index where the find string was found.
    pub found_line: usize,
    /// Sub-scope detected on the found line (if any).
    pub sub_scope: Option<ScopeResult>,
}

// ── Scope detection dispatch ─────────────────────────────────────

/// Detect scope boundaries starting from the anchor line.
pub fn detect_scope(
    lines: &[&str],
    anchor_line: usize,
    scope_type: &ScopeType,
) -> Result<ScopeResult, StructuredError> {
    match scope_type {
        ScopeType::Line => {
            Ok(ScopeResult {
                start_line: anchor_line,
                end_line: anchor_line,
                closing_line: None,
                is_empty: false,
            })
        }
        ScopeType::Block | ScopeType::ClassBody | ScopeType::FunctionBody => {
            indent::detect_indent_scope(lines, anchor_line, scope_type)
        }
        ScopeType::Braces => delimiter::detect_delimiter_scope(lines, anchor_line, '{', '}'),
        ScopeType::Brackets => delimiter::detect_delimiter_scope(lines, anchor_line, '[', ']'),
        ScopeType::Parens | ScopeType::FunctionSignature => {
            delimiter::detect_delimiter_scope(lines, anchor_line, '(', ')')
        }
    }
}

/// Search for a string within a scope, optionally detecting sub-scopes.
pub fn find_within_scope(
    lines: &[&str],
    scope: &ScopeResult,
    find_str: &str,
) -> Result<FindResult, StructuredError> {
    for i in scope.start_line..=scope.end_line {
        if i < lines.len() && lines[i].contains(find_str) {
            let sub_scope = detect_sub_scope(lines, i);
            return Ok(FindResult {
                found_line: i,
                sub_scope,
            });
        }
    }

    Err(StructuredError {
        what: format!("find string '{}' not found within scope", find_str),
        where_: format!("scope lines {}-{}", scope.start_line + 1, scope.end_line + 1),
        why: format!(
            "searched lines {}-{} but '{}' was not found",
            scope.start_line + 1, scope.end_line + 1, find_str,
        ),
        hint: "check the find string against the scope contents".into(),
    })
}

/// Detect if a line opens a sub-scope (trailing delimiter or assignment with delimiter).
fn detect_sub_scope(lines: &[&str], line_idx: usize) -> Option<ScopeResult> {
    if line_idx >= lines.len() {
        return None;
    }
    let line = lines[line_idx].trim_end();

    // Check for trailing delimiters.
    let (open, close) = if line.ends_with('{') {
        ('{', '}')
    } else if line.ends_with('[') {
        ('[', ']')
    } else if line.ends_with('(') {
        ('(', ')')
    } else if line.contains(" = [") || line.contains("= [") {
        ('[', ']')
    } else if line.contains(" = {") || line.contains("= {") {
        ('{', '}')
    } else {
        return None;
    };

    delimiter::detect_delimiter_scope(lines, line_idx, open, close).ok()
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::ScopeType;

    #[test]
    fn line_scope_is_single_line() {
        let lines: Vec<&str> = vec!["line0", "line1", "line2"];
        let scope = detect_scope(&lines, 1, &ScopeType::Line).unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 1);
        assert!(!scope.is_empty);
    }

    #[test]
    fn find_within_scope_basic() {
        let lines: Vec<&str> = vec!["class Foo:", "    name = 'x'", "    age = 10", ""];
        let scope = ScopeResult { start_line: 1, end_line: 2, closing_line: None, is_empty: false };
        let result = find_within_scope(&lines, &scope, "age").unwrap();
        assert_eq!(result.found_line, 2);
    }

    #[test]
    fn find_detects_bracket_sub_scope() {
        let lines: Vec<&str> = vec![
            "class Admin:",
            "    list_display = [",
            "        'name',",
            "    ]",
            "",
        ];
        let scope = ScopeResult { start_line: 1, end_line: 3, closing_line: None, is_empty: false };
        let result = find_within_scope(&lines, &scope, "list_display").unwrap();
        assert_eq!(result.found_line, 1);
        assert!(result.sub_scope.is_some());
        let sub = result.sub_scope.unwrap();
        assert_eq!(sub.closing_line, Some(3));
    }

    #[test]
    fn find_detects_brace_sub_scope() {
        let lines: Vec<&str> = vec![
            "fn main() {",
            "    let config = Config {",
            "        name: \"test\",",
            "    };",
            "}",
        ];
        let scope = ScopeResult { start_line: 1, end_line: 3, closing_line: Some(4), is_empty: false };
        let result = find_within_scope(&lines, &scope, "config").unwrap();
        assert_eq!(result.found_line, 1);
        assert!(result.sub_scope.is_some());
    }

    #[test]
    fn find_not_found() {
        let lines: Vec<&str> = vec!["line0", "line1", "line2"];
        let scope = ScopeResult { start_line: 0, end_line: 2, closing_line: None, is_empty: false };
        let err = find_within_scope(&lines, &scope, "nonexistent").unwrap_err();
        assert!(err.what.contains("not found within scope"));
    }

    #[test]
    fn find_before_close_on_sub_scope() {
        let lines: Vec<&str> = vec![
            "struct Config {",
            "    items: Vec<String>,",
            "    mapping = {",
            "        \"a\": 1,",
            "    }",
            "}",
        ];
        let scope = ScopeResult { start_line: 1, end_line: 4, closing_line: Some(5), is_empty: false };
        let result = find_within_scope(&lines, &scope, "mapping").unwrap();
        assert_eq!(result.found_line, 2);
        assert!(result.sub_scope.is_some());
    }
}
