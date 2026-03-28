# skytab

A command-line client for the SkyTab API.

## Install

### One-line install (recommended)

```bash
curl -fsSL https://github.com/LukasMurdock/skytab-cli/releases/latest/download/install.sh | sh
```

Install a specific release tag:

```bash
curl -fsSL https://github.com/LukasMurdock/skytab-cli/releases/download/v0.1.2/install.sh | sh
```

By default this installs to `~/.local/bin`. Override with `INSTALL_DIR`:

```bash
curl -fsSL https://github.com/LukasMurdock/skytab-cli/releases/latest/download/install.sh | INSTALL_DIR=/usr/local/bin sh
```

### From source (local)

```bash
cargo build --release
./target/release/skytab --help
./target/release/skytab-mcp --help
```

Optional: move it into your PATH.

```bash
cp ./target/release/skytab /usr/local/bin/skytab
cp ./target/release/skytab-mcp /usr/local/bin/skytab-mcp
```

### From packaged archive

After downloading a release archive like `skytab-v0.1.0-<target>.tar.gz`:

```bash
tar -xzf skytab-v0.1.0-<target>.tar.gz
./skytab --help
./skytab-mcp --help
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
If your account has exactly one location, the CLI auto-selects it.

## MCP Server (Read-Only)

`skytab-mcp` exposes all read-only SkyTab operations over Model Context Protocol (stdio transport).

Run locally:

```bash
cargo run --bin skytab-mcp --
```

Install `skytab-mcp` from latest release:

```bash
curl -fsSL https://github.com/LukasMurdock/skytab-cli/releases/latest/download/install.sh | BIN_NAME=skytab-mcp sh
```

Or with release binary:

```bash
./target/release/skytab-mcp
```

Available tools:

- `skytab.auth.login`
- `skytab.locations.list`
- `skytab.locations.show_default`
- `skytab.accounts.preferences`
- `skytab.reports.activity_summary`
- `skytab.reports.discount_summary`
- `skytab.reports.hourly_sales`
- `skytab.reports.ticket_detail_closed`
- `skytab.reports.sales_summary_by_item`
- `skytab.reports.sales_summary_by_revenue_class`
- `skytab.reports.till_transaction`
- `skytab.reports.payroll`
- `skytab.timeclock.shifts`
- `skytab.payments.transactions`
- `skytab.request.get` (GET only)
- `skytab.doctor`

Example Claude Desktop MCP config:

```json
{
  "mcpServers": {
    "skytab": {
      "command": "/absolute/path/to/skytab-mcp",
      "env": {
        "SKYTAB_USERNAME": "you@example.com",
        "SKYTAB_PASSWORD": "your-password"
      }
    }
  }
}
```

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
