mod error;
mod filters;
mod operations;
mod output;
mod recipe;
mod renderer;
mod variables;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use crate::error::JigError;
use crate::recipe::Recipe;

#[derive(Parser)]
#[command(name = "jig", version, about = "Template rendering CLI for LLM code generation workflows")]
struct Cli {
    /// Inline variables as JSON string
    #[arg(long, global = true)]
    vars: Option<String>,

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
    match cli.command {
        Commands::Validate { recipe } => cmd_validate(&recipe, cli.json),
        Commands::Vars { recipe } => cmd_vars(&recipe),
        Commands::Render { template, to } => {
            cmd_render(&template, to.as_deref(), cli.vars.as_deref(), cli.vars_file.as_deref(), cli.vars_stdin)
        }
        Commands::Run { recipe } => {
            cmd_run(
                &recipe,
                cli.vars.as_deref(),
                cli.vars_file.as_deref(),
                cli.vars_stdin,
                cli.dry_run,
                cli.json,
                cli.quiet,
                cli.force,
                cli.base_dir.as_deref(),
                cli.verbose,
            )
        }
    }
}

fn cmd_validate(path: &std::path::Path, json: bool) -> Result<i32, JigError> {
    let recipe = Recipe::load(path)?;

    if json {
        let output = build_validate_json(&recipe);
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
    }

    Ok(0)
}

