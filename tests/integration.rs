//! Fixture-based integration tests for jig CLI.
//!
//! Each subdirectory under `tests/fixtures/` is a test case. Adding a new test
//! case requires only adding a directory — no code changes.
//!
//! Fixture layout:
//!   recipe.yaml          — recipe to run
//!   vars.json            — variables (JSON)
//!   templates/           — template files (referenced by recipe)
//!   existing/            — pre-existing files, copied to temp dir before run
//!   expected/            — expected files after run (diffed against actual)
//!   expected_output.json — (optional) expected JSON output structure
//!   expected_exit_code   — (optional) expected exit code (default: 0)
//!   force                — (optional) if present, pass --force

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ── Helpers ──────────────────────────────────────────────────────────

fn jig_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jig"))
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Discover all fixture directories (immediate children of tests/fixtures/).
/// Supports both recipe fixtures (recipe.yaml) and workflow fixtures (workflow.yaml).
fn discover_fixtures() -> Vec<PathBuf> {
    let dir = fixtures_dir();
    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read fixtures dir {}: {}", dir.display(), e))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir()
                && (path.join("recipe.yaml").exists() || path.join("workflow.yaml").exists())
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    fixtures.sort();
    fixtures
}

/// Determine if a fixture is a workflow fixture (has workflow.yaml).
fn is_workflow_fixture(fixture: &Path) -> bool {
    fixture.join("workflow.yaml").exists()
}

/// Read expected exit code from fixture (default 0).
fn expected_exit_code(fixture: &Path) -> i32 {
    let path = fixture.join("expected_exit_code");
    if path.exists() {
        fs::read_to_string(&path)
            .unwrap()
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("bad expected_exit_code in {}: {}", fixture.display(), e))
    } else {
        0
    }
}

/// Run jig on a fixture, returning (exit_code, stdout, stderr).
/// Automatically detects recipe vs workflow fixtures.
fn run_fixture(fixture: &Path, work_dir: &Path) -> (i32, String, String) {
    let vars_json = fs::read_to_string(fixture.join("vars.json")).unwrap_or_else(|_| "{}".into());

    let mut cmd = Command::new(jig_bin());

    if is_workflow_fixture(fixture) {
        let workflow = fixture.join("workflow.yaml");
        cmd.args(["workflow", &workflow.display().to_string()]);
    } else {
        let recipe = fixture.join("recipe.yaml");
        cmd.args(["run", &recipe.display().to_string()]);
    }

    cmd.args(["--vars", &vars_json])
        .args(["--base-dir", &work_dir.display().to_string()])
        .arg("--json");

    if fixture.join("force").exists() {
        cmd.arg("--force");
    }

    if fixture.join("dry_run").exists() {
        cmd.arg("--dry-run");
    }

    let output = cmd.output().expect("failed to run jig");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

/// Copy existing/ files into work_dir.
fn setup_existing(fixture: &Path, work_dir: &Path) {
    let existing = fixture.join("existing");
    if existing.is_dir() {
        copy_dir_recursive(&existing, work_dir);
    }
}

/// Recursively copy src/ contents into dst/.
fn copy_dir_recursive(src: &Path, dst: &Path) {
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path).unwrap();
            copy_dir_recursive(&src_path, &dst_path);
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}

/// Compare work_dir contents against expected/ directory.
/// Returns a list of (relative_path, diff_description) for mismatches.
fn diff_expected(fixture: &Path, work_dir: &Path) -> Vec<(String, String)> {
    let expected = fixture.join("expected");
    if !expected.is_dir() {
        return vec![];
    }

    let mut diffs = vec![];
    diff_dir_recursive(&expected, work_dir, &expected, &mut diffs);
    diffs
}

