# Releases

This document explains how `skytab` releases are created and published.

## Versioning

Use semantic version tags:

- `v0.1.0`
- `v0.1.1`
- `v0.2.0`
- `v1.0.0`

Tag format must start with `v` because the release workflow is triggered by `v*` tags.

## What Happens on a Release Tag

When you push a tag like `v0.1.0`, GitHub Actions runs `.github/workflows/release.yml`.

The workflow:

1. Builds `skytab` in release mode for:
   - `x86_64-unknown-linux-musl`
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
2. Packages each build into a `.tar.gz` archive with:
   - `skytab` binary
   - `README.md`
   - `LICENSE`
3. Generates `checksums.txt` (SHA256 for all archives).
4. Publishes a GitHub Release with artifacts and generated notes.

## Release Artifact Names

Artifacts follow this pattern:

`skytab-vX.Y.Z-<target>.tar.gz`

Examples:

- `skytab-v0.1.0-x86_64-unknown-linux-musl.tar.gz`
- `skytab-v0.1.0-aarch64-apple-darwin.tar.gz`
- `skytab-v0.1.0-x86_64-apple-darwin.tar.gz`

## Maintainer Release Checklist

Before tagging:

1. Confirm branch is clean and pushed:
   - `git status`
   - `git push origin main`
2. Run local checks:
   - `cargo fmt`
   - `cargo check`
3. (Optional) Build local package:
   - `./scripts/package-local.sh`
4. Ensure docs are up to date (`README.md`, `CONTRIBUTING.md`, this file).

## Creating a Release

From `main`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Then verify in GitHub:

1. Open Actions tab and watch `release` workflow.
2. Confirm all matrix builds pass.
3. Open Releases tab and confirm:
   - artifacts are attached
   - `checksums.txt` exists

## Verifying Downloaded Binaries

After downloading release files:

```bash
shasum -a 256 -c checksums.txt
```

Expected: archive checksums validate successfully.

## Re-running a Failed Release

If workflow fails:

1. Fix the issue on `main`.
2. Create a new patch tag (recommended), for example:
   - `v0.1.1`
3. Push the new tag.

Avoid deleting/reusing published tags unless absolutely necessary.

## Notes

- Workflow also supports manual run via `workflow_dispatch`, but tag-based release is the canonical path.
- Release binaries are named `skytab`.
