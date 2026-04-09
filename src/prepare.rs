use minijinja::Environment;
use serde_json::Value;

use crate::error::{JigError, StructuredError};
use crate::operations::PreparedOp;
use crate::recipe::{Anchor, FileOp, InjectMode, Recipe, ReplaceSpec};
use crate::renderer;

pub fn prepare_operations(
    recipe: &Recipe,
    env: &Environment,
    vars: &Value,
) -> Result<Vec<PreparedOp>, JigError> {
    let mut prepared_ops = Vec::with_capacity(recipe.files.len());

    for (i, file_op) in recipe.files.iter().enumerate() {
        let rendered_content = renderer::render_template(env, file_op.template(), vars)?;
        let rendered_path = match file_op {
            FileOp::Create { to, .. } => {
                renderer::render_path_template(env, to, vars, &format!("files[{i}].to"))?
            }
            FileOp::Inject { inject, .. } => {
                renderer::render_path_template(env, inject, vars, &format!("files[{i}].inject"))?
            }
            FileOp::Replace { replace, .. } => {
                renderer::render_path_template(env, replace, vars, &format!("files[{i}].replace"))?
            }
            FileOp::Patch { patch, .. } => {
                renderer::render_path_template(env, patch, vars, &format!("files[{i}].patch"))?
            }
        };

        let rendered_skip_if = match file_op {
            FileOp::Inject {
                skip_if: Some(expr),
                ..
            }
            | FileOp::Patch {
                skip_if: Some(expr),
                ..
            } => Some(renderer::render_inline_template(
                env,
                expr,
                vars,
                &format!("files[{i}].skip_if"),
            )?),
            _ => None,
        };

        let rendered_inject_mode = match file_op {
            FileOp::Inject { mode, .. } => Some(render_inject_mode(env, mode, vars, i)?),
            _ => None,
        };
        let rendered_replace_spec = match file_op {
            FileOp::Replace { spec, .. } => Some(render_replace_spec(env, spec, vars, i)?),
            _ => None,
        };
        let rendered_anchor = match file_op {
            FileOp::Patch { anchor, .. } => Some(render_anchor(env, anchor, vars, i)?),
            _ => None,
        };

        prepared_ops.push(PreparedOp {
            file_op: file_op.clone(),
            rendered_content,
            rendered_path,
            rendered_skip_if,
            rendered_inject_mode,
            rendered_replace_spec,
            rendered_anchor,
        });
    }

    Ok(prepared_ops)
}

fn render_inject_mode(
    env: &Environment,
    mode: &InjectMode,
    vars: &Value,
    index: usize,
) -> Result<InjectMode, JigError> {
    match mode {
        InjectMode::After { pattern, at } => Ok(InjectMode::After {
            pattern: render_selector_regex(env, pattern, vars, &format!("files[{index}].after"))?,
            at: at.clone(),
        }),
        InjectMode::Before { pattern, at } => Ok(InjectMode::Before {
            pattern: render_selector_regex(env, pattern, vars, &format!("files[{index}].before"))?,
            at: at.clone(),
        }),
        InjectMode::Prepend => Ok(InjectMode::Prepend),
        InjectMode::Append => Ok(InjectMode::Append),
    }
}

fn render_replace_spec(
    env: &Environment,
    spec: &ReplaceSpec,
    vars: &Value,
    index: usize,
) -> Result<ReplaceSpec, JigError> {
    match spec {
        ReplaceSpec::Between { start, end } => Ok(ReplaceSpec::Between {
            start: render_selector_regex(
                env,
                start,
                vars,
                &format!("files[{index}].between.start"),
            )?,
            end: render_selector_regex(env, end, vars, &format!("files[{index}].between.end"))?,
        }),
        ReplaceSpec::Pattern(pattern) => Ok(ReplaceSpec::Pattern(render_selector_regex(
            env,
            pattern,
            vars,
            &format!("files[{index}].pattern"),
        )?)),
    }
}

fn render_anchor(
    env: &Environment,
    anchor: &Anchor,
    vars: &Value,
    index: usize,
) -> Result<Anchor, JigError> {
    Ok(Anchor {
        pattern: render_selector_regex(
            env,
            &anchor.pattern,
            vars,
            &format!("files[{index}].anchor.pattern"),
        )?,
        scope: anchor.scope.clone(),
        find: anchor
            .find
            .as_deref()
            .map(|find| render_selector_find(env, find, vars, index))
            .transpose()?,
        position: anchor.position.clone(),
    })
}

fn render_selector_regex(
    env: &Environment,
    template_expr: &str,
    vars: &Value,
    field_path: &str,
) -> Result<String, JigError> {
    let rendered = renderer::render_inline_template(env, template_expr, vars, field_path)?;
    validate_rendered_selector_regex(&rendered, field_path)?;
    Ok(rendered)
}

