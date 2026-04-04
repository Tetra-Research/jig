use minijinja::{Environment, UndefinedBehavior};
use serde_json::Value;

use crate::error::{JigError, StructuredError};
use crate::filters;
use crate::recipe::Recipe;

/// Create a minijinja Environment configured for a recipe context.
/// Loads all templates from the recipe directory and registers filters.
#[allow(dead_code)] // Used in later phases (jig run)
pub fn create_recipe_env(recipe: &Recipe) -> Result<Environment<'static>, JigError> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    filters::register_all(&mut env);

    for (i, op) in recipe.files.iter().enumerate() {
        let tmpl_path = recipe.recipe_dir.join(op.template());
        let content = std::fs::read_to_string(&tmpl_path).map_err(|e| {
            JigError::TemplateRendering(StructuredError {
                what: format!("cannot read template file '{}'", op.template()),
                where_: format!("files[{}].template", i),
                why: e.to_string(),
                hint: "ensure the template file is readable".into(),
            })
        })?;
        env.add_template_owned(op.template().to_string(), content)
            .map_err(|e| template_syntax_error(op.template(), &e))?;
    }

    Ok(env)
}

/// Create a standalone minijinja Environment (for `jig render`).
/// No templates pre-loaded; use `render_string` for one-off rendering.
pub fn create_standalone_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    filters::register_all(&mut env);
    env
}

/// Render a named template that's already loaded in the environment.
#[allow(dead_code)] // Used in later phases (jig run)
pub fn render_template(
    env: &Environment,
    template_name: &str,
    vars: &Value,
) -> Result<String, JigError> {
    let tmpl = env.get_template(template_name).map_err(|e| {
        JigError::TemplateRendering(StructuredError {
            what: format!("template '{}' not found in environment", template_name),
            where_: template_name.to_string(),
            why: e.to_string(),
            hint: "ensure the template is loaded before rendering".into(),
        })
    })?;

    let source = tmpl.source();
    tmpl.render(vars)
        .map_err(|e| convert_render_error(&e, template_name, source, vars))
}

/// Render a template string directly (for `jig render` standalone mode).
pub fn render_string(
    env: &Environment,
    source: &str,
    vars: &Value,
    source_path: &str,
) -> Result<String, JigError> {
    env.render_str(source, vars)
        .map_err(|e| convert_render_error(&e, source_path, source, vars))
}

/// Render a template string and return a Jinja2 template expression result.
/// Used for rendering path templates like `to` and `inject` fields.
#[allow(dead_code)] // Used in later phases (jig run)
pub fn render_path_template(
    env: &Environment,
    template_expr: &str,
    vars: &Value,
    context_desc: &str,
) -> Result<String, JigError> {
    env.render_str(template_expr, vars)
        .map_err(|e| convert_render_error(&e, context_desc, template_expr, vars))
}

// ── Error conversion ───────────────────────────────────────────────

fn convert_render_error(
    err: &minijinja::Error,
    source_path: &str,
    template_source: &str,
    vars: &Value,
) -> JigError {
    let kind = err.kind();
    let line_info = err.line().map(|l| format!(" (line {})", l)).unwrap_or_default();

    match kind {
        minijinja::ErrorKind::UndefinedError => {
            // minijinja doesn't include the variable name in the error.
            // Extract it from the template source by finding references not in vars.
            let var_name = find_undefined_variable(template_source, err.line(), vars);
            let hint = did_you_mean_hint(&var_name, vars);

            JigError::TemplateRendering(StructuredError {
                what: format!("undefined variable '{}'", var_name),
                where_: format!("{}{}", source_path, line_info),
                why: format!("variable '{}' is not defined in the provided context", var_name),
                hint,
            })
        }
        minijinja::ErrorKind::SyntaxError => {
            let detail = err.detail().unwrap_or("syntax error");
            JigError::TemplateRendering(StructuredError {
                what: "template syntax error".into(),
                where_: format!("{}{}", source_path, line_info),
                why: detail.to_string(),
                hint: "check Jinja2 syntax — ensure all blocks are properly closed".into(),
            })
        }
        _ => {
            let detail = err.detail().unwrap_or("rendering error");
            JigError::TemplateRendering(StructuredError {
                what: "template rendering error".into(),
                where_: format!("{}{}", source_path, line_info),
                why: detail.to_string(),
                hint: "check template syntax and variable references".into(),
            })
        }
    }
}

#[allow(dead_code)] // Used in recipe env creation
fn template_syntax_error(template_name: &str, err: &minijinja::Error) -> JigError {
    let line_info = err.line().map(|l| format!(" (line {})", l)).unwrap_or_default();
    let detail = err.detail().unwrap_or("syntax error");

    JigError::TemplateRendering(StructuredError {
        what: "template syntax error".into(),
        where_: format!("{}{}", template_name, line_info),
        why: detail.to_string(),
        hint: "check Jinja2 syntax — ensure all blocks are properly closed".into(),
    })
}

