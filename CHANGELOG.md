# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project uses Semantic Versioning.

## [Unreleased]

### Changed

- add a labeled header row (`DATE`, `HOUR`, `GROSS`, `NET`) to `reports hourly-sales` human output.

## [0.1.4] - 2026-03-03

### Changed

- auto-select a location when account access includes exactly one location and no `--location` is passed.

### Docs

- document single-location auto-selection in `README.md`.
- add explicit `CHANGELOG.md` update step to `RELEASES.md` checklist.

## [0.1.0] - 2026-03-03

### Added

- initial Rust CLI release with auth, locations, reports, timeclock, payments, accounts, and request mode.
- token caching and automatic auth refresh on 401.
- default location support.
- release workflow for macOS and Linux artifacts.

[Unreleased]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/LukasMurdock/skytab-cli/releases/tag/v0.1.4
[0.1.0]: https://github.com/LukasMurdock/skytab-cli/releases/tag/v0.1.0
