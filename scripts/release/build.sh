#!/bin/bash
set -euo pipefail

usage() {
    cat << 'EOF'
Usage: scripts/release/build.sh --version <version> [options]

Build and package release artifacts for jig.

Required:
  --version <version>          Release version (accepts 0.1.0 or v0.1.0)

Options:
  --target <triple>            Rust target triple (repeatable)
  --output-dir <path>          Output directory (default: dist/v<version>)
  --signing-key <path>         Private key PEM used to sign SHA256SUMS
  --public-key <path>          Public key PEM for signature verification
                               (default: keys/release-public-key.pem)
  --skip-build                 Package from existing binaries only
  --dry-run                    Print planned actions without writing files
  -h, --help                   Show this help text
EOF
}

log() {
    echo "[release] $*" >&2
}

die() {
    echo "[release] error: $*" >&2
    exit 1
}

run_cmd() {
    if [[ "$DRY_RUN" == "true" ]]; then
        printf '[dry-run] ' >&2
        printf '%q ' "$@" >&2
        printf '\n' >&2
    else
        "$@"
    fi
}

sha256_file() {
    local path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
    else
        die "missing checksum tool: need sha256sum or shasum"
    fi
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
HOST_TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"

VERSION=""
OUTPUT_DIR=""
SIGNING_KEY=""
PUBLIC_KEY="$REPO_ROOT/keys/release-public-key.pem"
SKIP_BUILD="false"
DRY_RUN="false"
TARGETS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            [[ $# -ge 2 ]] || die "--version requires a value"
            VERSION="$2"
            shift 2
            ;;
        --target)
            [[ $# -ge 2 ]] || die "--target requires a value"
            TARGETS+=("$2")
            shift 2
            ;;
        --output-dir)
            [[ $# -ge 2 ]] || die "--output-dir requires a value"
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --signing-key)
            [[ $# -ge 2 ]] || die "--signing-key requires a value"
            SIGNING_KEY="$2"
            shift 2
            ;;
        --public-key)
            [[ $# -ge 2 ]] || die "--public-key requires a value"
            PUBLIC_KEY="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD="true"
            shift
            ;;
        --dry-run)
            DRY_RUN="true"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "unknown argument: $1"
            ;;
    esac
done

[[ -n "$VERSION" ]] || die "--version is required"

if [[ "$VERSION" == v* ]]; then
    TAG_VERSION="$VERSION"
else
    TAG_VERSION="v$VERSION"
fi

if [[ -z "$OUTPUT_DIR" ]]; then
    OUTPUT_DIR="$REPO_ROOT/dist/$TAG_VERSION"
fi

if [[ -z "$HOST_TARGET" ]]; then
    die "could not determine host target from rustc -vV"
fi

if [[ ${#TARGETS[@]} -eq 0 ]]; then
    TARGETS=("$HOST_TARGET")
fi

if [[ "$SKIP_BUILD" == "false" ]]; then
    command -v cargo >/dev/null 2>&1 || die "cargo not found in PATH"
fi

if [[ -n "$SIGNING_KEY" ]]; then
    command -v openssl >/dev/null 2>&1 || die "openssl is required for signing"
    [[ -f "$SIGNING_KEY" ]] || die "signing key not found: $SIGNING_KEY"
fi

if [[ "$DRY_RUN" == "false" ]]; then
    rm -rf "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR"
else
    log "would reset output directory: $OUTPUT_DIR"
fi

ARCHIVES=()

for target in "${TARGETS[@]}"; do
    archive="jig-${TAG_VERSION}-${target}.tar.gz"
    ARCHIVES+=("$archive")

    if [[ "$SKIP_BUILD" == "false" ]]; then
        log "building target: $target"
        run_cmd cargo build --release --locked --target "$target"
    else
        log "skipping build for target: $target"
    fi

    binary_path="$REPO_ROOT/target/$target/release/jig"
    if [[ ! -f "$binary_path" && "$target" == "$HOST_TARGET" && -f "$REPO_ROOT/target/release/jig" ]]; then
        binary_path="$REPO_ROOT/target/release/jig"
    fi
    if [[ "$DRY_RUN" == "false" ]]; then
        [[ -f "$binary_path" ]] || die "missing binary for target $target at $binary_path"
        tmp_dir="$(mktemp -d)"
        cp "$binary_path" "$tmp_dir/jig"
        chmod 0755 "$tmp_dir/jig"
        tar -C "$tmp_dir" -czf "$OUTPUT_DIR/$archive" jig
        rm -rf "$tmp_dir"
    else
        log "would package $binary_path -> $OUTPUT_DIR/$archive"
    fi
done

if [[ "$DRY_RUN" == "false" ]]; then
    sums_file="$OUTPUT_DIR/SHA256SUMS"
    : > "$sums_file"
    for archive in "${ARCHIVES[@]}"; do
        digest="$(sha256_file "$OUTPUT_DIR/$archive")"
        printf '%s  %s\n' "$digest" "$archive" >> "$sums_file"
    done
    log "wrote checksum manifest: $sums_file"

    if [[ -n "$SIGNING_KEY" ]]; then
        sig_file="$OUTPUT_DIR/SHA256SUMS.sig"
        log "signing checksum manifest"
        openssl dgst -sha256 -sign "$SIGNING_KEY" -out "$sig_file" "$sums_file"

        if [[ -f "$PUBLIC_KEY" ]]; then
            log "verifying signature using public key: $PUBLIC_KEY"
            openssl dgst -sha256 -verify "$PUBLIC_KEY" -signature "$sig_file" "$sums_file" >/dev/null
        else
            die "public key not found for verification: $PUBLIC_KEY"
        fi
    fi
else
    log "would write checksum manifest: $OUTPUT_DIR/SHA256SUMS"
    if [[ -n "$SIGNING_KEY" ]]; then
        log "would sign checksum manifest to: $OUTPUT_DIR/SHA256SUMS.sig"
    fi
fi

log "release artifacts ready in $OUTPUT_DIR"
printf '%s\n' "${ARCHIVES[@]}" >&2
