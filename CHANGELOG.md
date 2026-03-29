# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project uses Semantic Versioning.

## [Unreleased]

## [0.1.5] - 2026-03-29

### Added

- add `skytab-mcp`, a read-only MCP server exposing auth, locations, reports, timeclock, payments, request GET, and doctor tools.
- extract shared read-only API logic into `src/read_api.rs` so CLI and MCP use the same core paths.
- add structured output export options: `--format json|csv|ndjson` and `--output <path>`.
- add stable CSV schemas for `reports hourly-sales`, `reports payroll`, `timeclock shifts`, `payments transactions`, and all `insights` commands.
- add `skytab completion <bash|zsh|fish>` to generate shell completion scripts.
- add installer opt-in completion snippets with `PRINT_COMPLETION_SNIPPETS=1`.
- add `scripts/update-csv-fixtures.sh` to regenerate CSV golden fixtures.
- add keyring-backed credential storage (with config fallback in `auto` mode).
- add `insights` command group with `daily-brief`, `labor-vs-sales`, and `payment-mix` decision summaries.
- add MCP tools for insights: `skytab.insights.daily_brief`, `skytab.insights.labor_vs_sales`, `skytab.insights.payment_mix`, and composite `skytab.insights.end_of_day`.
- add `--date-range` shortcuts (`today`, `yesterday`, `Ndays`) for report, insight, timeclock, and payment commands, with implicit `today` defaults.

### CI

- add a dedicated `ci` workflow with separate `rust-checks` and `mcp-protocol-tests` jobs.
- run `csv_schema_golden` integration tests in `rust-checks`.

### Changed

- add a labeled header row (`DATE`, `HOUR`, `GROSS`, `NET`) to `reports hourly-sales` human output.
- route report, timeclock, payments, and doctor commands through shared structured output handling.
- block mutating `request` methods by default; require explicit `--allow-write` for `post`, `put`, `patch`, and `delete`.
- enforce full env credential pairs (error on only one of `SKYTAB_USERNAME`/`SKYTAB_PASSWORD`).
- harden token cache writes to private file permissions on unix (`0600`).
- resolve relative date shortcuts for `payments transactions` in location timezone before querying.

### Docs

- add a "First Useful Output (5 Minutes)" quick path and shell completion examples to `README.md`.
- add task-oriented MCP prompt examples in `README.md` for daily brief, labor-vs-sales, and payment-mix workflows.

### Tests

- add CSV golden fixture tests for hourly sales, payroll, timeclock shifts, and payments transactions.

## [0.1.4] - 2026-03-03

### Changed

- auto-select a location when account access includes exactly one location and no `--location` is passed.

### Docs

- document single-location auto-selection in `README.md`.
- add explicit `CHANGELOG.md` update step to `RELEASES.md` checklist.

## [0.1.3] - 2026-03-03

### Changed

- publish installer script as release asset.

## [0.1.2] - 2026-03-03

### Changed

- fix release matrix for supported macOS runners.

## [0.1.1] - 2026-03-03

### Added

- add release docs and one-line installer.

## [0.1.0] - 2026-03-03

### Added

- initial Rust CLI release with auth, locations, reports, timeclock, payments, accounts, and request mode.
- token caching and automatic auth refresh on 401.
- default location support.
- release workflow for macOS and Linux artifacts.

[Unreleased]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.5...HEAD
[0.1.5]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/LukasMurdock/skytab-cli/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/LukasMurdock/skytab-cli/releases/tag/v0.1.0
