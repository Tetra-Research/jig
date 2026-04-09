mod error;
mod filters;
mod library;
mod operations;
mod output;
mod prepare;
mod recipe;
mod renderer;
mod scope;
mod variables;
mod workflow;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use crate::error::JigError;
use crate::prepare::prepare_operations;
use crate::recipe::Recipe;

#[derive(Parser)]
#[command(
    name = "jig",
    version,
    about = "Template rendering CLI for LLM code generation workflows"
)]
struct Cli {
    /// Inline variables as JSON string
    #[arg(long, global = true, conflicts_with = "json_args")]
    vars: Option<String>,

    /// Inline variables as JSON string (alias of --vars)
    #[arg(long = "json-args", global = true, conflicts_with = "vars")]
    json_args: Option<String>,

    /// Variables from a JSON file
    #[arg(long, global = true)]
    vars_file: Option<PathBuf>,

    /// Read variables from stdin
    #[arg(long, global = true)]
    vars_stdin: bool,

    /// Preview without writing files
    #[arg(long, global = true)]
    dry_run: bool,

    /// Force JSON output to stdout
    #[arg(long, global = true)]
    json: bool,

    /// Suppress stderr output
    #[arg(long, global = true)]
    quiet: bool,

    /// Overwrite existing files
    #[arg(long, global = true)]
    force: bool,

    /// Resolve output paths from this directory
    #[arg(long, global = true)]
    base_dir: Option<PathBuf>,

    /// Include rendered content in output
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a recipe and report whether it is valid
    Validate {
        /// Path to the recipe YAML file
        recipe: PathBuf,
    },
    /// Show expected variables for a recipe as JSON
    Vars {
        /// Path to the recipe YAML file
        recipe: PathBuf,
    },
    /// Render a template with variables
    Render {
        /// Path to the template file
        template: PathBuf,
        /// Write output to a file instead of stdout
        #[arg(long)]
        to: Option<PathBuf>,
    },
    /// Execute all file operations in a recipe
    Run {
        /// Path to the recipe YAML file
        recipe: PathBuf,
    },
    /// Execute a multi-recipe workflow
    Workflow {
        /// Path to the workflow YAML file
        path: PathBuf,
    },
    /// List available skills and recipes
    List {
        /// List agent skills (scans for skills/ directories)
        #[arg(long)]
        skills: bool,
        /// Scan .claude/skills/ (implies --skills)
        #[arg(long)]
        claude: bool,
        /// Scan .codex/skills/ (implies --skills)
        #[arg(long)]
        codex: bool,
    },
    /// Manage recipe libraries
    Library {
        #[command(subcommand)]
        action: LibraryAction,
    },
}

#[derive(Subcommand)]
enum LibraryAction {
    /// Install a library from a local directory or git URL
    Add {
        /// Path to the library directory or git URL
        source: String,
        /// Install globally instead of project-local
        #[arg(long)]
        global: bool,
        /// Overwrite existing library with same name
        #[arg(long)]
        force: bool,
    },
    /// Remove an installed library
    Remove {
        /// Library name
        name: String,
    },
    /// Update an installed library from source (or re-fetch from original)
    Update {
        /// Library name
        name: String,
        /// Path to the updated library directory (optional if metadata exists)
        source: Option<String>,
    },
    /// List installed libraries
    List,
    /// List all recipes in a library
    Recipes {
        /// Library name
        name: String,
    },
    /// Show details for a specific recipe
    Info {
        /// Library-qualified recipe path (e.g., django/model/add-field)
        path: String,
    },
    /// List all workflows in a library
    Workflows {
        /// Library name
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = run(cli);
    match result {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("{e}");
            process::exit(e.exit_code());
        }
    }
}

fn run(cli: Cli) -> Result<i32, JigError> {
    let base_dir = cli
        .base_dir
        .as_deref()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let inline_vars_owned = cli.vars.clone().or(cli.json_args.clone());

    match cli.command {
        Commands::Validate { recipe } => {
            let resolved = resolve_recipe_or_library(&recipe, &base_dir);
            cmd_validate(
                &resolved.path,
                inline_vars_owned.as_deref(),
                cli.vars_file.as_deref(),
                cli.vars_stdin,
                cli.json,
                &base_dir,
                resolved.library_name.as_deref(),
                resolved.recipe_path.as_deref(),
            )
        }
        Commands::Vars { recipe } => {
            let resolved = resolve_recipe_or_library(&recipe, &base_dir);
            cmd_vars(&resolved.path, &base_dir)
        }
        Commands::Render { template, to } => cmd_render(
            &template,
            to.as_deref(),
            inline_vars_owned.as_deref(),
            cli.vars_file.as_deref(),
            cli.vars_stdin,
        ),
        Commands::Run { recipe } => {
            let resolved = resolve_recipe_or_library(&recipe, &base_dir);
            cmd_run(
                &resolved.path,
                inline_vars_owned.as_deref(),
                cli.vars_file.as_deref(),
                cli.vars_stdin,
                cli.dry_run,
                cli.json,
                cli.quiet,
                cli.force,
                cli.base_dir.as_deref(),
                cli.verbose,
                &base_dir,
                resolved.library_name.as_deref(),
                resolved.recipe_path.as_deref(),
            )
        }
        Commands::Workflow { path } => cmd_workflow(
            &path,
            inline_vars_owned.as_deref(),
            cli.vars_file.as_deref(),
            cli.vars_stdin,
            cli.dry_run,
            cli.json,
            cli.quiet,
            cli.force,
            cli.base_dir.as_deref(),
            cli.verbose,
            &base_dir,
        ),
        Commands::List {
            skills,
            claude,
            codex,
        } => cmd_list(skills, claude, codex, &base_dir, cli.json, cli.quiet),
        Commands::Library { action } => cmd_library(action, &base_dir, cli.json, cli.quiet),
    }
}

/// Result of resolving a recipe path — either a filesystem path or a library recipe.
struct ResolvedRecipe {
    path: PathBuf,
    /// If this recipe came from a library, the library name.
    library_name: Option<String>,
    /// If this recipe came from a library, the recipe path within the library.
    recipe_path: Option<String>,
}

/// Resolve a recipe/workflow path: if the path doesn't exist as a file and looks
/// like a library-namespaced path (contains '/'), try to resolve via installed library.
/// Falls back to the original path for filesystem paths (backward compat: AC-4.5, AC-N2.1).
fn resolve_recipe_or_library(path: &std::path::Path, base_dir: &std::path::Path) -> ResolvedRecipe {
    // Filesystem paths take precedence (AC-N2.1).
    if path.exists() {
        return ResolvedRecipe {
            path: path.to_path_buf(),
            library_name: None,
            recipe_path: None,
        };
    }

    let path_str = path.to_string_lossy();
    if let Some(slash) = path_str.find('/') {
        let lib_name = &path_str[..slash];
        let recipe_path = &path_str[slash + 1..];
        // Check if this matches an installed library.
        if library::install::find_installed_library(lib_name, base_dir).is_ok() {
            // Try to resolve as a library recipe first (library takes precedence: AC-8.3).
            if let Ok((lib, rp, resolved)) =
                library::discover::resolve_library_recipe(&path_str, base_dir)
            {
                return ResolvedRecipe {
                    path: resolved,
                    library_name: Some(lib),
                    recipe_path: Some(rp),
                };
            }

            // Fall back to extension recipe (AC-8.2).
            let ext_recipe = base_dir
                .join(".jig/extensions")
                .join(lib_name)
                .join(recipe_path)
                .join("recipe.yaml");
            if ext_recipe.exists() {
                return ResolvedRecipe {
                    path: ext_recipe,
                    library_name: Some(lib_name.to_string()),
                    recipe_path: Some(recipe_path.to_string()),
                };
            }
        }
    }

    // Return the original path (will fail naturally with normal error messages).
    ResolvedRecipe {
        path: path.to_path_buf(),
        library_name: None,
        recipe_path: None,
    }
}

/// Resolve conventions for a library recipe: load .jigrc.yaml, merge with manifest
/// defaults, render convention templates with recipe variables, and return as a
/// serde_json::Value suitable for injection into template context.
fn resolve_library_conventions(
    library_name: &str,
    vars: &serde_json::Value,
    base_dir: &std::path::Path,
) -> Result<serde_json::Value, JigError> {
    let manifest = library::install::load_installed_manifest(library_name, base_dir)?;
    let project_config = library::conventions::ProjectConfig::load(base_dir)?;
    let conventions = library::conventions::resolve_conventions(&manifest, &project_config);

    // Two-pass: render convention templates with recipe variables (AC-5.3).
    // Skip conventions whose templates reference variables not present in the
    // current recipe (common when an extension recipe uses only a subset of vars).
    let env = renderer::create_standalone_env();
    let mut rendered = serde_json::Map::new();
    for (key, template) in &conventions {
        match renderer::render_string(&env, template, vars, &format!("conventions.{key}")) {
            Ok(value) => {
                rendered.insert(key.clone(), serde_json::Value::String(value));
            }
            Err(_) => {
                // Convention template references variables not available in this recipe.
                // Keep the raw template as-is so it doesn't vanish from context.
                rendered.insert(key.clone(), serde_json::Value::String(template.clone()));
            }
        }
    }

    Ok(serde_json::Value::Object(rendered))
}

struct SelectorValidationReport {
    mode: &'static str,
    deferred_fields: Vec<String>,
    validated_with_vars: bool,
}

