# Manual Release Runbook (v0.5 MVP)

This runbook is the canonical manual release process for `jig` while CI release automation is deferred.

## Security Model

- Official release authenticity is enforced through:
  - maintainer-controlled GitHub release/tag permissions
  - signed `SHA256SUMS`
  - installer signature + checksum verification
- Install exclusivity is not enforced for a public repo:
  - anyone can still clone and build from source

## Governance Checklist (Before Every Release)

Confirm in GitHub settings:

- `main` is protected (required review, restricted push as desired)
- tag protection for `v*` exists
- only designated maintainers can create tags/releases
- release credentials are limited to maintainers

Record completion in release notes or an internal checklist before publishing.

## Prerequisites

- `cargo`
- `rustup` targets for your release matrix
- `openssl`
- `gh` (GitHub CLI) authenticated for release publishing

## Key Setup and Custody

If this repo is ever initialized with a bootstrap key, rotate it before the first public release you announce externally.

1. Generate signing keypair (one-time, maintainer machine):

```bash
mkdir -p ~/.config/jig/release-signing
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:3072 \
  -out ~/.config/jig/release-signing/release-private-key.pem
chmod 600 ~/.config/jig/release-signing/release-private-key.pem
```

2. Export and commit the trusted public key:

```bash
openssl pkey -in ~/.config/jig/release-signing/release-private-key.pem -pubout \
  > keys/release-public-key.pem
```

3. Update the embedded key block in `install.sh` to match `keys/release-public-key.pem`.

4. Keep private key out of the repo and backed up securely.

If key compromise is suspected:
- stop releases immediately
- rotate keypair
- update `keys/release-public-key.pem` and `install.sh`
- publish next patch release with new trust material

## Artifact Contract

Tag format:
- `vX.Y.Z`

Per-target archive format:
- `jig-vX.Y.Z-<target>.tar.gz`

Required release assets:
- per-target archive(s)
- `SHA256SUMS`
- `SHA256SUMS.sig`

Archive contents:
- `jig` executable at archive root

## Build and Package

Build/package one or more targets:

```bash
./scripts/release/build.sh \
  --version v0.1.0 \
  --target aarch64-apple-darwin \
  --target x86_64-unknown-linux-gnu \
  --signing-key ~/.config/jig/release-signing/release-private-key.pem
```

Dry-run (shape validation only):

```bash
./scripts/release/build.sh --version v0.1.0 --dry-run
```

Artifacts are written to `dist/v0.1.0/` by default.

## Local Verification

Verify signed checksums before upload:

```bash
openssl dgst -sha256 \
  -verify keys/release-public-key.pem \
  -signature dist/v0.1.0/SHA256SUMS.sig \
  dist/v0.1.0/SHA256SUMS
```

## Publish GitHub Release

```bash
gh release create v0.1.0 \
  dist/v0.1.0/jig-v0.1.0-aarch64-apple-darwin.tar.gz \
  dist/v0.1.0/jig-v0.1.0-x86_64-unknown-linux-gnu.tar.gz \
  dist/v0.1.0/SHA256SUMS \
  dist/v0.1.0/SHA256SUMS.sig \
  --title "v0.1.0" \
  --notes "Manual release"
```

## Smoke Test Install

Latest:

```bash
curl -fsSL https://raw.githubusercontent.com/Tetra-Research/jig/main/install.sh | sh
```

Pinned:

```bash
curl -fsSL https://raw.githubusercontent.com/Tetra-Research/jig/main/install.sh | sh -s -- --version v0.1.0
```

Development repo override (non-official):

```bash
JIG_RELEASE_REPO=<owner>/<repo> \
JIG_ALLOW_UNOFFICIAL_REPO=1 \
curl -fsSL https://raw.githubusercontent.com/<owner>/<repo>/main/install.sh | sh
```

## Rollback Policy

- Do not mutate or reuse existing tags.
- If a release is bad, cut a new patch release (`vX.Y.(Z+1)`) with corrected artifacts.
