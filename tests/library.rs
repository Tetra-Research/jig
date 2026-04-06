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
    assert!(
        project
            .join(".jig/libraries/mylib/jig-library.yaml")
            .exists()
    );
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
        .args([
            "library",
            "update",
            "mylib",
            &source_v2.display().to_string(),
        ])
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
    assert_eq!(workflows[0]["description"], "Add a field across the stack");

    let steps = workflows[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0]["recipe"], "model/add-field");
    assert!(!steps[0]["conditional"].as_bool().unwrap());
    assert_eq!(steps[1]["recipe"], "model/add-model");
    assert!(steps[1]["conditional"].as_bool().unwrap());
}

// ── Phase 1: Execution integration ──────────────────────────

/// Create a library with actual file operations (create) for execution tests.
fn create_executable_library(dir: &Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    let manifest = format!(
        r#"name: {name}
version: 1.0.0
description: Executable test library

conventions:
  models: "src/models/{{{{ model | snakecase }}}}.rs"

recipes:
  model/create: "Create a model file"
  model/no-op: "A no-op recipe"

workflows:
  create-model:
    description: "Create a model file"
    steps:
      - recipe: model/create
"#
    );
    fs::write(dir.join("jig-library.yaml"), manifest).unwrap();

    // model/create — creates a file.
    let create_dir = dir.join("model/create");
    fs::create_dir_all(create_dir.join("templates")).unwrap();
    fs::write(
        create_dir.join("recipe.yaml"),
        r#"name: create
variables:
  model:
    type: string
    required: true
files:
  - template: templates/model.rs.j2
    to: "src/models/{{ model | snakecase }}.rs"
"#,
    )
    .unwrap();
    fs::write(
        create_dir.join("templates/model.rs.j2"),
        "pub struct {{ model | pascalcase }} {}\n",
    )
    .unwrap();

    // model/no-op — no file operations.
    let noop_dir = dir.join("model/no-op");
    fs::create_dir_all(noop_dir.join("templates")).unwrap();
    fs::write(noop_dir.join("recipe.yaml"), "name: no-op\nfiles: []\n").unwrap();
}

