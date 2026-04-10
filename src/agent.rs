use std::path::{Path, PathBuf};

use clap::ValueEnum;
use include_dir::{Dir, File, include_dir};
use serde::Serialize;

use crate::error::{JigError, StructuredError};

static BUNDLED_SKILLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/bundles/skills");

const INSTALL_MARKER_FILE: &str = ".jig-agent-install.json";
const BUNDLE_SOURCE_ROOT: &str = "bundles/skills";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Claude,
    Codex,
    #[value(name = "opencode")]
    OpenCode,
}

impl AgentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
            AgentKind::OpenCode => "opencode",
        }
    }

    fn skills_dir(self) -> &'static str {
        match self {
            AgentKind::Claude => ".claude/skills",
            AgentKind::Codex => ".codex/skills",
            AgentKind::OpenCode => ".opencode/skills",
        }
    }

    fn marker_paths(self) -> &'static [&'static str] {
        match self {
            AgentKind::Claude => &[".claude", ".claude/skills", ".claude-plugin"],
            AgentKind::Codex => &[
                ".codex",
                ".agents/plugins/marketplace.json",
                ".codex-plugin",
            ],
            AgentKind::OpenCode => &[".opencode", ".opencode/skills", ".opencode/plugins"],
        }
    }

    fn skill_env_var(self) -> &'static str {
        match self {
            AgentKind::Claude => "CLAUDE_SKILL_DIR",
            AgentKind::Codex => "CODEX_SKILL_DIR",
            AgentKind::OpenCode => "OPENCODE_SKILL_DIR",
        }
    }
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct InstallRequest {
    pub agent: Option<AgentKind>,
    pub target_root: Option<PathBuf>,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct TargetRequest {
    pub agent: Option<AgentKind>,
    pub target_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundledSkill {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallResult {
    pub agent: AgentKind,
    pub target_root: PathBuf,
    pub install_base: PathBuf,
    pub inferred_agent: bool,
    pub installed_skills: Vec<BundledSkill>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateResult {
    pub agent: AgentKind,
    pub target_root: PathBuf,
    pub install_base: PathBuf,
    pub inferred_agent: bool,
    pub removed_skills: Vec<BundledSkill>,
    pub installed_skills: Vec<BundledSkill>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoveResult {
    pub agent: AgentKind,
    pub target_root: PathBuf,
    pub install_base: PathBuf,
    pub inferred_agent: bool,
    pub removed_skills: Vec<BundledSkill>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStatus {
    pub agent: AgentKind,
    pub markers_present: bool,
    pub install_base: PathBuf,
    pub managed_install_present: bool,
    pub installed_skills: Vec<BundledSkill>,
    pub installed_versions: Vec<String>,
    pub current_bundle_version: String,
    pub up_to_date: bool,
    pub needs_update: bool,
    pub missing_skills: Vec<BundledSkill>,
    pub extra_skills: Vec<BundledSkill>,
    pub invalid_skills: Vec<String>,
    pub bundled_skills: Vec<BundledSkill>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorResult {
    pub requested_agent: Option<AgentKind>,
    pub detected_agents: Vec<AgentKind>,
    pub target_root: PathBuf,
    pub statuses: Vec<AgentStatus>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
struct InstallMarker {
    owner: String,
    agent: String,
    bundle_version: String,
    source: String,
    installed_at_unix: u64,
}

#[derive(Debug, Clone)]
struct InstalledSkillRecord {
    name: String,
    bundle_version: Option<String>,
    valid_marker: bool,
}

pub fn current_bundle_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn bundled_skills() -> Vec<BundledSkill> {
    let mut skills = BUNDLED_SKILLS_DIR
        .dirs()
        .map(|dir| BundledSkill {
            name: dir
                .path()
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| dir.path().display().to_string()),
        })
        .collect::<Vec<_>>();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

pub fn install(request: InstallRequest, base_dir: &Path) -> Result<InstallResult, JigError> {
    let resolved = resolve_target(request.agent, request.target_root, base_dir, "install")?;
    let install_base = resolved.target_root.join(resolved.agent.skills_dir());

    std::fs::create_dir_all(&install_base).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!("failed to create {} skills directory", resolved.agent),
            where_: install_base.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions or use --to with a writable path".into(),
        })
    })?;

    let mut installed_skills = Vec::new();
    for skill_dir in BUNDLED_SKILLS_DIR.dirs() {
        let skill_name = skill_name_from_dir(skill_dir)?;
        let target_dir = install_base.join(&skill_name);
        install_one_skill(
            skill_dir,
            &skill_name,
            resolved.agent,
            &target_dir,
            request.force,
        )?;
        installed_skills.push(BundledSkill { name: skill_name });
    }
    installed_skills.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(InstallResult {
        agent: resolved.agent,
        target_root: resolved.target_root,
        install_base,
        inferred_agent: resolved.inferred_agent,
        installed_skills,
    })
}

pub fn update(request: TargetRequest, base_dir: &Path) -> Result<UpdateResult, JigError> {
    let resolved = resolve_target(request.agent, request.target_root, base_dir, "update")?;
    let remove_result = remove_owned_for_agent(&resolved)?;
    let install_result = install(
        InstallRequest {
            agent: Some(resolved.agent),
            target_root: Some(resolved.target_root.clone()),
            force: false,
        },
        base_dir,
    )?;

    Ok(UpdateResult {
        agent: install_result.agent,
        target_root: install_result.target_root,
        install_base: install_result.install_base,
        inferred_agent: resolved.inferred_agent,
        removed_skills: remove_result.removed_skills,
        installed_skills: install_result.installed_skills,
    })
}

pub fn remove(request: TargetRequest, base_dir: &Path) -> Result<RemoveResult, JigError> {
    let resolved = resolve_target(request.agent, request.target_root, base_dir, "remove")?;
    remove_owned_for_agent(&resolved)
}

pub fn doctor(
    requested_agent: Option<AgentKind>,
    target_root: Option<PathBuf>,
    base_dir: &Path,
) -> Result<DoctorResult, JigError> {
    let target_root = target_root.unwrap_or_else(|| base_dir.to_path_buf());
    let detected_agents = detect_agents(&target_root);

    let agents = if let Some(agent) = requested_agent {
        vec![agent]
    } else if detected_agents.is_empty() {
        all_agents()
    } else {
        detected_agents.clone()
    };

    let mut statuses = Vec::new();
    for agent in agents {
        let install_base = target_root.join(agent.skills_dir());
        let installed_records = installed_skill_records(&install_base)?;
        let bundled = bundled_skills();
        let installed_skills = installed_records
            .iter()
            .map(|record| BundledSkill {
                name: record.name.clone(),
            })
            .collect::<Vec<_>>();
        let installed_versions = collect_installed_versions(&installed_records);
        let invalid_skills = installed_records
            .iter()
            .filter(|record| !record.valid_marker)
            .map(|record| record.name.clone())
            .collect::<Vec<_>>();
        let missing_skills = diff_skills(&bundled, &installed_skills);
        let extra_skills = diff_skills(&installed_skills, &bundled);
        let managed_install_present = !installed_skills.is_empty();
        let up_to_date = managed_install_present
            && invalid_skills.is_empty()
            && missing_skills.is_empty()
            && extra_skills.is_empty()
            && installed_versions.len() == 1
            && installed_versions[0] == current_bundle_version();

        statuses.push(AgentStatus {
            agent,
            markers_present: detected_agents.contains(&agent),
            install_base: install_base.clone(),
            managed_install_present,
            installed_skills,
            installed_versions,
            current_bundle_version: current_bundle_version().to_string(),
            up_to_date,
            needs_update: managed_install_present && !up_to_date,
            missing_skills,
            extra_skills,
            invalid_skills,
            bundled_skills: bundled,
        });
    }
    statuses.sort_by(|a, b| a.agent.cmp(&b.agent));

    Ok(DoctorResult {
        requested_agent,
        detected_agents,
        target_root,
        statuses,
    })
}

pub fn detect_agents(root: &Path) -> Vec<AgentKind> {
    let mut detected = all_agents()
        .into_iter()
        .filter(|agent| {
            agent
                .marker_paths()
                .iter()
                .any(|marker| root.join(marker).exists())
        })
        .collect::<Vec<_>>();
    detected.sort();
    detected
}

#[derive(Debug, Clone)]
struct ResolvedTarget {
    agent: AgentKind,
    target_root: PathBuf,
    inferred_agent: bool,
}

fn resolve_target(
    agent: Option<AgentKind>,
    target_root: Option<PathBuf>,
    base_dir: &Path,
    operation: &str,
) -> Result<ResolvedTarget, JigError> {
    let target_root = target_root.unwrap_or_else(|| base_dir.to_path_buf());
    let (agent, inferred_agent) = resolve_agent(agent, &target_root, operation)?;
    ensure_agent_marker_present(agent, &target_root, operation)?;
    Ok(ResolvedTarget {
        agent,
        target_root,
        inferred_agent,
    })
}

fn resolve_agent(
    agent: Option<AgentKind>,
    root: &Path,
    operation: &str,
) -> Result<(AgentKind, bool), JigError> {
    if let Some(agent) = agent {
        return Ok((agent, false));
    }

    let detected = detect_agents(root);
    match detected.as_slice() {
        [agent] => Ok((*agent, true)),
        [] => Err(JigError::FileOperation(StructuredError {
            what: format!("cannot infer agent for {operation}"),
            where_: root.display().to_string(),
            why: "no Claude, Codex, or OpenCode markers were found".into(),
            hint: format!(
                "pass an explicit agent, for example `jig agent {operation} claude`, or point --to at a repo that already has agent markers"
            ),
        })),
        _ => Err(JigError::FileOperation(StructuredError {
            what: format!("cannot infer agent for {operation}"),
            where_: root.display().to_string(),
            why: format!(
                "multiple agent markers are present: {}",
                detected
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            hint: "pass an explicit agent to avoid writing into the wrong skill tree".into(),
        })),
    }
}

fn ensure_agent_marker_present(
    agent: AgentKind,
    root: &Path,
    operation: &str,
) -> Result<(), JigError> {
    if agent
        .marker_paths()
        .iter()
        .any(|marker| root.join(marker).exists())
    {
        return Ok(());
    }

    Err(JigError::FileOperation(StructuredError {
        what: format!("cannot {operation} {} skills here", agent),
        where_: root.display().to_string(),
        why: format!(
            "no {} project markers were found in the target directory",
            agent
        ),
        hint: format!(
            "run this inside a repo that already has {} set up, or pass --to to that repo",
            agent
        ),
    }))
}

fn remove_owned_for_agent(resolved: &ResolvedTarget) -> Result<RemoveResult, JigError> {
    let install_base = resolved.target_root.join(resolved.agent.skills_dir());
    let removed_skills = remove_owned_skill_dirs(&install_base, resolved.agent)?;

    Ok(RemoveResult {
        agent: resolved.agent,
        target_root: resolved.target_root.clone(),
        install_base,
        inferred_agent: resolved.inferred_agent,
        removed_skills,
    })
}

fn install_one_skill(
    skill_dir: &Dir<'_>,
    skill_name: &str,
    agent: AgentKind,
    target_dir: &Path,
    force: bool,
) -> Result<(), JigError> {
    let marker_path = target_dir.join(INSTALL_MARKER_FILE);
    if target_dir.exists() {
        if force && is_owned_install(&marker_path, agent)? {
            std::fs::remove_dir_all(target_dir).map_err(|e| {
                JigError::FileOperation(StructuredError {
                    what: format!(
                        "failed to replace installed {} skill '{}'",
                        agent, skill_name
                    ),
                    where_: target_dir.display().to_string(),
                    why: e.to_string(),
                    hint: "check directory permissions and try again".into(),
                })
            })?;
        } else {
            return Err(existing_skill_error(target_dir, skill_name, agent, force));
        }
    }

    std::fs::create_dir_all(target_dir).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!("failed to create skill directory '{}'", skill_name),
            where_: target_dir.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions or choose a different --to path".into(),
        })
    })?;

    write_embedded_dir(skill_dir, skill_dir.path(), target_dir, agent, skill_name)?;
    write_install_marker(&marker_path, agent)?;
    Ok(())
}

fn write_embedded_dir(
    dir: &Dir<'_>,
    source_root: &Path,
    target_root: &Path,
    agent: AgentKind,
    skill_name: &str,
) -> Result<(), JigError> {
    for file in dir.files() {
        write_embedded_file(file, source_root, target_root, agent, skill_name)?;
    }
    for child in dir.dirs() {
        write_embedded_dir(child, source_root, target_root, agent, skill_name)?;
    }
    Ok(())
}

fn write_embedded_file(
    file: &File<'_>,
    source_root: &Path,
    target_root: &Path,
    agent: AgentKind,
    skill_name: &str,
) -> Result<(), JigError> {
    let relative = file.path().strip_prefix(source_root).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: "failed to resolve bundled skill file".into(),
            where_: file.path().display().to_string(),
            why: e.to_string(),
            hint: "check the embedded bundle layout".into(),
        })
    })?;
    let target_path = target_root.join(relative);
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            JigError::FileOperation(StructuredError {
                what: "failed to create bundled skill parent directory".into(),
                where_: parent.display().to_string(),
                why: e.to_string(),
                hint: "check directory permissions".into(),
            })
        })?;
    }

    let bytes = if relative == Path::new("SKILL.md") {
        rewrite_skill_markdown(file, agent, skill_name)?
    } else {
        file.contents().to_vec()
    };

    std::fs::write(&target_path, bytes).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: format!(
                "failed to write bundled skill file '{}'",
                relative.display()
            ),
            where_: target_path.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions".into(),
        })
    })
}

