use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use tempfile::TempDir;

fn jig_bin() -> String {
    env!("CARGO_BIN_EXE_jig").to_string()
}

/// AC-2.3: WHEN --vars-stdin is provided, the system SHALL read JSON from stdin as variable input.
#[test]
fn ac_2_3_vars_stdin() {
    let dir = TempDir::new().unwrap();
    let tmpl_path = dir.path().join("test.j2");
    fs::write(&tmpl_path, "Hello {{ name }}!").unwrap();
    let out_path = dir.path().join("out.txt");

    let mut child = Command::new(jig_bin())
        .args([
            "render",
            &tmpl_path.display().to_string(),
            "--vars-stdin",
            "--to",
            &out_path.display().to_string(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn jig");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"name": "FromStdin"}"#)
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "jig render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&out_path).unwrap();
    assert_eq!(content, "Hello FromStdin!");
}

/// AC-2.3 + AC-2.4: stdin has lower precedence than inline --vars.
#[test]
fn ac_2_3_stdin_precedence_under_inline() {
    let dir = TempDir::new().unwrap();
    let tmpl_path = dir.path().join("test.j2");
    fs::write(&tmpl_path, "{{ x }}").unwrap();
    let out_path = dir.path().join("out.txt");

    let mut child = Command::new(jig_bin())
        .args([
            "render",
            &tmpl_path.display().to_string(),
            "--vars-stdin",
            "--vars",
            r#"{"x": "from_inline"}"#,
            "--to",
            &out_path.display().to_string(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn jig");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"x": "from_stdin"}"#)
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "jig render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&out_path).unwrap();
    assert_eq!(content, "from_inline");
}
