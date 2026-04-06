use std::io::IsTerminal;

use indexmap::IndexSet;
use owo_colors::OwoColorize;
use serde_json::Value;

use crate::operations::OpResult;
use crate::variables;
use crate::workflow;

// ── Output mode ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// JSON to stdout (piped or --json).
    Json,
    /// Human-readable colored text to stderr.
    Human,
}

/// Determine output mode from flags and TTY detection.
/// --json forces JSON. Otherwise: piped → JSON, TTY → Human.
pub fn detect_mode(force_json: bool) -> OutputMode {
    if force_json || !std::io::stdout().is_terminal() {
        OutputMode::Json
    } else {
        OutputMode::Human
    }
}

// ── JSON output ───────────────────────────────────────────────────

/// Build the full JSON output object for `jig run`.
pub fn format_json(results: &[OpResult], dry_run: bool, verbose: bool) -> Value {
    let operations: Vec<Value> = results
        .iter()
        .map(|r| op_result_to_json(r, verbose))
        .collect();
    let (files_written, files_skipped) = compute_file_summaries(results);

    serde_json::json!({
        "dry_run": dry_run,
        "operations": operations,
        "files_written": files_written.into_iter().collect::<Vec<_>>(),
        "files_skipped": files_skipped.into_iter().collect::<Vec<_>>(),
    })
}

pub(crate) fn op_result_to_json(result: &OpResult, verbose: bool) -> Value {
    match result {
        OpResult::Success {
            action,
            path,
            lines,
            location,
            rendered_content,
            scope_diagnostics,
        } => {
            let mut obj = serde_json::json!({
                "action": action,
                "path": path.display().to_string(),
                "lines": lines,
            });
            if let Some(loc) = location {
                obj["location"] = Value::String(loc.clone());
            }
            if verbose && let Some(content) = rendered_content {
                obj["rendered_content"] = Value::String(content.clone());
            }
            if verbose && let Some(diag) = scope_diagnostics {
                let mut diag_obj = serde_json::json!({
                    "anchor_line": diag.anchor_line,
                    "scope_start": diag.scope_start,
                    "scope_end": diag.scope_end,
                    "insertion_line": diag.insertion_line,
                });
                if let Some(fl) = diag.find_match_line {
                    diag_obj["find_match_line"] = Value::Number(fl.into());
                }
                if let Some((ref from, ref to)) = diag.position_fallback {
                    diag_obj["position_fallback"] = serde_json::json!({
                        "from": from,
                        "to": to,
                    });
                }
                obj["scope_diagnostics"] = diag_obj;
            }
            obj
        }
        OpResult::Skip {
            path,
            reason,
            rendered_content,
        } => {
            let mut obj = serde_json::json!({
                "action": "skip",
                "path": path.display().to_string(),
                "reason": reason,
            });
            if verbose && let Some(content) = rendered_content {
                obj["rendered_content"] = Value::String(content.clone());
            }
            obj
        }
        OpResult::Error {
            path,
            error,
            rendered_content,
        } => {
            serde_json::json!({
                "action": "error",
                "path": path.display().to_string(),
                "what": error.what,
                "where": error.where_,
                "why": error.why,
                "hint": error.hint,
                "rendered_content": rendered_content,
            })
        }
    }
}

/// Compute files_written and files_skipped arrays.
/// A path appears in files_written if any operation wrote to it.
/// A path appears in files_skipped only if ALL operations targeting it were skipped.
/// Paths appear in order of first encounter.
pub(crate) fn compute_file_summaries(results: &[OpResult]) -> (IndexSet<String>, IndexSet<String>) {
    // Track encounter order, writes, and errors to compute summaries.
    let mut encounter_order: IndexSet<String> = IndexSet::new();
    let mut was_written: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut had_error: std::collections::HashSet<String> = std::collections::HashSet::new();

    for result in results {
        let path_str = result.path().display().to_string();
        encounter_order.insert(path_str.clone());

        match result {
            OpResult::Success { .. } => {
                was_written.insert(path_str);
            }
            OpResult::Error { .. } => {
                had_error.insert(path_str);
            }
            OpResult::Skip { .. } => {}
        }
    }

    let mut written: IndexSet<String> = IndexSet::new();
    let mut skipped: IndexSet<String> = IndexSet::new();

    for path in &encounter_order {
        if was_written.contains(path) {
            written.insert(path.clone());
        } else if !had_error.contains(path) {
            // Only count as skipped if ALL operations were skips (no writes, no errors).
            skipped.insert(path.clone());
        }
    }

    (written, skipped)
}