fn rewrite_skill_markdown(
    file: &File<'_>,
    agent: AgentKind,
    skill_name: &str,
) -> Result<Vec<u8>, JigError> {
    let content = file.contents_utf8().ok_or_else(|| {
        JigError::FileOperation(StructuredError {
            what: "bundled SKILL.md is not valid UTF-8".into(),
            where_: file.path().display().to_string(),
            why: "the embedded file could not be decoded as UTF-8".into(),
            hint: "keep bundled skill markdown files as UTF-8 text".into(),
        })
    })?;

    let replacement = format!(
        "${{{}:-{}/{}}}",
        agent.skill_env_var(),
        agent.skills_dir(),
        skill_name
    );
    let content = content.replace(
        &format!("${{CLAUDE_SKILL_DIR:-.claude/skills/{skill_name}}}"),
        &replacement,
    );
    let content = content.replace("${CLAUDE_SKILL_DIR}", &replacement);
    let content = content.replace(" (Jig)", "");
    Ok(content.into_bytes())
}

fn write_install_marker(marker_path: &Path, agent: AgentKind) -> Result<(), JigError> {
    let installed_at_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let marker = InstallMarker {
        owner: "jig".into(),
        agent: agent.as_str().into(),
        bundle_version: current_bundle_version().into(),
        source: BUNDLE_SOURCE_ROOT.into(),
        installed_at_unix,
    };
    let content = serde_json::to_vec_pretty(&marker).unwrap();
    std::fs::write(marker_path, content).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: "failed to write install marker".into(),
            where_: marker_path.display().to_string(),
            why: e.to_string(),
            hint: "check directory permissions".into(),
        })
    })
}

