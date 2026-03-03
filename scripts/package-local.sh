#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-$(rustc -Vv | awk '/host:/ {print $2}') }"
VERSION="$(grep '^version' Cargo.toml | head -n1 | cut -d '"' -f2)"
NAME="skytab"
ARTIFACT="${NAME}-v${VERSION}-${TARGET}.tar.gz"

rustup target add "${TARGET}" >/dev/null
cargo build --release --target "${TARGET}"

TMP_DIR="$(mktemp -d)"
cp "target/${TARGET}/release/${NAME}" "${TMP_DIR}/${NAME}"
cp README.md "${TMP_DIR}/README.md"
cp LICENSE "${TMP_DIR}/LICENSE"

tar -C "${TMP_DIR}" -czf "${ARTIFACT}" "${NAME}" README.md LICENSE
rm -rf "${TMP_DIR}"

echo "Created ${ARTIFACT}"