fn cmd_validate(
    path: &std::path::Path,
    inline_vars: Option<&str>,
    vars_file: Option<&std::path::Path>,
    vars_stdin: bool,
    json: bool,
    project_dir: &std::path::Path,
    library_name: Option<&str>,
    library_recipe_path: Option<&str>,
) -> Result<i32, JigError> {
    match workflow::detect_file_type(path)? {
        workflow::FileType::Recipe => {
            let recipe = Recipe::load(path)?;
            let selector_validation = validate_recipe_for_validate(
                &recipe,
                inline_vars,
                vars_file,
                vars_stdin,
                project_dir,
                library_name,
                library_recipe_path,
            )?;
            if json {
                let output = build_validate_json(&recipe, &selector_validation);
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                eprintln!("Recipe is valid: {}", path.display());
                eprintln!("  Variables: {}", recipe.variables.len());
                if !recipe.variables.is_empty() {
                    for name in recipe.variables.keys() {
                        eprintln!("    - {name}");
                    }
                }
                let op_types = summarize_op_types(&recipe);
                eprintln!("  Operations: {} ({})", recipe.files.len(), op_types);
                match (
                    selector_validation.mode,
                    selector_validation.validated_with_vars,
                ) {
                    ("deferred", _) => {
                        eprintln!(
                            "  Selector validation: deferred for {} templated field(s)",
                            selector_validation.deferred_fields.len()
                        );
                    }
                    ("complete", true) => {
                        eprintln!("  Selector validation: complete with provided vars");
                    }
                    _ => {}
                }
            }
        }
        workflow::FileType::Workflow => {
            let validation = workflow::validate_workflow(path)?;
            if json {
                let output = output::build_workflow_validate_json(&validation);
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                eprintln!("Workflow is valid: {}", path.display());
                if let Some(ref name) = validation.name {
                    eprintln!("  Name: {name}");
                }
                eprintln!("  Variables: {}", validation.variables.len());
                if !validation.variables.is_empty() {
                    for name in validation.variables.keys() {
                        eprintln!("    - {name}");
                    }
                }
                let unconditional = validation.steps.iter().filter(|s| !s.conditional).count();
                let conditional = validation.steps.iter().filter(|s| s.conditional).count();
                eprintln!(
                    "  Steps: {} ({} unconditional, {} conditional)",
                    validation.steps.len(),
                    unconditional,
                    conditional
                );
                for (i, step) in validation.steps.iter().enumerate() {
                    let cond = if step.conditional {
                        " (conditional)"
                    } else {
                        ""
                    };
                    let status = if step.valid { "valid" } else { "INVALID" };
                    eprintln!("    {}. {} — {}{}", i + 1, step.recipe, status, cond);
                }
            }
        }
    }

    Ok(0)
}

fn cmd_vars(path: &std::path::Path, base_dir: &std::path::Path) -> Result<i32, JigError> {
    // Check if this is a library workflow (path was resolved as recipe.yaml but user
    // might have meant a workflow). Try library workflow resolution from original path.
    let path_str = path.to_string_lossy();
    if path_str.contains('/') && !path.exists() {
        // Could be a library workflow — try resolving.
        if let Ok((_, wf_name, manifest)) =
            library::discover::resolve_library_workflow(&path_str, base_dir)
        {
            let wf_def = &manifest.workflows[&wf_name];
            let vars = build_library_workflow_vars(&manifest, wf_def);
            println!("{}", serde_json::to_string_pretty(&vars).unwrap());
            return Ok(0);
        }
    }

    let vars = match workflow::detect_file_type(path)? {
        workflow::FileType::Recipe => {
            let recipe = Recipe::load(path)?;
            variables::vars_json(&recipe.variables)
        }
        workflow::FileType::Workflow => {
            let wf = workflow::load_workflow(path)?;
            variables::vars_json(&wf.variables)
        }
    };
    println!("{}", serde_json::to_string_pretty(&vars).unwrap());
    Ok(0)
}

/// Build a vars JSON output for a library workflow by collecting variables
/// from all referenced recipes in the workflow steps.
fn build_library_workflow_vars(
    manifest: &library::manifest::LibraryManifest,
    wf_def: &library::manifest::ManifestWorkflow,
) -> serde_json::Value {
    let mut all_vars = indexmap::IndexMap::new();
    for step in &wf_def.steps {
        if let Some(recipe_path) = manifest.resolve_recipe_path(&step.recipe)
            && let Ok(recipe) = Recipe::load(&recipe_path)
        {
            for (name, decl) in &recipe.variables {
                all_vars.entry(name.clone()).or_insert_with(|| decl.clone());
            }
        }
    }
    variables::vars_json(&all_vars)
}

/// Build a Workflow struct from a library manifest's workflow definition.
/// Resolves all recipe paths relative to the library root (AC-4.9).
fn build_library_workflow(
    manifest: &library::manifest::LibraryManifest,
    workflow_name: &str,
) -> Result<workflow::Workflow, JigError> {
    let wf_def = manifest.workflows.get(workflow_name).ok_or_else(|| {
        JigError::RecipeValidation(crate::error::StructuredError {
            what: format!(
                "workflow '{workflow_name}' not found in library '{}'",
                manifest.name
            ),
            where_: manifest.name.clone(),
            why: format!("the library does not declare workflow '{workflow_name}'"),
            hint: format!(
                "use 'jig library workflows {}' to see available workflows",
                manifest.name
            ),
        })
    })?;

    let on_error = match wf_def.on_error.as_deref() {
        Some("continue") => workflow::OnError::Continue,
        Some("report") => workflow::OnError::Report,
        _ => workflow::OnError::Stop,
    };

    // Collect variables from all referenced recipes.
    let mut all_vars = indexmap::IndexMap::new();
    let mut steps = Vec::with_capacity(wf_def.steps.len());

    for step in &wf_def.steps {
        let resolved = manifest.resolve_recipe_path(&step.recipe).ok_or_else(|| {
            JigError::RecipeValidation(crate::error::StructuredError {
                what: format!(
                    "recipe '{}' not found in library '{}'",
                    step.recipe, manifest.name
                ),
                where_: format!("{}/{}", manifest.name, workflow_name),
                why: format!(
                    "workflow step references recipe '{}' which is not declared",
                    step.recipe
                ),
                hint: format!(
                    "use 'jig library recipes {}' to see available recipes",
                    manifest.name
                ),
            })
        })?;

        // Load recipe to collect variables.
        if let Ok(recipe) = Recipe::load(&resolved) {
            for (name, decl) in &recipe.variables {
                all_vars.entry(name.clone()).or_insert_with(|| decl.clone());
            }
        }

        let step_on_error = match step.on_error.as_deref() {
            Some("continue") => Some(workflow::OnError::Continue),
            Some("report") => Some(workflow::OnError::Report),
            Some("stop") => Some(workflow::OnError::Stop),
            _ => None,
        };

        steps.push(workflow::WorkflowStep {
            recipe: step.recipe.clone(),
            resolved_recipe: resolved,
            when: step.when.clone(),
            vars_map: step.vars_map.clone(),
            vars: step.vars.clone(),
            on_error: step_on_error,
        });
    }

    Ok(workflow::Workflow {
        name: Some(workflow_name.to_string()),
        description: wf_def.description.clone(),
        variables: all_vars,
        steps,
        on_error,
        workflow_dir: manifest.library_dir.clone(),
    })
}

fn cmd_render(
    template_path: &std::path::Path,
    to: Option<&std::path::Path>,
    inline_vars: Option<&str>,
    vars_file: Option<&std::path::Path>,
    vars_stdin: bool,
) -> Result<i32, JigError> {
    // Read the template file
    let source = std::fs::read_to_string(template_path).map_err(|e| {
        JigError::TemplateRendering(crate::error::StructuredError {
            what: format!("cannot read template file '{}'", template_path.display()),
            where_: template_path.display().to_string(),
            why: e.to_string(),
            hint: "check the template file path".into(),
        })
    })?;

    // Collect variables from all sources (no recipe context → no type validation)
    let vars = variables::collect_vars(inline_vars, vars_file, vars_stdin)?;

    // Create standalone environment and render
    let env = renderer::create_standalone_env();
    let source_name = template_path.display().to_string();
    let output = renderer::render_string(&env, &source, &vars, &source_name)?;

    // Write to file or stdout
    match to {
        Some(path) => {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).map_err(|e| {
                    JigError::FileOperation(crate::error::StructuredError {
                        what: format!("cannot create output directory '{}'", parent.display()),
                        where_: path.display().to_string(),
                        why: e.to_string(),
                        hint: "check directory permissions".into(),
                    })
                })?;
            }
            std::fs::write(path, &output).map_err(|e| {
                JigError::FileOperation(crate::error::StructuredError {
                    what: format!("cannot write output file '{}'", path.display()),
                    where_: path.display().to_string(),
                    why: e.to_string(),
                    hint: "check file permissions".into(),
                })
            })?;
        }
        None => {
            print!("{output}");
        }
    }

    Ok(0)
}

