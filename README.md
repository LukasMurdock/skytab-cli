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

Print shell completion setup snippets after install:

```bash
curl -fsSL https://github.com/LukasMurdock/skytab-cli/releases/latest/download/install.sh | PRINT_COMPLETION_SNIPPETS=1 sh
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

By default, `set-credentials` stores passwords in the OS credential store (keychain/keyring) when available.
If secure storage is unavailable, it falls back to `config.toml` in `auto` mode.

Credential store mode (`SKYTAB_CREDENTIAL_STORE`):

- `auto` (default): prefer keyring, fallback to config file
- `keyring`: require keyring, fail if unavailable
- `config`: always store/read password in config file (legacy)

Config file location:

- macOS: `~/Library/Application Support/skytab/config.toml`
- Linux: `~/.config/skytab/config.toml`

Credential precedence:

1. `SKYTAB_USERNAME` / `SKYTAB_PASSWORD`
2. username + base URL from `config.toml`, password from keyring/keychain
3. legacy `password` in `config.toml` (migration fallback)

## Quick Start

```bash
skytab auth login --json
skytab locations list
skytab locations set-default --location-id 43101562
```

With a default location set, most location flags can be omitted.
If your account has exactly one location, the CLI auto-selects it.

## First Useful Output (5 Minutes)

```bash
skytab auth login --json
skytab locations list
skytab reports hourly-sales --start "2026-03-01" --end "2026-03-01" --format csv --output hourly-sales.csv
```

This gives you a login check, location discovery, and an exportable report.

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

Export report results:

```bash
skytab reports payroll --start "2026-03-01" --end "2026-03-01" --format csv --output payroll.csv
skytab reports hourly-sales --start "2026-03-01" --end "2026-03-01" --format ndjson --output hourly-sales.ndjson
```

Date-only ranges (`YYYY-MM-DD`) are expanded using location timezone boundaries.
For multi-location calls, all locations must share a timezone when using date-only input.

### Timeclock

```bash
skytab timeclock shifts --start "2026-03-01" --end "2026-03-01"
skytab timeclock shifts --start "2026-03-01" --end "2026-03-01" --json
skytab timeclock shifts --start "2026-03-01" --end "2026-03-01" --format csv --output shifts.csv
```

### Payments

```bash
skytab payments transactions --start "2026-03-01" --end "2026-03-01" --order-type SALE --json
skytab payments transactions --start "2026-03-01" --end "2026-03-01" --format ndjson --output payments.ndjson
```

### Accounts

```bash
skytab accounts preferences --account-id 123456 --json
```

### Generic request mode

```bash
skytab request --method get --path /api/v2/locations --json
skytab request --method post --path /api/v2/example --body '{"hello":"world"}' --allow-write --json
```

Mutating request methods (`post`, `put`, `patch`, `delete`) are blocked unless `--allow-write` is passed.

### Diagnostics

```bash
skytab doctor
skytab doctor --json
```

### Shell completion

```bash
skytab completion bash > ~/.local/share/bash-completion/completions/skytab
skytab completion zsh > ~/.zfunc/_skytab
skytab completion fish > ~/.config/fish/completions/skytab.fish
```

### Stable CSV schemas

- `reports hourly-sales`: `date,hour,gross,net`
- `reports payroll`: `row_type,employee_id,employee_name,...,net_tips`
- `timeclock shifts`: `shift_guid,employee_name,clocked_in_at,...,location_id`
- `payments transactions`: `transaction_id,date,type,status,...,raw_json`
- Update golden fixtures after schema changes: `./scripts/update-csv-fixtures.sh`

## Output and Flags

- `--json` pretty JSON output
- `--format json|csv|ndjson` structured output format
- `--output <path>` write output to a file (without `--format`, writes JSON)
- `request --allow-write` required for mutating HTTP methods (`post`, `put`, `patch`, `delete`)
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
