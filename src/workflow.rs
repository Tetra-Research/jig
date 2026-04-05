use std::collections::HashSet;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{JigError, StructuredError};
use crate::operations::{ExecutionContext, OpResult};
use crate::recipe::{Recipe, VariableDecl};
use crate::renderer;
use crate::variables;

// ── OnError enum ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnError {
    Stop,
    Continue,
    Report,
}

#[allow(clippy::derivable_impls)]
impl Default for OnError {
    fn default() -> Self {
        OnError::Stop
    }
}

impl std::fmt::Display for OnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnError::Stop => write!(f, "stop"),
            OnError::Continue => write!(f, "continue"),
            OnError::Report => write!(f, "report"),
        }
    }
}

impl<'de> Deserialize<'de> for OnError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "stop" => Ok(OnError::Stop),
            "continue" => Ok(OnError::Continue),
            "report" => Ok(OnError::Report),
            _ => Err(serde::de::Error::custom(format!(
                "invalid on_error value '{}': allowed values are stop, continue, report",
                s
            ))),
        }
    }
}

// ── Workflow types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawWorkflow {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    variables: IndexMap<String, VariableDecl>,
    steps: Vec<RawWorkflowStep>,
    #[serde(default)]
    on_error: OnError,
}

#[derive(Debug, Deserialize)]
struct RawWorkflowStep {
    recipe: String,
    when: Option<String>,
    vars_map: Option<IndexMap<String, String>>,
    vars: Option<IndexMap<String, Value>>,
    on_error: Option<OnError>,
}

#[derive(Debug)]
pub struct Workflow {
    pub name: Option<String>,
    pub description: Option<String>,
    pub variables: IndexMap<String, VariableDecl>,
    pub steps: Vec<WorkflowStep>,
    pub on_error: OnError,
    #[allow(dead_code)]
    pub workflow_dir: PathBuf,
}

#[derive(Debug)]
pub struct WorkflowStep {
    /// Original recipe path as authored in the YAML.
    pub recipe: String,
    /// Resolved absolute path to the recipe file.
    pub resolved_recipe: PathBuf,
    pub when: Option<String>,
    pub vars_map: Option<IndexMap<String, String>>,
    pub vars: Option<IndexMap<String, Value>>,
    pub on_error: Option<OnError>,
}

// ── Result types ─────────────────────────────────────────────────

pub struct WorkflowResult {
    pub name: Option<String>,
    pub on_error: OnError,
    pub steps: Vec<StepResult>,
}

pub enum StepResult {
    Success {
        recipe: String,
        operations: Vec<OpResult>,
    },
    Skipped {
        recipe: String,
        reason: String,
    },
    Error {
        recipe: String,
        error: JigError,
        operations: Vec<OpResult>,
        rendered_content: Option<String>,
    },
}

impl StepResult {
    #[allow(dead_code)]
    pub fn recipe_path(&self) -> &str {
        match self {
            StepResult::Success { recipe, .. }
            | StepResult::Skipped { recipe, .. }
            | StepResult::Error { recipe, .. } => recipe,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, StepResult::Error { .. })
    }
}

// ── File type detection ──────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
pub enum FileType {
    Workflow,
    Recipe,
}

pub fn detect_file_type(path: &Path) -> Result<FileType, JigError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        JigError::RecipeValidation(StructuredError {
            what: format!("cannot read file '{}'", path.display()),
            where_: path.display().to_string(),
            why: e.to_string(),
            hint: "check the file path and permissions".into(),
        })
    })?;

    let value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| {
        JigError::RecipeValidation(StructuredError {
            what: "malformed YAML".into(),
            where_: path.display().to_string(),
            why: e.to_string(),
            hint: "check YAML syntax".into(),
        })
    })?;

    let mapping = value.as_mapping().ok_or_else(|| {
        JigError::RecipeValidation(StructuredError {
            what: "expected a YAML mapping at top level".into(),
            where_: path.display().to_string(),
            why: "the file does not contain a YAML mapping".into(),
            hint: "ensure the file is a valid recipe or workflow YAML".into(),
        })
    })?;

    let has_steps = mapping.contains_key(serde_yaml::Value::String("steps".into()));
    let has_files = mapping.contains_key(serde_yaml::Value::String("files".into()));

    match (has_steps, has_files) {
        (true, true) => Err(JigError::RecipeValidation(StructuredError {
            what: "ambiguous file type".into(),
            where_: path.display().to_string(),
            why: "file contains both 'steps' and 'files' top-level keys".into(),
            hint: "a workflow uses 'steps', a recipe uses 'files' — remove one".into(),
        })),
        (true, false) => Ok(FileType::Workflow),
        (false, true) => Ok(FileType::Recipe),
        (false, false) => Err(JigError::RecipeValidation(StructuredError {
            what: "missing structural key".into(),
            where_: path.display().to_string(),
            why: "file contains neither 'steps' (workflow) nor 'files' (recipe)".into(),
            hint: "add a 'steps' array for workflows or a 'files' array for recipes".into(),
        })),
    }
}

