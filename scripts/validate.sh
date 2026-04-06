#!/bin/bash
# validate.sh — Run tests and check VALIDATION.md coverage
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/workflow-lib.sh"

ws_name="${1:-}"
task_name="${2:-}"

repo_root="$(get_repo_root)"
cd "$repo_root"

# --- Run tests ---

test_result=0
if [[ -f "$repo_root/justfile" ]]; then
    log_info "Running: just test"
    just test 2>&1 || test_result=$?
elif [[ -f "$repo_root/Cargo.toml" ]]; then
    log_info "Running: cargo test"
    cargo test 2>&1 || test_result=$?
elif [[ -f "$repo_root/package.json" ]]; then
    log_info "Running: npm test"
    npm test 2>&1 || test_result=$?
else
    log_warn "No test runner detected"
fi

# --- Run strict Rust quality checks (if applicable) ---

fmt_result=0
clippy_result=0
has_rust_project=0

if [[ -f "$repo_root/Cargo.toml" ]]; then
    has_rust_project=1

    log_info "Running: cargo fmt --all -- --check"
    cargo fmt --all -- --check 2>&1 || fmt_result=$?

    log_info "Running: cargo clippy --all-targets -- -D warnings"
    cargo clippy --all-targets -- -D warnings 2>&1 || clippy_result=$?
fi

# --- Check VALIDATION.md coverage ---

validation_file=""
if [[ -n "$ws_name" && -n "$task_name" ]]; then
    task_dir="$(get_task_dir "$ws_name" "$task_name")"
    validation_file="$task_dir/VALIDATION.md"
elif [[ -n "$ws_name" ]]; then
    # Look for any task validation files
    ws_dir="$(get_workstream_dir "$ws_name")"
    validation_file=$(find "$ws_dir/tasks" -name "VALIDATION.md" -type f 2>/dev/null | head -1 || true)
fi

pass_count=0
fail_count=0
pending_count=0
missing_count=0

if [[ -n "$validation_file" && -f "$validation_file" ]]; then
    log_info "Checking $validation_file"
    pass_count=$(grep -c "| PASS |" "$validation_file" 2>/dev/null) || pass_count=0
    fail_count=$(grep -c "| FAIL |" "$validation_file" 2>/dev/null) || fail_count=0
    pending_count=$(grep -c "| PENDING |" "$validation_file" 2>/dev/null) || pending_count=0
    missing_count=$(grep -c "| MISSING |" "$validation_file" 2>/dev/null) || missing_count=0
fi

# --- Report ---

echo ""
echo "=== Validation Report ==="
echo "Tests: $(if [[ $test_result -eq 0 ]]; then echo "PASS"; else echo "FAIL (exit code $test_result)"; fi)"
if [[ $has_rust_project -eq 1 ]]; then
    echo "Rust fmt: $(if [[ $fmt_result -eq 0 ]]; then echo "PASS"; else echo "FAIL (exit code $fmt_result)"; fi)"
    echo "Rust clippy: $(if [[ $clippy_result -eq 0 ]]; then echo "PASS"; else echo "FAIL (exit code $clippy_result)"; fi)"
fi

if [[ -n "$validation_file" && -f "$validation_file" ]]; then
    echo "VALIDATION.md:"
    echo "  PASS:    $pass_count"
    echo "  FAIL:    $fail_count"
    echo "  PENDING: $pending_count"
    echo "  MISSING: $missing_count"
fi

echo ""

# Determine readiness
if [[ $test_result -eq 0 && $fmt_result -eq 0 && $clippy_result -eq 0 && $fail_count -eq 0 && $missing_count -eq 0 && $pending_count -eq 0 ]]; then
    echo "Recommendation: READY"
    exit 0
else
    echo "Recommendation: NOT READY"
    [[ $test_result -ne 0 ]] && echo "  - Tests failed"
    [[ $fmt_result -ne 0 ]] && echo "  - cargo fmt check failed"
    [[ $clippy_result -ne 0 ]] && echo "  - cargo clippy strict check failed"
    [[ $fail_count -gt 0 ]] && echo "  - $fail_count FAIL entries in VALIDATION.md"
    [[ $pending_count -gt 0 ]] && echo "  - $pending_count PENDING entries in VALIDATION.md"
    [[ $missing_count -gt 0 ]] && echo "  - $missing_count MISSING entries in VALIDATION.md"
    exit 1
fi
