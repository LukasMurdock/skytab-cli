#!/usr/bin/env sh
set -eu

REPO="LukasMurdock/skytab-cli"
BIN_NAME="skytab"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${VERSION:-latest}"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Error: required command '$1' is not installed" >&2
    exit 1
  fi
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin)
      case "$arch" in
        arm64|aarch64) echo "aarch64-apple-darwin" ;;
        x86_64|amd64) echo "x86_64-apple-darwin" ;;
        *)
          echo "Error: unsupported macOS architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    Linux)
      case "$arch" in
        x86_64|amd64) echo "x86_64-unknown-linux-musl" ;;
        *)
          echo "Error: unsupported Linux architecture: $arch" >&2
          echo "Only x86_64 Linux builds are currently published." >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "Error: unsupported OS: $os" >&2
      exit 1
      ;;
  esac
}

sha_file() {
  file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return
  fi

  echo "Error: sha256sum or shasum is required for checksum verification" >&2
  exit 1
}

resolve_version() {
  if [ "$VERSION" != "latest" ]; then
    echo "$VERSION"
    return
  fi

  latest_tag=""
  if latest_json="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null)"; then
    latest_tag="$(printf '%s' "$latest_json" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  fi

  if [ -z "$latest_tag" ]; then
    releases_json="$(curl -fsSL "https://api.github.com/repos/$REPO/releases")"
    latest_tag="$(printf '%s' "$releases_json" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  fi

  if [ -z "$latest_tag" ]; then
    echo "Error: unable to determine a published release tag" >&2
    echo "Try passing a specific tag: VERSION=v0.1.0" >&2
    exit 1
  fi

  echo "$latest_tag"
}

require_cmd curl
require_cmd tar

TARGET="$(detect_target)"
TAG="$(resolve_version)"
ASSET="${BIN_NAME}-${TAG}-${TARGET}.tar.gz"
CHECKSUMS_URL="https://github.com/$REPO/releases/download/$TAG/checksums.txt"
ASSET_URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

echo "Downloading $ASSET"
curl -fsSL "$ASSET_URL" -o "$tmp_dir/$ASSET"
curl -fsSL "$CHECKSUMS_URL" -o "$tmp_dir/checksums.txt"

expected_line="$(grep " $ASSET" "$tmp_dir/checksums.txt" || true)"
if [ -z "$expected_line" ]; then
  echo "Error: checksum for $ASSET not found in checksums.txt" >&2
  exit 1
fi

expected_sum="$(printf '%s' "$expected_line" | awk '{print $1}')"
actual_sum="$(sha_file "$tmp_dir/$ASSET")"

if [ "$expected_sum" != "$actual_sum" ]; then
  echo "Error: checksum verification failed" >&2
  exit 1
fi

mkdir -p "$tmp_dir/extract"
tar -xzf "$tmp_dir/$ASSET" -C "$tmp_dir/extract"

if [ ! -f "$tmp_dir/extract/$BIN_NAME" ]; then
  echo "Error: binary '$BIN_NAME' not found in archive" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
install -m 0755 "$tmp_dir/extract/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"

echo "Installed $BIN_NAME to $INSTALL_DIR/$BIN_NAME"

case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    ;;
  *)
    echo "Note: $INSTALL_DIR is not in your PATH"
    echo "Add this to your shell profile:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

echo "Run: $BIN_NAME --help"