// ── Workflow loading ─────────────────────────────────────────────

pub fn load_workflow(path: &Path) -> Result<Workflow, JigError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        JigError::RecipeValidation(StructuredError {
            what: format!("cannot read workflow file '{}'", path.display()),
            where_: path.display().to_string(),
            why: e.to_string(),
            hint: "check the file path and permissions".into(),
        })
    })?;

    // Pre-parse for ambiguity check.
    let value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| {
        JigError::RecipeValidation(StructuredError {
            what: "malformed workflow YAML".into(),
            where_: path.display().to_string(),
            why: e.to_string(),
            hint: "check YAML syntax".into(),
        })
    })?;

    if let Some(mapping) = value.as_mapping() {
        let has_steps = mapping.contains_key(serde_yaml::Value::String("steps".into()));
        let has_files = mapping.contains_key(serde_yaml::Value::String("files".into()));
        if has_steps && has_files {
            return Err(JigError::RecipeValidation(StructuredError {
                what: "ambiguous file type".into(),
                where_: path.display().to_string(),
                why: "file contains both 'steps' and 'files' top-level keys".into(),
                hint: "a workflow uses 'steps', a recipe uses 'files' — remove one".into(),
            }));
        }
    }

    // Deserialize into typed struct.
    let raw: RawWorkflow = serde_yaml::from_str(&content).map_err(|e| {
        JigError::RecipeValidation(StructuredError {
            what: "invalid workflow YAML".into(),
            where_: path.display().to_string(),
            why: e.to_string(),
            hint: "check workflow structure: steps (required), name, description, variables, on_error (optional)".into(),
        })
    })?;

    let workflow_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();

    // Validate vars_map for duplicate targets.
    for (i, step) in raw.steps.iter().enumerate() {
        if let Some(ref vm) = step.vars_map {
            let mut targets = HashSet::new();
            for target in vm.values() {
                if !targets.insert(target) {
                    return Err(JigError::RecipeValidation(StructuredError {
                        what: format!("duplicate vars_map target '{}'", target),
                        where_: format!("steps[{}].vars_map", i),
                        why: format!("multiple source variables map to the same target '{}'", target),
                        hint: "each vars_map target must be unique".into(),
                    }));
                }
            }
        }
    }

    // Resolve recipe paths and validate.
    let mut steps = Vec::with_capacity(raw.steps.len());
    let mut validation_errors = Vec::new();

    for (i, raw_step) in raw.steps.into_iter().enumerate() {
        let resolved = workflow_dir.join(&raw_step.recipe);
        if !resolved.exists() {
            validation_errors.push((i, raw_step.recipe.clone(), StructuredError {
                what: format!("recipe file not found: '{}'", raw_step.recipe),
                where_: format!("steps[{}].recipe", i),
                why: format!("resolved path '{}' does not exist", resolved.display()),
                hint: "check the recipe path — it is resolved relative to the workflow file's directory".into(),
            }));
        } else {
            // Validate the recipe is structurally valid.
            match Recipe::load(&resolved) {
                Ok(_) => {}
                Err(e) => {
                    let se = e.structured_error().clone();
                    validation_errors.push((i, raw_step.recipe.clone(), StructuredError {
                        what: format!("invalid recipe in step {}: {}", i + 1, se.what),
                        where_: format!("steps[{}].recipe ({})", i, raw_step.recipe),
                        why: se.why,
                        hint: se.hint,
                    }));
                }
            }
        }

        steps.push(WorkflowStep {
            recipe: raw_step.recipe,
            resolved_recipe: resolved,
            when: raw_step.when,
            vars_map: raw_step.vars_map,
            vars: raw_step.vars,
            on_error: raw_step.on_error,
        });
    }

    if !validation_errors.is_empty() {
        // Return first error (with step context).
        let (_, _, err) = validation_errors.into_iter().next().unwrap();
        return Err(JigError::RecipeValidation(err));
    }

    Ok(Workflow {
        name: raw.name,
        description: raw.description,
        variables: raw.variables,
        steps,
        on_error: raw.on_error,
        workflow_dir,
    })
}

// ── Workflow validation (for jig validate) ───────────────────────

pub struct WorkflowValidation {
    pub name: Option<String>,
    pub description: Option<String>,
    pub variables: IndexMap<String, VariableDecl>,
    pub steps: Vec<StepValidation>,
}