fn cmd_vars(path: &std::path::Path) -> Result<i32, JigError> {
    let recipe = Recipe::load(path)?;
    let json = variables::vars_json(&recipe.variables);
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    Ok(0)
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
    let vars = match variables::validate_variables(&recipe.variables, &provided) {
        Ok(v) => v,
        Err(e) => return handle_early_error(e),
    };

    // 3. Create recipe-aware environment and render ALL templates upfront (exit 2 on failure).
    let env = match renderer::create_recipe_env(&recipe) {
        Ok(e) => e,
        Err(e) => return handle_early_error(e),
    };

    let mut prepared_ops = Vec::with_capacity(recipe.files.len());
    for (i, file_op) in recipe.files.iter().enumerate() {
        // Render the template content.
        let rendered_content = match renderer::render_template(&env, file_op.template(), &vars) {
            Ok(c) => c,
            Err(e) => return handle_early_error(e),
        };

        // Render templated path fields.
        let rendered_path = match file_op {
            recipe::FileOp::Create { to, .. } => {
                match renderer::render_path_template(&env, to, &vars, &format!("files[{}].to", i)) {
                    Ok(p) => p,
                    Err(e) => return handle_early_error(e),
                }
            }
            recipe::FileOp::Inject { inject, .. } => {
                match renderer::render_path_template(&env, inject, &vars, &format!("files[{}].inject", i)) {
                    Ok(p) => p,
                    Err(e) => return handle_early_error(e),
                }
            }
        };

        // Render skip_if for inject operations.
        let rendered_skip_if = match file_op {
            recipe::FileOp::Inject { skip_if: Some(skip_if_expr), .. } => {
                match renderer::render_path_template(
                    &env, skip_if_expr, &vars, &format!("files[{}].skip_if", i),
                ) {
                    Ok(s) => Some(s),
                    Err(e) => return handle_early_error(e),
                }
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
    if has_error {
        if let Some(last) = results.last()
            && let Some(jig_err) = operations::op_error_to_jig_error(last) {
                return Ok(jig_err.exit_code());
            }
    }

    Ok(0)
}

fn build_validate_json(recipe: &Recipe) -> serde_json::Value {
    let vars = variables::vars_json(&recipe.variables);

    let ops: Vec<serde_json::Value> = recipe.files.iter().map(|op| {
        let mut m = serde_json::Map::new();
        m.insert("type".into(), serde_json::Value::String(op.op_type_str().into()));
        match op {
            recipe::FileOp::Create { to, .. } => {
                m.insert("to".into(), serde_json::Value::String(to.clone()));
            }
            recipe::FileOp::Inject { inject, .. } => {
                m.insert("inject".into(), serde_json::Value::String(inject.clone()));
            }
        }
        serde_json::Value::Object(m)
    }).collect();

    serde_json::json!({
        "valid": true,
        "name": recipe.name,
        "description": recipe.description,
        "variables": vars,
        "operations": ops,
    })
}

fn summarize_op_types(recipe: &Recipe) -> String {
    let mut create_count = 0usize;
    let mut inject_count = 0usize;
    for op in &recipe.files {
        match op {
            recipe::FileOp::Create { .. } => create_count += 1,
            recipe::FileOp::Inject { .. } => inject_count += 1,
        }
    }
    let mut parts = Vec::new();
    if create_count > 0 {
        parts.push(format!("{create_count} create"));
    }
    if inject_count > 0 {
        parts.push(format!("{inject_count} inject"));
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

    /// AC-7.1: jig validate parses recipe and reports validity
    #[test]
    fn ac_7_1_validate_command_valid() {
        let yaml = "name: test\nvariables:\n  name:\n    type: string\n    required: true\nfiles:\n  - template: t.j2\n    to: out.rs\n";
        let (_dir, path) = setup_recipe(yaml, &["t.j2"]);
        let result = cmd_validate(&path, false);
        assert_eq!(result.unwrap(), 0);
    }

    /// AC-7.1: jig validate --json outputs structured JSON with variables and operations
    #[test]
    fn ac_7_1_validate_json_output() {
        let yaml = "name: test\nvariables:\n  name:\n    type: string\nfiles:\n  - template: t.j2\n    to: out.rs\n";
        let (_dir, path) = setup_recipe(yaml, &["t.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        let json = build_validate_json(&recipe);
        assert_eq!(json["valid"], true);
        assert!(json["variables"]["name"].is_object());
        assert_eq!(json["operations"][0]["type"], "create");
    }

    /// AC-7.1: jig validate exits 1 for invalid recipe
    #[test]
    fn ac_7_1_validate_command_invalid() {
        let yaml = "bad yaml [";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let err = cmd_validate(&path, false).unwrap_err();
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
        let result = cmd_render(
            &tmpl_path,
            Some(&out_path),
            Some("{}"),
            None,
            false,
        );
        assert_eq!(result.unwrap(), 0);
        assert!(out_path.exists());
    }

    /// AC-7.6: --json flag works on validate (partial — other flags tested in later phases)
    #[test]
    fn ac_7_6_json_flag_exists() {
        let yaml = "files: []\n";
        let (_dir, path) = setup_recipe(yaml, &[]);
        let result = cmd_validate(&path, true);
        assert_eq!(result.unwrap(), 0);
    }

    /// AC-7.6: --vars, --vars-file, --vars-stdin are accepted global options
    #[test]
    fn ac_7_6_var_options_exist() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        // Check that the global options exist
        assert!(cmd.get_arguments().any(|a| a.get_id() == "vars"));
        assert!(cmd.get_arguments().any(|a| a.get_id() == "vars_file"));
        assert!(cmd.get_arguments().any(|a| a.get_id() == "vars_stdin"));
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
        let err = cmd_validate(&path, false).unwrap_err();
        assert_eq!(err.exit_code(), 1);
    }

    /// AC-7.1: validate output includes variable names and operation types
    #[test]
    fn ac_7_1_validate_json_lists_vars_and_ops() {
        let yaml = "variables:\n  name:\n    type: string\n  count:\n    type: number\nfiles:\n  - template: a.j2\n    to: out_a.rs\n  - template: b.j2\n    inject: target.rs\n    append: true\n";
        let (_dir, path) = setup_recipe(yaml, &["a.j2", "b.j2"]);
        let recipe = Recipe::load(&path).unwrap();
        let json = build_validate_json(&recipe);
        assert!(json["variables"]["name"].is_object());
        assert!(json["variables"]["count"].is_object());
        assert_eq!(json["operations"][0]["type"], "create");
        assert_eq!(json["operations"][1]["type"], "inject");
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
        let result = cmd_render(
            &tmpl_path,
            Some(&out_path),
            None,
            Some(&vars_path),
            false,
        );
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
        let result = run_recipe(&recipe_path, r#"{"name": "BookingService"}"#, &out_dir, false, false, false);
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out_dir.join("greetings/booking_service.txt")).unwrap();
        assert_eq!(content, "Hello BookingService!");
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[("svc.j2", "pub struct {{ class_name }};")]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, r#"{"class_name": "BookingService"}"#, &out, false, false, false);
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
        assert_eq!(fs::read_to_string(out.join("existing.txt")).unwrap(), "old content");
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
        assert_eq!(fs::read_to_string(out.join("existing.txt")).unwrap(), "new content");
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
            &recipe_path, Some("{}"), None, false,
            false, true, true, false,
            Some(&nonexistent), false,
        ).unwrap();
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("a.j2", "aaa"), ("b.j2", "bbb"), ("c.j2", "ccc"),
        ]);
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
        assert_eq!(fs::read_to_string(out.join("output.txt")).unwrap(), "content");
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("a.j2", "content A for {{ name }}"),
            ("b.j2", "content B for {{ name }}"),
        ]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, r#"{"name": "test"}"#, &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        assert_eq!(fs::read_to_string(out.join("test_a.txt")).unwrap(), "content A for test");
        assert_eq!(fs::read_to_string(out.join("test_b.txt")).unwrap(), "content B for test");
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("service.j2", "# imports\n\nclass {{ class_name }}:\n    pass"),
            ("import.j2", "import json"),
        ]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let result = run_recipe(&recipe_path, r#"{"class_name": "BookingService"}"#, &out, false, false, false);
        assert_eq!(result.unwrap(), 0);
        let content = fs::read_to_string(out.join("src/booking_service.py")).unwrap();
        assert!(content.contains("# imports"));
        assert!(content.contains("import json"));
        assert!(content.contains("class BookingService:"));
        // Verify order: import json comes after # imports.
        let lines: Vec<&str> = content.lines().collect();
        let imports_idx = lines.iter().position(|l| l.contains("# imports")).unwrap();
        let json_idx = lines.iter().position(|l| l.contains("import json")).unwrap();
        assert!(json_idx == imports_idx + 1, "import json should be right after # imports");
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("base.j2", "base content"),
            ("extra.j2", "appended content"),
        ]);
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("fixture.j2", "fixture_new = 42"),
        ]);
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("import.j2", "from services import {{ class_name }}"),
        ]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("app.py"), "from services import BookingService\n").unwrap();

        let result = run_recipe(&recipe_path, r#"{"class_name": "BookingService"}"#, &out, false, false, false);
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("base.j2", "# imports\n\nclass {{ name }}:\n    pass"),
            ("import.j2", "from utils import {{ name }}"),
        ]);
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();

        // First run: creates file and injects.
        let r1 = run_recipe(&recipe_path, r#"{"name": "BookingService"}"#, &out, false, false, false);
        assert_eq!(r1.unwrap(), 0);
        let content_after_first = fs::read_to_string(out.join("service.py")).unwrap();
        assert!(content_after_first.contains("from utils import BookingService"));

        // Second run: all skip.
        let r2 = run_recipe(&recipe_path, r#"{"name": "BookingService"}"#, &out, false, false, false);
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("content.j2", "injected"),
        ]);
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("content.j2", "injected"),
        ]);
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("content.j2", "# added"),
        ]);
        let out = dir.path().join("out");
        fs::create_dir_all(out.join("mymodule")).unwrap();
        fs::write(out.join("mymodule/init.py"), "# existing\n").unwrap();

        let result = run_recipe(&recipe_path, r#"{"module": "mymodule"}"#, &out, false, false, false);
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
        let (dir, recipe_path) = setup_run_recipe(yaml, &[
            ("header.j2", "=== HEADER ==="),
            ("footer.j2", "=== FOOTER ==="),
        ]);
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
}
