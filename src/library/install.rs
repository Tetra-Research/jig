use std::path::{Path, PathBuf};

use crate::error::{JigError, StructuredError};
use crate::library::manifest::LibraryManifest;

/// Information about an installed library.
#[derive(Debug, Clone)]
pub struct InstalledLibrary {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub framework: Option<String>,
    pub language: Option<String>,
    pub path: PathBuf,
    pub location: InstallLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallLocation {
    Global,
    ProjectLocal,
}

impl std::fmt::Display for InstallLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallLocation::Global => write!(f, "global"),
            InstallLocation::ProjectLocal => write!(f, "project"),
        }
    }
}

/// Return the global libraries directory (`~/.jig/libraries/`).
pub fn global_libraries_dir() -> Result<PathBuf, JigError> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| {
            JigError::FileOperation(StructuredError {
                what: "cannot determine home directory".into(),
                where_: "environment".into(),
                why: "neither HOME nor USERPROFILE is set".into(),
                hint: "set the HOME environment variable".into(),
            })
        })?;
    Ok(PathBuf::from(home).join(".jig").join("libraries"))
}

/// Return the project-local libraries directory (`.jig/libraries/`).
pub fn project_libraries_dir(base_dir: &Path) -> PathBuf {
    base_dir.join(".jig").join("libraries")
}

/// Install a library from a local directory path.
///
/// Copies the library directory to the target location (global or project-local).
/// Validates the manifest before installing.
pub fn add_from_path(
    source: &Path,
    location: InstallLocation,
    base_dir: &Path,
) -> Result<InstalledLibrary, JigError> {
    // Validate source exists and has a manifest.
    let manifest_path = source.join("jig-library.yaml");
    if !manifest_path.exists() {
        return Err(JigError::RecipeValidation(StructuredError {
            what: format!("no jig-library.yaml found in '{}'", source.display()),
            where_: source.display().to_string(),
            why: "the directory does not contain a library manifest".into(),
            hint: "ensure the path points to a jig library with a jig-library.yaml file".into(),
        }));
    }

    let manifest = LibraryManifest::load(&manifest_path)?;

    // Determine target directory.
    let target_dir = match location {
        InstallLocation::Global => global_libraries_dir()?,
        InstallLocation::ProjectLocal => project_libraries_dir(base_dir),
    };

    let install_dir = target_dir.join(&manifest.name);

    // Check if already installed.
    if install_dir.exists() {
        return Err(JigError::FileOperation(StructuredError {
            what: format!("library '{}' is already installed", manifest.name),
            where_: install_dir.display().to_string(),
            why: "a library with this name already exists at the target location".into(),
            hint: "use 'jig library update' to update, or 'jig library remove' first".into(),
        }));
    }

    // Create target directory and copy.
    copy_dir_recursive(source, &install_dir).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!("failed to install library '{}'", manifest.name),
            where_: install_dir.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions".into(),
        })
    })?;

    Ok(InstalledLibrary {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        framework: manifest.framework,
        language: manifest.language,
        path: install_dir,
        location,
    })
}

/// Remove an installed library by name.
///
/// Searches project-local first, then global.
pub fn remove(name: &str, base_dir: &Path) -> Result<InstalledLibrary, JigError> {
    let (path, location) = find_installed_library(name, base_dir)?;

    let manifest_path = path.join("jig-library.yaml");
    let manifest = LibraryManifest::load(&manifest_path)?;

    std::fs::remove_dir_all(&path).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!("failed to remove library '{name}'"),
            where_: path.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions".into(),
        })
    })?;

    Ok(InstalledLibrary {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        framework: manifest.framework,
        language: manifest.language,
        path,
        location,
    })
}

/// Update an installed library by replacing it from a source path.
pub fn update_from_path(
    name: &str,
    source: &Path,
    base_dir: &Path,
) -> Result<InstalledLibrary, JigError> {
    let (existing_path, location) = find_installed_library(name, base_dir)?;

    // Validate the source.
    let manifest_path = source.join("jig-library.yaml");
    if !manifest_path.exists() {
        return Err(JigError::RecipeValidation(StructuredError {
            what: format!("no jig-library.yaml found in '{}'", source.display()),
            where_: source.display().to_string(),
            why: "the directory does not contain a library manifest".into(),
            hint: "ensure the path points to a jig library with a jig-library.yaml file".into(),
        }));
    }

    let manifest = LibraryManifest::load(&manifest_path)?;

    // Remove old, copy new.
    std::fs::remove_dir_all(&existing_path).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!("failed to remove old version of library '{name}'"),
            where_: existing_path.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions".into(),
        })
    })?;

    copy_dir_recursive(source, &existing_path).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!("failed to update library '{name}'"),
            where_: existing_path.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions".into(),
        })
    })?;

    Ok(InstalledLibrary {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        framework: manifest.framework,
        language: manifest.language,
        path: existing_path,
        location,
    })
}