/// Find the undefined variable in a template by scanning for references
/// not present in the variable context.
///
/// Limitation: this is a best-effort heuristic using regex. It extracts only the
/// first identifier from expressions like `{{ user.name }}` (yields "user", not
/// "user.name"). Falls back to "unknown" if no match is found. minijinja does not
/// expose the undefined variable name directly, so this is the best we can do for v0.1.
fn find_undefined_variable(template_source: &str, error_line: Option<usize>, vars: &Value) -> String {
    let var_keys: Vec<&str> = match vars.as_object() {
        Some(obj) => obj.keys().map(|s| s.as_str()).collect(),
        None => vec![],
    };

    // Extract variable references from the template.
    // Matches {{ name }}, {{ name | filter }}, {{ name.attr }}, etc.
    let re = regex::Regex::new(r"\{\{[\s-]*([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    // Also check {% if/for/elif blocks
    let block_re = regex::Regex::new(r"\{%[\s-]*(?:if|elif|for\s+\w+\s+in|set\s+\w+\s*=)\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

    let lines: Vec<&str> = template_source.lines().collect();

    // If we have a line number, check that line first
    if let Some(line_num) = error_line.filter(|&l| l > 0 && l <= lines.len()) {
        let line = lines[line_num - 1];
        for cap in re.captures_iter(line).chain(block_re.captures_iter(line)) {
            let name = &cap[1];
            if !var_keys.contains(&name) {
                return name.to_string();
            }
        }
    }

    // Fall back to scanning the entire template
    for cap in re.captures_iter(template_source).chain(block_re.captures_iter(template_source)) {
        let name = &cap[1];
        if !var_keys.contains(&name) {
            return name.to_string();
        }
    }

    "unknown".to_string()
}

/// Generate a "did you mean?" hint using Levenshtein distance.
/// Looks at all keys in the provided variable context.
fn did_you_mean_hint(var_name: &str, vars: &Value) -> String {
    let keys: Vec<&str> = match vars.as_object() {
        Some(obj) => obj.keys().map(|s| s.as_str()).collect(),
        None => return format!("check variable name '{var_name}'"),
    };

    if keys.is_empty() {
        return format!("check variable name '{}' — no variables were provided", var_name);
    }

    let mut best_match: Option<(&str, usize)> = None;
    for key in &keys {
        let dist = strsim::levenshtein(var_name, key);
        if dist <= 3 && (best_match.is_none() || dist < best_match.unwrap().1) {
            best_match = Some((key, dist));
        }
    }

    match best_match {
        Some((suggestion, _)) => format!("did you mean '{suggestion}'?"),
        None => format!("check variable name '{}' — available variables: {}", var_name, keys.join(", ")),
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn render(template: &str, vars: &Value) -> Result<String, JigError> {
        let env = create_standalone_env();
        render_string(&env, template, vars, "test.j2")
    }

    // ── AC-3.1: Variable substitution ──────────────────────────

    #[test]
    fn ac_3_1_variable_substitution() {
        let vars = json!({"class_name": "BookingService", "module": "bookings"});
        let result = render("class {{ class_name }} in {{ module }}", &vars).unwrap();
        assert_eq!(result, "class BookingService in bookings");
    }

    // ── AC-3.2: Conditional blocks ─────────────────────────────

    #[test]
    fn ac_3_2_conditionals() {
        let vars = json!({"auth": true});
        let tmpl = "{% if auth %}secured{% else %}open{% endif %}";
        assert_eq!(render(tmpl, &vars).unwrap(), "secured");

        let vars = json!({"auth": false});
        assert_eq!(render(tmpl, &vars).unwrap(), "open");
    }

    // ── AC-3.3: For loops ──────────────────────────────────────

    #[test]
    fn ac_3_3_for_loops() {
        let vars = json!({"items": ["alpha", "beta", "gamma"]});
        let tmpl = "{% for item in items %}{{ item }}\n{% endfor %}";
        let result = render(tmpl, &vars).unwrap();
        assert_eq!(result, "alpha\nbeta\ngamma\n");
    }

    // ── AC-3.15: Comments stripped ─────────────────────────────

    #[test]
    fn ac_3_15_comments_stripped() {
        let vars = json!({"x": "hello"});
        let result = render("{# this is a comment #}{{ x }}", &vars).unwrap();
        assert_eq!(result, "hello");
    }

    // ── AC-3.16: Raw blocks ────────────────────────────────────

    #[test]
    fn ac_3_16_raw_blocks() {
        let vars = json!({});
        let result = render("{% raw %}{{ not_rendered }}{% endraw %}", &vars).unwrap();
        assert_eq!(result, "{{ not_rendered }}");
    }

    // ── AC-3.17: Undefined variable with did-you-mean ──────────

    #[test]
    fn ac_3_17_undefined_variable_did_you_mean() {
        let vars = json!({"class_name": "Foo", "module_name": "bar"});
        let err = render("{{ clss_name }}", &vars).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        let se = err.structured_error();
        assert!(se.what.contains("undefined variable"), "what was: {}", se.what);
        assert!(se.what.contains("clss_name"), "what was: {}", se.what);
        assert!(se.hint.contains("did you mean"), "hint was: {}", se.hint);
        assert!(se.hint.contains("class_name"), "hint was: {}", se.hint);
    }

    #[test]
    fn ac_3_17_undefined_variable_no_close_match() {
        let vars = json!({"foo": "bar"});
        let err = render("{{ completely_different }}", &vars).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        let se = err.structured_error();
        assert!(se.what.contains("undefined variable"));
        assert!(se.hint.contains("foo"), "hint was: {}", se.hint);
    }

    // ── AC-3.18: Syntax error with file + line ─────────────────

    #[test]
    fn ac_3_18_syntax_error() {
        let vars = json!({});
        let err = render("{% if unclosed %}", &vars).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        let se = err.structured_error();
        assert!(se.what.contains("syntax error"));
        assert!(se.where_.contains("test.j2"));
    }

    // ── AC-N1.1: Deterministic output ──────────────────────────

    #[test]
    fn ac_n1_1_deterministic() {
        let vars = json!({"name": "Foo", "items": ["a", "b"]});
        let tmpl = "{{ name | snakecase }}\n{% for x in items %}{{ x }}\n{% endfor %}";
        let r1 = render(tmpl, &vars).unwrap();
        let r2 = render(tmpl, &vars).unwrap();
        assert_eq!(r1, r2, "output must be byte-identical across runs");
    }

    // ── AC-N1.2: No timestamps or random values ────────────────

    #[test]
    fn ac_n1_2_no_nondeterminism() {
        let vars = json!({"name": "Test"});
        let result = render("{{ name }}", &vars).unwrap();
        assert_eq!(result, "Test");
    }

    // ── did-you-mean unit tests ────────────────────────────────

    #[test]
    fn did_you_mean_close_match() {
        let vars = json!({"class_name": "Foo", "module": "bar"});
        let hint = did_you_mean_hint("clss_name", &vars);
        assert!(hint.contains("class_name"));
        assert!(hint.starts_with("did you mean"));
    }

    #[test]
    fn did_you_mean_no_match() {
        let vars = json!({"x": 1});
        let hint = did_you_mean_hint("completely_unrelated_variable_name", &vars);
        assert!(!hint.starts_with("did you mean"));
        assert!(hint.contains("x"));
    }

    // ── find_undefined_variable ────────────────────────────────

    #[test]
    fn find_undefined_in_expression() {
        let vars = json!({"foo": 1});
        let name = find_undefined_variable("{{ bar }}", None, &vars);
        assert_eq!(name, "bar");
    }

    #[test]
    fn find_undefined_skips_defined() {
        let vars = json!({"foo": 1});
        let name = find_undefined_variable("{{ foo }}{{ bar }}", None, &vars);
        assert_eq!(name, "bar");
    }

    #[test]
    fn find_undefined_with_filter() {
        let vars = json!({"foo": 1});
        let name = find_undefined_variable("{{ baz | snakecase }}", None, &vars);
        assert_eq!(name, "baz");
    }

    // ── Recipe environment ─────────────────────────────────────

    #[test]
    fn recipe_env_loads_templates() {
        use std::fs;
        use tempfile::TempDir;
        use crate::recipe::Recipe;

        let dir = TempDir::new().unwrap();
        let recipe_yaml = "files:\n  - template: hello.j2\n    to: out.txt\n";
        let recipe_path = dir.path().join("recipe.yaml");
        fs::write(&recipe_path, recipe_yaml).unwrap();
        fs::write(dir.path().join("hello.j2"), "Hello {{ name }}!").unwrap();

        let recipe = Recipe::load(&recipe_path).unwrap();
        let env = create_recipe_env(&recipe).unwrap();

        let vars = json!({"name": "World"});
        let result = render_template(&env, "hello.j2", &vars).unwrap();
        assert_eq!(result, "Hello World!");
    }

    // ── Insta snapshots ───────────────────────────────────────

    #[test]
    fn snapshot_template_rendering() {
        let vars = json!({
            "class_name": "BookingService",
            "module": "bookings",
            "fields": ["name", "date", "status"],
            "abstract_class": false,
        });
        let tmpl = r#"class {{ class_name | pascalcase }}:
    """{{ module | capitalize }} model."""
{% for field in fields %}
    {{ field }}: str
{% endfor %}
{% if abstract_class %}
    class Meta:
        abstract = True
{% endif %}"#;
        let result = render(tmpl, &vars).unwrap();
        insta::assert_snapshot!(result);
    }

    #[test]
    fn snapshot_error_undefined_variable() {
        let vars = json!({"class_name": "Foo"});
        let err = render("{{ clss_name }}", &vars).unwrap_err();
        let se = err.structured_error();
        let error_display = format!(
            "what: {}\nwhere: {}\nwhy: {}\nhint: {}",
            se.what, se.where_, se.why, se.hint
        );
        insta::assert_snapshot!(error_display);
    }

    #[test]
    fn snapshot_error_syntax() {
        let vars = json!({});
        let err = render("{% if x %}", &vars).unwrap_err();
        let se = err.structured_error();
        let error_display = format!(
            "what: {}\nwhere: {}\nwhy: {}\nhint: {}",
            se.what, se.where_, se.why, se.hint
        );
        insta::assert_snapshot!(error_display);
    }
}
