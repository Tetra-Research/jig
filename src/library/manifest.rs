use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::{JigError, StructuredError};

/// A parsed jig library manifest (`jig-library.yaml`).
#[derive(Debug, Clone)]
pub struct LibraryManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub framework: Option<String>,
    pub language: Option<String>,
    pub conventions: IndexMap<String, String>,
    /// Recipe paths → descriptions.
    pub recipes: IndexMap<String, String>,
    /// Workflow definitions embedded in the manifest.
    pub workflows: IndexMap<String, ManifestWorkflow>,
    /// Directory containing the manifest file.
    pub library_dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields used in tests and future workflow generation
pub struct ManifestWorkflow {
    pub description: Option<String>,
    pub steps: Vec<ManifestWorkflowStep>,
    #[serde(default)]
    pub on_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields used in tests and future workflow generation
pub struct ManifestWorkflowStep {
    pub recipe: String,
    pub when: Option<String>,
    pub vars_map: Option<IndexMap<String, String>>,
    pub vars: Option<IndexMap<String, serde_json::Value>>,
    pub on_error: Option<String>,
}

// ── Raw deserialization ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawManifest {
    name: String,
    version: String,
    description: Option<String>,
    framework: Option<String>,
    language: Option<String>,
    #[serde(default)]
    conventions: IndexMap<String, String>,
    #[serde(default)]
    recipes: IndexMap<String, String>,
    #[serde(default)]
    workflows: IndexMap<String, ManifestWorkflow>,
}

