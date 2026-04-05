use std::path::Path;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::{JigError, StructuredError};
use crate::library::manifest::LibraryManifest;

/// Project-level configuration from `.jigrc.yaml`.
#[derive(Debug, Clone, Default)]
pub struct ProjectConfig {
    pub libraries: IndexMap<String, LibraryOverrides>,
}

/// Per-library overrides from `.jigrc.yaml`.
#[derive(Debug, Clone, Default)]
pub struct LibraryOverrides {
    pub conventions: IndexMap<String, String>,
}

// ── Raw deserialization ────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct RawProjectConfig {
    #[serde(default)]
    libraries: IndexMap<String, RawLibraryOverrides>,
}

#[derive(Debug, Deserialize, Default)]
struct RawLibraryOverrides {
    #[serde(default)]
    conventions: IndexMap<String, String>,
}

impl ProjectConfig {
    /// Load project config from `.jigrc.yaml` in the given directory.
    /// Returns a default (empty) config if the file doesn't exist.
    pub fn load(base_dir: &Path) -> Result<Self, JigError> {
        let config_path = base_dir.join(".jigrc.yaml");
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            JigError::RecipeValidation(StructuredError {
                what: format!("cannot read project config '{}'", config_path.display()),
                where_: config_path.display().to_string(),
                why: e.to_string(),
                hint: "check the .jigrc.yaml file".into(),
            })
        })?;

        let raw: RawProjectConfig = serde_yaml::from_str(&content).map_err(|e| {
            JigError::RecipeValidation(StructuredError {
                what: "invalid .jigrc.yaml".into(),
                where_: config_path.display().to_string(),
                why: e.to_string(),
                hint: "check the YAML syntax".into(),
            })
        })?;

        Ok(ProjectConfig {
            libraries: raw
                .libraries
                .into_iter()
                .map(|(name, overrides)| {
                    (
                        name,
                        LibraryOverrides {
                            conventions: overrides.conventions,
                        },
                    )
                })
                .collect(),
        })
    }
}

/// Resolve conventions for a library, applying project-level overrides.
///
/// Project overrides replace individual convention entries. Conventions
/// not overridden keep their library defaults.
pub fn resolve_conventions(
    manifest: &LibraryManifest,
    project_config: &ProjectConfig,
) -> IndexMap<String, String> {
    let mut conventions = manifest.conventions.clone();

    if let Some(overrides) = project_config.libraries.get(&manifest.name) {
        for (key, value) in &overrides.conventions {
            conventions.insert(key.clone(), value.clone());
        }
    }

    conventions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_missing_config_returns_default() {
        let tmp = TempDir::new().unwrap();
        let config = ProjectConfig::load(tmp.path()).unwrap();
        assert!(config.libraries.is_empty());
    }

    #[test]
    fn load_project_config() {
        let tmp = TempDir::new().unwrap();
        let config_content = r#"
libraries:
  django:
    conventions:
      models: "{{ app }}/models.py"
      services: "{{ app }}/domain/services.py"
"#;
        fs::write(tmp.path().join(".jigrc.yaml"), config_content).unwrap();

        let config = ProjectConfig::load(tmp.path()).unwrap();
        assert_eq!(config.libraries.len(), 1);
        let django = &config.libraries["django"];
        assert_eq!(django.conventions.len(), 2);
        assert_eq!(
            django.conventions["models"],
            "{{ app }}/models.py"
        );
    }

    #[test]
    fn invalid_config_errors() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".jigrc.yaml"), "not: valid: yaml: [[[").unwrap();
        let err = ProjectConfig::load(tmp.path()).unwrap_err();
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn resolve_conventions_no_overrides() {
        let manifest = LibraryManifest {
            name: "django".into(),
            version: "0.1.0".into(),
            description: None,
            framework: None,
            language: None,
            conventions: IndexMap::from([
                ("models".into(), "{{ app }}/models/{{ model }}.py".into()),
                ("services".into(), "{{ app }}/services/{{ model }}_service.py".into()),
            ]),
            recipes: IndexMap::new(),
            workflows: IndexMap::new(),
            library_dir: "/tmp".into(),
        };

        let config = ProjectConfig::default();
        let resolved = resolve_conventions(&manifest, &config);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved["models"], "{{ app }}/models/{{ model }}.py");
    }

    #[test]
    fn resolve_conventions_with_overrides() {
        let manifest = LibraryManifest {
            name: "django".into(),
            version: "0.1.0".into(),
            description: None,
            framework: None,
            language: None,
            conventions: IndexMap::from([
                ("models".into(), "{{ app }}/models/{{ model }}.py".into()),
                ("services".into(), "{{ app }}/services/{{ model }}_service.py".into()),
            ]),
            recipes: IndexMap::new(),
            workflows: IndexMap::new(),
            library_dir: "/tmp".into(),
        };

        let config = ProjectConfig {
            libraries: IndexMap::from([(
                "django".into(),
                LibraryOverrides {
                    conventions: IndexMap::from([
                        ("models".into(), "{{ app }}/models.py".into()),
                    ]),
                },
            )]),
        };

        let resolved = resolve_conventions(&manifest, &config);
        assert_eq!(resolved.len(), 2);
        // Overridden:
        assert_eq!(resolved["models"], "{{ app }}/models.py");
        // Kept from library default:
        assert_eq!(
            resolved["services"],
            "{{ app }}/services/{{ model }}_service.py"
        );
    }

    #[test]
    fn resolve_conventions_adds_new_keys() {
        let manifest = LibraryManifest {
            name: "django".into(),
            version: "0.1.0".into(),
            description: None,
            framework: None,
            language: None,
            conventions: IndexMap::from([
                ("models".into(), "{{ app }}/models.py".into()),
            ]),
            recipes: IndexMap::new(),
            workflows: IndexMap::new(),
            library_dir: "/tmp".into(),
        };

        let config = ProjectConfig {
            libraries: IndexMap::from([(
                "django".into(),
                LibraryOverrides {
                    conventions: IndexMap::from([
                        ("custom".into(), "{{ app }}/custom/{{ model }}.py".into()),
                    ]),
                },
            )]),
        };

        let resolved = resolve_conventions(&manifest, &config);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved["custom"], "{{ app }}/custom/{{ model }}.py");
    }
}