pub struct StepValidation {
    pub recipe: String,
    pub valid: bool,
    pub conditional: bool,
    pub when: Option<String>,
    pub error: Option<String>,
}

pub fn validate_workflow(path: &Path) -> Result<WorkflowValidation, JigError> {
    let workflow = load_workflow(path)?;

    let steps = workflow
        .steps
        .iter()
        .map(|step| StepValidation {
            recipe: step.recipe.clone(),
            valid: true, // already validated in load_workflow
            conditional: step.when.is_some(),
            when: step.when.clone(),
            error: None,
        })
        .collect();

    Ok(WorkflowValidation {
        name: workflow.name,
        description: workflow.description,
        variables: workflow.variables,
        steps,
    })
}

// ── Variable resolution ──────────────────────────────────────────

pub fn resolve_step_variables(
    workflow_vars: &Value,
    step: &WorkflowStep,
) -> Result<Value, JigError> {
    let mut vars = workflow_vars.clone();

    // Apply vars_map (simultaneous rename).
    if let Some(ref vm) = step.vars_map {
        let obj = vars.as_object_mut().unwrap();

        // 1. Snapshot all source values.
        let mut snapshots: Vec<(String, String, Option<Value>)> = Vec::new();
        for (source, target) in vm {
            let val = obj.get(source).cloned();
            snapshots.push((source.clone(), target.clone(), val));
        }

        // 2. Remove all source keys.
        for (source, _, _) in &snapshots {
            obj.remove(source);
        }

        // 3. Insert all target keys (with values from snapshots).
        for (_, target, val) in snapshots {
            if let Some(v) = val {
                obj.insert(target, v);
            }
            // AC-4.7: nonexistent source → silently ignored
        }
    }

    // Apply vars overrides.
    if let Some(ref overrides) = step.vars {
        let obj = vars.as_object_mut().unwrap();
        for (key, val) in overrides {
            obj.insert(key.clone(), val.clone());
        }
    }

    Ok(vars)
}

// ── When evaluation ──────────────────────────────────────────────

pub fn evaluate_when(when_expr: &str, vars: &Value) -> Result<bool, JigError> {
    let env = renderer::create_standalone_env();
    let rendered = renderer::render_string(&env, when_expr, vars, "when")?;
    let trimmed = rendered.trim();

    // Falsy: empty, "false" (case-insensitive), "0"
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("false") || trimmed == "0" {
        Ok(false)
    } else {
        Ok(true)
    }
}

// ── Workflow execution ───────────────────────────────────────────

pub fn execute_workflow(
    workflow: &Workflow,
    vars: Value,
    ctx: &mut ExecutionContext,
    verbose: bool,
) -> WorkflowResult {
    let mut step_results = Vec::with_capacity(workflow.steps.len());

    for (i, step) in workflow.steps.iter().enumerate() {
        // (a) Evaluate when condition against workflow-level vars.
        if let Some(ref when_expr) = step.when {
            match evaluate_when(when_expr, &vars) {
                Ok(true) => {} // proceed
                Ok(false) => {
                    step_results.push(StepResult::Skipped {
                        recipe: step.recipe.clone(),
                        reason: "when condition evaluated to false".into(),
                    });
                    continue;
                }
                Err(e) => {
                    let effective_mode = step.on_error.unwrap_or(workflow.on_error);
                    step_results.push(StepResult::Error {
                        recipe: step.recipe.clone(),
                        error: e,
                        operations: vec![],
                        rendered_content: None,
                    });
                    match effective_mode {
                        OnError::Stop => break,
                        OnError::Continue | OnError::Report => continue,
                    }
                }
            }
        }

        // (b) Resolve step variables.
        let step_vars = match resolve_step_variables(&vars, step) {
            Ok(v) => v,
            Err(e) => {
                let effective_mode = step.on_error.unwrap_or(workflow.on_error);
                step_results.push(StepResult::Error {
                    recipe: step.recipe.clone(),
                    error: e,
                    operations: vec![],
                    rendered_content: None,
                });
                match effective_mode {
                    OnError::Stop => break,
                    OnError::Continue | OnError::Report => continue,
                }
            }
        };

        // (c) Load recipe.
        let recipe = match Recipe::load(&step.resolved_recipe) {
            Ok(r) => r,
            Err(e) => {
                let effective_mode = step.on_error.unwrap_or(workflow.on_error);
                step_results.push(StepResult::Error {
                    recipe: step.recipe.clone(),
                    error: e,
                    operations: vec![],
                    rendered_content: None,
                });
                match effective_mode {
                    OnError::Stop => break,
                    OnError::Continue | OnError::Report => continue,
                }
            }
        };

        // (d) Validate step vars against recipe declarations.
        let validated_vars = match variables::validate_variables(&recipe.variables, &step_vars) {
            Ok(v) => v,
            Err(e) => {
                let effective_mode = step.on_error.unwrap_or(workflow.on_error);
                step_results.push(StepResult::Error {
                    recipe: step.recipe.clone(),
                    error: e,
                    operations: vec![],
                    rendered_content: None,
                });
                match effective_mode {
                    OnError::Stop => break,
                    OnError::Continue | OnError::Report => continue,
                }
            }
        };

        // (e) Execute recipe via run_recipe.
        match run_recipe(&recipe, &validated_vars, ctx, verbose) {
            Ok(results) => {
                step_results.push(StepResult::Success {
                    recipe: step.recipe.clone(),
                    operations: results,
                });
            }
            Err((e, partial_results)) => {
                let rendered = extract_rendered_from_error(&e);
                let effective_mode = step.on_error.unwrap_or(workflow.on_error);
                step_results.push(StepResult::Error {
                    recipe: step.recipe.clone(),
                    error: e,
                    operations: partial_results,
                    rendered_content: rendered,
                });
                match effective_mode {
                    OnError::Stop => break,
                    OnError::Continue | OnError::Report => {
                        // Record step index for error reporting
                        let _ = i; // used for context in output formatting
                        continue;
                    }
                }
            }
        }
    }

    WorkflowResult {
        name: workflow.name.clone(),
        on_error: workflow.on_error,
        steps: step_results,
    }
}

