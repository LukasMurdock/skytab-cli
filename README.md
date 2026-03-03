# skytab

A command-line client for the SkyTab API.

## Install

### One-line install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/LukasMurdock/skytab-cli/main/install.sh | sh
```

Install a specific release tag:

```bash
curl -fsSL https://raw.githubusercontent.com/LukasMurdock/skytab-cli/main/install.sh | VERSION=v0.1.0 sh
```

By default this installs to `~/.local/bin`. Override with `INSTALL_DIR`:

```bash
curl -fsSL https://raw.githubusercontent.com/LukasMurdock/skytab-cli/main/install.sh | INSTALL_DIR=/usr/local/bin sh
```

### From source (local)

```bash
cargo build --release
./target/release/skytab --help
```

Optional: move it into your PATH.

```bash
cp ./target/release/skytab /usr/local/bin/skytab
```

### From packaged archive

After downloading a release archive like `skytab-v0.1.0-<target>.tar.gz`:

```bash
tar -xzf skytab-v0.1.0-<target>.tar.gz
./skytab --help
```

## Configure Credentials

You can use env vars or save credentials once.

### Option 1: environment variables

```bash
export SKYTAB_USERNAME="you@example.com"
export SKYTAB_PASSWORD="your-password"
```

### Option 2: save credentials with prompt

```bash
skytab auth set-credentials --username "you@example.com" --prompt-password
```

Config file location:

- macOS: `~/Library/Application Support/skytab/config.toml`
- Linux: `~/.config/skytab/config.toml`

Credential precedence:

1. `SKYTAB_USERNAME` / `SKYTAB_PASSWORD`
2. platform `config.toml`

## Quick Start

```bash
skytab auth login --json
skytab locations list
skytab locations set-default --location-id 43101562
```

With a default location set, most location flags can be omitted.

## Common Commands

### Locations

```bash
skytab locations list --json
skytab locations show-default --json
skytab locations clear-default
```

### Reports

```bash
skytab reports hourly-sales --start "2026-03-01" --end "2026-03-01" --json
skytab reports activity-summary --start "2026-03-01" --end "2026-03-01" --json
skytab reports discount-summary --start "2026-03-01" --end "2026-03-01" --json
skytab reports ticket-detail-closed --start "2026-03-01" --end "2026-03-01" --json
skytab reports sales-summary-by-item --start "2026-03-01" --end "2026-03-01" --json
skytab reports sales-summary-by-revenue-class --start "2026-03-01" --end "2026-03-01" --json
skytab reports till-transaction --start "2026-03-01" --end "2026-03-01" --json
skytab reports payroll --start "2026-03-01" --end "2026-03-01" --json
```

Date-only ranges (`YYYY-MM-DD`) are expanded using location timezone boundaries.
For multi-location calls, all locations must share a timezone when using date-only input.

### Timeclock

```bash
skytab timeclock shifts --start "2026-03-01" --end "2026-03-01"
skytab timeclock shifts --start "2026-03-01" --end "2026-03-01" --json
```

### Payments

```bash
skytab payments transactions --start "2026-03-01" --end "2026-03-01" --order-type SALE --json
```

### Accounts

```bash
skytab accounts preferences --account-id 123456 --json
```

### Generic request mode

```bash
skytab request --method get --path /api/v2/locations --json
```

### Diagnostics

```bash
skytab doctor
skytab doctor --json
```

## Output and Flags

- `--json` pretty JSON output
- `-v, --verbose` request timing and diagnostics (`-vv` for debug-level detail)
- `--base-url` override API base URL

Example with diagnostics:

```bash
skytab -v reports payroll --start "2026-03-01" --end "2026-03-01" --json
```

## Notes

- Auth tokens are cached at `.../skytab/token.json` for 24 hours.
- On `401`, the CLI refreshes auth once and retries.

## Troubleshooting

### `401` invalid email/password

- Check if env vars are overriding saved config:
  - `echo $SKYTAB_USERNAME`
  - `echo $SKYTAB_PASSWORD`
- If needed, clear env vars and retry:

```bash
unset SKYTAB_USERNAME SKYTAB_PASSWORD SKYTAB_BASE_URL
skytab auth login --json
```

### Date-only range returns no data

- Date-only input uses location timezone boundaries.
- For cross-timezone multi-location queries, use explicit RFC3339 timestamps.

### Locations decoding or shape issues

- Use generic request mode to inspect the raw API payload:

```bash
skytab request --method get --path /api/v2/locations --json
```

## Development

See `CONTRIBUTING.md` for local development, formatting, and release workflow.

Release history is tracked in `CHANGELOG.md`.
