use crate::error::StructuredError;

use super::ScopeResult;

/// Detect scope via balanced delimiters (braces, brackets, parens).
pub fn detect_delimiter_scope(
    lines: &[&str],
    anchor_line: usize,
    open: char,
    close: char,
) -> Result<ScopeResult, StructuredError> {
    if anchor_line >= lines.len() {
        return Err(StructuredError {
            what: format!("anchor line {} is out of range", anchor_line + 1),
            where_: format!("line {}", anchor_line + 1),
            why: format!("file has only {} lines", lines.len()),
            hint: "check the anchor pattern matches a valid line".into(),
        });
    }

    // Find the opening delimiter starting from anchor_line.
    let mut scanner = CharScanner::new(lines, anchor_line, 0);
    let open_pos = find_opening(&mut scanner, open)?;

    // Record where the opener was found.
    let open_line = open_pos.0;
    let open_col = open_pos.1;

    // Advance past the opener.
    scanner.line = open_line;
    scanner.col = open_col + 1;

    // Track nesting depth.
    let mut depth: usize = 1;
    while depth > 0 {
        match scanner.peek() {
            None => {
                return Err(StructuredError {
                    what: format!("unbalanced '{}': closing '{}' not found", open, close),
                    where_: format!("line {} col {}", open_line + 1, open_col + 1),
                    why: format!(
                        "opening '{}' at line {} was never closed",
                        open, open_line + 1,
                    ),
                    hint: "check for mismatched delimiters, or use a simpler scope type".into(),
                });
            }
            Some(ch) => {
                if ch == '\\' {
                    // Escaped character — skip next.
                    scanner.advance();
                    scanner.advance();
                    continue;
                }
                if ch == '"' || ch == '\'' || ch == '`' {
                    scanner.skip_string_literal(ch);
                    continue;
                }
                if ch == '/' {
                    if scanner.peek_next() == Some('/') {
                        scanner.skip_line_comment();
                        continue;
                    }
                    if scanner.peek_next() == Some('*') {
                        scanner.skip_block_comment();
                        continue;
                    }
                }
                if ch == '#' {
                    scanner.skip_line_comment();
                    continue;
                }
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                    if depth == 0 {
                        let close_line = scanner.line;

                        // Determine scope content lines.
                        let content_start_line = open_line;
                        let content_end_line = close_line;

                        // Check for empty scope.
                        let is_empty = if open_line == close_line {
                            // Same line: check if there's content between delimiters.
                            let between = &lines[open_line][open_col + 1..scanner.col];
                            between.trim().is_empty()
                        } else {
                            // Multi-line: check if lines between are empty/whitespace.
                            let inner_start = open_line + 1;
                            let inner_end = close_line;
                            inner_start >= inner_end || (inner_start..inner_end).all(|i| lines[i].trim().is_empty())
                        };

                        // The scope body is between the delimiters (exclusive).
                        let body_start = open_line + 1;
                        let body_end = if close_line > 0 { close_line - 1 } else { open_line };

                        return Ok(ScopeResult {
                            start_line: if is_empty { content_start_line } else { body_start },
                            end_line: if is_empty { content_end_line } else { body_end.max(body_start) },
                            closing_line: Some(close_line),
                            is_empty,
                        });
                    }
                }
                scanner.advance();
            }
        }
    }

    unreachable!()
}

/// Find the opening delimiter starting from scanner position.
fn find_opening(scanner: &mut CharScanner, open: char) -> Result<(usize, usize), StructuredError> {
    while let Some(ch) = scanner.peek() {
        if ch == open {
            return Ok((scanner.line, scanner.col));
        }
        scanner.advance();
    }
    Err(StructuredError {
        what: format!("opening delimiter '{}' not found", open),
        where_: format!("starting from line {}", scanner.line + 1),
        why: format!("scanned from anchor line but '{}' was not found", open),
        hint: "check that the anchor line or a subsequent line contains the opening delimiter".into(),
    })
}

/// Character-by-character scanner over lines.
/// Uses byte offsets internally for correct UTF-8 handling and O(1) operations.
struct CharScanner<'a> {
    lines: &'a [&'a str],
    line: usize,
    col: usize, // byte offset within current line
}

