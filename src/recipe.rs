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
pub enum InjectMode {
    After { pattern: String, at: MatchPosition },
    Before { pattern: String, at: MatchPosition },
    Prepend,
    Append,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplaceSpec {
    Between { start: String, end: String },
    Pattern(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fallback {
    Append,
    Prepend,
    Skip,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeType {
    Line,
    Block,
    ClassBody,
    FunctionBody,
    FunctionSignature,
    Braces,
    Brackets,
    Parens,
}

impl std::fmt::Display for ScopeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeType::Line => write!(f, "line"),
            ScopeType::Block => write!(f, "block"),
            ScopeType::ClassBody => write!(f, "class_body"),
            ScopeType::FunctionBody => write!(f, "function_body"),
            ScopeType::FunctionSignature => write!(f, "function_signature"),
            ScopeType::Braces => write!(f, "braces"),
            ScopeType::Brackets => write!(f, "brackets"),
            ScopeType::Parens => write!(f, "parens"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    Before,
    After,
    BeforeClose,
    AfterLastField,
    AfterLastMethod,
    AfterLastImport,
    Sorted,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Before => write!(f, "before"),
            Position::After => write!(f, "after"),
            Position::BeforeClose => write!(f, "before_close"),
            Position::AfterLastField => write!(f, "after_last_field"),
            Position::AfterLastMethod => write!(f, "after_last_method"),
            Position::AfterLastImport => write!(f, "after_last_import"),
            Position::Sorted => write!(f, "sorted"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub pattern: String,
    pub scope: ScopeType,
    pub find: Option<String>,
    pub position: Position,
}

#[derive(Debug, Clone)]
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
    Replace {
        template: String,
        replace: String,
        spec: ReplaceSpec,
        fallback: Fallback,
    },
    Patch {
        template: String,
        patch: String,
        anchor: Anchor,
        skip_if: Option<String>,
    },
}

impl FileOp {
    pub fn op_type_str(&self) -> &'static str {
        match self {
            FileOp::Create { .. } => "create",
            FileOp::Inject { .. } => "inject",
            FileOp::Replace { .. } => "replace",
            FileOp::Patch { .. } => "patch",
        }
    }

    pub fn template(&self) -> &str {
        match self {
            FileOp::Create { template, .. }
            | FileOp::Inject { template, .. }
            | FileOp::Replace { template, .. }
            | FileOp::Patch { template, .. } => template,
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
struct RawBetween {
    start: Option<String>,
    end: Option<String>,
}

#[derive(Deserialize)]
struct RawAnchor {
    pattern: Option<String>,
    scope: Option<String>,
    find: Option<String>,
    position: Option<String>,
}

#[derive(Deserialize)]
struct RawFileOp {
    template: Option<String>,
    to: Option<String>,
    inject: Option<String>,
    replace: Option<String>,
    patch: Option<String>,
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
    between: Option<RawBetween>,
    pattern: Option<String>,
    fallback: Option<String>,
    anchor: Option<RawAnchor>,
    /// Catch-all for unknown fields to produce better error messages.
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_yaml::Value>,
}

// ── Parsing + validation ────────────────────────────────────────────

impl Recipe {
    /// Load and validate a recipe from a YAML file.
    pub fn load(path: &Path) -> Result<Self, JigError> {
        let path = path.canonicalize().map_err(|_| {
            recipe_err(
                "recipe file not found",
                &path.display().to_string(),
                "the file does not exist at the specified path",
                "check the file path and try again",
            )
        })?;

        let content = std::fs::read_to_string(&path).map_err(|e| {
            recipe_err(
                "cannot read recipe file",
                &path.display().to_string(),
                &e.to_string(),
                "check file permissions",
            )
        })?;

        let recipe_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self::parse(&content, recipe_dir, &path)
    }

    /// Parse recipe YAML content with a known recipe directory and source path.
    pub fn parse(yaml: &str, recipe_dir: PathBuf, source_path: &Path) -> Result<Self, JigError> {
        let raw: RawRecipe = serde_yaml::from_str(yaml).map_err(|e| {
            recipe_err(
                "malformed recipe YAML",
                &source_path.display().to_string(),
                &e.to_string(),
                "check YAML syntax — ensure proper indentation and field names",
            )
        })?;

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
    let template = raw.template.ok_or_else(|| {
        recipe_err(
            "missing required field 'template'",
            &loc,
            "every file operation must specify a template",
            "add a 'template' field pointing to the Jinja2 template file",
        )
    })?;

    // Reject unknown fields with a helpful message.
    if !raw.extra.is_empty() {
        let unknown: Vec<&String> = raw.extra.keys().collect();
        return Err(recipe_err(
            &format!(
                "unknown field(s): {}",
                unknown
                    .iter()
                    .map(|k| format!("'{k}'"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            &loc,
            "these fields are not recognized in a file operation",
            "remove the unknown fields — valid fields are: template, to, inject, replace, patch, skip_if_exists, after, before, prepend, append, at, skip_if, between, pattern, fallback, anchor",
        ));
    }

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
        if has_to {
            present.push("to");
        }
        if has_inject {
            present.push("inject");
        }
        if has_replace {
            present.push("replace");
        }
        if has_patch {
            present.push("patch");
        }
        return Err(recipe_err(
            &format!(
                "ambiguous operation type: found multiple of {}",
                present.join(", ")
            ),
            &loc,
            "a file operation must specify exactly one operation type",
            "remove extra fields so only one of 'to', 'inject', 'replace', or 'patch' remains",
        ));
    }

    if has_to {
        Ok(FileOp::Create {
            template,
            to: raw.to.unwrap(),
            skip_if_exists: raw.skip_if_exists,
        })
    } else if has_inject {
        let inject_target = raw.inject.unwrap();
        let mode = parse_inject_mode(
            &raw.after,
            &raw.before,
            raw.prepend,
            raw.append,
            &raw.at,
            index,
            source,
        )?;
        Ok(FileOp::Inject {
            template,
            inject: inject_target,
            mode,
            skip_if: raw.skip_if,
        })
    } else if has_replace {
        let replace_target = raw.replace.unwrap();
        let has_between = raw.between.is_some();
        let has_pattern = raw.pattern.is_some();

        if has_between && has_pattern {
            return Err(recipe_err(
                "replace operation has both 'between' and 'pattern'",
                &loc,
                "a replace operation must specify exactly one of 'between' or 'pattern', not both",
                "remove one of 'between' or 'pattern'",
            ));
        }
        if !has_between && !has_pattern {
            return Err(recipe_err(
                "replace operation missing match specification",
                &loc,
                "a replace operation must specify one of 'between' or 'pattern'",
                "add a 'between' block with start/end markers, or a 'pattern' field with a regex",
            ));
        }

        let spec = if has_between {
            let between = raw.between.unwrap();
            let start = between.start.ok_or_else(|| {
                recipe_err(
                    "replace 'between' missing required field 'start'",
                    &loc,
                    "between requires both 'start' and 'end' regex patterns",
                    "add a 'start' field to the 'between' block",
                )
            })?;
            let end = between.end.ok_or_else(|| {
                recipe_err(
                    "replace 'between' missing required field 'end'",
                    &loc,
                    "between requires both 'start' and 'end' regex patterns",
                    "add an 'end' field to the 'between' block",
                )
            })?;
            validate_regex(&start, "between.start", index, source)?;
            validate_regex(&end, "between.end", index, source)?;
            ReplaceSpec::Between { start, end }
        } else {
            let pattern = raw.pattern.unwrap();
            validate_regex(&pattern, "pattern", index, source)?;
            ReplaceSpec::Pattern(pattern)
        };

        let fallback = parse_fallback(raw.fallback.as_deref(), index, source)?;

        Ok(FileOp::Replace {
            template,
            replace: replace_target,
            spec,
            fallback,
        })
    } else {
        // has_patch
        let patch_target = raw.patch.unwrap();
        let raw_anchor = raw.anchor.ok_or_else(|| {
            recipe_err(
                "patch operation missing required 'anchor' field",
                &loc,
                "a patch operation must specify an 'anchor' block with at least a 'pattern' field",
                "add an 'anchor' block with a 'pattern' regex",
            )
        })?;
        let anchor_pattern = raw_anchor.pattern.ok_or_else(|| recipe_err(
            "patch 'anchor' missing required field 'pattern'",
            &loc,
            "the anchor block must include a 'pattern' field with a regex to find the anchor line",
            "add a 'pattern' field to the 'anchor' block",
        ))?;
        validate_regex(&anchor_pattern, "anchor.pattern", index, source)?;
        let scope = parse_scope_type(raw_anchor.scope.as_deref(), index, source)?;
        let position = parse_position(raw_anchor.position.as_deref(), index, source)?;

        Ok(FileOp::Patch {
            template,
            patch: patch_target,
            anchor: Anchor {
                pattern: anchor_pattern,
                scope,
                find: raw_anchor.find,
                position,
            },
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

    if let Some(pattern) = after {
        let pos = parse_match_position(at, index, source)?;
        validate_regex(pattern, "after", index, source)?;
        Ok(InjectMode::After {
            pattern: pattern.clone(),
            at: pos,
        })
    } else if let Some(pattern) = before {
        let pos = parse_match_position(at, index, source)?;
        validate_regex(pattern, "before", index, source)?;
        Ok(InjectMode::Before {
            pattern: pattern.clone(),
            at: pos,
        })
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
    if pattern.is_empty() {
        return Err(recipe_err(
            &format!("empty regex pattern in '{field}' field"),
            &format!("files[{index}] in {}", source.display()),
            "an empty pattern matches every line, which is almost certainly not intended",
            "provide a non-empty regex pattern",
        ));
    }
    regex::Regex::new(pattern).map_err(|e| {
        recipe_err(
            &format!("invalid regex in '{field}' field"),
            &format!("files[{index}] in {}", source.display()),
            &format!("pattern '{pattern}' failed to compile: {e}"),
            "check regex syntax — remember to escape special characters",
        )
    })?;
    Ok(())
}

fn parse_fallback(value: Option<&str>, index: usize, source: &Path) -> Result<Fallback, JigError> {
    match value {
        None | Some("error") => Ok(Fallback::Error),
        Some("append") => Ok(Fallback::Append),
        Some("prepend") => Ok(Fallback::Prepend),
        Some("skip") => Ok(Fallback::Skip),
        Some(other) => Err(recipe_err(
            &format!("invalid 'fallback' value: '{other}'"),
            &format!("files[{index}] in {}", source.display()),
            "fallback must be one of: append, prepend, skip, error",
            "valid values: append, prepend, skip, error",
        )),
    }
}

fn parse_scope_type(
    value: Option<&str>,
    index: usize,
    source: &Path,
) -> Result<ScopeType, JigError> {
    match value {
        None | Some("line") => Ok(ScopeType::Line),
        Some("block") => Ok(ScopeType::Block),
        Some("class_body") => Ok(ScopeType::ClassBody),
        Some("function_body") => Ok(ScopeType::FunctionBody),
        Some("function_signature") => Ok(ScopeType::FunctionSignature),
        Some("braces") => Ok(ScopeType::Braces),
        Some("brackets") => Ok(ScopeType::Brackets),
        Some("parens") => Ok(ScopeType::Parens),
        Some(other) => Err(recipe_err(
            &format!("invalid 'scope' value: '{other}'"),
            &format!("files[{index}] in {}", source.display()),
            "scope must be one of: line, block, class_body, function_body, function_signature, braces, brackets, parens",
            "valid values: line, block, class_body, function_body, function_signature, braces, brackets, parens",
        )),
    }
}

fn parse_position(value: Option<&str>, index: usize, source: &Path) -> Result<Position, JigError> {
    match value {
        None | Some("after") => Ok(Position::After),
        Some("before") => Ok(Position::Before),
        Some("before_close") => Ok(Position::BeforeClose),
        Some("after_last_field") => Ok(Position::AfterLastField),
        Some("after_last_method") => Ok(Position::AfterLastMethod),
        Some("after_last_import") => Ok(Position::AfterLastImport),
        Some("sorted") => Ok(Position::Sorted),
        Some(other) => Err(recipe_err(
            &format!("invalid 'position' value: '{other}'"),
            &format!("files[{index}] in {}", source.display()),
            "position must be one of: before, after, before_close, after_last_field, after_last_method, after_last_import, sorted",
            "valid values: before, after, before_close, after_last_field, after_last_method, after_last_import, sorted",
        )),
    }
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
            FileOp::Create {
                template,
                to,
                skip_if_exists,
            } => {
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
        let (_dir, path) =
            setup_recipe(yaml, &["fixture.j2", "import.j2", "header.j2", "footer.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        assert_eq!(recipe.files.len(), 4);

        // after mode
        match &recipe.files[0] {
            FileOp::Inject {
                inject,
                mode,
                skip_if,
                ..
            } => {
                assert_eq!(inject, "tests/conftest.py");
                assert!(
                    matches!(mode, InjectMode::After { pattern, at } if pattern == "^# fixtures" && *at == MatchPosition::First)
                );
                assert_eq!(skip_if.as_deref(), Some("BookingService"));
            }
            _ => panic!("expected Inject"),
        }

        // before with at:last
        match &recipe.files[1] {
            FileOp::Inject { mode, .. } => {
                assert!(
                    matches!(mode, InjectMode::Before { pattern, at } if pattern == "^class " && *at == MatchPosition::Last)
                );
            }
            _ => panic!("expected Inject"),
        }

        // prepend
        assert!(matches!(
            &recipe.files[2],
            FileOp::Inject {
                mode: InjectMode::Prepend,
                ..
            }
        ));

        // append
        assert!(matches!(
            &recipe.files[3],
            FileOp::Inject {
                mode: InjectMode::Append,
                ..
            }
        ));
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

    /// Replace operation with between spec parses correctly.
    #[test]
    fn parse_replace_between() {
        let yaml = r#"
files:
  - template: tmpl.j2
    replace: "target.txt"
    between:
      start: "^# START"
      end: "^# END"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Replace {
                replace,
                spec,
                fallback,
                ..
            } => {
                assert_eq!(replace, "target.txt");
                assert!(
                    matches!(spec, ReplaceSpec::Between { start, end } if start == "^# START" && end == "^# END")
                );
                assert_eq!(*fallback, Fallback::Error);
            }
            _ => panic!("expected Replace op"),
        }
    }

    /// Replace operation with pattern spec parses correctly.
    #[test]
    fn parse_replace_pattern() {
        let yaml = r#"
files:
  - template: tmpl.j2
    replace: "target.txt"
    pattern: "^old_.*"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Replace { spec, .. } => {
                assert!(matches!(spec, ReplaceSpec::Pattern(p) if p == "^old_.*"));
            }
            _ => panic!("expected Replace op"),
        }
    }

    /// All four fallback variants parse correctly.
    #[test]
    fn parse_replace_fallback_variants() {
        for (val, expected) in &[
            ("append", Fallback::Append),
            ("prepend", Fallback::Prepend),
            ("skip", Fallback::Skip),
            ("error", Fallback::Error),
        ] {
            let yaml = format!(
                "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    pattern: \"^x\"\n    fallback: {val}\n"
            );
            let (_dir, path) = setup_recipe(&yaml, &["tmpl.j2"]);
            let recipe = Recipe::load(&path).unwrap();
            match &recipe.files[0] {
                FileOp::Replace { fallback, .. } => assert_eq!(fallback, expected),
                _ => panic!("expected Replace"),
            }
        }
    }

    /// Omitted fallback defaults to Error.
    #[test]
    fn parse_replace_fallback_default() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    pattern: \"^x\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Replace { fallback, .. } => assert_eq!(*fallback, Fallback::Error),
            _ => panic!("expected Replace"),
        }
    }

    /// Patch with full anchor fields parses correctly.
    #[test]
    fn parse_patch_full_anchor() {
        let yaml = r#"
files:
  - template: tmpl.j2
    patch: "target.py"
    anchor:
      pattern: "^class User:"
      scope: class_body
      find: "list_display"
      position: before_close
    skip_if: "already_present"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Patch {
                patch,
                anchor,
                skip_if,
                ..
            } => {
                assert_eq!(patch, "target.py");
                assert_eq!(anchor.pattern, "^class User:");
                assert_eq!(anchor.scope, ScopeType::ClassBody);
                assert_eq!(anchor.find.as_deref(), Some("list_display"));
                assert_eq!(anchor.position, Position::BeforeClose);
                assert_eq!(skip_if.as_deref(), Some("already_present"));
            }
            _ => panic!("expected Patch op"),
        }
    }

    /// Patch with minimal anchor (just pattern) defaults scope=Line, position=After.
    #[test]
    fn parse_patch_minimal() {
        let yaml = "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      pattern: \"^class\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Patch { anchor, .. } => {
                assert_eq!(anchor.scope, ScopeType::Line);
                assert_eq!(anchor.position, Position::After);
                assert!(anchor.find.is_none());
            }
            _ => panic!("expected Patch"),
        }
    }

    /// Patch with skip_if captured.
    #[test]
    fn parse_patch_with_skip_if() {
        let yaml = "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      pattern: \"^x\"\n    skip_if: \"marker\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Patch { skip_if, .. } => assert_eq!(skip_if.as_deref(), Some("marker")),
            _ => panic!("expected Patch"),
        }
    }

    /// All 8 scope type strings parse correctly.
    #[test]
    fn parse_patch_all_scope_types() {
        let types = [
            ("line", ScopeType::Line),
            ("block", ScopeType::Block),
            ("class_body", ScopeType::ClassBody),
            ("function_body", ScopeType::FunctionBody),
            ("function_signature", ScopeType::FunctionSignature),
            ("braces", ScopeType::Braces),
            ("brackets", ScopeType::Brackets),
            ("parens", ScopeType::Parens),
        ];
        for (name, expected) in &types {
            let yaml = format!(
                "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      pattern: \"^x\"\n      scope: {name}\n"
            );
            let (_dir, path) = setup_recipe(&yaml, &["tmpl.j2"]);
            let recipe = Recipe::load(&path).unwrap();
            match &recipe.files[0] {
                FileOp::Patch { anchor, .. } => {
                    assert_eq!(&anchor.scope, expected, "failed for scope: {name}")
                }
                _ => panic!("expected Patch for scope: {name}"),
            }
        }
    }

    /// All 7 position type strings parse correctly.
    #[test]
    fn parse_patch_all_position_types() {
        let positions = [
            ("before", Position::Before),
            ("after", Position::After),
            ("before_close", Position::BeforeClose),
            ("after_last_field", Position::AfterLastField),
            ("after_last_method", Position::AfterLastMethod),
            ("after_last_import", Position::AfterLastImport),
            ("sorted", Position::Sorted),
        ];
        for (name, expected) in &positions {
            let yaml = format!(
                "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      pattern: \"^x\"\n      position: {name}\n"
            );
            let (_dir, path) = setup_recipe(&yaml, &["tmpl.j2"]);
            let recipe = Recipe::load(&path).unwrap();
            match &recipe.files[0] {
                FileOp::Patch { anchor, .. } => {
                    assert_eq!(&anchor.position, expected, "failed for position: {name}")
                }
                _ => panic!("expected Patch for position: {name}"),
            }
        }
    }

    /// Replace path is a string field.
    #[test]
    fn parse_replace_path_is_string() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"path/to/{{ name }}.py\"\n    pattern: \"^x\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        match &recipe.files[0] {
            FileOp::Replace { replace, .. } => assert_eq!(replace, "path/to/{{ name }}.py"),
            _ => panic!("expected Replace"),
        }
    }

    // ── Error case tests for replace/patch ──

    #[test]
    fn reject_replace_both_between_and_pattern() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    between:\n      start: \"^a\"\n      end: \"^b\"\n    pattern: \"^c\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("both"));
    }

    #[test]
    fn reject_replace_neither_between_nor_pattern() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("missing match"));
    }

    #[test]
    fn reject_replace_between_missing_start() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    between:\n      end: \"^b\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("start"));
    }

    #[test]
    fn reject_replace_between_missing_end() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    between:\n      start: \"^a\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("end"));
    }

    #[test]
    fn reject_replace_invalid_regex_start() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    between:\n      start: \"(unclosed\"\n      end: \"^b\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid regex"));
    }

    #[test]
    fn reject_replace_invalid_regex_end() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    between:\n      start: \"^a\"\n      end: \"(unclosed\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid regex"));
    }

    #[test]
    fn reject_replace_invalid_regex_pattern() {
        let yaml =
            "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    pattern: \"(unclosed\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid regex"));
    }

    #[test]
    fn reject_replace_invalid_fallback() {
        let yaml = "files:\n  - template: tmpl.j2\n    replace: \"t.txt\"\n    pattern: \"^x\"\n    fallback: banana\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid 'fallback'"));
    }

    #[test]
    fn reject_patch_missing_anchor() {
        let yaml = "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("anchor"));
    }

    #[test]
    fn reject_patch_missing_anchor_pattern() {
        let yaml = "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      scope: braces\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("pattern"));
    }

    #[test]
    fn reject_patch_invalid_anchor_regex() {
        let yaml = "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      pattern: \"(unclosed\"\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid regex"));
    }

    #[test]
    fn reject_patch_invalid_scope_type() {
        let yaml = "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      pattern: \"^x\"\n      scope: banana\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid 'scope'"));
    }

    #[test]
    fn reject_patch_invalid_position_type() {
        let yaml = "files:\n  - template: tmpl.j2\n    patch: \"t.py\"\n    anchor:\n      pattern: \"^x\"\n      position: banana\n";
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let err = Recipe::load(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("invalid 'position'"));
    }

    /// Existing create/inject still parse after adding replace/patch.
    #[test]
    fn existing_create_inject_still_parse() {
        let yaml = r#"
files:
  - template: tmpl.j2
    to: "out.rs"
  - template: tmpl.j2
    inject: "target.py"
    after: "^import"
"#;
        let (_dir, path) = setup_recipe(yaml, &["tmpl.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        assert_eq!(recipe.files.len(), 2);
        assert!(matches!(&recipe.files[0], FileOp::Create { .. }));
        assert!(matches!(&recipe.files[1], FileOp::Inject { .. }));
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
        assert!(
            err.structured_error()
                .what
                .contains("recipe file not found")
        );
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
        assert!(
            err.structured_error()
                .what
                .contains("missing operation type")
        );
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

    /// AC-5.12 (parser-level): `at` field is ignored when prepend/append is specified.
    #[test]
    fn ac_5_12_at_ignored_for_prepend_append_at_parse() {
        // prepend with at: first — should parse fine
        let yaml1 = "files:\n  - template: tmpl.j2\n    inject: \"target.py\"\n    prepend: true\n    at: first\n";
        let (_dir1, path1) = setup_recipe(yaml1, &["tmpl.j2"]);
        let r1 = Recipe::load(&path1).unwrap();
        assert!(
            matches!(r1.files[0], FileOp::Inject { ref mode, .. } if matches!(mode, InjectMode::Prepend))
        );

        // append with at: last — should parse fine
        let yaml2 = "files:\n  - template: tmpl.j2\n    inject: \"target.py\"\n    append: true\n    at: last\n";
        let (_dir2, path2) = setup_recipe(yaml2, &["tmpl.j2"]);
        let r2 = Recipe::load(&path2).unwrap();
        assert!(
            matches!(r2.files[0], FileOp::Inject { ref mode, .. } if matches!(mode, InjectMode::Append))
        );

        // prepend with at: banana — invalid at value, but should be ignored for prepend
        let yaml3 = "files:\n  - template: tmpl.j2\n    inject: \"target.py\"\n    prepend: true\n    at: banana\n";
        let (_dir3, path3) = setup_recipe(yaml3, &["tmpl.j2"]);
        let r3 = Recipe::load(&path3).unwrap();
        assert!(
            matches!(r3.files[0], FileOp::Inject { ref mode, .. } if matches!(mode, InjectMode::Prepend))
        );
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