fn is_owned_install(marker_path: &Path, agent: AgentKind) -> Result<bool, JigError> {
    let content = match std::fs::read_to_string(marker_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(JigError::FileOperation(StructuredError {
                what: "failed to inspect existing skill install".into(),
                where_: marker_path.display().to_string(),
                why: err.to_string(),
                hint: "check directory permissions or remove the existing directory manually"
                    .into(),
            }));
        }
    };

    let marker: InstallMarker = serde_json::from_str(&content).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: "failed to parse existing install marker".into(),
            where_: marker_path.display().to_string(),
            why: e.to_string(),
            hint: "remove the directory manually if it is not a jig-managed install".into(),
        })
    })?;
    Ok(marker.owner == "jig" && marker.agent == agent.as_str())
}

fn remove_owned_skill_dirs(
    install_base: &Path,
    agent: AgentKind,
) -> Result<Vec<BundledSkill>, JigError> {
    let Ok(entries) = std::fs::read_dir(install_base) else {
        return Ok(Vec::new());
    };

    let mut removed = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            JigError::FileOperation(StructuredError {
                what: "failed to read installed skill directory".into(),
                where_: install_base.display().to_string(),
                why: e.to_string(),
                hint: "check directory permissions".into(),
            })
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let marker_path = path.join(INSTALL_MARKER_FILE);
        if !is_owned_install(&marker_path, agent)? {
            continue;
        }

        std::fs::remove_dir_all(&path).map_err(|e| {
            JigError::FileOperation(StructuredError {
                what: format!("failed to remove installed {} skill", agent),
                where_: path.display().to_string(),
                why: e.to_string(),
                hint: "check directory permissions and try again".into(),
            })
        })?;
        removed.push(BundledSkill {
            name: entry.file_name().to_string_lossy().to_string(),
        });
    }
    removed.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(removed)
}