/// List all installed libraries (project-local first, then global).
/// Project-local libraries shadow global ones with the same name.
pub fn list_installed(base_dir: &Path) -> Result<Vec<InstalledLibrary>, JigError> {
    let mut libraries = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    // Project-local first (takes precedence).
    let project_dir = project_libraries_dir(base_dir);
    if project_dir.is_dir() {
        scan_libraries_dir(&project_dir, InstallLocation::ProjectLocal, &mut libraries, &mut seen_names)?;
    }

    // Then global.
    if let Ok(global_dir) = global_libraries_dir()
        && global_dir.is_dir()
    {
        scan_libraries_dir(&global_dir, InstallLocation::Global, &mut libraries, &mut seen_names)?;
    }

    Ok(libraries)
}

/// Find an installed library by name.
/// Project-local takes precedence over global.
pub fn find_installed_library(name: &str, base_dir: &Path) -> Result<(PathBuf, InstallLocation), JigError> {
    // Check project-local first.
    let project_path = project_libraries_dir(base_dir).join(name);
    if project_path.join("jig-library.yaml").exists() {
        return Ok((project_path, InstallLocation::ProjectLocal));
    }

    // Then global.
    if let Ok(global_dir) = global_libraries_dir() {
        let global_path = global_dir.join(name);
        if global_path.join("jig-library.yaml").exists() {
            return Ok((global_path, InstallLocation::Global));
        }
    }

    Err(JigError::RecipeValidation(StructuredError {
        what: format!("library '{name}' is not installed"),
        where_: "libraries".into(),
        why: format!("no library named '{name}' found in project or global libraries"),
        hint: "use 'jig library list' to see installed libraries, or 'jig library add' to install".into(),
    }))
}

/// Load the manifest for an installed library by name.
pub fn load_installed_manifest(name: &str, base_dir: &Path) -> Result<LibraryManifest, JigError> {
    let (path, _) = find_installed_library(name, base_dir)?;
    LibraryManifest::load(&path.join("jig-library.yaml"))
}

// ── Helpers ────────────────────────────────────────────────────────