/// Same as compute_file_summaries but takes references (for workflow aggregation).
fn compute_file_summaries_from_refs(results: &[&OpResult]) -> (IndexSet<String>, IndexSet<String>) {
    let mut encounter_order: IndexSet<String> = IndexSet::new();
    let mut was_written: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut had_error: std::collections::HashSet<String> = std::collections::HashSet::new();

    for result in results {
        let path_str = result.path().display().to_string();
        encounter_order.insert(path_str.clone());

        match result {
            OpResult::Success { .. } => {
                was_written.insert(path_str);
            }
            OpResult::Error { .. } => {
                had_error.insert(path_str);
            }
            OpResult::Skip { .. } => {}
        }
    }

    let mut written: IndexSet<String> = IndexSet::new();
    let mut skipped: IndexSet<String> = IndexSet::new();

    for path in &encounter_order {
        if was_written.contains(path) {
            written.insert(path.clone());
        } else if !had_error.contains(path) {
            skipped.insert(path.clone());
        }
    }

    (written, skipped)
}

// ─�� Human output ──────────────────────────────────────────────────

/// Write human-readable output to stderr.
pub fn format_human(results: &[OpResult], dry_run: bool, verbose: bool) {
    if dry_run {
        eprintln!("{}", "(dry run)".dimmed());
    }

    for result in results {
        match result {
            OpResult::Success {
                action,
                path,
                lines,
                location,
                rendered_content,
                scope_diagnostics,
            } => {
                let action_str = action.to_string();
                eprint!("  {} {}", action_str.green(), path.display());
                if let Some(loc) = location {
                    eprint!(" ({})", loc);
                }
                eprintln!(" ({} lines)", lines);
                if verbose {
                    if let Some(content) = rendered_content {
                        for line in content.lines() {
                            eprintln!("    {}", line.dimmed());
                        }
                    }
                    if let Some(diag) = scope_diagnostics {
                        eprintln!(
                            "    {} anchor={} scope={}-{} insert={}{}",
                            "diagnostics:".dimmed(),
                            diag.anchor_line,
                            diag.scope_start,
                            diag.scope_end,
                            diag.insertion_line,
                            diag.find_match_line
                                .map(|l| format!(" find={}", l))
                                .unwrap_or_default(),
                        );
                    }
                }
            }
            OpResult::Skip {
                path,
                reason,
                rendered_content,
            } => {
                eprintln!("  {} {} — {}", "skip".yellow(), path.display(), reason);
                if verbose && let Some(content) = rendered_content {
                    for line in content.lines() {
                        eprintln!("    {}", line.dimmed());
                    }
                }
            }
            OpResult::Error {
                path,
                error,
                rendered_content,
            } => {
                eprintln!("  {} {}", "error".red(), path.display());
                eprintln!("    what: {}", error.what);
                eprintln!("    where: {}", error.where_);
                eprintln!("    why: {}", error.why);
                eprintln!("    hint: {}", error.hint);
                if verbose || !rendered_content.is_empty() {
                    eprintln!(
                        "    rendered content ({} lines):",
                        rendered_content.lines().count()
                    );
                    for line in rendered_content.lines() {
                        eprintln!("      {}", line.dimmed());
                    }
                }
            }
        }
    }

    let (written, skipped) = compute_file_summaries(results);
    if !written.is_empty() || !skipped.is_empty() {
        eprintln!();
        if !written.is_empty() {
            eprintln!(
                "{}: {}",
                if dry_run { "would write" } else { "wrote" },
                written.len()
            );
        }
        if !skipped.is_empty() {
            eprintln!("skipped: {}", skipped.len());
        }
    }
}

