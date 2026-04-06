#!/bin/sh
set -eu

DEFAULT_REPO="Tetra-Research/jig"
REPO="${JIG_RELEASE_REPO:-$DEFAULT_REPO}"
ALLOW_UNOFFICIAL="${JIG_ALLOW_UNOFFICIAL_REPO:-0}"
VERSION="${JIG_VERSION:-latest}"
INSTALL_DIR="${JIG_INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="jig"
release_meta=""
tmp_dir=""

usage() {
    cat << 'EOF'
Usage: install.sh [--version <version>] [--install-dir <path>] [--repo <owner/repo>]

Environment variables:
  JIG_VERSION                  Version to install (default: latest)
  JIG_INSTALL_DIR              Install destination (default: $HOME/.local/bin)
  JIG_RELEASE_REPO             Release repo (default: Tetra-Research/jig)
  JIG_ALLOW_UNOFFICIAL_REPO    Set to 1 to allow non-default repo
  JIG_TRUSTED_PUBLIC_KEY_FILE  Override trusted public key file path
EOF
}

log() {
    printf '[jig-install] %s\n' "$*" >&2
}

die() {
    log "error: $*"
    exit 1
}

cleanup() {
    if [ -n "$release_meta" ] && [ -d "$release_meta" ]; then
        rm -rf "$release_meta"
    fi

    if [ -n "$tmp_dir" ] && [ -d "$tmp_dir" ]; then
        rm -rf "$tmp_dir"
    fi
}

trap cleanup EXIT INT TERM HUP

download() {
    url="$1"
    out="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fL --silent --show-error "$url" -o "$out"
        return
    fi

    if command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$out"
        return
    fi

    die "missing downloader: install curl or wget"
}

sha256_file() {
    path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
        return
    fi

    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
        return
    fi

    die "missing checksum tool: install sha256sum or shasum"
}

detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os:$arch" in
        Darwin:arm64|Darwin:aarch64)
            printf 'aarch64-apple-darwin'
            ;;
        Darwin:x86_64)
            printf 'x86_64-apple-darwin'
            ;;
        Linux:x86_64|Linux:amd64)
            printf 'x86_64-unknown-linux-gnu'
            ;;
        Linux:aarch64|Linux:arm64)
            printf 'aarch64-unknown-linux-gnu'
            ;;
        *)
            return 1
            ;;
    esac
}

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            [ $# -ge 2 ] || die "--version requires a value"
            VERSION="$2"
            shift 2
            ;;
        --install-dir)
            [ $# -ge 2 ] || die "--install-dir requires a value"
            INSTALL_DIR="$2"
            shift 2
            ;;
        --repo)
            [ $# -ge 2 ] || die "--repo requires a value"
            REPO="$2"
            shift 2
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

if [ "$REPO" != "$DEFAULT_REPO" ] && [ "$ALLOW_UNOFFICIAL" != "1" ]; then
    die "refusing unofficial repo '$REPO'. Set JIG_ALLOW_UNOFFICIAL_REPO=1 to override."
fi

if [ "$VERSION" = "latest" ]; then
    release_meta="$(mktemp -d 2>/dev/null || mktemp -d -t jig-install-meta)"
    meta_json="$release_meta/latest.json"
    download "https://api.github.com/repos/$REPO/releases/latest" "$meta_json"
    VERSION="$(sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' "$meta_json" | head -n1)"
    rm -rf "$release_meta"
    release_meta=""
    [ -n "$VERSION" ] || die "could not resolve latest release tag from GitHub API"
fi

case "$VERSION" in
    v*)
        TAG="$VERSION"
        ;;
    *)
        TAG="v$VERSION"
        ;;
esac

target="$(detect_target || true)"
[ -n "$target" ] || die "unsupported platform: $(uname -s) $(uname -m)"

tmp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t jig-install)"

archive="jig-${TAG}-${target}.tar.gz"
sums_file="SHA256SUMS"
sig_file="SHA256SUMS.sig"
base_url="https://github.com/$REPO/releases/download/$TAG"

