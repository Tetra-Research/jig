use std::io::Read;
use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::error::{JigError, StructuredError};
use crate::recipe::{VarType, VariableDecl};

// ── Public API: vars output ────────────────────────────────────────

/// Build the vars JSON output for `jig vars`.
pub fn vars_json(variables: &IndexMap<String, VariableDecl>) -> Value {
    let mut map = serde_json::Map::new();
    for (name, decl) in variables {
        let mut entry = serde_json::Map::new();
        entry.insert("type".into(), Value::String(decl.var_type.to_string()));
        entry.insert("required".into(), Value::Bool(decl.required));
        if let Some(ref default) = decl.default {
            entry.insert("default".into(), default.clone());
        }
        if let Some(ref desc) = decl.description {
            entry.insert("description".into(), Value::String(desc.clone()));
        }
        if let Some(ref values) = decl.values {
            entry.insert(
                "values".into(),
                Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()),
            );
        }
        if let Some(ref items) = decl.items {
            entry.insert("items".into(), Value::String(items.to_string()));
        }
        map.insert(name.clone(), Value::Object(entry));
    }
    Value::Object(map)
}

// ── Public API: variable collection ────────────────────────────────

/// Parse inline JSON from --vars.
pub fn parse_vars_inline(json_str: &str) -> Result<Value, JigError> {
    serde_json::from_str(json_str).map_err(|e| {
        JigError::VariableValidation(vec![StructuredError {
            what: "invalid JSON in --vars".into(),
            where_: "--vars argument".into(),
            why: e.to_string(),
            hint: "check JSON syntax — ensure keys are quoted and values are valid".into(),
        }])
    })
}

/// Parse JSON from a file (--vars-file).
pub fn parse_vars_file(path: &Path) -> Result<Value, JigError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        let (what, why) = if e.kind() == std::io::ErrorKind::NotFound {
            (
                "vars file not found".to_string(),
                "the file does not exist at the specified path".to_string(),
            )
        } else {
            ("cannot read vars file".to_string(), e.to_string())
        };
        JigError::VariableValidation(vec![StructuredError {
            what,
            where_: path.display().to_string(),
            why,
            hint: "check the --vars-file path and try again".into(),
        }])
    })?;
    serde_json::from_str(&content).map_err(|e| {
        JigError::VariableValidation(vec![StructuredError {
            what: "invalid JSON in vars file".into(),
            where_: path.display().to_string(),
            why: e.to_string(),
            hint: "check JSON syntax in the vars file".into(),
        }])
    })
}

/// Parse JSON from stdin (--vars-stdin).
pub fn parse_vars_stdin() -> Result<Value, JigError> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).map_err(|e| {
        JigError::VariableValidation(vec![StructuredError {
            what: "failed to read stdin".into(),
            where_: "stdin".into(),
            why: e.to_string(),
            hint: "ensure stdin contains valid JSON".into(),
        }])
    })?;
    serde_json::from_str(&buf).map_err(|e| {
        JigError::VariableValidation(vec![StructuredError {
            what: "invalid JSON from stdin".into(),
            where_: "stdin (--vars-stdin)".into(),
            why: e.to_string(),
            hint: "check JSON syntax in stdin input".into(),
        }])
    })
}

/// Collect variables from all sources and merge with precedence:
/// defaults < vars-file < vars-stdin < inline --vars.
/// Returns the merged JSON object (no type validation yet).
pub fn collect_vars(
    inline: Option<&str>,
    file: Option<&Path>,
    stdin: bool,
) -> Result<Value, JigError> {
    let file_val = match file {
        Some(path) => Some(parse_vars_file(path)?),
        None => None,
    };
    let stdin_val = if stdin {
        Some(parse_vars_stdin()?)
    } else {
        None
    };
    let inline_val = match inline {
        Some(s) => Some(parse_vars_inline(s)?),
        None => None,
    };

    let mut merged = Value::Object(serde_json::Map::new());
    if let Some(f) = file_val {
        merge_json(&mut merged, &f);
    }
    if let Some(s) = stdin_val {
        merge_json(&mut merged, &s);
    }
    if let Some(i) = inline_val {
        merge_json(&mut merged, &i);
    }
    Ok(merged)
}

// ── Public API: validation ─────────────────────────────────────────

