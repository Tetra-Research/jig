//! Integration tests for `jig library` subcommands.
//!
//! These tests exercise the CLI end-to-end: create temp directories with
//! library fixtures, run `jig library <action>`, and verify output.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn jig_bin() -> String {
    env!("CARGO_BIN_EXE_jig").to_string()
}

/// Create a minimal library source directory for testing.
fn create_library_source(dir: &Path, name: &str, version: &str) {
    fs::create_dir_all(dir).unwrap();
    let manifest = format!(
        r#"name: {name}
version: {version}
description: Test library for {name}
framework: test
language: rust

conventions:
  models: "src/models/{{{{ model | snakecase }}}}.rs"

recipes:
  model/add-field: "Add a field to a model"
  model/add-model: "Create a new model"

workflows:
  add-field:
    description: "Add a field across the stack"
    steps:
      - recipe: model/add-field
      - recipe: model/add-model
        when: "{{{{ create_model }}}}"
"#
    );
    fs::write(dir.join("jig-library.yaml"), manifest).unwrap();

    // Create recipe directories with recipe.yaml files.
    let add_field_dir = dir.join("model/add-field");
    fs::create_dir_all(add_field_dir.join("templates")).unwrap();
    fs::write(
        add_field_dir.join("recipe.yaml"),
        r#"name: add-field
description: Add a field to a model
variables:
  field_name:
    type: string
    required: true
    description: "Name of the field"
  field_type:
    type: string
    required: true
files: []
"#,
    )
    .unwrap();
    fs::write(
        add_field_dir.join("templates/field.rs.j2"),
        "pub {{ field_name }}: {{ field_type }},\n",
    )
    .unwrap();

    let add_model_dir = dir.join("model/add-model");
    fs::create_dir_all(add_model_dir.join("templates")).unwrap();
    fs::write(
        add_model_dir.join("recipe.yaml"),
        "name: add-model\nfiles: []\n",
    )
    .unwrap();
}

// ── jig library add ─────────────────────────────────────────────

#[test]
fn library_add_from_local_path() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(
        output.status.success(),
        "jig library add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["action"], "add");
    assert_eq!(json["library"], "mylib");
    assert_eq!(json["version"], "0.1.0");
    assert_eq!(json["location"], "project");

    // Verify library is installed.
    assert!(project.join(".jig/libraries/mylib/jig-library.yaml").exists());
}

#[test]
fn library_add_already_installed_errors() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // First install succeeds.
    let out1 = Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .expect("failed to run jig");
    assert!(out1.status.success());

    // Second install fails.
    let out2 = Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .expect("failed to run jig");
    assert!(!out2.status.success());
    let stderr = String::from_utf8_lossy(&out2.stderr);
    assert!(stderr.contains("already installed"));
}

#[test]
fn library_add_missing_manifest_errors() {
    let tmp = TempDir::new().unwrap();
    let empty_dir = tmp.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "add", &empty_dir.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .expect("failed to run jig");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no jig-library.yaml"));
}

// ── jig library list ────────────────────────────────────────────

#[test]
fn library_list_empty() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "list"])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["libraries"].as_array().unwrap().len(), 0);
}

#[test]
fn library_list_shows_installed() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.2.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Install.
    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // List.
    let output = Command::new(jig_bin())
        .args(["library", "list"])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let libs = json["libraries"].as_array().unwrap();
    assert_eq!(libs.len(), 1);
    assert_eq!(libs[0]["name"], "mylib");
    assert_eq!(libs[0]["version"], "0.2.0");
    assert_eq!(libs[0]["location"], "project");
}

// ── jig library remove ──────────────────────────────────────────

#[test]
fn library_remove() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Install.
    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Remove.
    let output = Command::new(jig_bin())
        .args(["library", "remove", "mylib"])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(
        output.status.success(),
        "jig library remove failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["action"], "remove");
    assert_eq!(json["library"], "mylib");

    // Verify removed.
    assert!(!project.join(".jig/libraries/mylib").exists());
}

#[test]
fn library_remove_not_installed_errors() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "remove", "nonexistent"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .expect("failed to run jig");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not installed"));
}