fn existing_skill_error(
    target_dir: &Path,
    skill_name: &str,
    agent: AgentKind,
    force: bool,
) -> JigError {
    let hint = if force {
        "jig refused to replace that directory because it is not marked as a jig-owned install"
            .into()
    } else {
        "remove the directory manually, use a different --to path, or re-run with --force if the existing install was created by jig".into()
    };
    JigError::FileOperation(StructuredError {
        what: format!("target skill '{}' already exists", skill_name),
        where_: target_dir.display().to_string(),
        why: format!("{agent} already has content at that skill path"),
        hint,
    })
}

fn installed_skill_records(install_base: &Path) -> Result<Vec<InstalledSkillRecord>, JigError> {
    let Ok(entries) = std::fs::read_dir(install_base) else {
        return Ok(Vec::new());
    };
    let mut installed = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            JigError::FileOperation(StructuredError {
                what: "failed to read installed skill directory".into(),
                where_: install_base.display().to_string(),
                why: e.to_string(),
                hint: "check directory permissions".into(),
            })
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let marker_path = path.join(INSTALL_MARKER_FILE);
        if !marker_path.exists() {
            continue;
        }
        let skill_name = entry.file_name().to_string_lossy().to_string();
        match try_read_install_marker(&marker_path) {
            Ok(Some(marker)) => installed.push(InstalledSkillRecord {
                name: skill_name,
                bundle_version: Some(marker.bundle_version),
                valid_marker: marker.owner == "jig",
            }),
            Ok(None) => {}
            Err(_) => installed.push(InstalledSkillRecord {
                name: skill_name,
                bundle_version: None,
                valid_marker: false,
            }),
        }
    }
    installed.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(installed)
}