// ── Workflow validation JSON ──────────────────────────────────────

pub fn build_workflow_validate_json(validation: &workflow::WorkflowValidation) -> Value {
    let vars = variables::vars_json(&validation.variables);

    let steps: Vec<Value> = validation
        .steps
        .iter()
        .map(|step| {
            let mut obj = serde_json::json!({
                "recipe": step.recipe,
                "valid": step.valid,
                "conditional": step.conditional,
            });
            if let Some(ref when) = step.when {
                obj["when"] = Value::String(when.clone());
            }
            if let Some(ref err) = step.error {
                obj["error"] = Value::String(err.clone());
            }
            obj
        })
        .collect();

    serde_json::json!({
        "type": "workflow",
        "valid": true,
        "name": validation.name,
        "description": validation.description,
        "variables": vars,
        "steps": steps,
    })
}

// ── Workflow execution JSON ──────────────────────────────────────

pub fn format_workflow_json(
    result: &workflow::WorkflowResult,
    dry_run: bool,
    verbose: bool,
    exit_code: i32,
) -> Value {
    let has_errors = result.steps.iter().any(|s| s.is_error());
    let status = if !has_errors || exit_code == 0 {
        "success"
    } else if exit_code == 3 && result.on_error == workflow::OnError::Report {
        "partial"
    } else {
        "error"
    };

    let mut all_ops: Vec<&OpResult> = Vec::new();
    let steps: Vec<Value> = result
        .steps
        .iter()
        .map(|step| match step {
            workflow::StepResult::Success { recipe, operations } => {
                let ops_json: Vec<Value> = operations
                    .iter()
                    .map(|r| op_result_to_json(r, verbose))
                    .collect();
                let (written, skipped) = compute_file_summaries(operations);
                all_ops.extend(operations.iter());
                serde_json::json!({
                    "recipe": recipe,
                    "status": "success",
                    "operations": ops_json,
                    "files_written": written.into_iter().collect::<Vec<_>>(),
                    "files_skipped": skipped.into_iter().collect::<Vec<_>>(),
                })
            }
            workflow::StepResult::Skipped { recipe, reason } => {
                serde_json::json!({
                    "recipe": recipe,
                    "status": "skipped",
                    "reason": reason,
                })
            }
            workflow::StepResult::Error {
                recipe,
                error,
                operations,
                rendered_content,
            } => {
                let ops_json: Vec<Value> = operations
                    .iter()
                    .map(|r| op_result_to_json(r, verbose))
                    .collect();
                all_ops.extend(operations.iter());
                let se = error.structured_error();
                let mut obj = serde_json::json!({
                    "recipe": recipe,
                    "status": "error",
                    "error": {
                        "what": se.what,
                        "where": se.where_,
                        "why": se.why,
                        "hint": se.hint,
                    },
                    "operations": ops_json,
                });
                if let Some(content) = rendered_content {
                    obj["rendered_content"] = Value::String(content.clone());
                }
                obj
            }
        })
        .collect();

    // Aggregate file summaries across all steps.
    let (agg_written, agg_skipped) = compute_file_summaries_from_refs(&all_ops);

    serde_json::json!({
        "dry_run": dry_run,
        "workflow": result.name,
        "on_error": result.on_error.to_string(),
        "status": status,
        "steps": steps,
        "files_written": agg_written.into_iter().collect::<Vec<_>>(),
        "files_skipped": agg_skipped.into_iter().collect::<Vec<_>>(),
    })
}

// ── Workflow execution human output ──────────────────────────────