if [ -n "${JIG_TRUSTED_PUBLIC_KEY_FILE:-}" ]; then
    pubkey="$JIG_TRUSTED_PUBLIC_KEY_FILE"
    [ -f "$pubkey" ] || die "trusted public key file not found: $pubkey"
else
    pubkey="$tmp_dir/release-public-key.pem"
    cat > "$pubkey" << 'EOF'
-----BEGIN PUBLIC KEY-----
MIIBojANBgkqhkiG9w0BAQEFAAOCAY8AMIIBigKCAYEAqVng4Z/gvxzIdsTkl9+u
yrt8lQL2wFKLDOpHjOJIv5pAv+uIH3rz2q3rmt2g9yhqhm9ofujp8khhQ+j3EpOz
rghkXTVPjOVM82WZL33IaYmwj6NdVPSkU3eCAuiQ6dt0lUyW34Y3ACK5vZSNqeyH
ky2XrKSyfF3PmPVDe25yyxJMXLRIx6WGEhyzWBzKUdOxx+59SPLn+nYC89jZ35xL
9D3soCYtFJHd08HphAD2EHuf4vRIoAOd2CaGEVdS3J6qWWTAidq6JOIAsZve1VXd
wRCahTj8AWatboYRUO6pm9iEndrjmmi4wMykeW5uDHWMJWRCAOXFe2dEBdi9A5HY
xIcqRc09scc8YxPEYZZ1DSdIHQVslzEP8pkNbCKOXQp6PQCuIQTp+OtUD4mnpxqe
nzyoQ6Iw5zypvR3hbTz4dqQLugZTfGfhyMyd/cEZGVBbxkmjHrww3Vu9eOEA39TP
Xy+vh/lMNGE4IicoZ6HQYWfLfM6peWFiku/IMSs60sUFAgMBAAE=
-----END PUBLIC KEY-----
EOF
fi

log "Installing $BIN_NAME $TAG for target $target from $REPO"
download "$base_url/$archive" "$tmp_dir/$archive"
download "$base_url/$sums_file" "$tmp_dir/$sums_file"
download "$base_url/$sig_file" "$tmp_dir/$sig_file"

command -v openssl >/dev/null 2>&1 || die "openssl is required for signature verification"
if ! openssl dgst -sha256 -verify "$pubkey" -signature "$tmp_dir/$sig_file" "$tmp_dir/$sums_file" >/dev/null 2>&1; then
    die "signature verification failed for $sums_file"
fi
log "signature verified"

expected_sum="$(grep "  $archive$" "$tmp_dir/$sums_file" | awk '{print $1}' | head -n1 || true)"
[ -n "$expected_sum" ] || die "checksum entry for $archive not found in $sums_file"

actual_sum="$(sha256_file "$tmp_dir/$archive")"
if [ "$expected_sum" != "$actual_sum" ]; then
    die "checksum mismatch for $archive"
fi
log "checksum verified"

extract_dir="$tmp_dir/extract"
mkdir -p "$extract_dir"
tar -xzf "$tmp_dir/$archive" -C "$extract_dir"

src_bin="$extract_dir/$BIN_NAME"
if [ ! -f "$src_bin" ]; then
    src_bin="$(find "$extract_dir" -type f -name "$BIN_NAME" | head -n1 || true)"
fi
[ -n "$src_bin" ] && [ -f "$src_bin" ] || die "binary '$BIN_NAME' not found in archive"

mkdir -p "$INSTALL_DIR"
if command -v install >/dev/null 2>&1; then
    if ! install -m 0755 "$src_bin" "$INSTALL_DIR/$BIN_NAME"; then
        die "failed to install into $INSTALL_DIR (check permissions)"
    fi
else
    cp "$src_bin" "$INSTALL_DIR/$BIN_NAME"
    chmod 0755 "$INSTALL_DIR/$BIN_NAME"
fi

log "installed to $INSTALL_DIR/$BIN_NAME"
if ! "$INSTALL_DIR/$BIN_NAME" --version >/dev/null 2>&1; then
    log "installed binary did not return version output (command still installed)"
fi

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        log "add $INSTALL_DIR to PATH to run '$BIN_NAME' directly"
        ;;
esac