fn collect_installed_versions(installed_records: &[InstalledSkillRecord]) -> Vec<String> {
    let mut versions = installed_records
        .iter()
        .filter_map(|record| record.bundle_version.clone())
        .collect::<Vec<_>>();
    versions.sort();
    versions.dedup();
    versions
}

fn diff_skills(left: &[BundledSkill], right: &[BundledSkill]) -> Vec<BundledSkill> {
    let right_names = right
        .iter()
        .map(|skill| skill.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    left.iter()
        .filter(|skill| !right_names.contains(skill.name.as_str()))
        .cloned()
        .collect()
}

fn try_read_install_marker(marker_path: &Path) -> Result<Option<InstallMarker>, JigError> {
    let content = match std::fs::read_to_string(marker_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(JigError::FileOperation(StructuredError {
                what: "failed to inspect existing skill install".into(),
                where_: marker_path.display().to_string(),
                why: err.to_string(),
                hint: "check directory permissions or remove the existing directory manually"
                    .into(),
            }));
        }
    };

    let marker: InstallMarker = serde_json::from_str(&content).map_err(|e| {
        JigError::FileOperation(StructuredError {
            what: "failed to parse existing install marker".into(),
            where_: marker_path.display().to_string(),
            why: e.to_string(),
            hint: "remove the directory manually if it is not a jig-managed install".into(),
        })
    })?;
    Ok(Some(marker))
}