fn diff_dir_recursive(
    expected_root: &Path,
    work_dir: &Path,
    current: &Path,
    diffs: &mut Vec<(String, String)>,
) {
    for entry in fs::read_dir(current).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let rel = path.strip_prefix(expected_root).unwrap();

        if path.is_dir() {
            diff_dir_recursive(expected_root, work_dir, &path, diffs);
        } else {
            let actual_path = work_dir.join(rel);
            if !actual_path.exists() {
                diffs.push((rel.display().to_string(), "file missing".into()));
                continue;
            }
            let expected_bytes = fs::read(&path).unwrap();
            let actual_bytes = fs::read(&actual_path).unwrap();
            if expected_bytes != actual_bytes {
                let expected_str = String::from_utf8_lossy(&expected_bytes);
                let actual_str = String::from_utf8_lossy(&actual_bytes);
                diffs.push((
                    rel.display().to_string(),
                    format!(
                        "content mismatch:\n--- expected\n+++ actual\n-{}\n+{}",
                        expected_str.replace('\n', "\n-"),
                        actual_str.replace('\n', "\n+"),
                    ),
                ));
            }
        }
    }
}

/// Normalize JSON output: replace absolute paths with relative paths
/// (strip the work_dir prefix).
fn normalize_json(json_str: &str, work_dir: &Path) -> serde_json::Value {
    let work_dir_str = work_dir.display().to_string();
    let normalized = json_str.replace(&format!("{}/", work_dir_str), "");
    serde_json::from_str(&normalized).unwrap_or_else(|e| {
        panic!(
            "failed to parse JSON output: {}\nraw output:\n{}",
            e, json_str
        )
    })
}

/// Assert JSON output matches expected_output.json.
/// Only checks fields present in expected — extra fields in actual are allowed.
fn assert_json_matches(expected: &serde_json::Value, actual: &serde_json::Value, path: &str) {
    match (expected, actual) {
        (serde_json::Value::Object(exp_map), serde_json::Value::Object(act_map)) => {
            for (key, exp_val) in exp_map {
                let act_val = act_map.get(key).unwrap_or_else(|| {
                    panic!("missing key '{}' at {}\nactual: {}", key, path, actual)
                });
                assert_json_matches(exp_val, act_val, &format!("{}.{}", path, key));
            }
        }
        (serde_json::Value::Array(exp_arr), serde_json::Value::Array(act_arr)) => {
            assert_eq!(
                exp_arr.len(),
                act_arr.len(),
                "array length mismatch at {}\nexpected: {}\nactual: {}",
                path,
                expected,
                actual
            );
            for (i, (exp, act)) in exp_arr.iter().zip(act_arr.iter()).enumerate() {
                assert_json_matches(exp, act, &format!("{}[{}]", path, i));
            }
        }
        _ => {
            assert_eq!(
                expected, actual,
                "value mismatch at {}\nexpected: {}\nactual: {}",
                path, expected, actual
            );
        }
    }
}

// ── Core test runner ─────────────────────────────────────────────────

/// Run a single fixture test case.
fn run_fixture_test(fixture: &Path) {
    let name = fixture.file_name().unwrap().to_str().unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();

    // Setup existing files.
    setup_existing(fixture, work_dir);

    // Run jig.
    let (exit_code, stdout, stderr) = run_fixture(fixture, work_dir);

    // Assert exit code.
    let expected_code = expected_exit_code(fixture);
    assert_eq!(
        exit_code, expected_code,
        "fixture '{}': expected exit code {}, got {}\nstdout: {}\nstderr: {}",
        name, expected_code, exit_code, stdout, stderr
    );

    // For success cases, diff expected files.
    if expected_code == 0 {
        let diffs = diff_expected(fixture, work_dir);
        assert!(
            diffs.is_empty(),
            "fixture '{}': file mismatches:\n{}",
            name,
            diffs
                .iter()
                .map(|(p, d)| format!("  {}: {}", p, d))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    // Assert JSON output if expected_output.json exists.
    let expected_output_path = fixture.join("expected_output.json");
    if expected_output_path.exists() && !stdout.is_empty() {
        let expected_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&expected_output_path).unwrap())
                .expect("invalid expected_output.json");
        let actual_json = normalize_json(&stdout, work_dir);
        assert_json_matches(&expected_json, &actual_json, "$");
    }
}