impl LibraryManifest {
    /// Load and validate a library manifest from a `jig-library.yaml` file.
    pub fn load(path: &Path) -> Result<Self, JigError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            JigError::RecipeValidation(StructuredError {
                what: format!("cannot read library manifest '{}'", path.display()),
                where_: path.display().to_string(),
                why: e.to_string(),
                hint: "check the file path and permissions".into(),
            })
        })?;

        Self::parse(&content, path)
    }

    /// Parse manifest from YAML content with a source path for error messages.
    pub fn parse(content: &str, source_path: &Path) -> Result<Self, JigError> {
        let raw: RawManifest = serde_yaml::from_str(content).map_err(|e| {
            JigError::RecipeValidation(StructuredError {
                what: "invalid library manifest YAML".into(),
                where_: source_path.display().to_string(),
                why: e.to_string(),
                hint: "check the jig-library.yaml format".into(),
            })
        })?;

        // Validate required fields.
        if raw.name.is_empty() {
            return Err(JigError::RecipeValidation(StructuredError {
                what: "library name is required".into(),
                where_: source_path.display().to_string(),
                why: "the 'name' field is empty".into(),
                hint: "add a non-empty 'name' field to jig-library.yaml".into(),
            }));
        }

        if raw.version.is_empty() {
            return Err(JigError::RecipeValidation(StructuredError {
                what: "library version is required".into(),
                where_: source_path.display().to_string(),
                why: "the 'version' field is empty".into(),
                hint: "add a non-empty 'version' field to jig-library.yaml".into(),
            }));
        }

        // Validate that recipe paths in workflows reference declared recipes.
        for (wf_name, wf) in &raw.workflows {
            for step in &wf.steps {
                if !raw.recipes.contains_key(&step.recipe) {
                    return Err(JigError::RecipeValidation(StructuredError {
                        what: format!(
                            "workflow '{}' references undeclared recipe '{}'",
                            wf_name, step.recipe
                        ),
                        where_: source_path.display().to_string(),
                        why: format!(
                            "recipe '{}' is not listed in the 'recipes' block",
                            step.recipe
                        ),
                        hint: "add the recipe to the 'recipes' block or fix the reference".into(),
                    }));
                }
            }
        }

        let library_dir = source_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();

        Ok(LibraryManifest {
            name: raw.name,
            version: raw.version,
            description: raw.description,
            framework: raw.framework,
            language: raw.language,
            conventions: raw.conventions,
            recipes: raw.recipes,
            workflows: raw.workflows,
            library_dir,
        })
    }

    /// Resolve a recipe path (e.g., "model/add-field") to the absolute path
    /// of the recipe.yaml file within this library.
    pub fn resolve_recipe_path(&self, recipe_path: &str) -> Option<PathBuf> {
        if self.recipes.contains_key(recipe_path) {
            let full_path = self.library_dir.join(recipe_path).join("recipe.yaml");
            Some(full_path)
        } else {
            None
        }
    }

    /// Check if a recipe exists in this library.
    #[allow(dead_code)] // Used in tests
    pub fn has_recipe(&self, recipe_path: &str) -> bool {
        self.recipes.contains_key(recipe_path)
    }

    /// Check if a workflow exists in this library.
    pub fn has_workflow(&self, workflow_name: &str) -> bool {
        self.workflows.contains_key(workflow_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_path() -> PathBuf {
        PathBuf::from("/tmp/test-lib/jig-library.yaml")
    }

    #[test]
    fn parse_minimal_manifest() {
        let yaml = r#"
name: django
version: 0.1.0
recipes: {}
"#;
        let m = LibraryManifest::parse(yaml, &dummy_path()).unwrap();
        assert_eq!(m.name, "django");
        assert_eq!(m.version, "0.1.0");
        assert!(m.description.is_none());
        assert!(m.framework.is_none());
        assert!(m.language.is_none());
        assert!(m.conventions.is_empty());
        assert!(m.recipes.is_empty());
        assert!(m.workflows.is_empty());
    }

    #[test]
    fn parse_full_manifest() {
        let yaml = r#"
name: django
version: 0.3.0
description: Recipes for Django development
framework: django
language: python

conventions:
  models: "{{ app }}/models/{{ model | snakecase }}.py"
  services: "{{ app }}/services/{{ model | snakecase }}_service.py"

recipes:
  model/add-field: "Add a field to an existing Django model"
  model/add-model: "Scaffold a new Django model"
  service/add-method: "Add a method to a service"

workflows:
  add-field:
    description: "Add a field across the full stack"
    steps:
      - recipe: model/add-field
      - recipe: service/add-method
        when: "{{ update_service }}"
"#;
        let m = LibraryManifest::parse(yaml, &dummy_path()).unwrap();
        assert_eq!(m.name, "django");
        assert_eq!(m.version, "0.3.0");
        assert_eq!(m.description.as_deref(), Some("Recipes for Django development"));
        assert_eq!(m.framework.as_deref(), Some("django"));
        assert_eq!(m.language.as_deref(), Some("python"));
        assert_eq!(m.conventions.len(), 2);
        assert_eq!(m.recipes.len(), 3);
        assert_eq!(m.workflows.len(), 1);

        let wf = &m.workflows["add-field"];
        assert_eq!(wf.steps.len(), 2);
        assert_eq!(wf.steps[0].recipe, "model/add-field");
        assert!(wf.steps[0].when.is_none());
        assert_eq!(wf.steps[1].recipe, "service/add-method");
        assert!(wf.steps[1].when.is_some());
    }

    #[test]
    fn empty_name_errors() {
        let yaml = r#"
name: ""
version: 0.1.0
"#;
        let err = LibraryManifest::parse(yaml, &dummy_path()).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        let msg = format!("{err}");
        assert!(msg.contains("library name is required"));
    }

    #[test]
    fn empty_version_errors() {
        let yaml = r#"
name: django
version: ""
"#;
        let err = LibraryManifest::parse(yaml, &dummy_path()).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        let msg = format!("{err}");
        assert!(msg.contains("library version is required"));
    }

    #[test]
    fn workflow_references_undeclared_recipe() {
        let yaml = r#"
name: django
version: 0.1.0
recipes:
  model/add-field: "Add a field"
workflows:
  add-field:
    description: "Full stack field"
    steps:
      - recipe: model/add-field
      - recipe: nonexistent/recipe
"#;
        let err = LibraryManifest::parse(yaml, &dummy_path()).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        let msg = format!("{err}");
        assert!(msg.contains("undeclared recipe 'nonexistent/recipe'"));
    }

    #[test]
    fn resolve_recipe_path_found() {
        let yaml = r#"
name: django
version: 0.1.0
recipes:
  model/add-field: "Add a field"
"#;
        let m = LibraryManifest::parse(yaml, &dummy_path()).unwrap();
        let resolved = m.resolve_recipe_path("model/add-field").unwrap();
        assert!(resolved.ends_with("model/add-field/recipe.yaml"));
    }

    #[test]
    fn resolve_recipe_path_not_found() {
        let yaml = r#"
name: django
version: 0.1.0
recipes:
  model/add-field: "Add a field"
"#;
        let m = LibraryManifest::parse(yaml, &dummy_path()).unwrap();
        assert!(m.resolve_recipe_path("nonexistent").is_none());
    }

    #[test]
    fn invalid_yaml_errors() {
        let yaml = "not: valid: yaml: [[[";
        let err = LibraryManifest::parse(yaml, &dummy_path()).unwrap_err();
        assert_eq!(err.exit_code(), 1);
    }
}