/// Validate and merge variables against recipe declarations.
/// Applies defaults, then merges provided values, then type-checks.
/// Accumulates all validation errors before returning (AC-2.11).
/// Extra keys not in declarations pass through without error (AC-2.12).
#[allow(dead_code)] // Used in later phases (jig run) and tests
pub fn validate_variables(
    decls: &IndexMap<String, VariableDecl>,
    provided: &Value,
) -> Result<Value, JigError> {
    let provided_obj = provided.as_object().cloned().unwrap_or_default();

    // Start with defaults, then overlay provided values.
    let mut merged = serde_json::Map::new();
    for (name, decl) in decls {
        if let Some(ref default) = decl.default {
            merged.insert(name.clone(), default.clone());
        }
    }
    for (key, val) in &provided_obj {
        merged.insert(key.clone(), val.clone());
    }

    let mut errors = Vec::new();

    for (name, decl) in decls {
        match merged.get(name) {
            None if decl.required => {
                errors.push(StructuredError {
                    what: format!("missing required variable '{name}'"),
                    where_: format!("variable '{name}'"),
                    why: format!(
                        "variable '{name}' is declared as required but no value was provided"
                    ),
                    hint: format!("add '{name}' to --vars, e.g. --vars '{{\"{}\":...}}'", name),
                });
            }
            None => {
                // Not required, no default — just absent. That's fine.
            }
            Some(val) => {
                check_type(name, decl, val, &mut errors);
            }
        }
    }

    if !errors.is_empty() {
        return Err(JigError::VariableValidation(errors));
    }

    Ok(Value::Object(merged))
}

// ── Internal helpers ───────────────────────────────────────────────

fn merge_json(base: &mut Value, overlay: &Value) {
    if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, val) in overlay_obj {
            base_obj.insert(key.clone(), val.clone());
        }
    }
}

fn check_type(name: &str, decl: &VariableDecl, val: &Value, errors: &mut Vec<StructuredError>) {
    let actual_type = json_type_name(val);

    match decl.var_type {
        VarType::String => {
            if !val.is_string() {
                errors.push(type_mismatch_error(name, "string", actual_type, val));
            }
        }
        VarType::Number => {
            if !val.is_number() {
                errors.push(type_mismatch_error(name, "number", actual_type, val));
            }
        }
        VarType::Boolean => {
            if !val.is_boolean() {
                errors.push(type_mismatch_error(name, "boolean", actual_type, val));
            }
        }
        VarType::Array => {
            if !val.is_array() {
                errors.push(type_mismatch_error(name, "array", actual_type, val));
            } else if let Some(ref item_type) = decl.items {
                check_array_items(name, item_type, val.as_array().unwrap(), errors);
            }
        }
        VarType::Object => {
            if !val.is_object() {
                errors.push(type_mismatch_error(name, "object", actual_type, val));
            }
        }
        VarType::Enum =>
        {
            #[allow(clippy::collapsible_if)]
            if let Some(s) = val.as_str() {
                if let Some(ref allowed) = decl.values {
                    if !allowed.contains(&s.to_string()) {
                        errors.push(StructuredError {
                            what: format!("invalid enum value for '{name}'"),
                            where_: format!("variable '{name}'"),
                            why: format!(
                                "value '{}' is not in the allowed set: [{}]",
                                s,
                                allowed.join(", ")
                            ),
                            hint: format!("use one of: {}", allowed.join(", ")),
                        });
                    }
                }
            } else {
                errors.push(type_mismatch_error(name, "enum (string)", actual_type, val));
            }
        }
    }
}

fn check_array_items(
    name: &str,
    item_type: &VarType,
    items: &[Value],
    errors: &mut Vec<StructuredError>,
) {
    for (i, item) in items.iter().enumerate() {
        let ok = match item_type {
            VarType::String => item.is_string(),
            VarType::Number => item.is_number(),
            VarType::Boolean => item.is_boolean(),
            VarType::Array => item.is_array(),
            VarType::Object => item.is_object(),
            VarType::Enum => item.is_string(), // enum items are strings
        };
        if !ok {
            errors.push(StructuredError {
                what: format!("array item type mismatch in '{name}'"),
                where_: format!("variable '{name}[{i}]'"),
                why: format!(
                    "expected item type {}, got {}",
                    item_type,
                    json_type_name(item)
                ),
                hint: format!("ensure all elements of '{name}' are of type {item_type}"),
            });
        }
    }
}

