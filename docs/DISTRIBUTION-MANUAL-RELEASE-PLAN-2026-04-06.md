# Distribution Manual Release Plan (v0.5 MVP)

Date: 2026-04-06  
Status: In Progress

## Objective

Ship a no-cost, manual-first distribution path so friends can install `jig` from GitHub Releases via a shell installer, with explicit controls so only trusted maintainers can publish official releases.

## Scope

In scope:
- Manual GitHub Releases as the only distribution backend
- Shell installer (`curl -fsSL .../install.sh | sh`) that fetches release assets
- Release governance controls (maintainer-only release/tag authority)
- Signed release metadata + checksum verification in installer
- Documentation and runbook for repeatable manual release execution

Out of scope (explicitly deferred):
- Preventing arbitrary users from building from source after cloning a public repo
- Private-only distribution/auth-gated downloads
- Homebrew tap
- GitHub Actions release automation
- crates.io publish
- npm wrapper packaging
- Nix flake distribution

## Decisions Locked

1. MVP distribution channels are GitHub Releases + shell installer only.
2. Release execution is manual to keep operating cost at $0.
3. Public-repo model guarantees authenticity of official releases, not install exclusivity.
4. Official installer trusts one source repo (`Tetra-Research/jig`) unless explicitly overridden for development.
5. Asset naming must be stable and installer-compatible before first public install.

## Execution Plan

### Phase 1: Release Governance + Trust Model

Define who is allowed to publish and how users verify trust.

- Lock GitHub permissions:
  - protect `main` branch
  - protect `v*` tags
  - restrict release creation/tag push to designated maintainers
- Define release signing mechanism (prefer minisign/sigstore-style detached signatures for installer automation).
- Check in trusted public key material used by installer verification.
- Define key custody/rotation process (owner, backup, rotation trigger, compromise response).

Validation criteria:
- Governance settings are documented step-by-step in runbook.
- Trusted public key is versioned in repo and referenced by installer design.
- Threat model statement is explicit: official authenticity yes, public-source exclusivity no.

### Phase 2: Release Artifact Contract

Define the release contract the installer will depend on.

- Choose MVP platform matrix (minimum: macOS arm64 + Linux x86_64; expand only if needed).
- Define deterministic asset names (example: `jig-vX.Y.Z-<target>.tar.gz`).
- Define checksum/signature artifacts (`SHA256SUMS` + detached signature file).
- Define what each archive contains (`jig` binary, license/readme optional).

Validation criteria:
- Contract documented and versioned in repo.
- Two sample artifact names, checksum entries, and signature paths are validated end-to-end in docs/examples.

### Phase 3: Local Packaging Script

Create local tooling to produce release artifacts consistently.

- Add release packaging script(s) under `scripts/release/` to:
  - build per target
  - archive outputs with contract-compliant names
  - generate `SHA256SUMS`
  - sign `SHA256SUMS` with release key
- Add guardrails for reproducibility (clean output dir, explicit version argument, fail on missing toolchain).
- Add a local dry-run mode to validate output shape without publishing.

Validation criteria:
- Running the script twice with same version emits same filenames and manifest shape.
- Generated `SHA256SUMS` entries match artifact files.
- Signature verification succeeds against checked-in trusted public key.

### Phase 4: Installer Script

Implement install flow that pulls from GitHub Releases.

- Add `install.sh` that:
  - detects OS/arch
  - resolves release version (`latest` by default, pinned version optional)
  - downloads matching asset + `SHA256SUMS` + signature
  - verifies signature of `SHA256SUMS` using trusted public key
  - verifies asset checksum before install
  - enforces official release source repo by default
  - installs to a predictable path (`$HOME/.local/bin` default; override supported)
- Add clear failure messages for unsupported platform, missing asset, signature failure, checksum mismatch.

Validation criteria:
- Fresh machine/local sandbox install succeeds for supported platforms.
- Signature mismatch is detected and aborts.
- Checksum mismatch is detected and aborts.
- Pinned version install path works.

### Phase 5: Manual Release Runbook + Docs

Document exactly how to cut and validate a manual release.

- Add `docs/RELEASE-MANUAL.md` runbook:
  - bump version
  - build artifacts
  - create GitHub Release/tag
  - upload artifacts + checksums + signatures
  - smoke test install command
- Add release-governance checklist (confirm protected tags/permissions before release).
- Update README install section with canonical one-liner and version pin example.
- Add security model section to README (official release verification vs public-source limitations).
- Add rollback notes (bad release replaced by newer patch release; avoid mutating existing tags).

Validation criteria:
- A human following runbook from scratch can cut a release without tribal knowledge.
- README instructions are sufficient to install and verify `jig --version`.
- Security model is explicit and matches installer behavior.

## Proposed File Touches

- `scripts/release/build.sh` (new)
- `scripts/release/package.sh` (new, optional split)
- `install.sh` (new)
- `docs/RELEASE-MANUAL.md` (new)
- `docs/RELEASE-SECURITY.md` (new, optional if split from runbook)
- `README.md` (install section)
- `docs/ROADMAP.md` (status updates as phases complete)

## Acceptance Criteria (MVP Ready)

1. WHEN a maintainer runs the release script with a version, the system SHALL produce contract-compliant artifacts and `SHA256SUMS`.
2. WHEN a maintainer runs the release script, the system SHALL also produce signature artifacts verifiable by the installer's trusted public key.
3. WHEN a user runs the shell installer on a supported platform, the system SHALL download, verify, and install `jig` successfully from the official repo.
4. IF signature verification fails, the installer SHALL abort with a clear error before checksum or install steps.
5. IF the downloaded artifact checksum does not match `SHA256SUMS`, the installer SHALL abort with a clear error.
6. WHEN a user pins a version in installer input, the system SHALL install that exact release version.
7. WHILE Homebrew/CI automation is deferred, the release runbook SHALL fully describe the manual release path and governance checks.

## Risks and Mitigations

- Risk: Cross-compilation complexity on a single machine.
  - Mitigation: Start with a minimal platform matrix and expand after first successful release.
- Risk: Installer breakage from asset naming drift.
  - Mitigation: Treat naming contract as immutable; gate changes behind explicit plan update.
- Risk: Signing key compromise.
  - Mitigation: Document key custody, rotate keys on compromise, and publish trust-update procedure.
- Risk: Manual process errors during release.
  - Mitigation: Checklist-driven runbook with smoke test before announcement.

## Rollout Sequence

1. Commit A: governance/trust model + artifact contract.
2. Commit B: packaging scripts + signature generation.
3. Commit C: installer + signature/checksum verification.
4. Commit D: runbook + README security/install docs.
5. Cut first manual GitHub Release and run smoke test from clean environment.
