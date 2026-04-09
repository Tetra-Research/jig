use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent;

const LOCAL_NOTICE_INTERVAL_RUNS: u64 = 25;

#[derive(Debug, Default, Serialize, Deserialize)]
struct NoticeState {
    run_count: u64,
    last_local_notice_run: Option<u64>,
    last_notified_bundle_version: Option<String>,
}

pub fn maybe_emit_local_update_notice(
    base_dir: &Path,
    force_json: bool,
    quiet: bool,
    eligible: bool,
) {
    if !eligible
        || force_json
        || quiet
        || std::env::var_os("CI").is_some()
        || !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        return;
    }

    let state_dir = match state_dir() {
        Some(path) => path,
        None => return,
    };
    let mut state = load_state(&state_dir).unwrap_or_default();
    state.run_count = state.run_count.saturating_add(1);

    let report = match agent::doctor(None, Some(base_dir.to_path_buf()), base_dir) {
        Ok(report) => report,
        Err(_) => {
            let _ = save_state(&state_dir, &state);
            return;
        }
    };

    let stale_statuses = report
        .statuses
        .into_iter()
        .filter(|status| {
            status.markers_present && status.managed_install_present && status.needs_update
        })
        .collect::<Vec<_>>();

    if stale_statuses.is_empty() {
        let _ = save_state(&state_dir, &state);
        return;
    }

    let current_bundle_version = agent::current_bundle_version().to_string();
    let should_emit = state.last_notified_bundle_version.as_deref()
        != Some(current_bundle_version.as_str())
        || state
            .last_local_notice_run
            .map(|last| state.run_count.saturating_sub(last) >= LOCAL_NOTICE_INTERVAL_RUNS)
            .unwrap_or(true);

    if should_emit {
        eprintln!("Update available: this Jig binary bundles newer agent skills.");
        for status in &stale_statuses {
            let installed_versions = if status.installed_versions.is_empty() {
                "unknown".to_string()
            } else {
                status.installed_versions.join(", ")
            };
            eprintln!(
                "  {} skills: installed {}, bundled {}",
                status.agent, installed_versions, status.current_bundle_version
            );
            eprintln!("  Run: jig agent update {}", status.agent);
        }
        state.last_local_notice_run = Some(state.run_count);
        state.last_notified_bundle_version = Some(current_bundle_version);
    }

    let _ = save_state(&state_dir, &state);
}

fn state_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("JIG_STATE_DIR") {
        return Some(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".jig"))
}

fn state_path(state_dir: &Path) -> PathBuf {
    state_dir.join("state.json")
}

fn load_state(state_dir: &Path) -> Result<NoticeState, ()> {
    let path = state_path(state_dir);
    let content = std::fs::read_to_string(path).map_err(|_| ())?;
    serde_json::from_str(&content).map_err(|_| ())
}

fn save_state(state_dir: &Path, state: &NoticeState) -> Result<(), ()> {
    std::fs::create_dir_all(state_dir).map_err(|_| ())?;
    let content = serde_json::to_vec_pretty(state).map_err(|_| ())?;
    std::fs::write(state_path(state_dir), content).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let state = NoticeState {
            run_count: 7,
            last_local_notice_run: Some(3),
            last_notified_bundle_version: Some("1.2.3".into()),
        };

        save_state(dir.path(), &state).unwrap();
        let loaded = load_state(dir.path()).unwrap();
        assert_eq!(loaded.run_count, 7);
        assert_eq!(loaded.last_local_notice_run, Some(3));
        assert_eq!(
            loaded.last_notified_bundle_version.as_deref(),
            Some("1.2.3")
        );
    }
}