// ── jig library update ──────────────────────────────────────────

#[test]
fn library_update() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Install v1.
    let source_v1 = tmp.path().join("v1");
    create_library_source(&source_v1, "mylib", "0.1.0");
    Command::new(jig_bin())
        .args(["library", "add", &source_v1.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Create v2 source.
    let source_v2 = tmp.path().join("v2");
    create_library_source(&source_v2, "mylib", "0.2.0");

    // Update.
    let output = Command::new(jig_bin())
        .args(["library", "update", "mylib", &source_v2.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(
        output.status.success(),
        "jig library update failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["action"], "update");
    assert_eq!(json["version"], "0.2.0");
}

// ── jig library recipes ─────────────────────────────────────────

#[test]
fn library_recipes() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "recipes", "mylib"])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["library"], "mylib");
    let recipes = json["recipes"].as_array().unwrap();
    assert_eq!(recipes.len(), 2);
    assert_eq!(recipes[0]["path"], "model/add-field");
    assert_eq!(recipes[0]["description"], "Add a field to a model");
}

// ── jig library info ────────────────────────────────────────────

#[test]
fn library_info() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "info", "mylib/model/add-field"])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(
        output.status.success(),
        "jig library info failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["library"], "mylib");
    assert_eq!(json["recipe"], "model/add-field");
    assert_eq!(json["description"], "Add a field to a model");

    let vars = json["variables"].as_array().unwrap();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0]["name"], "field_name");
    assert_eq!(vars[0]["type"], "string");
    assert!(vars[0]["required"].as_bool().unwrap());
    assert_eq!(
        vars[0]["description"].as_str().unwrap(),
        "Name of the field"
    );
}

#[test]
fn library_info_recipe_not_found() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "info", "mylib/nonexistent"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .expect("failed to run jig");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

// ── jig library workflows ───────────────────────────────────────

#[test]
fn library_workflows() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let output = Command::new(jig_bin())
        .args(["library", "workflows", "mylib"])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .expect("failed to run jig");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["library"], "mylib");
    let workflows = json["workflows"].as_array().unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0]["name"], "add-field");
    assert_eq!(
        workflows[0]["description"],
        "Add a field across the stack"
    );

    let steps = workflows[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0]["recipe"], "model/add-field");
    assert!(!steps[0]["conditional"].as_bool().unwrap());
    assert_eq!(steps[1]["recipe"], "model/add-model");
    assert!(steps[1]["conditional"].as_bool().unwrap());
}

// ── Full lifecycle ──────────────────────────────────────────────

#[test]
fn library_full_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // 1. List — empty.
    let out = Command::new(jig_bin())
        .args(["library", "list", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["libraries"].as_array().unwrap().len(), 0);

    // 2. Add.
    let source = tmp.path().join("lib-source");
    create_library_source(&source, "testlib", "1.0.0");
    let out = Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    // 3. List — should show 1 library.
    let out = Command::new(jig_bin())
        .args(["library", "list", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["libraries"].as_array().unwrap().len(), 1);
    assert_eq!(json["libraries"][0]["name"], "testlib");

    // 4. Recipes.
    let out = Command::new(jig_bin())
        .args(["library", "recipes", "testlib", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["recipes"].as_array().unwrap().len(), 2);

    // 5. Info.
    let out = Command::new(jig_bin())
        .args(["library", "info", "testlib/model/add-field", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["recipe"], "model/add-field");

    // 6. Update.
    let source_v2 = tmp.path().join("lib-source-v2");
    create_library_source(&source_v2, "testlib", "2.0.0");
    let out = Command::new(jig_bin())
        .args([
            "library",
            "update",
            "testlib",
            &source_v2.display().to_string(),
            "--json",
        ])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["version"], "2.0.0");

    // 7. Remove.
    let out = Command::new(jig_bin())
        .args(["library", "remove", "testlib", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());

    // 8. List — empty again.
    let out = Command::new(jig_bin())
        .args(["library", "list", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["libraries"].as_array().unwrap().len(), 0);
}