fn type_mismatch_error(name: &str, expected: &str, actual: &str, val: &Value) -> StructuredError {
    let val_str = val.to_string();
    let truncated = if val_str.len() > 80 {
        format!("{}...", &val_str[..77])
    } else {
        val_str
    };
    StructuredError {
        what: format!("type mismatch for variable '{name}'"),
        where_: format!("variable '{name}'"),
        why: format!("expected {expected}, got {actual}: {truncated}"),
        hint: format!("provide a {expected} value for '{name}'"),
    }
}

fn json_type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::VarType;
    use std::fs;
    use tempfile::TempDir;

    fn decl(var_type: VarType, required: bool, default: Option<Value>) -> VariableDecl {
        VariableDecl {
            var_type,
            required,
            default,
            description: None,
            values: None,
            items: None,
        }
    }

    fn enum_decl(values: Vec<&str>, required: bool) -> VariableDecl {
        VariableDecl {
            var_type: VarType::Enum,
            required,
            default: None,
            description: None,
            values: Some(values.into_iter().map(String::from).collect()),
            items: None,
        }
    }

    fn array_decl(items: VarType, required: bool) -> VariableDecl {
        VariableDecl {
            var_type: VarType::Array,
            required,
            default: None,
            description: None,
            values: None,
            items: Some(items),
        }
    }

    // ── vars_json tests (carried from Phase 1) ─────────────────

    #[test]
    fn vars_json_includes_all_fields() {
        let mut variables = IndexMap::new();
        variables.insert(
            "name".into(),
            VariableDecl {
                var_type: VarType::String,
                required: true,
                default: Some(Value::String("default_val".into())),
                description: Some("A name".into()),
                values: None,
                items: None,
            },
        );
        variables.insert(
            "status".into(),
            VariableDecl {
                var_type: VarType::Enum,
                required: false,
                default: None,
                description: None,
                values: Some(vec!["active".into(), "inactive".into()]),
                items: None,
            },
        );
        variables.insert(
            "tags".into(),
            VariableDecl {
                var_type: VarType::Array,
                required: false,
                default: None,
                description: None,
                values: None,
                items: Some(VarType::String),
            },
        );

        let json = vars_json(&variables);
        let obj = json.as_object().unwrap();

        let name = obj["name"].as_object().unwrap();
        assert_eq!(name["type"], "string");
        assert_eq!(name["required"], true);
        assert_eq!(name["default"], "default_val");
        assert_eq!(name["description"], "A name");

        let status = obj["status"].as_object().unwrap();
        assert_eq!(status["type"], "enum");
        assert_eq!(status["required"], false);
        assert!(status.get("default").is_none());
        let vals = status["values"].as_array().unwrap();
        assert_eq!(
            vals,
            &[
                Value::String("active".into()),
                Value::String("inactive".into())
            ]
        );

        let tags = obj["tags"].as_object().unwrap();
        assert_eq!(tags["type"], "array");
        assert_eq!(tags["items"], "string");
    }

    #[test]
    fn vars_json_preserves_declaration_order() {
        let mut variables = IndexMap::new();
        for name in &["zebra", "alpha", "middle"] {
            variables.insert(name.to_string(), decl(VarType::String, false, None));
        }
        let json = vars_json(&variables);
        let keys: Vec<&String> = json.as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["zebra", "alpha", "middle"]);
    }

    // ── AC-2.1: Parse inline --vars ────────────────────────────

    #[test]
    fn ac_2_1_parse_inline_vars() {
        let val = parse_vars_inline(r#"{"name": "Foo", "count": 3}"#).unwrap();
        assert_eq!(val["name"], "Foo");
        assert_eq!(val["count"], 3);
    }

    // ── AC-2.2: Parse --vars-file ──────────────────────────────

    #[test]
    fn ac_2_2_parse_vars_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vars.json");
        fs::write(&path, r#"{"class_name": "BookingService"}"#).unwrap();
        let val = parse_vars_file(&path).unwrap();
        assert_eq!(val["class_name"], "BookingService");
    }

    // ── AC-2.3: --vars-stdin tested via integration (requires stdin pipe) ──

    // ── AC-2.4: Merge precedence ───────────────────────────────

    #[test]
    fn ac_2_4_merge_precedence() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert(
            "x".into(),
            decl(
                VarType::String,
                false,
                Some(Value::String("default".into())),
            ),
        );

        // defaults < file < stdin < inline
        // We can test this through validate_variables by providing overlapping values.
        // But for collect_vars, we test merge directly.

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("vars.json");
        fs::write(&file_path, r#"{"x": "from_file", "y": "file_only"}"#).unwrap();

        // file value overrides default
        let merged = collect_vars(None, Some(&file_path), false).unwrap();
        assert_eq!(merged["x"], "from_file");
        assert_eq!(merged["y"], "file_only");

        // inline overrides file
        let merged =
            collect_vars(Some(r#"{"x": "from_inline"}"#), Some(&file_path), false).unwrap();
        assert_eq!(merged["x"], "from_inline");
        assert_eq!(merged["y"], "file_only");
    }

    #[test]
    fn ac_2_4_defaults_lowest_precedence() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert(
            "x".into(),
            decl(
                VarType::String,
                false,
                Some(Value::String("default".into())),
            ),
        );

        // No provided value → default used
        let result = validate_variables(&decls, &Value::Object(serde_json::Map::new())).unwrap();
        assert_eq!(result["x"], "default");

        // Provided value overrides default
        let provided = serde_json::json!({"x": "provided"});
        let result = validate_variables(&decls, &provided).unwrap();
        assert_eq!(result["x"], "provided");
    }

    // ── AC-2.5: Required missing ───────────────────────────────

    #[test]
    fn ac_2_5_required_missing() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("name".into(), decl(VarType::String, true, None));

        let err = validate_variables(&decls, &Value::Object(serde_json::Map::new())).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        let se = err.structured_error();
        assert!(se.what.contains("missing required variable"));
        assert!(se.what.contains("name"));
        assert!(!se.hint.is_empty());
    }

    // ── AC-2.6: Default fallback ───────────────────────────────

    #[test]
    fn ac_2_6_default_fallback() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert(
            "color".into(),
            decl(VarType::String, false, Some(Value::String("blue".into()))),
        );

        let result = validate_variables(&decls, &Value::Object(serde_json::Map::new())).unwrap();
        assert_eq!(result["color"], "blue");
    }

    // ── AC-2.7: Type mismatch ──────────────────────────────────

    #[test]
    fn ac_2_7_type_mismatch_string_got_number() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("name".into(), decl(VarType::String, true, None));

        let provided = serde_json::json!({"name": 42});
        let err = validate_variables(&decls, &provided).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        let se = err.structured_error();
        assert!(se.why.contains("expected string"));
        assert!(se.why.contains("got number"));
    }

    #[test]
    fn ac_2_7_type_mismatch_number_got_string() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("count".into(), decl(VarType::Number, true, None));

        let provided = serde_json::json!({"count": "five"});
        let err = validate_variables(&decls, &provided).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        assert!(err.structured_error().why.contains("expected number"));
    }

    #[test]
    fn ac_2_7_type_mismatch_boolean_got_string() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("flag".into(), decl(VarType::Boolean, true, None));

        let provided = serde_json::json!({"flag": "true"});
        let err = validate_variables(&decls, &provided).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        assert!(err.structured_error().why.contains("expected boolean"));
    }

    #[test]
    fn ac_2_7_type_mismatch_object_for_array() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("items".into(), decl(VarType::Array, true, None));

        let provided = serde_json::json!({"items": {"key": "val"}});
        let err = validate_variables(&decls, &provided).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        assert!(err.structured_error().why.contains("expected array"));
        assert!(err.structured_error().why.contains("got object"));
    }

    // ── AC-2.8: Enum validation ────────────────────────────────

    #[test]
    fn ac_2_8_enum_valid() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("status".into(), enum_decl(vec!["active", "inactive"], true));

        let provided = serde_json::json!({"status": "active"});
        let result = validate_variables(&decls, &provided).unwrap();
        assert_eq!(result["status"], "active");
    }

    #[test]
    fn ac_2_8_enum_rejection() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("status".into(), enum_decl(vec!["active", "inactive"], true));

        let provided = serde_json::json!({"status": "deleted"});
        let err = validate_variables(&decls, &provided).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        let se = err.structured_error();
        assert!(se.what.contains("invalid enum value"));
        assert!(se.why.contains("deleted"));
        assert!(se.why.contains("active"));
    }

    // ── AC-2.9: Array item type validation ─────────────────────

    #[test]
    fn ac_2_9_array_items_valid() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("tags".into(), array_decl(VarType::String, true));

        let provided = serde_json::json!({"tags": ["a", "b", "c"]});
        let result = validate_variables(&decls, &provided).unwrap();
        assert_eq!(result["tags"], serde_json::json!(["a", "b", "c"]));
    }

    #[test]
    fn ac_2_9_array_item_type_mismatch() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("tags".into(), array_decl(VarType::String, true));

        let provided = serde_json::json!({"tags": ["a", 42, "c"]});
        let err = validate_variables(&decls, &provided).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        let se = err.structured_error();
        assert!(se.what.contains("array item type mismatch"));
        assert!(se.where_.contains("tags[1]"));
    }

    // ── AC-2.10: All six variable types accepted ───────────────

    #[test]
    fn ac_2_10_all_six_types() {
        let pi = std::f64::consts::PI;
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("s".into(), decl(VarType::String, true, None));
        decls.insert("n".into(), decl(VarType::Number, true, None));
        decls.insert("b".into(), decl(VarType::Boolean, true, None));
        decls.insert("a".into(), decl(VarType::Array, true, None));
        decls.insert("o".into(), decl(VarType::Object, true, None));
        decls.insert("e".into(), enum_decl(vec!["x", "y"], true));

        let provided = serde_json::json!({
            "s": "hello",
            "n": pi,
            "b": true,
            "a": [1, 2],
            "o": {"key": "val"},
            "e": "x",
        });

        let result = validate_variables(&decls, &provided).unwrap();
        assert_eq!(result["s"], "hello");
        assert_eq!(result["n"], pi);
        assert_eq!(result["b"], true);
    }

    // ── AC-2.11: Multiple errors accumulated ───────────────────

    #[test]
    fn ac_2_11_multiple_errors_accumulated() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("name".into(), decl(VarType::String, true, None));
        decls.insert("count".into(), decl(VarType::Number, true, None));
        decls.insert("flag".into(), decl(VarType::Boolean, true, None));

        // Missing name, wrong type for count, missing flag
        let provided = serde_json::json!({"count": "not_a_number"});
        let err = validate_variables(&decls, &provided).unwrap_err();
        assert_eq!(err.exit_code(), 4);

        let errors = err.structured_errors();
        assert!(
            errors.len() >= 2,
            "expected at least 2 errors, got {}",
            errors.len()
        );

        let whats: Vec<&str> = errors.iter().map(|e| e.what.as_str()).collect();
        assert!(whats.iter().any(|w| w.contains("missing required")));
        assert!(whats.iter().any(|w| w.contains("type mismatch")));
    }

    // ── AC-2.12: Extra keys pass through ───────────────────────

    #[test]
    fn ac_2_12_extra_keys_pass_through() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert("name".into(), decl(VarType::String, true, None));

        let provided = serde_json::json!({"name": "Foo", "extra_field": 42, "another": true});
        let result = validate_variables(&decls, &provided).unwrap();
        assert_eq!(result["name"], "Foo");
        assert_eq!(result["extra_field"], 42);
        assert_eq!(result["another"], true);
    }

    // ── AC-2.13: Invalid JSON in --vars ────────────────────────

    #[test]
    fn ac_2_13_invalid_json_inline() {
        let err = parse_vars_inline("{bad json}").unwrap_err();
        assert_eq!(err.exit_code(), 4);
        assert!(err.structured_error().what.contains("invalid JSON"));
    }

    // ── AC-2.14: --vars-file nonexistent ───────────────────────

    #[test]
    fn ac_2_14_vars_file_not_found() {
        let err = parse_vars_file(Path::new("/tmp/nonexistent_jig_vars.json")).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        assert!(err.structured_error().what.contains("vars file not found"));
    }

    // ── AC-2.15: --vars-file with invalid JSON ─────────────────

    #[test]
    fn ac_2_15_vars_file_invalid_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        fs::write(&path, "not json {").unwrap();
        let err = parse_vars_file(&path).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        let se = err.structured_error();
        assert!(se.what.contains("invalid JSON"));
        assert!(se.where_.contains("bad.json"));
    }

    // ── AC-2.16: No variable sources → empty + defaults ────────

    #[test]
    fn ac_2_16_no_sources_uses_defaults() {
        let mut decls: IndexMap<String, VariableDecl> = IndexMap::new();
        decls.insert(
            "color".into(),
            decl(VarType::String, false, Some(Value::String("red".into()))),
        );
        decls.insert("label".into(), decl(VarType::String, false, None));

        let provided = collect_vars(None, None, false).unwrap();
        let result = validate_variables(&decls, &provided).unwrap();
        assert_eq!(result["color"], "red");
        // label has no default and is not required, so it's absent
        assert!(result.get("label").is_none());
    }
}