fn skill_name_from_dir(skill_dir: &Dir<'_>) -> Result<String, JigError> {
    skill_dir
        .path()
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or_else(|| {
            JigError::FileOperation(StructuredError {
                what: "failed to resolve bundled skill name".into(),
                where_: skill_dir.path().display().to_string(),
                why: "the embedded skill directory has no final path component".into(),
                hint: "check the embedded bundle layout".into(),
            })
        })
}

fn all_agents() -> Vec<AgentKind> {
    vec![AgentKind::Claude, AgentKind::Codex, AgentKind::OpenCode]
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn detect_agents_from_repo_markers() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".opencode/plugins")).unwrap();

        let detected = detect_agents(tmp.path());
        assert_eq!(detected, vec![AgentKind::Claude, AgentKind::OpenCode]);
    }

    #[test]
    fn install_infers_agent_from_repo_marker() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();

        let result = install(
            InstallRequest {
                agent: None,
                target_root: None,
                force: false,
            },
            tmp.path(),
        )
        .unwrap();

        assert!(result.inferred_agent);
        assert_eq!(result.agent, AgentKind::Claude);
        let skill_md = tmp.path().join(".claude/skills/create-recipe/SKILL.md");
        let content = std::fs::read_to_string(skill_md).unwrap();
        assert!(content.contains("${CLAUDE_SKILL_DIR:-.claude/skills/create-recipe}"));
        assert_eq!(result.installed_skills.len(), bundled_skills().len());
    }

    #[test]
    fn install_rewrites_skill_paths_for_codex() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".codex")).unwrap();

        let result = install(
            InstallRequest {
                agent: Some(AgentKind::Codex),
                target_root: Some(tmp.path().to_path_buf()),
                force: false,
            },
            tmp.path(),
        )
        .unwrap();

        assert!(!result.inferred_agent);
        let skill_md = tmp.path().join(".codex/skills/create-recipe/SKILL.md");
        let content = std::fs::read_to_string(skill_md).unwrap();
        assert!(content.contains("${CODEX_SKILL_DIR:-.codex/skills/create-recipe}"));
        assert!(!content.contains("CLAUDE_SKILL_DIR"));
    }

    #[test]
    fn install_errors_when_multiple_agents_are_present() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".codex")).unwrap();

        let err = install(
            InstallRequest {
                agent: None,
                target_root: None,
                force: false,
            },
            tmp.path(),
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("multiple agent markers are present")
        );
    }

    #[test]
    fn force_refuses_to_replace_unowned_skill_directory() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        let existing = tmp.path().join(".claude/skills/create-recipe");
        std::fs::create_dir_all(&existing).unwrap();
        std::fs::write(existing.join("SKILL.md"), "# user-owned\n").unwrap();

        let err = install(
            InstallRequest {
                agent: Some(AgentKind::Claude),
                target_root: Some(tmp.path().to_path_buf()),
                force: true,
            },
            tmp.path(),
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("not marked as a jig-owned install")
        );
    }

    #[test]
    fn doctor_reports_installed_skills() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".opencode")).unwrap();
        install(
            InstallRequest {
                agent: Some(AgentKind::OpenCode),
                target_root: Some(tmp.path().to_path_buf()),
                force: false,
            },
            tmp.path(),
        )
        .unwrap();

        let report = doctor(None, Some(tmp.path().to_path_buf()), tmp.path()).unwrap();
        let status = report
            .statuses
            .iter()
            .find(|status| status.agent == AgentKind::OpenCode)
            .unwrap();
        assert_eq!(status.installed_skills.len(), bundled_skills().len());
        assert!(status.managed_install_present);
        assert!(status.up_to_date);
        assert_eq!(status.current_bundle_version, current_bundle_version());
    }

    #[test]
    fn update_reinstalls_owned_skills() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        install(
            InstallRequest {
                agent: Some(AgentKind::Claude),
                target_root: Some(tmp.path().to_path_buf()),
                force: false,
            },
            tmp.path(),
        )
        .unwrap();

        let skill_md = tmp.path().join(".claude/skills/create-recipe/SKILL.md");
        std::fs::write(&skill_md, "customized\n").unwrap();

        let updated = update(
            TargetRequest {
                agent: Some(AgentKind::Claude),
                target_root: Some(tmp.path().to_path_buf()),
            },
            tmp.path(),
        )
        .unwrap();

        assert_eq!(updated.removed_skills.len(), bundled_skills().len());
        let content = std::fs::read_to_string(skill_md).unwrap();
        assert!(content.contains("create-recipe"));
        assert!(!content.contains("customized"));
    }

    #[test]
    fn remove_deletes_only_owned_skills() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".codex")).unwrap();
        install(
            InstallRequest {
                agent: Some(AgentKind::Codex),
                target_root: Some(tmp.path().to_path_buf()),
                force: false,
            },
            tmp.path(),
        )
        .unwrap();

        let user_skill_dir = tmp.path().join(".codex/skills/user-skill");
        std::fs::create_dir_all(&user_skill_dir).unwrap();
        std::fs::write(user_skill_dir.join("SKILL.md"), "# mine\n").unwrap();

        let removed = remove(
            TargetRequest {
                agent: Some(AgentKind::Codex),
                target_root: Some(tmp.path().to_path_buf()),
            },
            tmp.path(),
        )
        .unwrap();

        assert_eq!(removed.removed_skills.len(), bundled_skills().len());
        assert!(user_skill_dir.exists());
        assert!(!tmp.path().join(".codex/skills/create-recipe").exists());
    }

    #[test]
    fn update_infers_agent_from_repo_marker() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".opencode")).unwrap();

        let updated = update(
            TargetRequest {
                agent: None,
                target_root: None,
            },
            tmp.path(),
        )
        .unwrap();

        assert!(updated.inferred_agent);
        assert_eq!(updated.agent, AgentKind::OpenCode);
        assert_eq!(updated.installed_skills.len(), bundled_skills().len());
    }

    #[test]
    fn explicit_install_requires_existing_agent_marker() {
        let tmp = TempDir::new().unwrap();

        let err = install(
            InstallRequest {
                agent: Some(AgentKind::Claude),
                target_root: Some(tmp.path().to_path_buf()),
                force: false,
            },
            tmp.path(),
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("no claude project markers were found")
        );
    }

    #[test]
    fn doctor_marks_old_versions_as_needing_update() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        install(
            InstallRequest {
                agent: Some(AgentKind::Claude),
                target_root: Some(tmp.path().to_path_buf()),
                force: false,
            },
            tmp.path(),
        )
        .unwrap();

        let marker_path = tmp
            .path()
            .join(".claude/skills/create-recipe/.jig-agent-install.json");
        let content = std::fs::read_to_string(&marker_path).unwrap();
        let content = content.replace(current_bundle_version(), "0.0.1");
        std::fs::write(&marker_path, content).unwrap();

        let report = doctor(
            Some(AgentKind::Claude),
            Some(tmp.path().to_path_buf()),
            tmp.path(),
        )
        .unwrap();
        let status = &report.statuses[0];
        assert!(status.needs_update);
        assert!(!status.up_to_date);
        assert!(status.installed_versions.contains(&"0.0.1".to_string()));
    }
}