// ── run_recipe (extracted from cmd_run logic) ────────────────────

/// Execute a loaded recipe with provided variables in the given context.
/// Returns Ok(results) on success, or Err((error, partial_results)) on failure.
#[allow(clippy::result_large_err)]
pub fn run_recipe(
    recipe: &Recipe,
    vars: &Value,
    ctx: &mut ExecutionContext,
    verbose: bool,
) -> Result<Vec<OpResult>, (JigError, Vec<OpResult>)> {
    use crate::operations;
    use crate::recipe::FileOp;

    // Create recipe-aware environment.
    let env = renderer::create_recipe_env(recipe).map_err(|e| (e, vec![]))?;

    // Render ALL templates and paths upfront.
    let mut prepared_ops = Vec::with_capacity(recipe.files.len());
    for (i, file_op) in recipe.files.iter().enumerate() {
        let rendered_content = renderer::render_template(&env, file_op.template(), vars)
            .map_err(|e| (e, vec![]))?;

        let rendered_path = match file_op {
            FileOp::Create { to, .. } => {
                renderer::render_path_template(&env, to, vars, &format!("files[{}].to", i))
            }
            FileOp::Inject { inject, .. } => {
                renderer::render_path_template(&env, inject, vars, &format!("files[{}].inject", i))
            }
            FileOp::Replace { replace, .. } => {
                renderer::render_path_template(&env, replace, vars, &format!("files[{}].replace", i))
            }
            FileOp::Patch { patch, .. } => {
                renderer::render_path_template(&env, patch, vars, &format!("files[{}].patch", i))
            }
        }
        .map_err(|e| (e, vec![]))?;

        let rendered_skip_if = match file_op {
            FileOp::Inject { skip_if: Some(expr), .. }
            | FileOp::Patch { skip_if: Some(expr), .. } => {
                Some(
                    renderer::render_path_template(&env, expr, vars, &format!("files[{}].skip_if", i))
                        .map_err(|e| (e, vec![]))?,
                )
            }
            _ => None,
        };

        prepared_ops.push(operations::PreparedOp {
            file_op: file_op.clone(),
            rendered_content,
            rendered_path,
            rendered_skip_if,
        });
    }

    // Execute operations in order, fail-fast.
    let mut results = Vec::with_capacity(prepared_ops.len());
    for prepared in &prepared_ops {
        let result = operations::execute_operation(prepared, ctx, verbose);
        let is_err = result.is_error();
        results.push(result);
        if is_err {
            // Extract the error from the last result.
            if let Some(jig_err) = operations::op_error_to_jig_error(results.last().unwrap()) {
                return Err((jig_err, results));
            }
        }
    }

    Ok(results)
}