#[test]
fn library_run_recipe_end_to_end() {
    // AC-4.1: jig run <library>/<recipe-path> resolves and executes.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Install library.
    let out = Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Run a library recipe.
    let out = Command::new(jig_bin())
        .args(["run", "testlib/model/create"])
        .args(["--vars", r#"{"model": "BookingService"}"#])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "jig run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify file was created.
    let created = project.join("src/models/booking_service.rs");
    assert!(
        created.exists(),
        "expected file not created: {}",
        created.display()
    );
    let content = fs::read_to_string(&created).unwrap();
    assert!(content.contains("pub struct BookingService {}"));
}

#[test]
fn library_validate_recipe() {
    // AC-4.3: jig validate <library>/<recipe-path> resolves and validates.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let out = Command::new(jig_bin())
        .args(["validate", "testlib/model/create"])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "jig validate failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["valid"], true);
}

#[test]
fn library_vars_recipe() {
    // AC-4.4: jig vars <library>/<recipe-path> resolves and shows variables.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let out = Command::new(jig_bin())
        .args(["vars", "testlib/model/create"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "jig vars failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        json["model"].is_object(),
        "expected 'model' variable in output"
    );
}

#[test]
fn library_workflow_execution() {
    // AC-4.2: jig workflow <library>/<workflow-name> resolves and executes.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let out = Command::new(jig_bin())
        .args(["workflow", "testlib/create-model"])
        .args(["--vars", r#"{"model": "Payment"}"#])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "jig workflow failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify the workflow created the file.
    let created = project.join("src/models/payment.rs");
    assert!(created.exists(), "expected file not created by workflow");
    let content = fs::read_to_string(&created).unwrap();
    assert!(content.contains("pub struct Payment {}"));
}

#[test]
fn library_filesystem_path_takes_precedence() {
    // AC-4.5, AC-N2.1: filesystem paths take precedence over library resolution.
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Create a filesystem recipe at a path that looks like a library path.
    let recipe_dir = project.join("testlib/model/create");
    fs::create_dir_all(&recipe_dir).unwrap();
    fs::write(
        recipe_dir.join("recipe.yaml"),
        "name: local-recipe\nfiles: []\n",
    )
    .unwrap();

    // Validate should succeed using the filesystem path even without a library installed.
    let out = Command::new(jig_bin())
        .args(["validate", "testlib/model/create/recipe.yaml"])
        .args(["--base-dir", &project.display().to_string()])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(out.status.success());
}

// ── Phase 2: Convention injection ───────────────────────────

#[test]
fn library_conventions_in_templates() {
    // AC-5.3, AC-5.5: {{ conventions.models }} renders to the correct path.
    let tmp = TempDir::new().unwrap();

    // Create a library with a recipe that uses conventions.
    let source = tmp.path().join("lib-source");
    fs::create_dir_all(&source).unwrap();
    let manifest = r#"name: convlib
version: 1.0.0
conventions:
  models: "src/models/{{ model | snakecase }}.rs"
recipes:
  model/create: "Create a model"
"#;
    fs::write(source.join("jig-library.yaml"), manifest).unwrap();

    let recipe_dir = source.join("model/create");
    fs::create_dir_all(recipe_dir.join("templates")).unwrap();
    fs::write(
        recipe_dir.join("recipe.yaml"),
        r#"name: create
variables:
  model:
    type: string
    required: true
files:
  - template: templates/model.rs.j2
    to: "{{ conventions.models }}"
"#,
    )
    .unwrap();
    fs::write(
        recipe_dir.join("templates/model.rs.j2"),
        "pub struct {{ model | pascalcase }} {}\n",
    )
    .unwrap();

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Install library.
    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Run with conventions.
    let out = Command::new(jig_bin())
        .args(["run", "convlib/model/create"])
        .args(["--vars", r#"{"model": "User"}"#])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "jig run with conventions failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify convention-resolved path was used.
    let created = project.join("src/models/user.rs");
    assert!(created.exists(), "conventions path not resolved correctly");
}

#[test]
fn library_conventions_jigrc_override() {
    // AC-5.2, AC-6.2: .jigrc.yaml override for convention paths.
    let tmp = TempDir::new().unwrap();

    let source = tmp.path().join("lib-source");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("jig-library.yaml"),
        r#"name: overlib
version: 1.0.0
conventions:
  models: "src/models/{{ model | snakecase }}.rs"
recipes:
  model/create: "Create a model"
"#,
    )
    .unwrap();

    let recipe_dir = source.join("model/create");
    fs::create_dir_all(recipe_dir.join("templates")).unwrap();
    fs::write(
        recipe_dir.join("recipe.yaml"),
        r#"name: create
variables:
  model:
    type: string
    required: true
files:
  - template: templates/model.rs.j2
    to: "{{ conventions.models }}"
"#,
    )
    .unwrap();
    fs::write(
        recipe_dir.join("templates/model.rs.j2"),
        "pub struct {{ model | pascalcase }} {}\n",
    )
    .unwrap();

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Write .jigrc.yaml with convention override.
    fs::write(
        project.join(".jigrc.yaml"),
        r#"libraries:
  overlib:
    conventions:
      models: "custom/{{ model | snakecase }}_model.rs"
"#,
    )
    .unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    let out = Command::new(jig_bin())
        .args(["run", "overlib/model/create"])
        .args(["--vars", r#"{"model": "User"}"#])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "jig run with .jigrc.yaml failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify the overridden convention path was used.
    let created = project.join("custom/user_model.rs");
    assert!(
        created.exists(),
        "convention override not applied: expected custom/user_model.rs"
    );
}

// ── Phase 3: Template overrides ─────────────────────────────

#[test]
fn library_template_override() {
    // AC-7.1, AC-7.2: Template overrides from .jig/overrides/.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Create a template override. The override dir structure is:
    // .jig/overrides/<library>/<recipe-path>/templates/<template-name>
    // The recipe's template path is "templates/model.rs.j2", so the override is at:
    let override_dir = project.join(".jig/overrides/testlib/model/create");
    fs::create_dir_all(override_dir.join("templates")).unwrap();
    fs::write(
        override_dir.join("templates/model.rs.j2"),
        "// OVERRIDDEN\npub struct {{ model | pascalcase }} { id: u64 }\n",
    )
    .unwrap();

    let out = Command::new(jig_bin())
        .args(["run", "testlib/model/create"])
        .args(["--vars", r#"{"model": "Order"}"#])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "jig run with template override failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let created = project.join("src/models/order.rs");
    assert!(created.exists());
    let content = fs::read_to_string(&created).unwrap();
    assert!(
        content.contains("OVERRIDDEN"),
        "template override not applied, got: {content}"
    );
}

// ── Phase 4: Extensions ─────────────────────────────────────

#[test]
fn library_extension_recipe_execution() {
    // AC-8.2: Extension recipes can be executed via jig run.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Create an extension recipe.
    let ext_dir = project.join(".jig/extensions/testlib/custom/hello");
    fs::create_dir_all(ext_dir.join("templates")).unwrap();
    fs::write(
        ext_dir.join("recipe.yaml"),
        r#"name: hello
description: Extension recipe
variables:
  greeting:
    type: string
    required: true
files:
  - template: templates/hello.txt.j2
    to: "hello.txt"
"#,
    )
    .unwrap();
    fs::write(
        ext_dir.join("templates/hello.txt.j2"),
        "{{ greeting }} from extension!\n",
    )
    .unwrap();

    let out = Command::new(jig_bin())
        .args(["run", "testlib/custom/hello"])
        .args(["--vars", r#"{"greeting": "Hi"}"#])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "extension recipe failed: stderr={}, stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );

    let created = project.join("hello.txt");
    assert!(created.exists(), "extension recipe did not create file");
    assert!(
        fs::read_to_string(&created)
            .unwrap()
            .contains("Hi from extension!")
    );
}

#[test]
fn library_extension_listed_with_marker() {
    // AC-8.4: Extension recipes listed with [ext] marker or "source": "extension".
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Create an extension recipe.
    let ext_dir = project.join(".jig/extensions/testlib/custom/hello");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("recipe.yaml"), "name: hello\nfiles: []\n").unwrap();

    let out = Command::new(jig_bin())
        .args(["library", "recipes", "testlib", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let recipes = json["recipes"].as_array().unwrap();

    // Should include both library and extension recipes.
    let ext_recipe = recipes.iter().find(|r| r["path"] == "custom/hello");
    assert!(ext_recipe.is_some(), "extension recipe not listed");
    assert_eq!(ext_recipe.unwrap()["source"], "extension");

    // Library recipes should have source "library".
    let lib_recipe = recipes.iter().find(|r| r["path"] == "model/create");
    assert!(lib_recipe.is_some());
    assert_eq!(lib_recipe.unwrap()["source"], "library");
}

#[test]
fn library_extension_no_shadow() {
    // AC-8.3: Library recipes take precedence — extensions cannot shadow.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("testlib-source");
    create_executable_library(&source, "testlib");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Create extension at same path as library recipe.
    let ext_dir = project.join(".jig/extensions/testlib/model/create");
    fs::create_dir_all(ext_dir.join("templates")).unwrap();
    fs::write(
        ext_dir.join("recipe.yaml"),
        r#"name: create
variables:
  model:
    type: string
    required: true
files:
  - template: templates/model.rs.j2
    to: "src/models/{{ model | snakecase }}.rs"
"#,
    )
    .unwrap();
    fs::write(
        ext_dir.join("templates/model.rs.j2"),
        "// EXTENSION VERSION\n",
    )
    .unwrap();

    // Run — should use library version, not extension.
    let out = Command::new(jig_bin())
        .args(["run", "testlib/model/create"])
        .args(["--vars", r#"{"model": "Item"}"#])
        .args(["--base-dir", &project.display().to_string()])
        .args(["--json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let created = project.join("src/models/item.rs");
    let content = fs::read_to_string(&created).unwrap();
    assert!(
        !content.contains("EXTENSION"),
        "extension shadowed library recipe!"
    );
    assert!(
        content.contains("pub struct Item"),
        "library recipe should have been used"
    );
}

// ── Phase 5: Metadata ───────────────────────────────────────

#[test]
fn library_install_creates_metadata() {
    // AC-2.13: Install records source in metadata.
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

    let meta_path = project.join(".jig/libraries/mylib/_install_meta.json");
    assert!(meta_path.exists(), "_install_meta.json not created");

    let meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
    assert_eq!(meta["source_type"], "local");
    assert!(meta["source"].as_str().unwrap().contains("mylib-source"));
}

#[test]
fn library_force_install_overwrites() {
    // AC-2.6: --force overwrites existing library.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("mylib-source");
    create_library_source(&source, "mylib", "0.1.0");

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // First install.
    Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Second install with --force should succeed.
    let out = Command::new(jig_bin())
        .args([
            "library",
            "add",
            &source.display().to_string(),
            "--force",
            "--json",
        ])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "--force install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Phase 6: Bug fixes ──────────────────────────────────────

#[test]
fn library_update_name_mismatch_errors() {
    // C1 fix: update_from_path validates name match.
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Install "mylib".
    let source_v1 = tmp.path().join("v1");
    create_library_source(&source_v1, "mylib", "0.1.0");
    Command::new(jig_bin())
        .args(["library", "add", &source_v1.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();

    // Try to update "mylib" with source that has name "otherlib".
    let source_other = tmp.path().join("other");
    create_library_source(&source_other, "otherlib", "0.2.0");

    let out = Command::new(jig_bin())
        .args([
            "library",
            "update",
            "mylib",
            &source_other.display().to_string(),
        ])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "name mismatch should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("mismatch"), "error: {stderr}");
}

#[test]
fn library_semver_validation() {
    // AC-1.13: Invalid semver rejects.
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("bad-ver");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("jig-library.yaml"),
        "name: badver\nversion: not-semver\nrecipes: {}\n",
    )
    .unwrap();

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let out = Command::new(jig_bin())
        .args(["library", "add", &source.display().to_string()])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "bad semver should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("semver") || stderr.contains("MAJOR.MINOR.PATCH"),
        "error should mention semver: {stderr}"
    );
}

#[test]
fn library_list_deterministic_order() {
    // M5 fix: list output is sorted by name.
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    // Install libraries in reverse alphabetical order.
    for name in &["zlib", "alib", "mlib"] {
        let source = tmp.path().join(format!("source-{name}"));
        create_library_source(&source, name, "0.1.0");
        Command::new(jig_bin())
            .args(["library", "add", &source.display().to_string()])
            .args(["--base-dir", &project.display().to_string()])
            .output()
            .unwrap();
    }

    let out = Command::new(jig_bin())
        .args(["library", "list", "--json"])
        .args(["--base-dir", &project.display().to_string()])
        .output()
        .unwrap();
    assert!(out.status.success());

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let libs = json["libraries"].as_array().unwrap();
    assert_eq!(libs.len(), 3);
    assert_eq!(libs[0]["name"], "alib");
    assert_eq!(libs[1]["name"], "mlib");
    assert_eq!(libs[2]["name"], "zlib");
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