#[allow(clippy::too_many_arguments)]
fn cmd_run(
    recipe_path: &std::path::Path,
    inline_vars: Option<&str>,
    vars_file: Option<&std::path::Path>,
    vars_stdin: bool,
    dry_run: bool,
    force_json: bool,
    quiet: bool,
    force: bool,
    base_dir: Option<&std::path::Path>,
    verbose: bool,
    project_dir: &std::path::Path,
    library_name: Option<&str>,
    library_recipe_path: Option<&str>,
) -> Result<i32, JigError> {
    let mode = output::detect_mode(force_json);

    // Helper: handle pre-operation errors respecting --json and --quiet.
    // In JSON/piped mode, emit a JSON error envelope to stdout.
    // In human mode with --quiet, suppress stderr.
    // Returns Ok(exit_code) so main() doesn't double-print.
    let handle_early_error = |e: JigError| -> Result<i32, JigError> {
        let code = e.exit_code();
        match mode {
            output::OutputMode::Json => {
                let se = e.structured_error().clone();
                let json = serde_json::json!({
                    "dry_run": dry_run,
                    "operations": [{
                        "action": "error",
                        "path": "",
                        "what": se.what,
                        "where": se.where_,
                        "why": se.why,
                        "hint": se.hint,
                        "rendered_content": "",
                    }],
                    "files_written": [],
                    "files_skipped": [],
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
                Ok(code)
            }
            output::OutputMode::Human => {
                if !quiet {
                    eprintln!("{e}");
                }
                Ok(code)
            }
        }
    };

    // 1. Load and validate recipe (exit 1 on failure).
    let recipe = match Recipe::load(recipe_path) {
        Ok(r) => r,
        Err(e) => return handle_early_error(e),
    };

    // 2. Collect and validate variables (exit 4 on failure).
    let provided = match variables::collect_vars(inline_vars, vars_file, vars_stdin) {
        Ok(v) => v,
        Err(e) => return handle_early_error(e),
    };
    let mut vars = match variables::validate_variables(&recipe.variables, &provided) {
        Ok(v) => v,
        Err(e) => return handle_early_error(e),
    };

    // 2b. Inject conventions for library recipes (AC-5.3, AC-5.5).
    if let Some(lib_name) = library_name {
        match resolve_library_conventions(lib_name, &vars, project_dir) {
            Ok(conventions) => {
                if let Some(obj) = vars.as_object_mut() {
                    obj.insert("conventions".to_string(), conventions);
                }
            }
            Err(e) => return handle_early_error(e),
        }
    }

    // 3. Create recipe-aware environment and render ALL templates upfront (exit 2 on failure).
    // For library recipes, check .jig/overrides/<library>/<recipe-path>/ (AC-7.1).
    // Template paths like "templates/model.rs.j2" are resolved relative to this dir.
    let override_dir = resolve_override_dir(project_dir, library_name, library_recipe_path);
    let env = match renderer::create_recipe_env_with_overrides(&recipe, override_dir.as_deref()) {
        Ok(e) => e,
        Err(e) => return handle_early_error(e),
    };

    let prepared_ops = match prepare_operations(&recipe, &env, &vars) {
        Ok(ops) => ops,
        Err(e) => return handle_early_error(e),
    };

    // 4. Validate base_dir exists (AC-4.10). Done after rendering to respect
    // pipeline stage ordering: recipe(1) → variables(4) → rendering(2) → file ops(3) (AC-N5.2).
    let resolved_base = match base_dir {
        Some(bd) => {
            if !bd.is_dir() {
                return handle_early_error(JigError::FileOperation(crate::error::StructuredError {
                    what: format!("base directory does not exist: '{}'", bd.display()),
                    where_: bd.display().to_string(),
                    why: "the specified --base-dir path is not an existing directory".into(),
                    hint: "create the directory first, or omit --base-dir to use the current directory".into(),
                }));
            }
            bd.to_path_buf()
        }
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    // 5. Execute operations in declaration order, fail-fast on error.
    let mut ctx = operations::ExecutionContext::new(resolved_base, dry_run, force);
    let mut results: Vec<operations::OpResult> = Vec::with_capacity(prepared_ops.len());

    for prepared in &prepared_ops {
        let result = operations::execute_operation(prepared, &mut ctx, verbose);
        let is_err = result.is_error();
        results.push(result);
        if is_err {
            break; // Fail-fast: stop on first error (AC-6.10).
        }
    }

    // 6. Format and emit output.
    let has_error = results.iter().any(|r| r.is_error());

    match mode {
        output::OutputMode::Json => {
            let json = output::format_json(&results, dry_run, verbose);
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        output::OutputMode::Human => {
            if !quiet {
                output::format_human(&results, dry_run, verbose);
            }
        }
    }

    // 7. Return appropriate exit code.
    // Error details are already in the JSON/human output above, so return Ok
    // with the exit code to avoid duplicate stderr output from main().
    if has_error
        && let Some(last) = results.last()
        && let Some(jig_err) = operations::op_error_to_jig_error(last)
    {
        return Ok(jig_err.exit_code());
    }

    Ok(0)
}

#[allow(clippy::too_many_arguments)]
fn cmd_workflow(
    workflow_path: &std::path::Path,
    inline_vars: Option<&str>,
    vars_file: Option<&std::path::Path>,
    vars_stdin: bool,
    dry_run: bool,
    force_json: bool,
    quiet: bool,
    force: bool,
    base_dir: Option<&std::path::Path>,
    verbose: bool,
    project_dir: &std::path::Path,
) -> Result<i32, JigError> {
    let mode = output::detect_mode(force_json);

    // Helper for early errors.
    let handle_early_error = |e: JigError| -> Result<i32, JigError> {
        let code = e.exit_code();
        match mode {
            output::OutputMode::Json => {
                let se = e.structured_error().clone();
                let json = serde_json::json!({
                    "dry_run": dry_run,
                    "workflow": serde_json::Value::Null,
                    "on_error": "stop",
                    "status": "error",
                    "steps": [{
                        "recipe": "",
                        "status": "error",
                        "error": {
                            "what": se.what,
                            "where": se.where_,
                            "why": se.why,
                            "hint": se.hint,
                        },
                        "operations": [],
                    }],
                    "files_written": [],
                    "files_skipped": [],
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
                Ok(code)
            }
            output::OutputMode::Human => {
                if !quiet {
                    eprintln!("{e}");
                }
                Ok(code)
            }
        }
    };

    // Try library workflow resolution if the path doesn't exist as a file.
    let wf = if !workflow_path.exists() {
        let path_str = workflow_path.to_string_lossy();
        match library::discover::resolve_library_workflow(&path_str, project_dir) {
            Ok((_, wf_name, manifest)) => match build_library_workflow(&manifest, &wf_name) {
                Ok(w) => w,
                Err(e) => return handle_early_error(e),
            },
            Err(e) => return handle_early_error(e),
        }
    } else {
        // Guard: check if file is a recipe instead of a workflow.
        match workflow::detect_file_type(workflow_path) {
            Ok(workflow::FileType::Recipe) => {
                return handle_early_error(JigError::RecipeValidation(
                    crate::error::StructuredError {
                        what: "expected a workflow file, got a recipe file".into(),
                        where_: workflow_path.display().to_string(),
                        why: "file has 'files' (recipe) instead of 'steps' (workflow)".into(),
                        hint: "use 'jig run' to execute recipes, or provide a workflow file with 'steps'".into(),
                    },
                ));
            }
            Ok(workflow::FileType::Workflow) => {} // proceed
            Err(e) => return handle_early_error(e),
        }

        // Load workflow.
        match workflow::load_workflow(workflow_path) {
            Ok(w) => w,
            Err(e) => return handle_early_error(e),
        }
    };

    // Collect and validate workflow-level variables.
    let provided = match variables::collect_vars(inline_vars, vars_file, vars_stdin) {
        Ok(v) => v,
        Err(e) => return handle_early_error(e),
    };
    let vars = match variables::validate_variables(&wf.variables, &provided) {
        Ok(v) => v,
        Err(e) => return handle_early_error(e),
    };

    // Validate base_dir exists.
    let resolved_base = match base_dir {
        Some(bd) => {
            if !bd.is_dir() {
                return handle_early_error(JigError::FileOperation(crate::error::StructuredError {
                    what: format!("base directory does not exist: '{}'", bd.display()),
                    where_: bd.display().to_string(),
                    why: "the specified --base-dir path is not an existing directory".into(),
                    hint: "create the directory first, or omit --base-dir to use the current directory".into(),
                }));
            }
            bd.to_path_buf()
        }
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    // Execute workflow.
    let mut ctx = operations::ExecutionContext::new(resolved_base, dry_run, force);
    let result = workflow::execute_workflow(&wf, vars, &mut ctx, verbose);

    // Determine exit code based on step results and their effective on_error modes.
    let exit_code = compute_workflow_exit_code(&wf, &result);

    // Format output.
    match mode {
        output::OutputMode::Json => {
            let json = output::format_workflow_json(&result, dry_run, verbose, exit_code);
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        output::OutputMode::Human => {
            if !quiet {
                output::format_workflow_human(&result, dry_run, verbose);
            }
        }
    }

    Ok(exit_code)
}

/// Compute the exit code for a completed workflow based on step results
/// and their effective on_error modes.
fn compute_workflow_exit_code(wf: &workflow::Workflow, result: &workflow::WorkflowResult) -> i32 {
    let mut has_report_error = false;

    for (i, step_result) in result.steps.iter().enumerate() {
        if let workflow::StepResult::Error { error, .. } = step_result {
            let effective_mode = wf
                .steps
                .get(i)
                .and_then(|s| s.on_error)
                .unwrap_or(wf.on_error);

            match effective_mode {
                workflow::OnError::Stop => {
                    // This step halted execution — return its error code.
                    return error.exit_code();
                }
                workflow::OnError::Continue => {
                    // Tolerated — no effect on exit code.
                }
                workflow::OnError::Report => {
                    has_report_error = true;
                }
            }
        }
    }

    if has_report_error { 3 } else { 0 }
}

// ── jig list ──────────────────────────────────────────────────────

/// A discovered skill directory with metadata extracted from SKILL.md frontmatter.
#[derive(Debug)]
struct SkillEntry {
    /// Directory name (e.g., "add-endpoint").
    name: String,
    /// Description from frontmatter, or None.
    description: Option<String>,
    /// Relative path from base_dir to the skill directory.
    path: String,
}

/// Extract `name` and `description` from YAML frontmatter in a SKILL.md file.
/// Frontmatter is delimited by `---` lines at the start of the file.
fn parse_skill_frontmatter(content: &str) -> (Option<String>, Option<String>) {
    let content = content.trim_start_matches('\u{feff}'); // strip BOM
    if !content.starts_with("---") {
        return (None, None);
    }
    let after_open = &content[3..];
    let close = match after_open.find("\n---") {
        Some(pos) => pos,
        None => return (None, None),
    };
    let frontmatter = &after_open[..close];

    // Use serde_yaml to handle block scalars, quoting, etc.
    let parsed: serde_json::Value = match serde_yaml::from_str(frontmatter) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let obj = match parsed.as_object() {
        Some(o) => o,
        None => return (None, None),
    };
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    // For descriptions, take only the first line (the summary).
    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.lines().next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty());
    (name, description)
}

/// Scan a directory for skill subdirectories (each containing SKILL.md).
fn scan_skills_dir(skills_dir: &std::path::Path, base_dir: &std::path::Path) -> Vec<SkillEntry> {
    let Ok(entries) = std::fs::read_dir(skills_dir) else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&skill_md).unwrap_or_default();
        let (fm_name, fm_desc) = parse_skill_frontmatter(&content);

        let rel_path = path
            .strip_prefix(base_dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());

        results.push(SkillEntry {
            name: fm_name.unwrap_or(dir_name),
            description: fm_desc,
            path: rel_path,
        });
    }
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

/// Recursively find directories named "skills" under `root`.
fn find_skills_dirs(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    find_skills_dirs_rec(root, &mut found, 0);
    found
}

fn find_skills_dirs_rec(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>, depth: usize) {
    // Don't recurse too deep or into hidden dirs (except agent config dirs).
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip build artifacts, node_modules, etc.
        if matches!(
            name_str.as_ref(),
            "node_modules" | "target" | ".git" | "__pycache__" | "venv" | ".venv"
        ) {
            continue;
        }

        if name_str == "skills" {
            // Check if this dir actually contains skill subdirs with SKILL.md.
            if has_skill_children(&path) {
                found.push(path.clone());
            }
        }

        // Recurse (including into dotfiles like .claude, .codex).
        find_skills_dirs_rec(&path, found, depth + 1);
    }
}

/// Returns true if the directory contains at least one subdirectory with SKILL.md.
fn has_skill_children(dir: &std::path::Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .flatten()
        .any(|e| e.path().is_dir() && e.path().join("SKILL.md").exists())
}

fn cmd_list(
    skills: bool,
    claude: bool,
    codex: bool,
    base_dir: &std::path::Path,
    force_json: bool,
    quiet: bool,
) -> Result<i32, JigError> {
    let mode = output::detect_mode(force_json);

    // --claude or --codex imply --skills.
    let skills = skills || claude || codex;

    if !skills {
        // No flags: list installed libraries (quick summary).
        let libraries = library::install::list_installed(base_dir)?;
        match mode {
            output::OutputMode::Json => {
                let items: Vec<serde_json::Value> = libraries
                    .iter()
                    .map(|lib| {
                        serde_json::json!({
                            "name": lib.name,
                            "version": lib.version,
                            "description": lib.description,
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({ "libraries": items }))
                        .unwrap()
                );
            }
            output::OutputMode::Human => {
                if !quiet {
                    if libraries.is_empty() {
                        eprintln!("No libraries installed. Use --skills to scan for agent skills.");
                    } else {
                        eprintln!("Installed libraries:");
                        for lib in &libraries {
                            let desc = lib
                                .description
                                .as_deref()
                                .map(|d| format!(" — {d}"))
                                .unwrap_or_default();
                            eprintln!("  {} v{}{desc}", lib.name, lib.version);
                        }
                        eprintln!();
                        eprintln!(
                            "Use --skills to scan for agent skills, or `jig library recipes <name>` for recipe details."
                        );
                    }
                }
            }
        }
        return Ok(0);
    }

    // Skills mode: scan specific or all skills directories.
    let mut skills_dirs: Vec<std::path::PathBuf> = Vec::new();

    if claude {
        let dir = base_dir.join(".claude/skills");
        if dir.is_dir() {
            skills_dirs.push(dir);
        }
    }
    if codex {
        let dir = base_dir.join(".codex/skills");
        if dir.is_dir() {
            skills_dirs.push(dir);
        }
    }
    if !claude && !codex {
        // Generic --skills: find all skills/ directories.
        skills_dirs = find_skills_dirs(base_dir);
    }

    let mut all_skills: Vec<SkillEntry> = Vec::new();
    for dir in &skills_dirs {
        all_skills.extend(scan_skills_dir(dir, base_dir));
    }

    match mode {
        output::OutputMode::Json => {
            let items: Vec<serde_json::Value> = all_skills
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "description": s.description,
                        "path": s.path,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({ "skills": items })).unwrap()
            );
        }
        output::OutputMode::Human => {
            if !quiet {
                if all_skills.is_empty() {
                    if claude {
                        eprintln!("No skills found in .claude/skills/");
                    } else if codex {
                        eprintln!("No skills found in .codex/skills/");
                    } else {
                        eprintln!("No skills found.");
                    }
                } else {
                    for skill in &all_skills {
                        let desc = skill
                            .description
                            .as_deref()
                            .map(|d| format!(" — {d}"))
                            .unwrap_or_default();
                        eprintln!("  {}{desc}", skill.name);
                        eprintln!("    {}", skill.path);
                    }
                }
            }
        }
    }
    Ok(0)
}

fn cmd_library(
    action: LibraryAction,
    base_dir: &std::path::Path,
    force_json: bool,
    quiet: bool,
) -> Result<i32, JigError> {
    let mode = output::detect_mode(force_json);

    match action {
        LibraryAction::Add {
            source,
            global,
            force: force_install,
        } => {
            let location = if global {
                library::install::InstallLocation::Global
            } else {
                library::install::InstallLocation::ProjectLocal
            };

            let installed = if library::install::is_git_url(&source) {
                // Git install (AC-2.2).
                let clone_dir = library::install::git_clone(&source)?;
                let result = library::install::add_from_path_with_options(
                    &clone_dir,
                    location,
                    base_dir,
                    force_install,
                    &source,
                    "git",
                );
                let _ = std::fs::remove_dir_all(&clone_dir);
                result?
            } else {
                let path = PathBuf::from(&source);
                let resolved = path.canonicalize().map_err(|e| {
                    JigError::FileOperation(crate::error::StructuredError {
                        what: format!("cannot resolve path '{source}'"),
                        where_: source.clone(),
                        why: e.to_string(),
                        hint: "check the path exists".into(),
                    })
                })?;
                library::install::add_from_path_with_options(
                    &resolved,
                    location,
                    base_dir,
                    force_install,
                    &resolved.display().to_string(),
                    "local",
                )?
            };
            match mode {
                output::OutputMode::Json => {
                    let json = serde_json::json!({
                        "action": "add",
                        "library": installed.name,
                        "version": installed.version,
                        "location": installed.location.to_string(),
                        "path": installed.path.display().to_string(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                output::OutputMode::Human => {
                    if !quiet {
                        eprintln!(
                            "Installed library '{}' v{} ({})",
                            installed.name, installed.version, installed.location
                        );
                    }
                }
            }
            Ok(0)
        }

        LibraryAction::Remove { name } => {
            let removed = library::install::remove(&name, base_dir)?;
            match mode {
                output::OutputMode::Json => {
                    let json = serde_json::json!({
                        "action": "remove",
                        "library": removed.name,
                        "version": removed.version,
                        "location": removed.location.to_string(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                output::OutputMode::Human => {
                    if !quiet {
                        eprintln!("Removed library '{}' v{}", removed.name, removed.version);
                    }
                }
            }
            Ok(0)
        }

        LibraryAction::Update { name, source } => {
            let updated = match source {
                Some(src) => {
                    if library::install::is_git_url(&src) {
                        let clone_dir = library::install::git_clone(&src)?;
                        let result =
                            library::install::update_from_path(&name, &clone_dir, base_dir);
                        let _ = std::fs::remove_dir_all(&clone_dir);
                        let lib = result?;
                        // Re-write metadata with git source.
                        library::install::write_install_meta(&lib.path, &src, "git", &lib.version)?;
                        lib
                    } else {
                        let path = PathBuf::from(&src);
                        let resolved = path.canonicalize().map_err(|e| {
                            JigError::FileOperation(crate::error::StructuredError {
                                what: format!("cannot resolve path '{src}'"),
                                where_: src.clone(),
                                why: e.to_string(),
                                hint: "check the path exists".into(),
                            })
                        })?;
                        library::install::update_from_path(&name, &resolved, base_dir)?
                    }
                }
                None => {
                    // No source provided — try to update from recorded metadata (AC-2.9, AC-2.10).
                    library::install::update_from_meta(&name, base_dir)?
                }
            };
            match mode {
                output::OutputMode::Json => {
                    let json = serde_json::json!({
                        "action": "update",
                        "library": updated.name,
                        "version": updated.version,
                        "location": updated.location.to_string(),
                        "path": updated.path.display().to_string(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                output::OutputMode::Human => {
                    if !quiet {
                        eprintln!(
                            "Updated library '{}' to v{} ({})",
                            updated.name, updated.version, updated.location
                        );
                    }
                }
            }
            Ok(0)
        }

        LibraryAction::List => {
            let libraries = library::install::list_installed(base_dir)?;
            match mode {
                output::OutputMode::Json => {
                    let items: Vec<serde_json::Value> = libraries
                        .iter()
                        .map(|lib| {
                            serde_json::json!({
                                "name": lib.name,
                                "version": lib.version,
                                "description": lib.description,
                                "framework": lib.framework,
                                "language": lib.language,
                                "location": lib.location.to_string(),
                                "path": lib.path.display().to_string(),
                            })
                        })
                        .collect();
                    let json = serde_json::json!({ "libraries": items });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                output::OutputMode::Human => {
                    if !quiet {
                        if libraries.is_empty() {
                            eprintln!("No libraries installed.");
                        } else {
                            for lib in &libraries {
                                let desc = lib
                                    .description
                                    .as_deref()
                                    .map(|d| format!(" — {d}"))
                                    .unwrap_or_default();
                                eprintln!(
                                    "  {} v{} ({}){desc}",
                                    lib.name, lib.version, lib.location
                                );
                            }
                        }
                    }
                }
            }
            Ok(0)
        }

        LibraryAction::Recipes { name } => {
            let recipes = library::discover::list_recipes_with_extensions(&name, base_dir)?;
            match mode {
                output::OutputMode::Json => {
                    let items: Vec<serde_json::Value> = recipes
                        .iter()
                        .map(|entry| {
                            serde_json::json!({
                                "path": entry.path,
                                "description": entry.description,
                                "source": match entry.source {
                                    library::discover::RecipeSource::Library => "library",
                                    library::discover::RecipeSource::Extension => "extension",
                                },
                            })
                        })
                        .collect();
                    let json = serde_json::json!({
                        "library": name,
                        "recipes": items,
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                output::OutputMode::Human => {
                    if !quiet {
                        if recipes.is_empty() {
                            eprintln!("Library '{name}' has no recipes.");
                        } else {
                            eprintln!("Recipes in '{name}':");
                            for entry in &recipes {
                                let marker = match entry.source {
                                    library::discover::RecipeSource::Extension => " [ext]",
                                    _ => "",
                                };
                                eprintln!("  {} — {}{marker}", entry.path, entry.description);
                            }
                        }
                    }
                }
            }
            Ok(0)
        }

        LibraryAction::Info { path } => {
            // Parse "library/recipe/path".
            let slash = path.find('/').ok_or_else(|| {
                JigError::RecipeValidation(crate::error::StructuredError {
                    what: format!("invalid recipe path '{path}'"),
                    where_: path.clone(),
                    why: "expected format: <library>/<recipe-path>".into(),
                    hint: "example: django/model/add-field".into(),
                })
            })?;
            let lib_name = &path[..slash];
            let recipe_path = &path[slash + 1..];
            let info = library::discover::recipe_info(lib_name, recipe_path, base_dir)?;
            match mode {
                output::OutputMode::Json => {
                    let vars: Vec<serde_json::Value> = info
                        .variables
                        .iter()
                        .map(|v| {
                            serde_json::json!({
                                "name": v.name,
                                "type": v.var_type,
                                "required": v.required,
                                "description": v.description,
                            })
                        })
                        .collect();
                    let json = serde_json::json!({
                        "library": info.library,
                        "recipe": info.path,
                        "description": info.description,
                        "variables": vars,
                        "operations": info.operations,
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                output::OutputMode::Human => {
                    if !quiet {
                        eprintln!("{}/{}", info.library, info.path);
                        eprintln!("  {}", info.description);
                        if !info.variables.is_empty() {
                            eprintln!("  Variables:");
                            for v in &info.variables {
                                let req = if v.required { " (required)" } else { "" };
                                let desc = v
                                    .description
                                    .as_deref()
                                    .map(|d| format!(" — {d}"))
                                    .unwrap_or_default();
                                eprintln!("    {} [{}]{req}{desc}", v.name, v.var_type);
                            }
                        }
                        if !info.operations.is_empty() {
                            eprintln!("  Operations: {}", info.operations.join(", "));
                        }
                    }
                }
            }
            Ok(0)
        }

        LibraryAction::Workflows { name } => {
            let workflows = library::discover::list_workflows(&name, base_dir)?;
            match mode {
                output::OutputMode::Json => {
                    let items: Vec<serde_json::Value> = workflows
                        .iter()
                        .map(|wf| {
                            let steps: Vec<serde_json::Value> = wf
                                .steps
                                .iter()
                                .map(|s| {
                                    serde_json::json!({
                                        "recipe": s.recipe,
                                        "conditional": s.conditional,
                                    })
                                })
                                .collect();
                            serde_json::json!({
                                "name": wf.name,
                                "description": wf.description,
                                "steps": steps,
                            })
                        })
                        .collect();
                    let json = serde_json::json!({
                        "library": name,
                        "workflows": items,
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                output::OutputMode::Human => {
                    if !quiet {
                        if workflows.is_empty() {
                            eprintln!("Library '{name}' has no workflows.");
                        } else {
                            eprintln!("Workflows in '{name}':");
                            for wf in &workflows {
                                let desc = wf
                                    .description
                                    .as_deref()
                                    .map(|d| format!(" — {d}"))
                                    .unwrap_or_default();
                                let steps_info = format!("{} steps", wf.steps.len());
                                eprintln!("  {} ({steps_info}){desc}", wf.name);
                            }
                        }
                    }
                }
            }
            Ok(0)
        }
    }
}

fn validate_recipe_for_validate(
    recipe: &Recipe,
    inline_vars: Option<&str>,
    vars_file: Option<&std::path::Path>,
    vars_stdin: bool,
    project_dir: &std::path::Path,
    library_name: Option<&str>,
    library_recipe_path: Option<&str>,
) -> Result<SelectorValidationReport, JigError> {
    let deferred_selector_fields = recipe.deferred_selector_fields();
    let has_var_inputs = inline_vars.is_some() || vars_file.is_some() || vars_stdin;

    if !has_var_inputs {
        return Ok(SelectorValidationReport {
            mode: if deferred_selector_fields.is_empty() {
                "complete"
            } else {
                "deferred"
            },
            deferred_fields: deferred_selector_fields,
            validated_with_vars: false,
        });
    }

    let provided = variables::collect_vars(inline_vars, vars_file, vars_stdin)?;
    let mut vars = variables::validate_variables(&recipe.variables, &provided)?;
    if let Some(lib_name) = library_name {
        let conventions = resolve_library_conventions(lib_name, &vars, project_dir)?;
        if let Some(obj) = vars.as_object_mut() {
            obj.insert("conventions".to_string(), conventions);
        }
    }

    let override_dir = resolve_override_dir(project_dir, library_name, library_recipe_path);
    let env = renderer::create_recipe_env_with_overrides(recipe, override_dir.as_deref())?;
    prepare_operations(recipe, &env, &vars)?;

    Ok(SelectorValidationReport {
        mode: "complete",
        deferred_fields: deferred_selector_fields,
        validated_with_vars: true,
    })
}

fn build_validate_json(
    recipe: &Recipe,
    selector_validation: &SelectorValidationReport,
) -> serde_json::Value {
    let vars = variables::vars_json(&recipe.variables);

    let ops: Vec<serde_json::Value> = recipe
        .files
        .iter()
        .map(|op| {
            let mut m = serde_json::Map::new();
            m.insert(
                "type".into(),
                serde_json::Value::String(op.op_type_str().into()),
            );
            match op {
                recipe::FileOp::Create { to, .. } => {
                    m.insert("to".into(), serde_json::Value::String(to.clone()));
                }
                recipe::FileOp::Inject { inject, .. } => {
                    m.insert("inject".into(), serde_json::Value::String(inject.clone()));
                }
                recipe::FileOp::Replace { replace, .. } => {
                    m.insert("replace".into(), serde_json::Value::String(replace.clone()));
                }
                recipe::FileOp::Patch { patch, .. } => {
                    m.insert("patch".into(), serde_json::Value::String(patch.clone()));
                }
            }
            serde_json::Value::Object(m)
        })
        .collect();

    serde_json::json!({
        "valid": true,
        "name": recipe.name,
        "description": recipe.description,
        "variables": vars,
        "operations": ops,
        "selector_validation": {
            "mode": selector_validation.mode,
            "deferred_fields": selector_validation.deferred_fields,
            "validated_with_vars": selector_validation.validated_with_vars,
        },
    })
}

fn resolve_override_dir(
    project_dir: &std::path::Path,
    library_name: Option<&str>,
    library_recipe_path: Option<&str>,
) -> Option<PathBuf> {
    match (library_name, library_recipe_path) {
        (Some(lib), Some(rp)) => {
            let d = project_dir.join(".jig/overrides").join(lib).join(rp);
            if d.is_dir() { Some(d) } else { None }
        }
        _ => None,
    }
}

fn summarize_op_types(recipe: &Recipe) -> String {
    let mut create_count = 0usize;
    let mut inject_count = 0usize;
    let mut replace_count = 0usize;
    let mut patch_count = 0usize;
    for op in &recipe.files {
        match op {
            recipe::FileOp::Create { .. } => create_count += 1,
            recipe::FileOp::Inject { .. } => inject_count += 1,
            recipe::FileOp::Replace { .. } => replace_count += 1,
            recipe::FileOp::Patch { .. } => patch_count += 1,
        }
    }
    let mut parts = Vec::new();
    if create_count > 0 {
        parts.push(format!("{create_count} create"));
    }
    if inject_count > 0 {
        parts.push(format!("{inject_count} inject"));
    }
    if replace_count > 0 {
        parts.push(format!("{replace_count} replace"));
    }
    if patch_count > 0 {
        parts.push(format!("{patch_count} patch"));
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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

    fn validate_recipe(path: &std::path::Path, json: bool) -> Result<i32, JigError> {
        cmd_validate(
            path,
            None,
            None,
            false,
            json,
            path.parent().unwrap(),
            None,
            None,
        )
    }

    /// AC-7.1: jig validate parses recipe and reports validity
    #[test]
    fn ac_7_1_validate_command_valid() {
        let yaml = "name: test\nvariables:\n  name:\n    type: string\n    required: true\nfiles:\n  - template: t.j2\n    to: out.rs\n";
        let (_dir, path) = setup_recipe(yaml, &["t.j2"]);
        let result = validate_recipe(&path, false);
        assert_eq!(result.unwrap(), 0);
    }

    /// AC-7.1: jig validate --json outputs structured JSON with variables and operations
    #[test]
    fn ac_7_1_validate_json_output() {
        let yaml = "name: test\nvariables:\n  name:\n    type: string\nfiles:\n  - template: t.j2\n    to: out.rs\n";
        let (_dir, path) = setup_recipe(yaml, &["t.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        let selector_validation = validate_recipe_for_validate(
            &recipe,
            None,
            None,
            false,
            path.parent().unwrap(),
            None,
            None,
        )
        .unwrap();
        let json = build_validate_json(&recipe, &selector_validation);
        assert_eq!(json["valid"], true);
        assert!(json["variables"]["name"].is_object());
        assert_eq!(json["operations"][0]["type"], "create");
        assert_eq!(json["selector_validation"]["mode"], "complete");
        assert_eq!(json["selector_validation"]["validated_with_vars"], false);
    }

    /// AC-7.1: jig validate exits 1 for invalid recipe
    #[test]
    fn ac_7_1_validate_command_invalid() {
        let yaml = "bad yaml [";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let err = validate_recipe(&path, false).unwrap_err();
        assert_eq!(err.exit_code(), 1);
    }

    /// AC-7.2: jig vars outputs expected variables as JSON with type, required, default, description
    #[test]
    fn ac_7_2_vars_command() {
        let yaml = "variables:\n  class_name:\n    type: string\n    required: true\n    description: The class name\n    default: Foo\nfiles: []\n";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let recipe = Recipe::load(&path).unwrap();
        let json = variables::vars_json(&recipe.variables);
        let obj = json["class_name"].as_object().unwrap();
        assert_eq!(obj["type"], "string");
        assert_eq!(obj["required"], true);
        assert_eq!(obj["default"], "Foo");
        assert_eq!(obj["description"], "The class name");
    }

    /// AC-7.3: jig render renders template to stdout
    #[test]
    fn ac_7_3_render_to_stdout() {
        let dir = TempDir::new().unwrap();
        let tmpl_path = dir.path().join("test.j2");
        fs::write(&tmpl_path, "Hello {{ class_name }}!").unwrap();

        let result = cmd_render(
            &tmpl_path,
            None,
            Some(r#"{"class_name": "BookingService"}"#),
            None,
            false,
        );
        assert_eq!(result.unwrap(), 0);
    }

    /// AC-7.4: jig render --to writes to file
    #[test]
    fn ac_7_4_render_to_file() {
        let dir = TempDir::new().unwrap();
        let tmpl_path = dir.path().join("test.j2");
        fs::write(&tmpl_path, "Hello {{ name }}!").unwrap();

        let out_path = dir.path().join("output.txt");
        let result = cmd_render(
            &tmpl_path,
            Some(&out_path),
            Some(r#"{"name": "World"}"#),
            None,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(&out_path).unwrap();
        assert_eq!(content, "Hello World!");
    }

    /// AC-7.4: --to creates parent directories
    #[test]
    fn ac_7_4_render_to_creates_dirs() {
        let dir = TempDir::new().unwrap();
        let tmpl_path = dir.path().join("test.j2");
        fs::write(&tmpl_path, "content").unwrap();

        let out_path = dir.path().join("sub/dir/output.txt");
        let result = cmd_render(&tmpl_path, Some(&out_path), Some("{}"), None, false);
        assert_eq!(result.unwrap(), 0);
        assert!(out_path.exists());
    }

    /// AC-7.6: --json flag works on validate (partial — other flags tested in later phases)
    #[test]
    fn ac_7_6_json_flag_exists() {
        let yaml = "files: []\n";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let result = validate_recipe(&path, true);
        assert_eq!(result.unwrap(), 0);
    }

    /// AC-7.6: --vars, --vars-file, --vars-stdin are accepted global options
    #[test]
    fn ac_7_6_var_options_exist() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        // Check that the global options exist
        assert!(cmd.get_arguments().any(|a| a.get_id() == "vars"));
        assert!(cmd.get_arguments().any(|a| a.get_id() == "json_args"));
        assert!(cmd.get_arguments().any(|a| a.get_id() == "vars_file"));
        assert!(cmd.get_arguments().any(|a| a.get_id() == "vars_stdin"));
    }

    #[test]
    fn ac_7_6_json_args_flag_parses() {
        use clap::Parser;
        let parsed = Cli::try_parse_from(["jig", "--json-args", "{}", "render", "template.j2"]);
        assert!(
            parsed.is_ok(),
            "--json-args should be accepted as a global option"
        );
    }

    #[test]
    fn ac_7_6_vars_and_json_args_conflict() {
        use clap::Parser;
        let parsed = Cli::try_parse_from([
            "jig",
            "--vars",
            "{}",
            "--json-args",
            "{}",
            "render",
            "template.j2",
        ]);
        assert!(parsed.is_err(), "--vars and --json-args should conflict");
    }

    /// AC-7.7: --version is configured (clap handles the flag)
    #[test]
    fn ac_7_7_version_configured() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        assert!(cmd.get_version().is_some());
    }

    /// AC-N5.2: Recipe validation (exit 1) runs before other pipeline stages
    #[test]
    fn ac_n5_2_recipe_validation_first() {
        let yaml = "invalid: [yaml: broken";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let err = validate_recipe(&path, false).unwrap_err();
        assert_eq!(err.exit_code(), 1);
    }

    /// AC-7.1: validate output includes variable names and operation types
    #[test]
    fn ac_7_1_validate_json_lists_vars_and_ops() {
        let yaml = "variables:\n  name:\n    type: string\n  count:\n    type: number\nfiles:\n  - template: a.j2\n    to: out_a.rs\n  - template: b.j2\n    inject: target.rs\n    append: true\n";
        let (_dir, path) = setup_recipe(yaml, &["a.j2", "b.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        let selector_validation = validate_recipe_for_validate(
            &recipe,
            None,
            None,
            false,
            path.parent().unwrap(),
            None,
            None,
        )
        .unwrap();
        let json = build_validate_json(&recipe, &selector_validation);
        assert!(json["variables"]["name"].is_object());
        assert!(json["variables"]["count"].is_object());
        assert_eq!(json["operations"][0]["type"], "create");
        assert_eq!(json["operations"][1]["type"], "inject");
    }

    #[test]
    fn validate_json_marks_deferred_selector_fields() {
        let yaml = r#"
variables:
  function_name:
    type: string
  member_name:
    type: string
files:
  - template: t.j2
    inject: target.py
    before: "^def {{ function_name | regex_escape }}\\("
  - template: p.j2
    patch: target.py
    anchor:
      pattern: "^class Example:"
      scope: class_body
      find: "{{ member_name }}"
"#;
        let (_dir, path) = setup_recipe(yaml, &["t.j2", "p.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        let selector_validation = validate_recipe_for_validate(
            &recipe,
            None,
            None,
            false,
            path.parent().unwrap(),
            None,
            None,
        )
        .unwrap();
        let json = build_validate_json(&recipe, &selector_validation);
        assert_eq!(json["selector_validation"]["mode"], "deferred");
        assert_eq!(json["selector_validation"]["validated_with_vars"], false);
        assert_eq!(
            json["selector_validation"]["deferred_fields"][0],
            "files[0].before"
        );
        assert_eq!(
            json["selector_validation"]["deferred_fields"][1],
            "files[1].anchor.find"
        );
    }

    #[test]
    fn validate_with_vars_completes_selector_validation() {
        let yaml = r#"
variables:
  model_name:
    type: string
    required: true
  member_name:
    type: string
    required: true
files:
  - template: p.j2
    patch: target.py
    anchor:
      pattern: "^class {{ model_name | regex_escape }}:"
      scope: class_body
      find: "{{ member_name }}"
"#;
        let (_dir, path) = setup_recipe(yaml, &["p.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        let selector_validation = validate_recipe_for_validate(
            &recipe,
            Some(r#"{"model_name":"EntityAdmin","member_name":"list_display"}"#),
            None,
            false,
            path.parent().unwrap(),
            None,
            None,
        )
        .unwrap();
        let json = build_validate_json(&recipe, &selector_validation);
        assert_eq!(json["selector_validation"]["mode"], "complete");
        assert_eq!(json["selector_validation"]["validated_with_vars"], true);
        assert_eq!(
            json["selector_validation"]["deferred_fields"][0],
            "files[0].anchor.pattern"
        );
        assert_eq!(
            json["selector_validation"]["deferred_fields"][1],
            "files[0].anchor.find"
        );
    }

    #[test]
    fn validate_with_vars_reports_invalid_rendered_selector() {
        let yaml = r#"
variables:
  model_name:
    type: string
    required: true
files:
  - template: p.j2
    patch: target.py
    anchor:
      pattern: "^class {{ model_name }}:"
      scope: class_body
"#;
        let (_dir, path) = setup_recipe(yaml, &["p.j2"]);
        let err = cmd_validate(
            &path,
            Some(r#"{"model_name":"User["}"#),
            None,
            false,
            false,
            path.parent().unwrap(),
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(
            err.structured_error()
                .what
                .contains("invalid rendered selector regex")
        );
    }

    /// AC-7.3: render with --vars-file
    #[test]
    fn ac_7_3_render_with_vars_file() {
        let dir = TempDir::new().unwrap();
        let tmpl_path = dir.path().join("test.j2");
        fs::write(&tmpl_path, "Hello {{ name }}!").unwrap();
        let vars_path = dir.path().join("vars.json");
        fs::write(&vars_path, r#"{"name": "FileVars"}"#).unwrap();

        let out_path = dir.path().join("out.txt");
        let result = cmd_render(&tmpl_path, Some(&out_path), None, Some(&vars_path), false);
        assert_eq!(result.unwrap(), 0);
        assert_eq!(fs::read_to_string(&out_path).unwrap(), "Hello FileVars!");
    }

    /// Render with inline vars overriding file vars
    #[test]
    fn render_inline_overrides_file() {
        let dir = TempDir::new().unwrap();
        let tmpl_path = dir.path().join("test.j2");
        fs::write(&tmpl_path, "{{ x }}").unwrap();
        let vars_path = dir.path().join("vars.json");
        fs::write(&vars_path, r#"{"x": "from_file"}"#).unwrap();

        let out_path = dir.path().join("out.txt");
        let result = cmd_render(
            &tmpl_path,
            Some(&out_path),
            Some(r#"{"x": "from_inline"}"#),
            Some(&vars_path),
            false,
        );
        assert_eq!(result.unwrap(), 0);
        assert_eq!(fs::read_to_string(&out_path).unwrap(), "from_inline");
    }

    // ── cmd_run helpers ───────────────────────────────────────────

    /// Helper: create recipe + template with given content, return (dir, recipe_path).
    fn setup_run_recipe(yaml: &str, templates: &[(&str, &str)]) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let recipe_path = dir.path().join("recipe.yaml");
        fs::write(&recipe_path, yaml).unwrap();
        for (name, content) in templates {
            let p = dir.path().join(name);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&p, content).unwrap();
        }
        (dir, recipe_path)
    }

    fn run_recipe(
        recipe_path: &std::path::Path,
        vars: &str,
        base_dir: &std::path::Path,
        dry_run: bool,
        force: bool,
        verbose: bool,
    ) -> Result<i32, JigError> {
        cmd_run(
            recipe_path,
            Some(vars),
            None,
            false,
            dry_run,
            true, // force_json for deterministic output in tests
            true, // quiet — suppress stderr in tests
            force,
            Some(base_dir),
            verbose,
            base_dir,
            None,
            None,
        )
    }

    // ── AC-7.5: jig run executes all file operations in declaration order ──

    #[test]
    fn ac_7_5_run_executes_operations() {
        let yaml = r#"
variables:
  name:
    type: string
    required: true
files:
  - template: greeting.j2
    to: "greetings/{{ name | snakecase }}.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("greeting.j2", "Hello {{ name }}!")]);
        let out_dir = dir.path().join("output");
        fs::create_dir_all(&out_dir).unwrap();
        let result = run_recipe(
            &recipe_path,
            r#"{"name": "BookingService"}"#,
            &out_dir,
            false,
            false,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out_dir.join("greetings/booking_service.txt")).unwrap();
        assert_eq!(content, "Hello BookingService!");
    }

    #[test]
    fn run_supports_templated_patch_anchor_patterns() {
        let yaml = r#"
variables:
  model_name:
    type: string
    required: true
files:
  - template: manager.j2
    patch: "models.py"
    anchor:
      pattern: "^class {{ model_name | regex_escape }}:"
      scope: class_body
      position: before_close
"#;
        let (dir, recipe_path) =
            setup_run_recipe(yaml, &[("manager.j2", "    objects = EntityManager()\n")]);
        let out_dir = dir.path().join("output");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(out_dir.join("models.py"), "class Entity:\n    pass\n").unwrap();

        let result = run_recipe(
            &recipe_path,
            r#"{"model_name": "Entity"}"#,
            &out_dir,
            false,
            false,
            false,
        );

        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out_dir.join("models.py")).unwrap();
        assert!(content.contains("objects = EntityManager()"));
    }

    #[test]
    fn run_supports_templated_patch_anchor_find() {
        let yaml = r#"
variables:
  model_name:
    type: string
    required: true
  member_name:
    type: string
    required: true
files:
  - template: field.j2
    patch: "admin.py"
    anchor:
      pattern: "^class {{ model_name | regex_escape }}:"
      scope: class_body
      find: "{{ member_name }}"
      position: before_close
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("field.j2", "        'status',\n")]);
        let out_dir = dir.path().join("output");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(
            out_dir.join("admin.py"),
            "class EntityAdmin:\n    list_display = [\n        'name',\n    ]\n",
        )
        .unwrap();

        let result = run_recipe(
            &recipe_path,
            r#"{"model_name": "EntityAdmin", "member_name": "list_display"}"#,
            &out_dir,
            false,
            false,
            false,
        );

        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out_dir.join("admin.py")).unwrap();
        assert!(content.contains("        'status',"));
    }

    #[test]
    fn run_supports_templated_inject_selectors() {
        let yaml = r#"
variables:
  function_name:
    type: string
    required: true
files:
  - template: log.j2
    inject: "service.py"
    before: "^def {{ function_name | regex_escape }}\\("
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("log.j2", "# inserted\n")]);
        let out_dir = dir.path().join("output");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(
            out_dir.join("service.py"),
            "def create_record(entity_id):\n    return entity_id\n",
        )
        .unwrap();

        let result = run_recipe(
            &recipe_path,
            r#"{"function_name": "create_record"}"#,
            &out_dir,
            false,
            false,
            false,
        );

        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out_dir.join("service.py")).unwrap();
        assert!(content.starts_with("# inserted\ndef create_record"));
    }

    #[test]
    fn run_supports_templated_replace_between_selectors() {
        let yaml = r#"
variables:
  section_name:
    type: string
    required: true
files:
  - template: block.j2
    replace: "notes.txt"
    between:
      start: "^# START {{ section_name | regex_escape }}$"
      end: "^# END {{ section_name | regex_escape }}$"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("block.j2", "new line\n")]);
        let out_dir = dir.path().join("output");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(
            out_dir.join("notes.txt"),
            "# START entities\nold line\n# END entities\n",
        )
        .unwrap();

        let result = run_recipe(
            &recipe_path,
            r#"{"section_name": "entities"}"#,
            &out_dir,
            false,
            false,
            false,
        );

        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out_dir.join("notes.txt")).unwrap();
        assert!(content.contains("new line"));
        assert!(!content.contains("old line"));
    }

    #[test]
    fn run_reports_invalid_rendered_selector_regexes() {
        let yaml = r#"
variables:
  model_name:
    type: string
    required: true
files:
  - template: manager.j2
    patch: "models.py"
    anchor:
      pattern: "^class {{ model_name }}:"
      scope: class_body
"#;
        let (dir, recipe_path) =
            setup_run_recipe(yaml, &[("manager.j2", "    objects = EntityManager()\n")]);
        let out_dir = dir.path().join("output");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(out_dir.join("models.py"), "class User[:\n    pass\n").unwrap();

        let code = run_recipe(
            &recipe_path,
            r#"{"model_name": "User["}"#,
            &out_dir,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(code, 2);
    }

    // ── AC-4.1 + AC-4.2: Create writes rendered content with templated path ──

    #[test]
    fn ac_4_1_4_2_run_creates_file_at_templated_path() {
        let yaml = r#"
variables:
  class_name:
    type: string
    required: true
files:
  - template: svc.j2
    to: "src/{{ class_name | snakecase }}.rs"
"#;
        let (dir, recipe_path) =
            setup_run_recipe(yaml, &[("svc.j2", "pub struct {{ class_name }};")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(
            &recipe_path,
            r#"{"class_name": "BookingService"}"#,
            &out,
            false,
            false,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out.join("src/booking_service.rs")).unwrap();
        assert_eq!(content, "pub struct BookingService;");
    }

    // ── AC-4.3: Parent directories created automatically ──

    #[test]
    fn ac_4_3_run_creates_parent_dirs() {
        let yaml = r#"
files:
  - template: t.j2
    to: "deep/nested/path/file.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "content")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        assert!(out.join("deep/nested/path/file.txt").exists());
    }

    // ── AC-4.4: skip_if_exists skips with reason ──

    #[test]
    fn ac_4_4_run_skip_if_exists() {
        let yaml = r#"
files:
  - template: t.j2
    to: "existing.txt"
    skip_if_exists: true
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "new content")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("existing.txt"), "old content").unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        // File unchanged.
        assert_eq!(
            fs::read_to_string(out.join("existing.txt")).unwrap(),
            "old content"
        );
    }

    // ── AC-4.5: File exists without force → exit 3 ──

    #[test]
    fn ac_4_5_run_file_exists_error() {
        let yaml = r#"
files:
  - template: t.j2
    to: "existing.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "new")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("existing.txt"), "old").unwrap();
        let code = run_recipe(&recipe_path, "{}", &out, false, false, false).unwrap();
        assert_eq!(code, 3);
    }

    // ── AC-4.6: --force overwrites ──

    #[test]
    fn ac_4_6_run_force_overwrite() {
        let yaml = r#"
files:
  - template: t.j2
    to: "existing.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "new content")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("existing.txt"), "old").unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, false, true, false);
        assert_eq!(result.unwrap(), 0);
        assert_eq!(
            fs::read_to_string(out.join("existing.txt")).unwrap(),
            "new content"
        );
    }

    // ── AC-4.7: --base-dir changes output root ──

    #[test]
    fn ac_4_7_run_base_dir() {
        let yaml = r#"
files:
  - template: t.j2
    to: "output.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "hello")]);
        let custom_base = dir.path().join("custom");
        fs::create_dir(&custom_base).unwrap();
        let result = run_recipe(&recipe_path, "{}", &custom_base, false, false, false);
        assert_eq!(result.unwrap(), 0);
        assert!(custom_base.join("output.txt").exists());
    }

    // ── AC-4.10: --base-dir nonexistent directory → exit 3 ──

    #[test]
    fn ac_4_10_run_base_dir_not_found() {
        let yaml = "files: []\n";
        let (dir, recipe_path) = setup_run_recipe(yaml, &[]);
        let nonexistent = dir.path().join("does_not_exist");
        let code = cmd_run(
            &recipe_path,
            Some("{}"),
            None,
            false,
            false,
            true,
            true,
            false,
            Some(&nonexistent),
            false,
            dir.path(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(code, 3);
    }

    // ── AC-6.8: --dry-run writes nothing ──

    #[test]
    fn ac_6_8_run_dry_run() {
        let yaml = r#"
files:
  - template: t.j2
    to: "output.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "content")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, true, false, false);
        assert_eq!(result.unwrap(), 0);
        assert!(!out.join("output.txt").exists());
    }

    // ── AC-6.10: Fail-fast — stop on first error ──

    #[test]
    fn ac_6_10_fail_fast() {
        let yaml = r#"
files:
  - template: t.j2
    to: "existing.txt"
  - template: t.j2
    to: "should_not_exist.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "content")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("existing.txt"), "old").unwrap();
        let code = run_recipe(&recipe_path, "{}", &out, false, false, false).unwrap();
        assert_eq!(code, 3);
        // Second file should not have been created.
        assert!(!out.join("should_not_exist.txt").exists());
    }

    // ── AC-N6.1: Operations execute in declaration order ──

    #[test]
    fn ac_n6_1_declaration_order() {
        let yaml = r#"
files:
  - template: a.j2
    to: "first.txt"
  - template: b.j2
    to: "second.txt"
  - template: c.j2
    to: "third.txt"
"#;
        let (dir, recipe_path) =
            setup_run_recipe(yaml, &[("a.j2", "aaa"), ("b.j2", "bbb"), ("c.j2", "ccc")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        assert_eq!(fs::read_to_string(out.join("first.txt")).unwrap(), "aaa");
        assert_eq!(fs::read_to_string(out.join("second.txt")).unwrap(), "bbb");
        assert_eq!(fs::read_to_string(out.join("third.txt")).unwrap(), "ccc");
    }

    // ── AC-N2.1: Second run with skip_if_exists reports all skips ──

    #[test]
    fn ac_n2_1_idempotent_second_run() {
        let yaml = r#"
files:
  - template: t.j2
    to: "output.txt"
    skip_if_exists: true
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "content")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();

        // First run creates.
        let r1 = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(r1.unwrap(), 0);
        assert!(out.join("output.txt").exists());

        // Second run skips.
        let r2 = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(r2.unwrap(), 0);
        // File content unchanged.
        assert_eq!(
            fs::read_to_string(out.join("output.txt")).unwrap(),
            "content"
        );
    }

    // ── AC-1.11: Empty files array → exit 0, no operations ──

    #[test]
    fn ac_1_11_run_empty_files() {
        let yaml = "files: []\n";
        let (dir, recipe_path) = setup_run_recipe(yaml, &[]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
    }

    // ── AC-N5.2: Pipeline stage ordering — recipe (1) before vars (4) before render (2) ──

    #[test]
    fn ac_n5_2_pipeline_stage_order_vars_before_render() {
        // Recipe is valid but variable is missing → exit 4.
        let yaml = r#"
variables:
  name:
    type: string
    required: true
files:
  - template: t.j2
    to: "out.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "{{ name }}")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let code = run_recipe(&recipe_path, "{}", &out, false, false, false).unwrap();
        assert_eq!(code, 4);
    }

    // ── AC-7.6: All global options accepted by run subcommand ──

    #[test]
    fn ac_7_6_run_accepts_all_global_options() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let flags = ["dry_run", "json", "quiet", "force", "base_dir", "verbose"];
        for flag in &flags {
            assert!(
                cmd.get_arguments().any(|a| a.get_id() == *flag),
                "missing global option: {flag}"
            );
        }
    }

    // ── AC-4.8: Success reports line count ──

    #[test]
    fn ac_4_8_run_reports_line_count() {
        let yaml = r#"
files:
  - template: t.j2
    to: "out.txt"
"#;
        // minijinja strips trailing newline from template source, so content won't have trailing \n.
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "line 1\nline 2\nline 3")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        let written = fs::read_to_string(out.join("out.txt")).unwrap();
        assert_eq!(written, "line 1\nline 2\nline 3");
        assert_eq!(written.lines().count(), 3);
    }

    // ── Template rendering error exits 2 ──

    #[test]
    fn template_rendering_error_exits_2() {
        let yaml = r#"
files:
  - template: t.j2
    to: "out.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("t.j2", "{{ undefined_var }}")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let code = run_recipe(&recipe_path, "{}", &out, false, false, false).unwrap();
        assert_eq!(code, 2);
    }

    // ── Multiple creates in one recipe ──

    #[test]
    fn multiple_creates_in_one_recipe() {
        let yaml = r#"
variables:
  name:
    type: string
files:
  - template: a.j2
    to: "{{ name }}_a.txt"
  - template: b.j2
    to: "{{ name }}_b.txt"
"#;
        let (dir, recipe_path) = setup_run_recipe(
            yaml,
            &[
                ("a.j2", "content A for {{ name }}"),
                ("b.j2", "content B for {{ name }}"),
            ],
        );
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(
            &recipe_path,
            r#"{"name": "test"}"#,
            &out,
            false,
            false,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        assert_eq!(
            fs::read_to_string(out.join("test_a.txt")).unwrap(),
            "content A for test"
        );
        assert_eq!(
            fs::read_to_string(out.join("test_b.txt")).unwrap(),
            "content B for test"
        );
    }

    // ── AC-N6.2: Create-then-inject in same recipe ──

    #[test]
    fn ac_n6_2_create_then_inject() {
        let yaml = r#"
variables:
  class_name:
    type: string
    required: true
files:
  - template: service.j2
    to: "src/{{ class_name | snakecase }}.py"
  - template: import.j2
    inject: "src/{{ class_name | snakecase }}.py"
    after: "^# imports"
"#;
        let (dir, recipe_path) = setup_run_recipe(
            yaml,
            &[
                (
                    "service.j2",
                    "# imports\n\nclass {{ class_name }}:\n    pass",
                ),
                ("import.j2", "import json"),
            ],
        );
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(
            &recipe_path,
            r#"{"class_name": "BookingService"}"#,
            &out,
            false,
            false,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out.join("src/booking_service.py")).unwrap();
        assert!(content.contains("# imports"));
        assert!(content.contains("import json"));
        assert!(content.contains("class BookingService:"));
        // Verify order: import json comes after # imports.
        let lines: Vec<&str> = content.lines().collect();
        let imports_idx = lines.iter().position(|l| l.contains("# imports")).unwrap();
        let json_idx = lines
            .iter()
            .position(|l| l.contains("import json"))
            .unwrap();
        assert!(
            json_idx == imports_idx + 1,
            "import json should be right after # imports"
        );
    }

    // ── AC-N6.2: Create-then-inject in dry-run mode ──

    #[test]
    fn ac_n6_2_create_then_inject_dry_run() {
        let yaml = r#"
files:
  - template: base.j2
    to: "output.txt"
  - template: extra.j2
    inject: "output.txt"
    append: true
"#;
        let (dir, recipe_path) = setup_run_recipe(
            yaml,
            &[
                ("base.j2", "base content"),
                ("extra.j2", "appended content"),
            ],
        );
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, "{}", &out, true, false, false);
        assert_eq!(result.unwrap(), 0);
        // Nothing on disk.
        assert!(!out.join("output.txt").exists());
    }

    // ── AC-5.1: Inject after pattern via full pipeline ──

    #[test]
    fn ac_5_1_inject_after_via_run() {
        let yaml = r#"
files:
  - template: fixture.j2
    inject: "conftest.py"
    after: "^# fixtures"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("fixture.j2", "fixture_new = 42")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("conftest.py"), "# fixtures\nfixture_a = 1\n").unwrap();

        let result = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out.join("conftest.py")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "# fixtures");
        assert_eq!(lines[1], "fixture_new = 42");
        assert_eq!(lines[2], "fixture_a = 1");
    }

    // ── AC-5.7: skip_if via full pipeline ──

    #[test]
    fn ac_5_7_skip_if_via_run() {
        let yaml = r#"
variables:
  class_name:
    type: string
    required: true
files:
  - template: import.j2
    inject: "app.py"
    append: true
    skip_if: "{{ class_name }}"
"#;
        let (dir, recipe_path) = setup_run_recipe(
            yaml,
            &[("import.j2", "from services import {{ class_name }}")],
        );
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("app.py"), "from services import BookingService\n").unwrap();

        let result = run_recipe(
            &recipe_path,
            r#"{"class_name": "BookingService"}"#,
            &out,
            false,
            false,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        // File should be unchanged — skip_if matched.
        let content = fs::read_to_string(out.join("app.py")).unwrap();
        assert_eq!(content, "from services import BookingService\n");
    }

    // ── AC-N2.1 + AC-N2.2: Idempotent create+inject second run all skips ──

    #[test]
    fn ac_n2_1_idempotent_create_inject() {
        let yaml = r#"
variables:
  name:
    type: string
    required: true
files:
  - template: base.j2
    to: "service.py"
    skip_if_exists: true
  - template: import.j2
    inject: "service.py"
    after: "^# imports"
    skip_if: "from utils import {{ name }}"
"#;
        let (dir, recipe_path) = setup_run_recipe(
            yaml,
            &[
                ("base.j2", "# imports\n\nclass {{ name }}:\n    pass"),
                ("import.j2", "from utils import {{ name }}"),
            ],
        );
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();

        // First run: creates file and injects.
        let r1 = run_recipe(
            &recipe_path,
            r#"{"name": "BookingService"}"#,
            &out,
            false,
            false,
            false,
        );
        assert_eq!(r1.unwrap(), 0);
        let content_after_first = fs::read_to_string(out.join("service.py")).unwrap();
        assert!(content_after_first.contains("from utils import BookingService"));

        // Second run: all skip.
        let r2 = run_recipe(
            &recipe_path,
            r#"{"name": "BookingService"}"#,
            &out,
            false,
            false,
            false,
        );
        assert_eq!(r2.unwrap(), 0);
        // Content unchanged — no duplicates.
        let content_after_second = fs::read_to_string(out.join("service.py")).unwrap();
        assert_eq!(content_after_first, content_after_second);
    }

    // ── AC-5.8: Regex no-match via full pipeline exits 3 ──

    #[test]
    fn ac_5_8_regex_no_match_exits_3() {
        let yaml = r#"
files:
  - template: content.j2
    inject: "target.py"
    after: "^NONEXISTENT_PATTERN"
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("content.j2", "injected")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("target.py"), "line1\nline2\n").unwrap();

        let code = run_recipe(&recipe_path, "{}", &out, false, false, false).unwrap();
        assert_eq!(code, 3);
    }

    // ── AC-5.9: Missing inject target via full pipeline exits 3 ──

    #[test]
    fn ac_5_9_missing_target_exits_3() {
        let yaml = r#"
files:
  - template: content.j2
    inject: "nonexistent.py"
    append: true
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("content.j2", "injected")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();

        let code = run_recipe(&recipe_path, "{}", &out, false, false, false).unwrap();
        assert_eq!(code, 3);
    }

    // ── AC-5.11: Templated inject path via full pipeline ──

    #[test]
    fn ac_5_11_templated_inject_path_via_run() {
        let yaml = r#"
variables:
  module:
    type: string
    required: true
files:
  - template: content.j2
    inject: "{{ module }}/init.py"
    append: true
"#;
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("content.j2", "# added")]);
        let out = dir.path().join("out");
        fs::create_dir_all(out.join("mymodule")).unwrap();
        fs::write(out.join("mymodule/init.py"), "# existing\n").unwrap();

        let result = run_recipe(
            &recipe_path,
            r#"{"module": "mymodule"}"#,
            &out,
            false,
            false,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out.join("mymodule/init.py")).unwrap();
        assert!(content.contains("# added"));
    }

    // ── AC-5.3 + AC-5.4: Prepend and append via full pipeline ──

    #[test]
    fn ac_5_3_5_4_prepend_append_via_run() {
        let yaml = r#"
files:
  - template: header.j2
    inject: "target.txt"
    prepend: true
  - template: footer.j2
    inject: "target.txt"
    append: true
"#;
        let (dir, recipe_path) = setup_run_recipe(
            yaml,
            &[
                ("header.j2", "=== HEADER ==="),
                ("footer.j2", "=== FOOTER ==="),
            ],
        );
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("target.txt"), "middle content\n").unwrap();

        let result = run_recipe(&recipe_path, "{}", &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out.join("target.txt")).unwrap();
        assert!(content.starts_with("=== HEADER ===\n"));
        assert!(content.ends_with("=== FOOTER ==="));
        assert!(content.contains("middle content"));
    }

    // ── jig list --skills ──

    #[test]
    fn parse_skill_frontmatter_simple() {
        let content =
            "---\nname: add-endpoint\ndescription: Add a REST endpoint\n---\n# Add Endpoint\n";
        let (name, desc) = super::parse_skill_frontmatter(content);
        assert_eq!(name.unwrap(), "add-endpoint");
        assert_eq!(desc.unwrap(), "Add a REST endpoint");
    }

    #[test]
    fn parse_skill_frontmatter_block_scalar() {
        let content = "---\nname: review\ndescription: |\n  Adversarial code review.\n  Multi-line description.\n---\n";
        let (name, desc) = super::parse_skill_frontmatter(content);
        assert_eq!(name.unwrap(), "review");
        assert_eq!(desc.unwrap(), "Adversarial code review.");
    }

    #[test]
    fn parse_skill_frontmatter_missing() {
        let content = "# No frontmatter\nJust a markdown file.\n";
        let (name, desc) = super::parse_skill_frontmatter(content);
        assert!(name.is_none());
        assert!(desc.is_none());
    }

    #[test]
    fn scan_skills_dir_finds_skills() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join(".claude/skills");
        let skill_a = skills_dir.join("add-field");
        let skill_b = skills_dir.join("add-view");
        fs::create_dir_all(&skill_a).unwrap();
        fs::create_dir_all(&skill_b).unwrap();
        fs::write(
            skill_a.join("SKILL.md"),
            "---\nname: add-field\ndescription: Add a model field\n---\n",
        )
        .unwrap();
        fs::write(
            skill_b.join("SKILL.md"),
            "---\nname: add-view\ndescription: Add a view\n---\n",
        )
        .unwrap();

        let results = super::scan_skills_dir(&skills_dir, tmp.path());
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "add-field");
        assert_eq!(results[0].description.as_deref(), Some("Add a model field"));
        assert_eq!(results[1].name, "add-view");
    }

    #[test]
    fn scan_skills_dir_uses_dir_name_without_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join("skills/my-skill");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# My Skill\nNo frontmatter here.\n").unwrap();

        let results = super::scan_skills_dir(&tmp.path().join("skills"), tmp.path());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "my-skill"); // falls back to dir name
        assert!(results[0].description.is_none());
    }

    #[test]
    fn find_skills_dirs_recursive() {
        let tmp = TempDir::new().unwrap();
        // .claude/skills/ with a skill
        let claude_skills = tmp.path().join(".claude/skills/test-skill");
        fs::create_dir_all(&claude_skills).unwrap();
        fs::write(claude_skills.join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        // top-level skills/ with a skill
        let top_skills = tmp.path().join("skills/another");
        fs::create_dir_all(&top_skills).unwrap();
        fs::write(top_skills.join("SKILL.md"), "---\nname: another\n---\n").unwrap();

        let dirs = super::find_skills_dirs(tmp.path());
        assert_eq!(dirs.len(), 2);
    }
}
