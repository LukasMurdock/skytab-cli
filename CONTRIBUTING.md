# Contributing

Thanks for contributing to `skytab`.

## Prerequisites

- Rust stable toolchain
- SkyTab credentials for manual endpoint checks

## Local Setup

```bash
git clone https://github.com/lukasmurdock/skytab-cli
cd skytab-cli
cargo check
```

Run the CLI locally:

```bash
cargo run -- --help
```

## Credentials for Development

Use one of these:

- env vars: `SKYTAB_USERNAME`, `SKYTAB_PASSWORD`
- saved config via prompt:

```bash
cargo run -- auth set-credentials --username "you@example.com" --prompt-password
```

Config path:

- macOS: `~/Library/Application Support/skytab/config.toml`
- Linux: `~/.config/skytab/config.toml`

## Coding Workflow

1. Make your changes.
2. Format and type-check.
3. Run key commands manually against a known location.

```bash
cargo fmt
cargo check
```

Suggested manual smoke test:

```bash
cargo run -- auth login --json
cargo run -- locations list
cargo run -- timeclock shifts --start "2026-03-01" --end "2026-03-01"
```

## Project Layout

- `src/cli.rs` command and argument definitions
- `src/main.rs` command execution and output formatting
- `src/client.rs` HTTP/auth/retry logic
- `src/config.rs` credential/config loading and saving
- `src/cache.rs` token cache

## Packaging and Releases

Create a local release archive:

```bash
./scripts/package-local.sh
```

Cross-target local package:

```bash
./scripts/package-local.sh aarch64-apple-darwin
```

CI release workflow:

- `.github/workflows/release.yml`
- trigger by pushing a `v*` tag
- builds platform archives and publishes checksums

## Pull Requests

- Keep changes focused and scoped.
- Update docs when behavior or flags change.
- Include before/after command examples for CLI UX changes.

## Changelog

Update `CHANGELOG.md` for user-facing changes.

- Add a short bullet under `## [Unreleased]` in the most relevant section.
- Keep entries concise and outcome-focused.
- At release time, move `Unreleased` entries into the new version section.