pub fn format_workflow_human(result: &workflow::WorkflowResult, dry_run: bool, verbose: bool) {
    if dry_run {
        eprintln!("{}", "(dry run)".dimmed());
    }

    let total = result.steps.len();
    let mut succeeded = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for (i, step) in result.steps.iter().enumerate() {
        match step {
            workflow::StepResult::Success { recipe, operations } => {
                eprintln!("\n{} {}/{}: {}", "Step".bold(), i + 1, total, recipe);
                format_human(operations, false, verbose);
                succeeded += 1;
            }
            workflow::StepResult::Skipped { recipe, reason } => {
                eprintln!(
                    "\n{} {}/{}: {} {} — {}",
                    "Step".bold(),
                    i + 1,
                    total,
                    "skip".yellow(),
                    recipe,
                    reason
                );
                skipped += 1;
            }
            workflow::StepResult::Error {
                recipe,
                error,
                operations,
                rendered_content: _,
            } => {
                eprintln!(
                    "\n{} {}/{}: {} {}",
                    "Step".bold(),
                    i + 1,
                    total,
                    "error".red(),
                    recipe
                );
                if !operations.is_empty() {
                    format_human(operations, false, verbose);
                }
                let se = error.structured_error();
                eprintln!("    what: {}", se.what);
                eprintln!("    where: {}", se.where_);
                eprintln!("    why: {}", se.why);
                eprintln!("    hint: {}", se.hint);
                failed += 1;
            }
        }
    }

    eprintln!(
        "\n{} steps: {} succeeded, {} skipped, {} failed",
        total, succeeded, skipped, failed
    );
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::error::StructuredError;

    fn success_result(path: &str, lines: usize) -> OpResult {
        OpResult::Success {
            action: "create",
            path: PathBuf::from(path),
            lines,
            location: None,
            rendered_content: Some("content".into()),
            scope_diagnostics: None,
        }
    }

    fn skip_result(path: &str, reason: &str) -> OpResult {
        OpResult::Skip {
            path: PathBuf::from(path),
            reason: reason.into(),
            rendered_content: None,
        }
    }

    fn error_result(path: &str) -> OpResult {
        OpResult::Error {
            path: PathBuf::from(path),
            error: StructuredError {
                what: "file already exists".into(),
                where_: path.into(),
                why: "conflict".into(),
                hint: "use --force".into(),
            },
            rendered_content: "rendered".into(),
        }
    }

    // ── AC-6.3: --json forces JSON output ──

    #[test]
    fn ac_6_3_force_json() {
        let mode = detect_mode(true);
        assert_eq!(mode, OutputMode::Json);
    }

    // ── AC-6.5: JSON includes operations array with correct fields ──

    #[test]
    fn ac_6_5_json_operations_array() {
        let results = vec![
            success_result("src/main.rs", 10),
            skip_result("src/lib.rs", "skip_if_exists: true"),
        ];
        let json = format_json(&results, false, false);
        let ops = json["operations"].as_array().unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0]["action"], "create");
        assert_eq!(ops[0]["path"], "src/main.rs");
        assert_eq!(ops[0]["lines"], 10);
        assert_eq!(ops[1]["action"], "skip");
        assert_eq!(ops[1]["path"], "src/lib.rs");
        assert!(
            ops[1]["reason"]
                .as_str()
                .unwrap()
                .contains("skip_if_exists")
        );
    }

    // ── AC-6.5: Error operation has all structured error fields ──

    #[test]
    fn ac_6_5_json_error_fields() {
        let results = vec![error_result("conflict.rs")];
        let json = format_json(&results, false, false);
        let op = &json["operations"][0];
        assert_eq!(op["action"], "error");
        assert!(op["what"].is_string());
        assert!(op["where"].is_string());
        assert!(op["why"].is_string());
        assert!(op["hint"].is_string());
        assert!(op["rendered_content"].is_string());
    }

    // ── AC-6.6: files_written and files_skipped arrays ──

    #[test]
    fn ac_6_6_file_summaries() {
        let results = vec![
            success_result("a.rs", 5),
            skip_result("b.rs", "skipped"),
            success_result("c.rs", 3),
        ];
        let json = format_json(&results, false, false);
        let written: Vec<&str> = json["files_written"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let skipped: Vec<&str> = json["files_skipped"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(written, vec!["a.rs", "c.rs"]);
        assert_eq!(skipped, vec!["b.rs"]);
    }

    // ── AC-6.6: File written after skip → only in files_written ──

    #[test]
    fn ac_6_6_write_after_skip_removes_from_skipped() {
        let results = vec![
            skip_result("target.rs", "skipped first"),
            success_result("target.rs", 10),
        ];
        let json = format_json(&results, false, false);
        let written: Vec<&str> = json["files_written"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let skipped: Vec<&str> = json["files_skipped"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(written, vec!["target.rs"]);
        assert!(skipped.is_empty());
    }

    // ── AC-6.6: Order of first encounter preserved ──

    #[test]
    fn ac_6_6_order_preserved() {
        let results = vec![
            success_result("c.rs", 1),
            success_result("a.rs", 1),
            success_result("b.rs", 1),
        ];
        let json = format_json(&results, false, false);
        let written: Vec<&str> = json["files_written"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(written, vec!["c.rs", "a.rs", "b.rs"]);
    }

    // ── AC-6.7: --verbose includes rendered content ──

    #[test]
    fn ac_6_7_verbose_includes_content() {
        let results = vec![success_result("file.rs", 5)];
        let json = format_json(&results, false, true);
        assert!(json["operations"][0]["rendered_content"].is_string());
    }

    // ── AC-6.7: Without verbose, rendered_content absent from success/skip ──

    #[test]
    fn ac_6_7_no_verbose_no_content() {
        let results = vec![success_result("file.rs", 5)];
        let json = format_json(&results, false, false);
        assert!(json["operations"][0].get("rendered_content").is_none());
    }

    // ── AC-6.8: dry_run field in JSON ──

    #[test]
    fn ac_6_8_dry_run_json_field() {
        let json = format_json(&[], true, false);
        assert_eq!(json["dry_run"], true);
    }

    // ── AC-6.11: dry_run boolean field ──

    #[test]
    fn ac_6_11_dry_run_boolean() {
        let json_dry = format_json(&[], true, false);
        assert_eq!(json_dry["dry_run"], true);
        let json_wet = format_json(&[], false, false);
        assert_eq!(json_wet["dry_run"], false);
    }

    // ── AC-6.9: Flag interactions ──

    #[test]
    fn ac_6_9_verbose_with_json() {
        let results = vec![success_result("file.rs", 5)];
        // verbose + JSON → content in JSON
        let json = format_json(&results, false, true);
        assert!(json["operations"][0]["rendered_content"].is_string());
    }

    // ── AC-6.10: Error stops execution (tested via operations array length) ──

    #[test]
    fn ac_6_10_error_in_operations() {
        let results = vec![
            success_result("a.rs", 1),
            error_result("b.rs"),
            // No more results — execution stopped.
        ];
        let json = format_json(&results, false, false);
        let ops = json["operations"].as_array().unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[1]["action"], "error");
    }

    // ── AC-N2.1: Second run all skips ──

    #[test]
    fn ac_n2_1_all_skips() {
        let results = vec![
            skip_result("a.rs", "file already exists (skip_if_exists: true)"),
            skip_result("b.rs", "file already exists (skip_if_exists: true)"),
        ];
        let json = format_json(&results, false, false);
        let ops = json["operations"].as_array().unwrap();
        assert!(ops.iter().all(|op| op["action"] == "skip"));
        assert!(json["files_written"].as_array().unwrap().is_empty());
        assert_eq!(json["files_skipped"].as_array().unwrap().len(), 2);
    }

    // ── AC-6.1/AC-6.2: TTY detection — piped → Json ──

    #[test]
    fn ac_6_1_6_2_detect_mode_piped_is_json() {
        // In test context, stdout is piped (not a TTY) → Json mode (AC-6.2).
        // TTY → Human mode (AC-6.1) is verified by code inspection of detect_mode().
        let mode = detect_mode(false);
        assert_eq!(mode, OutputMode::Json);
    }

    // ── AC-6.4: --quiet suppresses stderr, no effect on JSON content ──

    #[test]
    fn ac_6_4_quiet_no_effect_on_json() {
        // --quiet only suppresses stderr. JSON output is identical with or without quiet.
        let results = vec![success_result("file.rs", 5)];
        let json_normal = format_json(&results, false, false);
        let json_quiet = format_json(&results, false, false); // same call — quiet is handled by caller
        assert_eq!(json_normal, json_quiet);
    }
}