// ── Auto-discovered fixture tests ────────────────────────────────────

/// Run ALL fixture directories. Each is an independent test case.
/// This function discovers fixtures at runtime, so adding a directory
/// requires no code changes.
#[test]
fn fixture_tests() {
    let fixtures = discover_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found in {}",
        fixtures_dir().display()
    );

    let mut failures = vec![];

    for fixture in &fixtures {
        let name = fixture.file_name().unwrap().to_str().unwrap().to_string();
        let result = std::panic::catch_unwind(|| run_fixture_test(fixture));
        if let Err(e) = result {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".into()
            };
            failures.push((name, msg));
        }
    }

    if !failures.is_empty() {
        let report = failures
            .iter()
            .map(|(name, msg)| format!("FAIL: {}\n  {}", name, msg))
            .collect::<Vec<_>>()
            .join("\n\n");
        panic!(
            "{} of {} fixtures failed:\n\n{}",
            failures.len(),
            fixtures.len(),
            report
        );
    }
}

// ── Error fixtures: assert structured error fields ───────────────────

/// Verify that error fixtures produce JSON with what/where/why/hint fields.
/// At least one fixture per exit code (1, 2, 3, 4).
#[test]
fn error_fixtures_have_structured_fields() {
    let error_fixtures: Vec<PathBuf> = discover_fixtures()
        .into_iter()
        .filter(|f| {
            let name = f.file_name().unwrap().to_str().unwrap();
            name.starts_with("error-")
        })
        .collect();

    assert!(!error_fixtures.is_empty(), "no error fixtures found");

    let mut exit_codes_seen = std::collections::HashSet::new();

    for fixture in &error_fixtures {
        let name = fixture.file_name().unwrap().to_str().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let work_dir = tmp.path();
        setup_existing(fixture, work_dir);

        let (exit_code, stdout, stderr) = run_fixture(fixture, work_dir);
        let expected_code = expected_exit_code(fixture);
        assert_eq!(
            exit_code, expected_code,
            "fixture '{}': exit code mismatch",
            name
        );
        assert_ne!(exit_code, 0, "error fixture '{}' should not exit 0", name);
        exit_codes_seen.insert(exit_code);

        if is_workflow_fixture(fixture) {
            // Workflow error fixtures: JSON output has steps array with error details,
            // or pre-execution errors go to stderr.
            if !stdout.trim().is_empty() {
                let json: serde_json::Value = normalize_json(&stdout, work_dir);
                // Workflow JSON has steps array.
                if let Some(steps) = json["steps"].as_array() {
                    let error_step = steps.iter().find(|s| s["status"] == "error");
                    if let Some(step) = error_step
                        && let Some(err) = step.get("error")
                    {
                        assert!(
                            err["what"].is_string(),
                            "fixture '{}': error missing 'what'",
                            name
                        );
                        assert!(
                            err["where"].is_string(),
                            "fixture '{}': error missing 'where'",
                            name
                        );
                        assert!(
                            err["why"].is_string(),
                            "fixture '{}': error missing 'why'",
                            name
                        );
                        assert!(
                            err["hint"].is_string(),
                            "fixture '{}': error missing 'hint'",
                            name
                        );
                    }
                }
            }
            // Pre-execution errors (exit 1, 4) go to stderr.
            if (exit_code == 1 || exit_code == 2 || exit_code == 4) && !stderr.is_empty() {
                assert!(
                    stderr.contains("where:") || stderr.contains("why:"),
                    "fixture '{}': stderr should contain structured error fields\nstderr: {}",
                    name,
                    stderr
                );
            }
        } else {
            // Recipe error fixtures: JSON has operations array.
            if exit_code == 3 && !stdout.trim().is_empty() {
                let json: serde_json::Value = normalize_json(&stdout, work_dir);
                let ops = json["operations"].as_array().expect("operations array");
                let error_op = ops
                    .iter()
                    .find(|op| op["action"] == "error")
                    .expect("expected an error operation in output");

                assert!(
                    error_op["what"].is_string(),
                    "fixture '{}': error missing 'what'",
                    name
                );
                assert!(
                    error_op["where"].is_string(),
                    "fixture '{}': error missing 'where'",
                    name
                );
                assert!(
                    error_op["why"].is_string(),
                    "fixture '{}': error missing 'why'",
                    name
                );
                assert!(
                    error_op["hint"].is_string(),
                    "fixture '{}': error missing 'hint'",
                    name
                );
            }
            if (exit_code == 1 || exit_code == 2 || exit_code == 4) && !stderr.is_empty() {
                assert!(
                    stderr.contains("where:") || stderr.contains("why:"),
                    "fixture '{}': stderr should contain structured error fields\nstderr: {}",
                    name,
                    stderr
                );
            }
        }
    }

    // Verify we have at least one fixture per error exit code.
    for code in [1, 2, 3, 4] {
        assert!(
            exit_codes_seen.contains(&code),
            "no error fixture covers exit code {}",
            code
        );
    }
}