impl<'a> CharScanner<'a> {
    fn new(lines: &'a [&'a str], line: usize, col: usize) -> Self {
        Self { lines, line, col }
    }

    fn normalize(&mut self) {
        while self.line < self.lines.len() {
            if self.col < self.lines[self.line].len() {
                return;
            }
            self.line += 1;
            self.col = 0;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.normalize();
        if self.line >= self.lines.len() {
            return None;
        }
        self.lines[self.line][self.col..].chars().next()
    }

    fn current_char_len(&self) -> usize {
        if self.line < self.lines.len() && self.col < self.lines[self.line].len() {
            self.lines[self.line][self.col..]
                .chars()
                .next()
                .map_or(1, |c| c.len_utf8())
        } else {
            1
        }
    }

    fn peek_next(&mut self) -> Option<char> {
        self.normalize();
        if self.line >= self.lines.len() {
            return None;
        }
        let next_col = self.col + self.current_char_len();
        if next_col < self.lines[self.line].len() {
            return self.lines[self.line][next_col..].chars().next();
        }
        // Look at subsequent non-empty lines (M4 fix: skip empty lines).
        for next_line in (self.line + 1)..self.lines.len() {
            if !self.lines[next_line].is_empty() {
                return self.lines[next_line].chars().next();
            }
        }
        None
    }

    fn advance(&mut self) {
        if self.line >= self.lines.len() {
            return;
        }
        self.col += self.current_char_len();
        if self.col >= self.lines[self.line].len() {
            self.line += 1;
            self.col = 0;
        }
    }

    fn skip_string_literal(&mut self, quote: char) {
        self.advance(); // Skip opening quote.
        loop {
            match self.peek() {
                None => return, // Unterminated string — bail.
                Some('\\') => {
                    self.advance(); // Skip backslash.
                    self.advance(); // Skip escaped character.
                }
                Some(ch) if ch == quote => {
                    self.advance(); // Skip closing quote.
                    return;
                }
                _ => self.advance(),
            }
        }
    }

    fn skip_line_comment(&mut self) {
        // Skip to end of line.
        if self.line < self.lines.len() {
            self.line += 1;
            self.col = 0;
        }
    }

    fn skip_block_comment(&mut self) {
        self.advance(); // Skip '/'
        self.advance(); // Skip '*'
        loop {
            match self.peek() {
                None => return,
                Some('*') => {
                    if self.peek_next() == Some('/') {
                        self.advance(); // Skip '*'
                        self.advance(); // Skip '/'
                        return;
                    }
                    self.advance();
                }
                _ => self.advance(),
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_struct_braces() {
        let lines: Vec<&str> = vec![
            "struct Config {",
            "    name: String,",
            "    value: i32,",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert_eq!(scope.closing_line, Some(3));
        assert!(!scope.is_empty);
    }

    #[test]
    fn typescript_interface_braces() {
        let lines: Vec<&str> = vec![
            "interface User {",
            "    name: string;",
            "    age: number;",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn go_function_braces() {
        let lines: Vec<&str> = vec![
            "func main() {",
            "    fmt.Println(\"hello\")",
            "    return",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn python_list_brackets() {
        let lines: Vec<&str> = vec![
            "items = [",
            "    'a',",
            "    'b',",
            "]",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '[', ']').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn function_signature_parens() {
        let lines: Vec<&str> = vec![
            "def process(",
            "    arg1: str,",
            "    arg2: int,",
            "):",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '(', ')').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn nested_braces() {
        let lines: Vec<&str> = vec![
            "fn main() {",
            "    if true {",
            "        println!(\"hi\");",
            "    }",
            "    return;",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 4);
        assert_eq!(scope.closing_line, Some(5));
    }

    #[test]
    fn delimiters_in_double_quotes() {
        let lines: Vec<&str> = vec![
            "let x = {",
            "    msg: \"{not a brace}\",",
            "};",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 1);
        assert_eq!(scope.closing_line, Some(2));
    }

    #[test]
    fn delimiters_in_single_quotes() {
        let lines: Vec<&str> = vec![
            "let x = {",
            "    msg: '{not}',",
            "};",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(2));
    }

    #[test]
    fn delimiters_in_backticks() {
        let lines: Vec<&str> = vec![
            "const x = {",
            "    msg: `{not}`,",
            "};",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(2));
    }

    #[test]
    fn delimiters_in_line_comments() {
        let lines: Vec<&str> = vec![
            "fn f() {",
            "    // this { won't count",
            "    x();",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn delimiters_in_block_comments() {
        let lines: Vec<&str> = vec![
            "fn f() {",
            "    /* { { { */",
            "    x();",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn escaped_delimiters() {
        let lines: Vec<&str> = vec![
            "let re = {",
            "    pattern: \"\\{escaped\\}\",",
            "};",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(2));
    }

    #[test]
    fn missing_opening_delimiter() {
        let lines: Vec<&str> = vec![
            "no delimiters here",
        ];
        let err = detect_delimiter_scope(&lines, 0, '{', '}').unwrap_err();
        assert!(err.what.contains("opening delimiter"));
    }

    #[test]
    fn unbalanced_delimiter() {
        let lines: Vec<&str> = vec![
            "fn f() {",
            "    never closed",
        ];
        let err = detect_delimiter_scope(&lines, 0, '{', '}').unwrap_err();
        assert!(err.what.contains("unbalanced"));
    }

    #[test]
    fn empty_braces() {
        let lines: Vec<&str> = vec![
            "struct Empty {}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert!(scope.is_empty);
    }

    #[test]
    fn opener_on_anchor_line() {
        let lines: Vec<&str> = vec![
            "struct Foo {",
            "    x: i32,",
            "}",
        ];
        // Anchor on line 0, opener is on anchor line.
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(2));
        assert!(!scope.is_empty);
    }

    #[test]
    fn non_ascii_content() {
        // C1: byte/char index mismatch must not panic on multi-byte UTF-8.
        let lines: Vec<&str> = vec![
            "struct Café {",
            "    naïve: String,",
            "    über: i32,",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.start_line, 1);
        assert_eq!(scope.end_line, 2);
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn emoji_in_strings() {
        let lines: Vec<&str> = vec![
            "let x = {",
            "    msg: \"hello 🌍\",",
            "    flag: \"🇺🇸\",",
            "};",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(3));
    }

    #[test]
    fn blank_lines_between_content() {
        // M4: peek_next should work across blank lines.
        let lines: Vec<&str> = vec![
            "fn f() {",
            "    // comment",
            "",
            "    x();",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(4));
    }

    #[test]
    fn hash_line_comments() {
        let lines: Vec<&str> = vec![
            "config = {",
            "    # 'key': { 'nested' }",
            "    'actual': 'value',",
            "}",
        ];
        let scope = detect_delimiter_scope(&lines, 0, '{', '}').unwrap();
        assert_eq!(scope.closing_line, Some(3));
    }
}
