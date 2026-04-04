mod error;
mod filters;
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

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a recipe and report whether it is valid
    Validate {
        /// Path to the recipe YAML file
        recipe: PathBuf,
        /// Output validation result as JSON to stdout
        #[arg(long)]
        json: bool,
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
        Commands::Validate { recipe, json } => cmd_validate(&recipe, json),
        Commands::Vars { recipe } => cmd_vars(&recipe),
        Commands::Render { template, to } => {
            cmd_render(&template, to.as_deref(), cli.vars.as_deref(), cli.vars_file.as_deref(), cli.vars_stdin)
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
}
