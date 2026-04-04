use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::{JigError, StructuredError};

// ── Public types ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Recipe {
    pub name: Option<String>,
    pub description: Option<String>,
    pub variables: IndexMap<String, VariableDecl>,
    pub files: Vec<FileOp>,
    /// Directory containing the recipe file (used for template resolution).
    pub recipe_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VarType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Enum,
}

impl std::fmt::Display for VarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VarType::String => write!(f, "string"),
            VarType::Number => write!(f, "number"),
            VarType::Boolean => write!(f, "boolean"),
            VarType::Array => write!(f, "array"),
            VarType::Object => write!(f, "object"),
            VarType::Enum => write!(f, "enum"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct VariableDecl {
    #[serde(rename = "type")]
    pub var_type: VarType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub description: Option<String>,
    /// For enum type: allowed values.
    #[serde(default)]
    pub values: Option<Vec<String>>,
    /// For array type: element type.
    #[serde(default)]
    pub items: Option<VarType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPosition {
    First,
    Last,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields read in later phases
pub enum InjectMode {
    After { pattern: String, at: MatchPosition },
    Before { pattern: String, at: MatchPosition },
    Prepend,
    Append,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields read in later phases
pub enum FileOp {
    Create {
        template: String,
        to: String,
        skip_if_exists: bool,
    },
    Inject {
        template: String,
        inject: String,
        mode: InjectMode,
        skip_if: Option<String>,
    },
}

impl FileOp {
    pub fn op_type_str(&self) -> &'static str {
        match self {
            FileOp::Create { .. } => "create",
            FileOp::Inject { .. } => "inject",
        }
    }

    pub fn template(&self) -> &str {
        match self {
            FileOp::Create { template, .. } | FileOp::Inject { template, .. } => template,
        }
    }
}

// ── Raw YAML deserialization (intermediate) ─────────────────────────

#[derive(Deserialize)]
struct RawRecipe {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    variables: IndexMap<String, VariableDecl>,
    #[serde(default)]
    files: Vec<RawFileOp>,
}

#[derive(Deserialize)]
struct RawFileOp {
    template: Option<String>,
    to: Option<String>,
    inject: Option<String>,
    replace: Option<serde_yaml::Value>,
    patch: Option<serde_yaml::Value>,
    #[serde(default)]
    skip_if_exists: bool,
    after: Option<String>,
    before: Option<String>,
    #[serde(default)]
    prepend: bool,
    #[serde(default)]
    append: bool,
    #[serde(default)]
    at: Option<String>,
    skip_if: Option<String>,
}

// ── Parsing + validation ────────────────────────────────────────────

impl Recipe {
    /// Load and validate a recipe from a YAML file.
    pub fn load(path: &Path) -> Result<Self, JigError> {
        let path = path
            .canonicalize()
            .map_err(|_| recipe_err(
                "recipe file not found",
                &path.display().to_string(),
                "the file does not exist at the specified path",
                "check the file path and try again",
            ))?;

        let content = std::fs::read_to_string(&path)
            .map_err(|e| recipe_err(
                "cannot read recipe file",
                &path.display().to_string(),
                &e.to_string(),
                "check file permissions",
            ))?;

        let recipe_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self::parse(&content, recipe_dir, &path)
    }

    /// Parse recipe YAML content with a known recipe directory and source path.
    pub fn parse(yaml: &str, recipe_dir: PathBuf, source_path: &Path) -> Result<Self, JigError> {
        let raw: RawRecipe = serde_yaml::from_str(yaml)
            .map_err(|e| recipe_err(
                "malformed recipe YAML",
                &source_path.display().to_string(),
                &e.to_string(),
                "check YAML syntax — ensure proper indentation and field names",
            ))?;

        let mut files = Vec::with_capacity(raw.files.len());
        for (i, raw_op) in raw.files.into_iter().enumerate() {
            let op = convert_file_op(raw_op, i, source_path)?;
            files.push(op);
        }

        let recipe = Recipe {
            name: raw.name,
            description: raw.description,
            variables: raw.variables,
            files,
            recipe_dir: recipe_dir.clone(),
        };

        // Validate template files exist.
        recipe.validate_templates()?;

        Ok(recipe)
    }

    fn validate_templates(&self) -> Result<(), JigError> {
        for (i, op) in self.files.iter().enumerate() {
            let tmpl = op.template();
            let resolved = self.recipe_dir.join(tmpl);
            if !resolved.is_file() {
                return Err(recipe_err(
                    &format!("template file not found: '{tmpl}'"),
                    &format!("files[{i}].template in recipe"),
                    &format!("looked for '{}' but it does not exist", resolved.display()),
                    "ensure the template file exists relative to the recipe file location",
                ));
            }
        }
        Ok(())
    }

    /// Resolve a template path relative to the recipe directory.
    #[allow(dead_code)] // Used in later phases
    pub fn resolve_template(&self, template: &str) -> PathBuf {
        self.recipe_dir.join(template)
    }
}

/// Convert a raw file op into a typed FileOp, validating structure.
fn convert_file_op(raw: RawFileOp, index: usize, source: &Path) -> Result<FileOp, JigError> {
    let loc = format!("files[{index}] in {}", source.display());

    // Template is required for all operations.
    let template = raw.template.ok_or_else(|| recipe_err(
        "missing required field 'template'",
        &loc,
        "every file operation must specify a template",
        "add a 'template' field pointing to the Jinja2 template file",
    ))?;

    // Count how many operation-type fields are present.
    let has_to = raw.to.is_some();
    let has_inject = raw.inject.is_some();
    let has_replace = raw.replace.is_some();
    let has_patch = raw.patch.is_some();
    let type_count = [has_to, has_inject, has_replace, has_patch]
        .iter()
        .filter(|&&b| b)
        .count();

    if type_count == 0 {
        return Err(recipe_err(
            "missing operation type",
            &loc,
            "file operation must specify one of: 'to' (create), 'inject', 'replace', or 'patch'",
            "add a 'to' field for create or an 'inject' field for injection",
        ));
    }
    if type_count > 1 {
        let mut present = Vec::new();
        if has_to { present.push("to"); }
        if has_inject { present.push("inject"); }
        if has_replace { present.push("replace"); }
        if has_patch { present.push("patch"); }
        return Err(recipe_err(
            &format!("ambiguous operation type: found multiple of {}", present.join(", ")),
            &loc,
            "a file operation must specify exactly one operation type",
            "remove extra fields so only one of 'to', 'inject', 'replace', or 'patch' remains",
        ));
    }

    // Reject unsupported operation types (v0.1).
    if has_replace {
        return Err(recipe_err(
            "unknown operation type 'replace' \u{2014} this operation is not supported in v0.1",
            &loc,
            "replace operations are planned for v0.2",
            "use 'to' (create) or 'inject' for v0.1 recipes",
        ));
    }
    if has_patch {
        return Err(recipe_err(
            "unknown operation type 'patch' \u{2014} this operation is not supported in v0.1",
            &loc,
            "patch operations are planned for v0.2",
            "use 'to' (create) or 'inject' for v0.1 recipes",
        ));
    }

    if has_to {
        Ok(FileOp::Create {
            template,
            to: raw.to.unwrap(),
            skip_if_exists: raw.skip_if_exists,
        })
    } else {
        // has_inject
        let inject_target = raw.inject.unwrap();
        let mode = parse_inject_mode(&raw.after, &raw.before, raw.prepend, raw.append, &raw.at, index, source)?;
        Ok(FileOp::Inject {
            template,
            inject: inject_target,
            mode,
            skip_if: raw.skip_if,
        })
    }
}

/// Parse and validate injection mode from raw fields.
fn parse_inject_mode(
    after: &Option<String>,
    before: &Option<String>,
    prepend: bool,
    append: bool,
    at: &Option<String>,
    index: usize,
    source: &Path,
) -> Result<InjectMode, JigError> {
    let loc = format!("files[{index}] in {}", source.display());

    let mode_count = [after.is_some(), before.is_some(), prepend, append]
        .iter()
        .filter(|&&b| b)
        .count();

    if mode_count == 0 {
        return Err(recipe_err(
            "inject operation missing mode",
            &loc,
            "inject must specify one of: 'after', 'before', 'prepend: true', or 'append: true'",
            "add an injection mode field",
        ));
    }
    if mode_count > 1 {
        return Err(recipe_err(
            "inject operation has conflicting modes",
            &loc,
            "only one of 'after', 'before', 'prepend', 'append' may be specified",
            "remove extra injection mode fields so only one remains",
        ));
    }

    let pos = parse_match_position(at, index, source)?;

    if let Some(pattern) = after {
        validate_regex(pattern, "after", index, source)?;
        Ok(InjectMode::After { pattern: pattern.clone(), at: pos })
    } else if let Some(pattern) = before {
        validate_regex(pattern, "before", index, source)?;
        Ok(InjectMode::Before { pattern: pattern.clone(), at: pos })
    } else if prepend {
        Ok(InjectMode::Prepend)
    } else {
        Ok(InjectMode::Append)
    }
}

fn parse_match_position(
    at: &Option<String>,
    _index: usize,
    _source: &Path,
) -> Result<MatchPosition, JigError> {
    match at.as_deref() {
        None | Some("first") => Ok(MatchPosition::First),
        Some("last") => Ok(MatchPosition::Last),
        Some(other) => Err(recipe_err(
            &format!("invalid 'at' value: '{other}'"),
            &format!("files[{_index}] in {}", _source.display()),
            "at must be 'first' or 'last'",
            "use at: first or at: last",
        )),
    }
}

fn validate_regex(pattern: &str, field: &str, index: usize, source: &Path) -> Result<(), JigError> {
    regex::Regex::new(pattern).map_err(|e| recipe_err(
        &format!("invalid regex in '{field}' field"),
        &format!("files[{index}] in {}", source.display()),
        &format!("pattern '{pattern}' failed to compile: {e}"),
        "check regex syntax — remember to escape special characters",
    ))?;
    Ok(())
}

fn recipe_err(what: &str, where_: &str, why: &str, hint: &str) -> JigError {
    JigError::RecipeValidation(StructuredError {
        what: what.to_string(),
        where_: where_.to_string(),
        why: why.to_string(),
        hint: hint.to_string(),
    })
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a temp recipe directory with given files.
    fn setup_recipe(yaml: &str, templates: &[&str]) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let recipe_path = dir.path().join("recipe.yaml");
        fs::write(&recipe_path, yaml).unwrap();
        for t in templates {
            let p = dir.path().join(t);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&p, "template content").unwrap();
        }
        (dir, recipe_path)
    }

    // ── FR-1 acceptance criteria ─────────────────────────────────

    /// AC-1.1: Valid recipe parses into Recipe struct with name, description, variables, files.
    #[test]
    fn ac_1_1_valid_recipe_parses() {
        let yaml = r#"
name: test-recipe
description: A test recipe
variables:
  class_name:
    type: string
    required: true
files:
  - template: service.j2
    to: "src/{{ class_name }}.rs"
"#;
        let (_dir, path) = setup_recipe(yaml, &["service.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        assert_eq!(recipe.name.as_deref(), Some("test-recipe"));
        assert_eq!(recipe.description.as_deref(), Some("A test recipe"));
        assert_eq!(recipe.variables.len(), 1);
        assert!(recipe.variables.contains_key("class_name"));
        assert_eq!(recipe.files.len(), 1);
    }

    /// AC-1.2: Variables with all fields parse correctly.
    #[test]
    fn ac_1_2_variable_fields_parse() {
        let yaml = r#"
variables:
  status:
    type: enum
    required: true
    description: "The status value"
    values: ["active", "inactive"]
    default: "active"
  tags:
    type: array
    items: string
    default: []
files: []
"#;
        let (_dir, path) = setup_recipe(yaml, &[]);
        let recipe = Recipe::load(&path).unwrap();

        let status = &recipe.variables["status"];
        assert_eq!(status.var_type, VarType::Enum);
        assert!(status.required);
        assert_eq!(status.description.as_deref(), Some("The status value"));
        assert_eq!(status.values.as_ref().unwrap(), &["active", "inactive"]);
        assert!(status.default.is_some());

        let tags = &recipe.variables["tags"];
        assert_eq!(tags.var_type, VarType::Array);
        assert_eq!(tags.items, Some(VarType::String));
    }

    /// AC-1.3: Create operations parse correctly.
    #[test]
    fn ac_1_3_create_op_parses() {
        let yaml = r#"
files:
  - template: tmpl.j2
    to: "out/file.rs"
    skip_if_exists: true
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Create { template, to, skip_if_exists } => {
                assert_eq!(template, "tmpl.j2");
                assert_eq!(to, "out/file.rs");
                assert!(*skip_if_exists);
            }
            _ => panic!("expected Create op"),
        }
    }

    /// AC-1.4: Inject operations parse with all mode fields.
    #[test]
    fn ac_1_4_inject_op_parses() {
        let yaml = r#"
files:
  - template: fixture.j2
    inject: "tests/conftest.py"
    after: "^# fixtures"
    skip_if: "BookingService"
  - template: import.j2
    inject: "tests/conftest.py"
    before: "^class "
    at: last
  - template: header.j2
    inject: "src/main.rs"
    prepend: true
  - template: footer.j2
    inject: "src/main.rs"
    append: true
"#;
        let (_dir, path) = setup_recipe(yaml, &["fixture.j2", "import.j2", "header.j2", "footer.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        assert_eq!(recipe.files.len(), 4);

        // after mode
        match &recipe.files[0] {
            FileOp::Inject { inject, mode, skip_if, .. } => {
                assert_eq!(inject, "tests/conftest.py");
                assert!(matches!(mode, InjectMode::After { pattern, at } if pattern == "^# fixtures" && *at == MatchPosition::First));
                assert_eq!(skip_if.as_deref(), Some("BookingService"));
            }
            _ => panic!("expected Inject"),
        }

        // before with at:last
        match &recipe.files[1] {
            FileOp::Inject { mode, .. } => {
                assert!(matches!(mode, InjectMode::Before { pattern, at } if pattern == "^class " && *at == MatchPosition::Last));
            }
            _ => panic!("expected Inject"),
        }

        // prepend
        assert!(matches!(&recipe.files[2], FileOp::Inject { mode: InjectMode::Prepend, .. }));

        // append
        assert!(matches!(&recipe.files[3], FileOp::Inject { mode: InjectMode::Append, .. }));
    }

    /// AC-1.5: Malformed YAML exits with code 1.
    #[test]
    fn ac_1_5_malformed_yaml() {
        let yaml = "files:\n  - template: [broken\n";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        let se = err.structured_error();
        assert!(se.what.contains("malformed recipe YAML"));
    }

    /// AC-1.6: Missing required field 'template'.
    #[test]
    fn ac_1_6_missing_template_field() {
        let yaml = r#"
files:
  - to: "out/file.rs"
"#;
        let (_dir, path) = setup_recipe(yaml, &[]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("template"));
    }

    /// AC-1.7: Template paths resolve relative to recipe file.
    #[test]
    fn ac_1_7_template_relative_to_recipe() {
        let yaml = r#"
files:
  - template: templates/service.j2
    to: "out.rs"
"#;
        let (_dir, path) = setup_recipe(yaml, &["templates/service.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        let resolved = recipe.resolve_template("templates/service.j2");
        assert!(resolved.is_file());
    }

    /// AC-1.8: Missing template file reports error.
    #[test]
    fn ac_1_8_missing_template_file() {
        let yaml = r#"
files:
  - template: nonexistent.j2
    to: "out.rs"
"#;
        let (_dir, path) = setup_recipe(yaml, &[]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        let se = err.structured_error();
        assert!(se.what.contains("template file not found"));
        assert!(se.what.contains("nonexistent.j2"));
    }

    /// AC-1.9: Optional metadata accepted with or without.
    #[test]
    fn ac_1_9_optional_metadata() {
        // With metadata
        let yaml_with = "name: test\ndescription: desc\nfiles: []\n";
        let (_d1, p1) = setup_recipe(yaml_with, &[]);
        let r1 = Recipe::load(&p1).unwrap();
        assert_eq!(r1.name.as_deref(), Some("test"));

        // Without metadata
        let yaml_without = "files: []\n";
        let (_d2, p2) = setup_recipe(yaml_without, &[]);
        let r2 = Recipe::load(&p2).unwrap();
        assert!(r2.name.is_none());
        assert!(r2.description.is_none());
    }

    /// AC-1.10: Unknown operation type 'replace' or 'patch' rejected with v0.1 message.
    #[test]
    fn ac_1_10_unknown_op_replace() {
        let yaml = r#"
files:
  - template: tmpl.j2
    replace: "target.txt"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        let msg = err.structured_error().what.to_string();
        assert!(msg.contains("replace"), "expected 'replace' in: {msg}");
        assert!(msg.contains("not supported in v0.1"), "expected v0.1 message in: {msg}");
    }

    #[test]
    fn ac_1_10_unknown_op_patch() {
        let yaml = r#"
files:
  - template: tmpl.j2
    patch: "target.txt"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        let msg = err.structured_error().what.to_string();
        assert!(msg.contains("patch"), "expected 'patch' in: {msg}");
        assert!(msg.contains("not supported in v0.1"));
    }

    /// AC-1.11: Empty files array accepted (tested via parse — run behavior in Phase 3).
    #[test]
    fn ac_1_11_empty_files_array() {
        let yaml = "files: []\n";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let recipe = Recipe::load(&path).unwrap();
        assert!(recipe.files.is_empty());
    }

    /// AC-1.12: No variables key or empty variables accepted.
    #[test]
    fn ac_1_12_no_variables() {
        let yaml_none = "files: []\n";
        let (_d1, p1) = setup_recipe(yaml_none, &[]);
        let r1 = Recipe::load(&p1).unwrap();
        assert!(r1.variables.is_empty());

        let yaml_empty = "variables: {}\nfiles: []\n";
        let (_d2, p2) = setup_recipe(yaml_empty, &[]);
        let r2 = Recipe::load(&p2).unwrap();
        assert!(r2.variables.is_empty());
    }

    /// AC-1.13: Recipe file not found.
    #[test]
    fn ac_1_13_recipe_file_not_found() {
        let err = Recipe::load(Path::new("/tmp/nonexistent_jig_test_recipe.yaml")).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("recipe file not found"));
    }

    /// AC-1.14: Multiple operation types present.
    #[test]
    fn ac_1_14_ambiguous_op_type() {
        let yaml = r#"
files:
  - template: tmpl.j2
    to: "out.rs"
    inject: "target.rs"
    after: "^use"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("ambiguous"));
    }

    /// AC-1.15: No operation type present.
    #[test]
    fn ac_1_15_missing_op_type() {
        let yaml = r#"
files:
  - template: tmpl.j2
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("missing operation type"));
    }

    /// AC-5.14 (partial — recipe validation side): Invalid regex rejected at parse time.
    #[test]
    fn invalid_regex_rejected_at_parse() {
        let yaml = r#"
files:
  - template: tmpl.j2
    inject: "target.py"
    after: "(unclosed"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid regex"));
    }

    /// AC-5.15 (partial): Multiple inject modes rejected.
    #[test]
    fn multiple_inject_modes_rejected() {
        let yaml = r#"
files:
  - template: tmpl.j2
    inject: "target.py"
    after: "^import"
    prepend: true
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("conflicting"));
    }

    /// Inject with no mode specified.
    #[test]
    fn inject_missing_mode_rejected() {
        let yaml = r#"
files:
  - template: tmpl.j2
    inject: "target.py"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("missing mode"));
    }

    /// Error structures always have what/where/why/hint (AC-N4.1).
    #[test]
    fn ac_n4_1_errors_have_all_fields() {
        let yaml = "not: valid: yaml: [";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let err = Recipe::load(&path).unwrap_err();
        let se = err.structured_error();
        assert!(!se.what.is_empty());
        assert!(!se.where_.is_empty());
        assert!(!se.why.is_empty());
        assert!(!se.hint.is_empty());
    }

    /// AC-N5.1: Recipe validation errors use exit code 1.
    #[test]
    fn ac_n5_1_exit_code_is_1() {
        let cases: Vec<(&str, &[&str])> = vec![
            ("bad yaml [", &[]),
            ("files:\n  - to: x\n", &[]),
            ("files:\n  - template: missing.j2\n    to: x\n", &[]),
        ];
        for (yaml, tmpls) in cases {
            let (_dir, path) = setup_recipe(yaml, tmpls);
            let err = Recipe::load(&path).unwrap_err();
            assert_eq!(err.exit_code(), 1, "expected exit 1 for yaml: {yaml}");
        }
    }
}