fn extract_rendered_from_error(err: &JigError) -> Option<String> {
    match err {
        JigError::TemplateRendering(se) | JigError::FileOperation(se) => {
            Some(se.what.clone())
        }
        _ => None,
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    // ── Helper: create a minimal recipe ──
    fn create_recipe_dir(dir: &Path, name: &str, template_content: &str) -> PathBuf {
        let recipe_dir = dir.join(name);
        fs::create_dir_all(&recipe_dir).unwrap();
        fs::write(
            recipe_dir.join("recipe.yaml"),
            format!(
                "files:\n  - template: t.j2\n    to: {}/output.txt\n",
                name
            ),
        )
        .unwrap();
        fs::write(recipe_dir.join("t.j2"), template_content).unwrap();
        recipe_dir.join("recipe.yaml")
    }

    fn create_workflow_yaml(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join("workflow.yaml");
        fs::write(&path, content).unwrap();
        path
    }

    // ── File type detection ──

    #[test]
    fn detect_workflow_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.yaml");
        fs::write(&path, "steps:\n  - recipe: foo.yaml\n").unwrap();
        assert_eq!(detect_file_type(&path).unwrap(), FileType::Workflow);
    }

    #[test]
    fn detect_recipe_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.yaml");
        fs::write(&path, "files:\n  - template: t.j2\n    to: out.txt\n").unwrap();
        assert_eq!(detect_file_type(&path).unwrap(), FileType::Recipe);
    }

    #[test]
    fn detect_ambiguous_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.yaml");
        fs::write(&path, "steps: []\nfiles: []\n").unwrap();
        let err = detect_file_type(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("ambiguous"));
    }

    #[test]
    fn detect_missing_structural_key() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.yaml");
        fs::write(&path, "name: something\n").unwrap();
        let err = detect_file_type(&path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("missing structural key"));
    }

    // ── Workflow parsing ──

    #[test]
    fn parse_minimal_workflow() {
        let dir = TempDir::new().unwrap();
        create_recipe_dir(dir.path(), "step1", "hello");
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: step1/recipe.yaml\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        assert!(wf.name.is_none());
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.on_error, OnError::Stop);
    }

    #[test]
    fn parse_full_workflow() {
        let dir = TempDir::new().unwrap();
        create_recipe_dir(dir.path(), "s1", "content");
        create_recipe_dir(dir.path(), "s2", "content");
        let wf_path = create_workflow_yaml(
            dir.path(),
            r#"name: test-workflow
description: A test
variables:
  name:
    type: string
    required: true
on_error: report
steps:
  - recipe: s1/recipe.yaml
  - recipe: s2/recipe.yaml
    when: "{{ name }}"
    vars_map:
      name: model_name
    vars:
      extra: value
    on_error: continue
"#,
        );
        let wf = load_workflow(&wf_path).unwrap();
        assert_eq!(wf.name.as_deref(), Some("test-workflow"));
        assert_eq!(wf.on_error, OnError::Report);
        assert_eq!(wf.steps.len(), 2);
        assert!(wf.steps[1].when.is_some());
        assert!(wf.steps[1].vars_map.is_some());
        assert!(wf.steps[1].vars.is_some());
        assert_eq!(wf.steps[1].on_error, Some(OnError::Continue));
    }

    #[test]
    fn parse_empty_steps() {
        let dir = TempDir::new().unwrap();
        let wf_path = create_workflow_yaml(dir.path(), "steps: []\n");
        let wf = load_workflow(&wf_path).unwrap();
        assert!(wf.steps.is_empty());
    }

    #[test]
    fn parse_no_variables() {
        let dir = TempDir::new().unwrap();
        create_recipe_dir(dir.path(), "s1", "content");
        let wf_path = create_workflow_yaml(dir.path(), "steps:\n  - recipe: s1/recipe.yaml\n");
        let wf = load_workflow(&wf_path).unwrap();
        assert!(wf.variables.is_empty());
    }

    #[test]
    fn parse_missing_recipe_error() {
        let dir = TempDir::new().unwrap();
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: nonexistent/recipe.yaml\n",
        );
        let err = load_workflow(&wf_path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("not found"));
    }

    #[test]
    fn parse_ambiguous_yaml_error() {
        let dir = TempDir::new().unwrap();
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps: []\nfiles: []\n",
        );
        let err = load_workflow(&wf_path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("ambiguous"));
    }

    #[test]
    fn parse_bad_on_error_value() {
        let dir = TempDir::new().unwrap();
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps: []\non_error: crash\n",
        );
        let err = load_workflow(&wf_path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn parse_duplicate_vars_map_target() {
        let dir = TempDir::new().unwrap();
        create_recipe_dir(dir.path(), "s1", "content");
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: s1/recipe.yaml\n    vars_map:\n      a: c\n      b: c\n",
        );
        let err = load_workflow(&wf_path).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(err.structured_error().what.contains("duplicate vars_map target"));
    }

    // ── Variable resolution ──

    #[test]
    fn resolve_no_map_no_override() {
        let vars = json!({"a": 1, "b": 2});
        let step = WorkflowStep {
            recipe: "r.yaml".into(),
            resolved_recipe: PathBuf::from("r.yaml"),
            when: None,
            vars_map: None,
            vars: None,
            on_error: None,
        };
        let resolved = resolve_step_variables(&vars, &step).unwrap();
        assert_eq!(resolved, json!({"a": 1, "b": 2}));
    }

    #[test]
    fn resolve_vars_map_rename() {
        let vars = json!({"name": "Foo", "other": "bar"});
        let mut vm = IndexMap::new();
        vm.insert("name".into(), "model_name".into());
        let step = WorkflowStep {
            recipe: "r.yaml".into(),
            resolved_recipe: PathBuf::from("r.yaml"),
            when: None,
            vars_map: Some(vm),
            vars: None,
            on_error: None,
        };
        let resolved = resolve_step_variables(&vars, &step).unwrap();
        assert_eq!(resolved["model_name"], "Foo");
        assert!(resolved.get("name").is_none()); // original removed
        assert_eq!(resolved["other"], "bar");
    }

    #[test]
    fn resolve_vars_map_nonexistent_source_ignored() {
        let vars = json!({"a": 1});
        let mut vm = IndexMap::new();
        vm.insert("nonexistent".into(), "target".into());
        let step = WorkflowStep {
            recipe: "r.yaml".into(),
            resolved_recipe: PathBuf::from("r.yaml"),
            when: None,
            vars_map: Some(vm),
            vars: None,
            on_error: None,
        };
        let resolved = resolve_step_variables(&vars, &step).unwrap();
        assert_eq!(resolved, json!({"a": 1}));
    }

    #[test]
    fn resolve_vars_map_target_collision() {
        let vars = json!({"a": 1, "b": 2, "c": 3});
        let mut vm = IndexMap::new();
        vm.insert("a".into(), "c".into()); // a→c, overriding existing c=3
        let step = WorkflowStep {
            recipe: "r.yaml".into(),
            resolved_recipe: PathBuf::from("r.yaml"),
            when: None,
            vars_map: Some(vm),
            vars: None,
            on_error: None,
        };
        let resolved = resolve_step_variables(&vars, &step).unwrap();
        assert_eq!(resolved["c"], 1); // a's value overrides c
        assert!(resolved.get("a").is_none());
    }

    #[test]
    fn resolve_vars_map_simultaneous() {
        // a→b and b→c should NOT chain.
        let vars = json!({"a": "A", "b": "B"});
        let mut vm = IndexMap::new();
        vm.insert("a".into(), "b".into());
        vm.insert("b".into(), "c".into());
        let step = WorkflowStep {
            recipe: "r.yaml".into(),
            resolved_recipe: PathBuf::from("r.yaml"),
            when: None,
            vars_map: Some(vm),
            vars: None,
            on_error: None,
        };
        let resolved = resolve_step_variables(&vars, &step).unwrap();
        assert_eq!(resolved["b"], "A"); // original a value
        assert_eq!(resolved["c"], "B"); // original b value
        assert!(resolved.get("a").is_none());
    }

    #[test]
    fn resolve_vars_override() {
        let vars = json!({"a": 1, "b": 2});
        let mut overrides = IndexMap::new();
        overrides.insert("a".into(), json!(99));
        overrides.insert("new_key".into(), json!("hello"));
        let step = WorkflowStep {
            recipe: "r.yaml".into(),
            resolved_recipe: PathBuf::from("r.yaml"),
            when: None,
            vars_map: None,
            vars: Some(overrides),
            on_error: None,
        };
        let resolved = resolve_step_variables(&vars, &step).unwrap();
        assert_eq!(resolved["a"], 99);
        assert_eq!(resolved["b"], 2);
        assert_eq!(resolved["new_key"], "hello");
    }

    #[test]
    fn resolve_vars_map_then_override() {
        let vars = json!({"name": "Foo", "x": 1});
        let mut vm = IndexMap::new();
        vm.insert("name".into(), "model".into());
        let mut overrides = IndexMap::new();
        overrides.insert("model".into(), json!("Overridden"));
        let step = WorkflowStep {
            recipe: "r.yaml".into(),
            resolved_recipe: PathBuf::from("r.yaml"),
            when: None,
            vars_map: Some(vm),
            vars: Some(overrides),
            on_error: None,
        };
        let resolved = resolve_step_variables(&vars, &step).unwrap();
        assert_eq!(resolved["model"], "Overridden"); // override wins
        assert!(resolved.get("name").is_none());
    }

    // ── When evaluation ──

    #[test]
    fn when_truthy_values() {
        let vars = json!({});
        for expr in &["true", "yes", "1", "anything", "hello"] {
            assert!(
                evaluate_when(expr, &vars).unwrap(),
                "expected '{}' to be truthy",
                expr
            );
        }
    }

    #[test]
    fn when_falsy_values() {
        let vars = json!({});
        assert!(!evaluate_when("", &vars).unwrap());
        assert!(!evaluate_when("false", &vars).unwrap());
        assert!(!evaluate_when("False", &vars).unwrap());
        assert!(!evaluate_when("FALSE", &vars).unwrap());
        assert!(!evaluate_when("0", &vars).unwrap());
        assert!(!evaluate_when("  false  ", &vars).unwrap());
    }

    #[test]
    fn when_with_variable() {
        let vars = json!({"enabled": true});
        assert!(evaluate_when("{{ enabled }}", &vars).unwrap());

        let vars = json!({"enabled": false});
        assert!(!evaluate_when("{{ enabled }}", &vars).unwrap());
    }

    #[test]
    fn when_with_jinja2_control_flow() {
        let vars = json!({"methods": ["get", "post"]});
        assert!(evaluate_when(
            "{% if methods | length > 0 %}yes{% endif %}",
            &vars,
        ).unwrap());

        let vars = json!({"methods": []});
        assert!(!evaluate_when(
            "{% if methods | length > 0 %}yes{% endif %}",
            &vars,
        ).unwrap());
    }

    #[test]
    fn when_undefined_var_is_error() {
        let vars = json!({});
        let err = evaluate_when("{{ nonexistent }}", &vars).unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn when_syntax_error() {
        let vars = json!({});
        let err = evaluate_when("{% if unclosed %}", &vars).unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn when_no_when_is_unconditional() {
        // Just verifying the logic: if when is None, step executes.
        // This is tested at the execution level, but the function is clear.
        let vars = json!({"x": true});
        assert!(evaluate_when("{{ x }}", &vars).unwrap());
    }

    // ── Execution tests ──

    #[test]
    fn execute_empty_workflow() {
        let dir = TempDir::new().unwrap();
        let wf_path = create_workflow_yaml(dir.path(), "steps: []\n");
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), false, false);
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        assert!(result.steps.is_empty());
    }

    #[test]
    fn execute_single_step_success() {
        let dir = TempDir::new().unwrap();
        create_recipe_dir(dir.path(), "s1", "hello world");
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: s1/recipe.yaml\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), false, false);
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        assert_eq!(result.steps.len(), 1);
        assert!(matches!(&result.steps[0], StepResult::Success { .. }));
    }

    #[test]
    fn execute_conditional_skip() {
        let dir = TempDir::new().unwrap();
        create_recipe_dir(dir.path(), "s1", "content");
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: s1/recipe.yaml\n    when: \"false\"\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), false, false);
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        assert_eq!(result.steps.len(), 1);
        assert!(matches!(&result.steps[0], StepResult::Skipped { .. }));
    }

    #[test]
    fn execute_on_error_continue() {
        let dir = TempDir::new().unwrap();
        // Step 1: inject into nonexistent file (will fail).
        let s1_dir = dir.path().join("s1");
        fs::create_dir_all(&s1_dir).unwrap();
        fs::write(
            s1_dir.join("recipe.yaml"),
            "files:\n  - template: t.j2\n    inject: nonexistent.txt\n    append: true\n",
        )
        .unwrap();
        fs::write(s1_dir.join("t.j2"), "injected").unwrap();
        // Step 2: create a file (should still run).
        create_recipe_dir(dir.path(), "s2", "success");

        let wf_path = create_workflow_yaml(
            dir.path(),
            "on_error: continue\nsteps:\n  - recipe: s1/recipe.yaml\n  - recipe: s2/recipe.yaml\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), false, false);
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        assert_eq!(result.steps.len(), 2);
        assert!(result.steps[0].is_error());
        assert!(matches!(&result.steps[1], StepResult::Success { .. }));
    }

    #[test]
    fn execute_on_error_stop() {
        let dir = TempDir::new().unwrap();
        let s1_dir = dir.path().join("s1");
        fs::create_dir_all(&s1_dir).unwrap();
        fs::write(
            s1_dir.join("recipe.yaml"),
            "files:\n  - template: t.j2\n    inject: nonexistent.txt\n    append: true\n",
        )
        .unwrap();
        fs::write(s1_dir.join("t.j2"), "injected").unwrap();
        create_recipe_dir(dir.path(), "s2", "success");

        let wf_path = create_workflow_yaml(
            dir.path(),
            "on_error: stop\nsteps:\n  - recipe: s1/recipe.yaml\n  - recipe: s2/recipe.yaml\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), false, false);
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        // Should stop after first step failure.
        assert_eq!(result.steps.len(), 1);
        assert!(result.steps[0].is_error());
    }

    #[test]
    fn execute_per_step_on_error_override() {
        let dir = TempDir::new().unwrap();
        let s1_dir = dir.path().join("s1");
        fs::create_dir_all(&s1_dir).unwrap();
        fs::write(
            s1_dir.join("recipe.yaml"),
            "files:\n  - template: t.j2\n    inject: nonexistent.txt\n    append: true\n",
        )
        .unwrap();
        fs::write(s1_dir.join("t.j2"), "injected").unwrap();
        create_recipe_dir(dir.path(), "s2", "success");

        // Workflow is stop, but step 1 overrides to continue.
        let wf_path = create_workflow_yaml(
            dir.path(),
            "on_error: stop\nsteps:\n  - recipe: s1/recipe.yaml\n    on_error: continue\n  - recipe: s2/recipe.yaml\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), false, false);
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        // Step 1 fails but continues due to per-step override.
        assert_eq!(result.steps.len(), 2);
        assert!(result.steps[0].is_error());
        assert!(matches!(&result.steps[1], StepResult::Success { .. }));
    }

    #[test]
    fn execute_dry_run_cross_step() {
        let dir = TempDir::new().unwrap();
        // Step 1 creates a file.
        let s1_dir = dir.path().join("s1");
        fs::create_dir_all(&s1_dir).unwrap();
        fs::write(
            s1_dir.join("recipe.yaml"),
            "files:\n  - template: t.j2\n    to: shared.txt\n",
        )
        .unwrap();
        fs::write(s1_dir.join("t.j2"), "line one\n").unwrap();
        // Step 2 injects into it.
        let s2_dir = dir.path().join("s2");
        fs::create_dir_all(&s2_dir).unwrap();
        fs::write(
            s2_dir.join("recipe.yaml"),
            "files:\n  - template: t.j2\n    inject: shared.txt\n    append: true\n",
        )
        .unwrap();
        fs::write(s2_dir.join("t.j2"), "line two\n").unwrap();

        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: s1/recipe.yaml\n  - recipe: s2/recipe.yaml\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), true, false); // dry_run!
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        assert_eq!(result.steps.len(), 2);
        assert!(matches!(&result.steps[0], StepResult::Success { .. }));
        assert!(matches!(&result.steps[1], StepResult::Success { .. }));
        // Verify virtual_files has the combined content.
        let vf = ctx.virtual_files.get(&dir.path().join("shared.txt")).unwrap();
        assert!(vf.contains("line one"));
        assert!(vf.contains("line two"));
    }

    #[test]
    fn execute_chain_create_inject() {
        let dir = TempDir::new().unwrap();
        let s1_dir = dir.path().join("s1");
        fs::create_dir_all(&s1_dir).unwrap();
        fs::write(
            s1_dir.join("recipe.yaml"),
            "files:\n  - template: t.j2\n    to: target.txt\n",
        )
        .unwrap();
        fs::write(s1_dir.join("t.j2"), "original content\n").unwrap();

        let s2_dir = dir.path().join("s2");
        fs::create_dir_all(&s2_dir).unwrap();
        fs::write(
            s2_dir.join("recipe.yaml"),
            "files:\n  - template: t.j2\n    inject: target.txt\n    append: true\n",
        )
        .unwrap();
        fs::write(s2_dir.join("t.j2"), "injected line\n").unwrap();

        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: s1/recipe.yaml\n  - recipe: s2/recipe.yaml\n",
        );
        let wf = load_workflow(&wf_path).unwrap();
        let mut ctx = ExecutionContext::new(dir.path().to_path_buf(), false, false);
        let result = execute_workflow(&wf, json!({}), &mut ctx, false);
        assert_eq!(result.steps.len(), 2);
        assert!(matches!(&result.steps[0], StepResult::Success { .. }));
        assert!(matches!(&result.steps[1], StepResult::Success { .. }));
        // Verify file on disk.
        let content = fs::read_to_string(dir.path().join("target.txt")).unwrap();
        assert!(content.contains("original content"));
        assert!(content.contains("injected line"));
    }

    #[test]
    fn determinism_same_output() {
        let dir = TempDir::new().unwrap();
        create_recipe_dir(dir.path(), "s1", "deterministic output");
        let wf_path = create_workflow_yaml(
            dir.path(),
            "steps:\n  - recipe: s1/recipe.yaml\n",
        );

        let mut results = vec![];
        for _ in 0..3 {
            let work = TempDir::new().unwrap();
            let wf = load_workflow(&wf_path).unwrap();
            let mut ctx = ExecutionContext::new(work.path().to_path_buf(), true, false);
            let result = execute_workflow(&wf, json!({}), &mut ctx, false);
            results.push(format!("{:?}", result.steps.len()));
        }
        assert_eq!(results[0], results[1]);
        assert_eq!(results[1], results[2]);
    }
}