fn validate_rendered_selector_regex(pattern: &str, field_path: &str) -> Result<(), JigError> {
    if pattern.is_empty() {
        return Err(JigError::TemplateRendering(StructuredError {
            what: format!("empty rendered selector regex in '{field_path}'"),
            where_: field_path.to_string(),
            why: "the selector template rendered to an empty string".into(),
            hint: "provide a non-empty selector, or guard this operation so it is skipped when the selector would be empty".into(),
        }));
    }

    regex::Regex::new(pattern).map_err(|e| {
        JigError::TemplateRendering(StructuredError {
            what: format!("invalid rendered selector regex in '{field_path}'"),
            where_: field_path.to_string(),
            why: format!("rendered value '{pattern}' failed to compile: {e}"),
            hint: "check regex syntax; if the variable should match literally, use the regex_escape filter".into(),
        })
    })?;

    Ok(())
}

fn render_selector_find(
    env: &Environment,
    template_expr: &str,
    vars: &Value,
    index: usize,
) -> Result<String, JigError> {
    let field_path = format!("files[{index}].anchor.find");
    let rendered = renderer::render_inline_template(env, template_expr, vars, &field_path)?;
    validate_rendered_selector_find(&rendered, &field_path)?;
    Ok(rendered)
}

fn validate_rendered_selector_find(find_str: &str, field_path: &str) -> Result<(), JigError> {
    if find_str.trim().is_empty() {
        return Err(JigError::TemplateRendering(StructuredError {
            what: format!("empty rendered selector find in '{field_path}'"),
            where_: field_path.to_string(),
            why: "the selector find template rendered to an empty string".into(),
            hint: "provide a non-empty find string, or guard this operation so it is skipped when the find would be empty".into(),
        }));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Recipe;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    fn setup_recipe(yaml: &str, templates: &[(&str, &str)]) -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let recipe_path = dir.path().join("recipe.yaml");
        fs::write(&recipe_path, yaml).unwrap();
        for (name, content) in templates {
            fs::write(dir.path().join(name), content).unwrap();
        }
        (dir, recipe_path)
    }

    #[test]
    fn prepare_renders_templated_selector_fields() {
        let yaml = r#"
variables:
  model_name:
    type: string
  member_name:
    type: string
files:
  - template: t.j2
    patch: models.py
    anchor:
      pattern: "^class {{ model_name | regex_escape }}:"
      scope: class_body
      find: "{{ member_name }}"
"#;
        let (_dir, recipe_path) = setup_recipe(yaml, &[("t.j2", "content")]);
        let recipe = Recipe::load(&recipe_path).unwrap();
        let env = renderer::create_recipe_env(&recipe).unwrap();
        let prepared = prepare_operations(
            &recipe,
            &env,
            &json!({"model_name": "User[Legacy]", "member_name": "list_display"}),
        )
        .unwrap();

        assert_eq!(
            prepared[0].rendered_anchor.as_ref().unwrap().pattern,
            "^class User\\[Legacy\\]:"
        );
        assert_eq!(
            prepared[0]
                .rendered_anchor
                .as_ref()
                .unwrap()
                .find
                .as_deref(),
            Some("list_display")
        );
    }

    #[test]
    fn prepare_rejects_invalid_rendered_selector_regex() {
        let yaml = r#"
variables:
  model_name:
    type: string
files:
  - template: t.j2
    patch: models.py
    anchor:
      pattern: "^class {{ model_name }}:"
      scope: class_body
"#;
        let (_dir, recipe_path) = setup_recipe(yaml, &[("t.j2", "content")]);
        let recipe = Recipe::load(&recipe_path).unwrap();
        let env = renderer::create_recipe_env(&recipe).unwrap();
        let err = prepare_operations(&recipe, &env, &json!({"model_name": "User["})).unwrap_err();

        assert_eq!(err.exit_code(), 2);
        assert!(err.structured_error().hint.contains("regex_escape"));
    }

    #[test]
    fn prepare_rejects_empty_rendered_anchor_find() {
        let yaml = r#"
variables:
  member_name:
    type: string
files:
  - template: t.j2
    patch: models.py
    anchor:
      pattern: "^class Entity:"
      scope: class_body
      find: "{{ member_name }}"
"#;
        let (_dir, recipe_path) = setup_recipe(yaml, &[("t.j2", "content")]);
        let recipe = Recipe::load(&recipe_path).unwrap();
        let env = renderer::create_recipe_env(&recipe).unwrap();
        let err = prepare_operations(&recipe, &env, &json!({"member_name": ""})).unwrap_err();

        assert_eq!(err.exit_code(), 2);
        assert!(
            err.structured_error()
                .what
                .contains("empty rendered selector find")
        );
    }
}