// ── Determinism test ─────────────────────────────────────────────────

/// AC-N1.1: Same recipe + same variables + same files = byte-identical output.
#[test]
fn determinism_identical_output_across_runs() {
    let fixture = fixtures_dir().join("create-simple");

    let mut outputs = vec![];
    for _ in 0..3 {
        let tmp = tempfile::TempDir::new().unwrap();
        let work_dir = tmp.path();

        let (exit_code, stdout, _stderr) = run_fixture(&fixture, work_dir);
        assert_eq!(exit_code, 0, "create-simple should succeed");

        // Collect JSON output (normalized) and file contents.
        let json = normalize_json(&stdout, work_dir);
        let file_content =
            fs::read(work_dir.join("src/service.rs")).expect("output file should exist");

        outputs.push((json.to_string(), file_content));
    }

    // All runs must produce identical output.
    for i in 1..outputs.len() {
        assert_eq!(
            outputs[0].0, outputs[i].0,
            "JSON output differs between run 0 and run {}",
            i
        );
        assert_eq!(
            outputs[0].1, outputs[i].1,
            "file content differs between run 0 and run {}",
            i
        );
    }
}

// ── Idempotency test ─────────────────────────────────────────────────

/// AC-N2.1: Second run with skip_if_exists + skip_if = all skips.
#[test]
fn idempotency_second_run_all_skips() {
    let fixture = fixtures_dir().join("combined-idempotency");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();
    setup_existing(&fixture, work_dir);

    // First run: creates files and injects.
    let (code1, stdout1, stderr1) = run_fixture(&fixture, work_dir);
    assert_eq!(
        code1, 0,
        "first run should succeed\nstdout: {}\nstderr: {}",
        stdout1, stderr1
    );

    // Second run: everything should be skipped.
    let (code2, stdout2, stderr2) = run_fixture(&fixture, work_dir);
    assert_eq!(
        code2, 0,
        "second run should succeed\nstdout: {}\nstderr: {}",
        stdout2, stderr2
    );

    let json = normalize_json(&stdout2, work_dir);
    let ops = json["operations"].as_array().expect("operations array");
    assert!(
        !ops.is_empty(),
        "second run should still produce operations"
    );
    for (i, op) in ops.iter().enumerate() {
        assert_eq!(
            op["action"], "skip",
            "second run op[{}] should be 'skip', got '{}'\nfull output: {}",
            i, op["action"], json
        );
    }

    // files_written should be empty, files_skipped should have entries.
    let written = json["files_written"].as_array().unwrap();
    let skipped = json["files_skipped"].as_array().unwrap();
    assert!(
        written.is_empty(),
        "second run should write no files\nfiles_written: {:?}",
        written
    );
    assert!(
        !skipped.is_empty(),
        "second run should have skipped files\nfiles_skipped: {:?}",
        skipped
    );
}

// ── Snapshot tests ───────────────────────────────────────────────────

/// Snapshot test: JSON output format for a successful create.
#[test]
fn snapshot_json_output_create() {
    let fixture = fixtures_dir().join("create-simple");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();

    let (_, stdout, _) = run_fixture(&fixture, work_dir);
    let json = normalize_json(&stdout, work_dir);
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    insta::assert_snapshot!("json_output_create", pretty);
}