fn scan_libraries_dir(
    dir: &Path,
    location: InstallLocation,
    libraries: &mut Vec<InstalledLibrary>,
    seen: &mut std::collections::HashSet<String>,
) -> Result<(), JigError> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!("cannot read libraries directory '{}'", dir.display()),
            where_: dir.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions".into(),
        })
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            JigError::FileOperation(StructuredError {
                what: "cannot read directory entry".into(),
                where_: dir.display().to_string(),
                why: e.to_string(),
                hint: "check directory permissions".into(),
            })
        })?;

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("jig-library.yaml");
        if !manifest_path.exists() {
            continue;
        }

        let manifest = match LibraryManifest::load(&manifest_path) {
            Ok(m) => m,
            Err(_) => continue, // Skip malformed manifests in listing
        };

        if seen.contains(&manifest.name) {
            continue; // Already seen from a higher-precedence location
        }

        seen.insert(manifest.name.clone());
        libraries.push(InstalledLibrary {
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            framework: manifest.framework,
            language: manifest.language,
            path,
            location,
        });
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_library(dir: &Path, name: &str, version: &str) {
        let lib_dir = dir.join(name);
        fs::create_dir_all(&lib_dir).unwrap();
        let manifest = format!(
            r#"name: {name}
version: {version}
description: Test library
recipes:
  model/add-field: "Add a field"
"#
        );
        fs::write(lib_dir.join("jig-library.yaml"), manifest).unwrap();
        let recipe_dir = lib_dir.join("model/add-field/templates");
        fs::create_dir_all(&recipe_dir).unwrap();
        fs::write(
            lib_dir.join("model/add-field/recipe.yaml"),
            "name: add-field\nfiles: []\n",
        )
        .unwrap();
    }

    #[test]
    fn add_from_local_path() {
        let tmp = TempDir::new().unwrap();
        let source_dir = tmp.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(
            source_dir.join("jig-library.yaml"),
            "name: mylib\nversion: 0.1.0\nrecipes: {}\n",
        )
        .unwrap();

        let project_dir = tmp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();

        let result = add_from_path(&source_dir, InstallLocation::ProjectLocal, &project_dir).unwrap();
        assert_eq!(result.name, "mylib");
        assert_eq!(result.version, "0.1.0");
        assert_eq!(result.location, InstallLocation::ProjectLocal);
        assert!(result.path.join("jig-library.yaml").exists());
    }

    #[test]
    fn add_already_installed_errors() {
        let tmp = TempDir::new().unwrap();
        let source_dir = tmp.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(
            source_dir.join("jig-library.yaml"),
            "name: mylib\nversion: 0.1.0\nrecipes: {}\n",
        )
        .unwrap();

        let project_dir = tmp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();

        add_from_path(&source_dir, InstallLocation::ProjectLocal, &project_dir).unwrap();
        let err = add_from_path(&source_dir, InstallLocation::ProjectLocal, &project_dir).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("already installed"));
    }

    #[test]
    fn add_missing_manifest_errors() {
        let tmp = TempDir::new().unwrap();
        let empty_dir = tmp.path().join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        let project_dir = tmp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();

        let err = add_from_path(&empty_dir, InstallLocation::ProjectLocal, &project_dir).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("no jig-library.yaml"));
    }

    #[test]
    fn remove_installed_library() {
        let tmp = TempDir::new().unwrap();
        let source_dir = tmp.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(
            source_dir.join("jig-library.yaml"),
            "name: mylib\nversion: 0.1.0\nrecipes: {}\n",
        )
        .unwrap();

        let project_dir = tmp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();

        add_from_path(&source_dir, InstallLocation::ProjectLocal, &project_dir).unwrap();
        let removed = remove("mylib", &project_dir).unwrap();
        assert_eq!(removed.name, "mylib");
        assert!(!removed.path.exists());
    }

    #[test]
    fn remove_not_installed_errors() {
        let tmp = TempDir::new().unwrap();
        let err = remove("nonexistent", tmp.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not installed"));
    }

    #[test]
    fn list_installed_libraries() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();

        // Install two libraries.
        for name in &["lib-a", "lib-b"] {
            let source = tmp.path().join(format!("source-{name}"));
            fs::create_dir_all(&source).unwrap();
            fs::write(
                source.join("jig-library.yaml"),
                format!("name: {name}\nversion: 0.1.0\nrecipes: {{}}\n"),
            )
            .unwrap();
            add_from_path(&source, InstallLocation::ProjectLocal, &project_dir).unwrap();
        }

        let list = list_installed(&project_dir).unwrap();
        assert_eq!(list.len(), 2);
        let names: Vec<&str> = list.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"lib-a"));
        assert!(names.contains(&"lib-b"));
    }

    #[test]
    fn update_installed_library() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();

        // Install v1.
        let source_v1 = tmp.path().join("source-v1");
        fs::create_dir_all(&source_v1).unwrap();
        fs::write(
            source_v1.join("jig-library.yaml"),
            "name: mylib\nversion: 0.1.0\nrecipes: {}\n",
        )
        .unwrap();
        add_from_path(&source_v1, InstallLocation::ProjectLocal, &project_dir).unwrap();

        // Update with v2 source.
        let source_v2 = tmp.path().join("source-v2");
        fs::create_dir_all(&source_v2).unwrap();
        fs::write(
            source_v2.join("jig-library.yaml"),
            "name: mylib\nversion: 0.2.0\nrecipes: {}\n",
        )
        .unwrap();

        let updated = update_from_path("mylib", &source_v2, &project_dir).unwrap();
        assert_eq!(updated.version, "0.2.0");
    }

    #[test]
    fn project_local_shadows_global() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();

        // Create a project-local library.
        let local_lib_dir = project_libraries_dir(&project_dir).join("mylib");
        fs::create_dir_all(&local_lib_dir).unwrap();
        fs::write(
            local_lib_dir.join("jig-library.yaml"),
            "name: mylib\nversion: 0.2.0\nrecipes: {}\n",
        )
        .unwrap();

        let (path, location) = find_installed_library("mylib", &project_dir).unwrap();
        assert_eq!(location, InstallLocation::ProjectLocal);
        assert_eq!(path, local_lib_dir);
    }
}
