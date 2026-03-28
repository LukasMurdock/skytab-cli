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

Runtime auth is token-first: when a valid cached token exists, operational commands use it without
touching keychain/keyring. Credential store access happens when a fresh auth token is required.

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
skytab insights daily-brief
```

This gives you a login check, location discovery, and an actionable operations summary.

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
- `skytab.insights.daily_brief`
- `skytab.insights.end_of_day`
- `skytab.insights.labor_vs_sales`
- `skytab.insights.payment_mix`
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

### Task-Oriented MCP Prompt Examples

These prompts help agents use high-value tools first, then drill down only when needed.

```text
Build my daily operations brief for 2026-03-01. Use skytab.insights.daily_brief.
Return the top 3 actions based on highlights.
```

```text
Compare labor vs sales for 2026-03-01 through 2026-03-07 for locations [43101562, 43101563]
using skytab.insights.labor_vs_sales. If labor_percent_of_net_sales is above 35,
call skytab.reports.payroll for the same range and list the highest total pay rows.
```

```text
Analyze payment mix for 2026-03-01 with skytab.insights.payment_mix.
If highlights mention unsettled transactions, call skytab.payments.transactions
and break down unsettled transactions by status.
```

```text
Give me an end-of-day summary for 2026-03-01:
Use skytab.insights.end_of_day first.
If I ask follow-ups, then drill into skytab.insights.daily_brief,
skytab.insights.labor_vs_sales, or skytab.insights.payment_mix.
Keep it concise and include one follow-up recommendation per insight.
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
skytab reports hourly-sales --date-range today --json
skytab reports activity-summary --date-range yesterday --json
skytab reports discount-summary --date-range 7days --json
skytab reports ticket-detail-closed --start "2026-03-01" --end "2026-03-01" --json
skytab reports sales-summary-by-item --start "2026-03-01" --end "2026-03-01" --json
skytab reports sales-summary-by-revenue-class --start "2026-03-01" --end "2026-03-01" --json
skytab reports till-transaction --date-range 30days --json
skytab reports payroll --date-range 7days --json
```

Export report results:

```bash
skytab reports payroll --start "2026-03-01" --end "2026-03-01" --format csv --output payroll.csv
skytab reports hourly-sales --start "2026-03-01" --end "2026-03-01" --format ndjson --output hourly-sales.ndjson
```

Date-only ranges (`YYYY-MM-DD`) are expanded using location timezone boundaries.
For multi-location calls, all locations must share a timezone when using date-only input.
For payment-backed endpoints, RFC3339 inputs are normalized to millisecond precision automatically.

Date shortcuts:

- No date flags defaults to `today`.
- `--date-range` supports `today`, `yesterday`, and `Ndays` (for example `7days`, `30days`).
- `--date-range` (without a value) is equivalent to `--date-range today`.
- `--date-range` cannot be combined with `--start`/`--end`.

### Insights

```bash
skytab insights daily-brief
skytab insights labor-vs-sales --date-range 7days --json
skytab insights payment-mix --date-range yesterday --format csv --output payment-mix.csv
```

### Timeclock

```bash
skytab timeclock shifts
skytab timeclock shifts --date-range 7days --json
skytab timeclock shifts --date-range 30days --format csv --output shifts.csv
```

### Payments

```bash
skytab payments transactions --date-range yesterday --order-type SALE --json
skytab payments transactions --date-range 7days --format ndjson --output payments.ndjson
skytab payments transactions --start "2026-03-01T00:00:00Z" --end "2026-03-01T23:59:59Z" --json
```

`payments transactions` accepts RFC3339 with or without milliseconds.
The CLI normalizes payment query timestamps to millisecond precision automatically.

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
- `insights daily-brief`: `period_start,period_end,location_ids,...,highlights`
- `insights labor-vs-sales`: `period_start,period_end,location_ids,...,highlights`
- `insights payment-mix`: `row_type,key,count,amount,...,highlights`
- Update golden fixtures after schema changes: `./scripts/update-csv-fixtures.sh`

## Output and Flags

- `--json` pretty JSON output
- `--format json|csv|ndjson` structured output format
- `--output <path>` write output to a file (without `--format`, writes JSON)
- `--date-range [today|yesterday|Ndays]` date shortcut for report/insight/timeclock/payment commands (defaults to `today` when omitted)
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
- Credential lookup (env/keyring/config) is lazy and happens when auth refresh is needed.

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
