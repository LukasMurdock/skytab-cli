# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project uses Semantic Versioning.

## [Unreleased]

### Added

- `doctor` command for env/config/auth/cache diagnostics.
- `reports till-transaction` and `reports payroll` commands.
- typed payroll and till-transaction output models.
- `-v/--verbose` request timing and retry diagnostics.

### Changed

- CLI binary name changed from `skytab-cli` to `skytab`.
- credentials and token cache paths moved to `skytab/` with legacy fallback reads.
- date-only range inputs now resolve using location timezone boundaries.

### Docs

- split user-facing docs (`README.md`) and contributor docs (`CONTRIBUTING.md`).
- added release process guide in `RELEASES.md`.

## [0.1.0] - 2026-03-03

### Added

- initial Rust CLI release with auth, locations, reports, timeclock, payments, accounts, and request mode.
- token caching and automatic auth refresh on 401.
- default location support.
- release workflow for macOS and Linux artifacts.

[Unreleased]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/LukasMurdock/skytab-cli/releases/tag/v0.1.0
