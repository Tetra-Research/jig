use std::path::Path;

use crate::error::{JigError, StructuredError};
use crate::library::install;
use crate::library::manifest::LibraryManifest;
use crate::recipe::Recipe;

/// Information about a recipe within a library.
#[derive(Debug, Clone)]
pub struct RecipeInfo {
    pub library: String,
    pub path: String,
    pub description: String,
    pub variables: Vec<RecipeVarInfo>,
    pub operations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RecipeVarInfo {
    pub name: String,
    pub var_type: String,
    pub required: bool,
    pub description: Option<String>,
}

/// Information about a workflow within a library.
#[derive(Debug, Clone)]
pub struct WorkflowInfo {
    #[allow(dead_code)] // Used in JSON output serialization
    pub library: String,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStepInfo>,
}

#[derive(Debug, Clone)]
pub struct WorkflowStepInfo {
    pub recipe: String,
    pub conditional: bool,
}

/// List all recipes in an installed library.
pub fn list_recipes(library_name: &str, base_dir: &Path) -> Result<Vec<(String, String)>, JigError> {
    let manifest = install::load_installed_manifest(library_name, base_dir)?;
    Ok(manifest
        .recipes
        .into_iter()
        .collect())
}

/// Get detailed info about a specific recipe in a library.
pub fn recipe_info(
    library_name: &str,
    recipe_path: &str,
    base_dir: &Path,
) -> Result<RecipeInfo, JigError> {
    let manifest = install::load_installed_manifest(library_name, base_dir)?;

    let description = manifest
        .recipes
        .get(recipe_path)
        .ok_or_else(|| {
            JigError::RecipeValidation(StructuredError {
                what: format!(
                    "recipe '{recipe_path}' not found in library '{library_name}'"
                ),
                where_: library_name.to_string(),
                why: format!("the library does not declare a recipe at '{recipe_path}'"),
                hint: format!(
                    "use 'jig library recipes {library_name}' to see available recipes"
                ),
            })
        })?
        .clone();

    // Try to load the actual recipe.yaml for detailed info.
    let recipe_yaml_path = manifest
        .resolve_recipe_path(recipe_path)
        .unwrap();

    let (variables, operations) = if recipe_yaml_path.exists() {
        match Recipe::load(&recipe_yaml_path) {
            Ok(recipe) => {
                let vars: Vec<RecipeVarInfo> = recipe
                    .variables
                    .iter()
                    .map(|(name, decl)| RecipeVarInfo {
                        name: name.clone(),
                        var_type: decl.var_type.to_string(),
                        required: decl.required,
                        description: decl.description.clone(),
                    })
                    .collect();
                let ops: Vec<String> = recipe
                    .files
                    .iter()
                    .map(|op| op.op_type_str().to_string())
                    .collect();
                (vars, ops)
            }
            Err(_) => (Vec::new(), Vec::new()),
        }
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(RecipeInfo {
        library: library_name.to_string(),
        path: recipe_path.to_string(),
        description,
        variables,
        operations,
    })
}

/// List all workflows in an installed library.
pub fn list_workflows(
    library_name: &str,
    base_dir: &Path,
) -> Result<Vec<WorkflowInfo>, JigError> {
    let manifest = install::load_installed_manifest(library_name, base_dir)?;

    Ok(manifest
        .workflows
        .iter()
        .map(|(name, wf)| WorkflowInfo {
            library: library_name.to_string(),
            name: name.clone(),
            description: wf.description.clone(),
            steps: wf
                .steps
                .iter()
                .map(|s| WorkflowStepInfo {
                    recipe: s.recipe.clone(),
                    conditional: s.when.is_some(),
                })
                .collect(),
        })
        .collect())
}

/// Resolve a library-qualified recipe path (e.g., "django/model/add-field")
/// to the absolute path of the recipe.yaml file.
///
/// Returns (library_name, recipe_path_within_library, absolute_recipe_yaml_path).
#[allow(dead_code)] // Used in tests and future library-aware run command
pub fn resolve_library_recipe(
    qualified_path: &str,
    base_dir: &Path,
) -> Result<(String, String, std::path::PathBuf), JigError> {
    // Split "library_name/recipe/path" — the first segment is the library name.
    let slash_pos = qualified_path.find('/').ok_or_else(|| {
        JigError::RecipeValidation(StructuredError {
            what: format!("invalid library recipe path '{qualified_path}'"),
            where_: qualified_path.to_string(),
            why: "expected format: <library>/<recipe-path>".into(),
            hint: "example: django/model/add-field".into(),
        })
    })?;

    let library_name = &qualified_path[..slash_pos];
    let recipe_path = &qualified_path[slash_pos + 1..];

    let manifest = install::load_installed_manifest(library_name, base_dir)?;

    let resolved = manifest.resolve_recipe_path(recipe_path).ok_or_else(|| {
        let available: Vec<&str> = manifest.recipes.keys().map(|s| s.as_str()).collect();
        let hint = if available.is_empty() {
            format!("library '{library_name}' has no recipes")
        } else {
            format!(
                "available recipes: {}",
                available.join(", ")
            )
        };
        JigError::RecipeValidation(StructuredError {
            what: format!(
                "recipe '{recipe_path}' not found in library '{library_name}'"
            ),
            where_: qualified_path.to_string(),
            why: format!("the library does not declare recipe '{recipe_path}'"),
            hint,
        })
    })?;

    Ok((library_name.to_string(), recipe_path.to_string(), resolved))
}

/// Resolve a library-qualified workflow name (e.g., "django/add-field").
///
/// Returns the manifest so the caller can use workflow metadata.
#[allow(dead_code)] // Used in tests and future library-aware workflow command
pub fn resolve_library_workflow(
    qualified_name: &str,
    base_dir: &Path,
) -> Result<(String, String, LibraryManifest), JigError> {
    let slash_pos = qualified_name.find('/').ok_or_else(|| {
        JigError::RecipeValidation(StructuredError {
            what: format!("invalid library workflow path '{qualified_name}'"),
            where_: qualified_name.to_string(),
            why: "expected format: <library>/<workflow-name>".into(),
            hint: "example: django/add-field".into(),
        })
    })?;

    let library_name = &qualified_name[..slash_pos];
    let workflow_name = &qualified_name[slash_pos + 1..];

    let manifest = install::load_installed_manifest(library_name, base_dir)?;

    if !manifest.has_workflow(workflow_name) {
        let available: Vec<&str> = manifest.workflows.keys().map(|s| s.as_str()).collect();
        let hint = if available.is_empty() {
            format!("library '{library_name}' has no workflows")
        } else {
            format!("available workflows: {}", available.join(", "))
        };
        return Err(JigError::RecipeValidation(StructuredError {
            what: format!(
                "workflow '{workflow_name}' not found in library '{library_name}'"
            ),
            where_: qualified_name.to_string(),
            why: format!("the library does not declare workflow '{workflow_name}'"),
            hint,
        }));
    }

    Ok((
        library_name.to_string(),
        workflow_name.to_string(),
        manifest,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_library(tmp: &TempDir) -> std::path::PathBuf {
        let project_dir = tmp.path().join("project");
        let lib_dir = project_dir.join(".jig/libraries/testlib");
        fs::create_dir_all(&lib_dir).unwrap();

        let manifest = r#"
name: testlib
version: 0.1.0
description: Test library

conventions:
  models: "{{ app }}/models.py"

recipes:
  model/add-field: "Add a field"
  model/add-model: "Add a model"

workflows:
  add-field:
    description: "Full stack field addition"
    steps:
      - recipe: model/add-field
      - recipe: model/add-model
        when: "{{ create_model }}"
"#;
        fs::write(lib_dir.join("jig-library.yaml"), manifest).unwrap();

        // Create a recipe.yaml for model/add-field.
        let recipe_dir = lib_dir.join("model/add-field");
        fs::create_dir_all(&recipe_dir).unwrap();
        let recipe = r#"
name: add-field
variables:
  field_name:
    type: string
    required: true
  field_type:
    type: string
    required: true
    description: "The Django field type"
files: []
"#;
        fs::write(recipe_dir.join("recipe.yaml"), recipe).unwrap();

        project_dir
    }

    #[test]
    fn list_recipes_from_library() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let recipes = list_recipes("testlib", &project_dir).unwrap();
        assert_eq!(recipes.len(), 2);
        assert_eq!(recipes[0].0, "model/add-field");
        assert_eq!(recipes[0].1, "Add a field");
    }

    #[test]
    fn get_recipe_info() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let info = recipe_info("testlib", "model/add-field", &project_dir).unwrap();
        assert_eq!(info.library, "testlib");
        assert_eq!(info.path, "model/add-field");
        assert_eq!(info.description, "Add a field");
        assert_eq!(info.variables.len(), 2);
        assert_eq!(info.variables[0].name, "field_name");
        assert!(info.variables[0].required);
    }

    #[test]
    fn recipe_info_not_found() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let err = recipe_info("testlib", "nonexistent", &project_dir).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
    }

    #[test]
    fn list_library_workflows() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let workflows = list_workflows("testlib", &project_dir).unwrap();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].name, "add-field");
        assert_eq!(workflows[0].steps.len(), 2);
        assert!(!workflows[0].steps[0].conditional);
        assert!(workflows[0].steps[1].conditional);
    }

    #[test]
    fn resolve_library_recipe_path() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let (lib, recipe, path) =
            resolve_library_recipe("testlib/model/add-field", &project_dir).unwrap();
        assert_eq!(lib, "testlib");
        assert_eq!(recipe, "model/add-field");
        assert!(path.ends_with("model/add-field/recipe.yaml"));
    }

    #[test]
    fn resolve_library_recipe_not_found() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let err = resolve_library_recipe("testlib/nonexistent", &project_dir).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
    }

    #[test]
    fn resolve_library_workflow_found() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let (lib, wf, _manifest) =
            resolve_library_workflow("testlib/add-field", &project_dir).unwrap();
        assert_eq!(lib, "testlib");
        assert_eq!(wf, "add-field");
    }

    #[test]
    fn resolve_library_workflow_not_found() {
        let tmp = TempDir::new().unwrap();
        let project_dir = setup_library(&tmp);

        let err = resolve_library_workflow("testlib/nonexistent", &project_dir).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
    }
}