/// Snapshot test: JSON output format for an inject operation.
#[test]
fn snapshot_json_output_inject() {
    let fixture = fixtures_dir().join("inject-after");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();
    setup_existing(&fixture, work_dir);

    let (_, stdout, _) = run_fixture(&fixture, work_dir);
    let json = normalize_json(&stdout, work_dir);
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    insta::assert_snapshot!("json_output_inject", pretty);
}

/// Snapshot test: JSON output format for a skip.
#[test]
fn snapshot_json_output_skip() {
    let fixture = fixtures_dir().join("inject-skip-if");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();
    setup_existing(&fixture, work_dir);

    let (_, stdout, _) = run_fixture(&fixture, work_dir);
    let json = normalize_json(&stdout, work_dir);
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    insta::assert_snapshot!("json_output_skip", pretty);
}

/// Snapshot test: JSON output for a file-operation error (file exists without force).
#[test]
fn snapshot_json_output_error() {
    let fixture = fixtures_dir().join("error-file-exists");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();
    setup_existing(&fixture, work_dir);

    let (code, stdout, _) = run_fixture(&fixture, work_dir);
    assert_eq!(code, 3);
    let json = normalize_json(&stdout, work_dir);
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    insta::assert_snapshot!("json_output_error", pretty);
}

/// Snapshot test: stderr error message for missing required variable.
#[test]
fn snapshot_error_missing_var() {
    let fixture = fixtures_dir().join("error-missing-vars");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();

    let (code, _, stderr) = run_fixture(&fixture, work_dir);
    assert_eq!(code, 4);
    insta::assert_snapshot!("error_missing_var", stderr);
}

/// Snapshot test: stderr error message for malformed YAML.
#[test]
fn snapshot_error_malformed_yaml() {
    let fixture = fixtures_dir().join("error-malformed-yaml");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();

    let (code, _, stderr) = run_fixture(&fixture, work_dir);
    assert_eq!(code, 1);
    // Normalize the path in stderr to be fixture-relative for stable snapshots.
    let normalized = stderr.replace(
        &fixture.join("recipe.yaml").display().to_string(),
        "<recipe-path>",
    );
    insta::assert_snapshot!("error_malformed_yaml", normalized);
}

/// Snapshot test: combined create+inject JSON output.
#[test]
fn snapshot_json_output_combined() {
    let fixture = fixtures_dir().join("combined-create-inject");
    let tmp = tempfile::TempDir::new().unwrap();
    let work_dir = tmp.path();

    let (_, stdout, _) = run_fixture(&fixture, work_dir);
    let json = normalize_json(&stdout, work_dir);
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    insta::assert_snapshot!("json_output_combined", pretty);
}

// ── Static binary check ──────────────────────────────────────────────

/// AC-N3.1: Binary has no dynamic dependencies beyond system libc.
#[test]
fn binary_no_extra_dynamic_deps() {
    let binary = jig_bin();

    if cfg!(target_os = "macos") {
        let output = Command::new("otool")
            .args(["-L", &binary.display().to_string()])
            .output()
            .expect("otool failed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        // On macOS, only system libraries should appear.
        for line in stdout.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            assert!(
                line.contains("/usr/lib/") || line.contains("/System/"),
                "unexpected dynamic dependency: {}",
                line
            );
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("ldd")
            .arg(binary.display().to_string())
            .output()
            .expect("ldd failed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.contains("statically linked") {
                continue;
            }
            // Allow libc, libpthread, libdl, libm, libgcc, ld-linux
            let allowed = [
                "libc.",
                "libpthread.",
                "libdl.",
                "libm.",
                "libgcc_s.",
                "ld-linux",
                "linux-vdso",
                "librt.",
            ];
            assert!(
                allowed.iter().any(|a| line.contains(a)),
                "unexpected dynamic dependency: {}",
                line
            );
        }
    }
}
