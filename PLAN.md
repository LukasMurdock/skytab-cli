# skytab Value Plan

This plan focuses on increasing project value, not just adding endpoints.

## Strategy

- Position `skytab` as a decision tool, not only an API wrapper.
- Optimize three value levers in order: activation, retention, leverage.
- Keep `src/read_api.rs` as the single shared core for CLI and MCP.
- Prefer small, high-impact changes before larger product expansion.

## Phase 1 (Week 1): Activation + Usability

- Add output ergonomics across report, timeclock, and payment commands:
  - `--format json|csv|ndjson`
  - `--output <path>`
  - touch points: `src/cli.rs`, `src/main.rs`
- Improve first-run onboarding in `README.md`:
  - install -> auth -> first report -> MCP in one quick path
- Add shell completions (`bash`, `zsh`, `fish`) for discoverability.
- Success metric: new user reaches first useful result in less than 5 minutes.

## Phase 2 (Weeks 2-3): Decision-Grade Features

- Add an `insights` command group:
  - `daily-brief`
  - `labor-vs-sales`
  - `payment-mix`
- Build insights from existing read endpoints; keep raw report commands intact.
- Standardize human-mode summaries so output is consistently actionable.
- Add fixture-based tests for transform-heavy paths in `src/read_api.rs`:
  - payroll parsing
  - till transaction parsing
  - timeclock normalization/aggregation
- Success metric: insight commands become the default path for common workflows.

## Phase 3 (Weeks 3-4): Agent/MCP Value

- Add composite MCP tools in `src/mcp_server.rs` to reduce round trips.
- Publish task-oriented MCP docs in `README.md` with concrete prompt examples.
- Preserve strict read-only guarantees for MCP tools.
- Keep clear structured error kinds to improve agent behavior and retries.
- Success metric: fewer MCP calls per task and higher successful task completion.

## Phase 4 (Weeks 4-5): Trust + Enterprise Readiness

- Move credentials from plaintext config to OS keychain-backed storage with env fallback.
- Add safety guard for non-GET `request` usage in CLI (confirmation or explicit unsafe flag).
- Expand release trust posture in `.github/workflows/release.yml` (distribution hardening).
- Success metric: fewer security objections and easier adoption in stricter environments.

## Recommended Execution Order

1. Activation-first (recommended): complete Phase 1 before insights.
2. Insights-first: prioritize `insights` commands immediately.
3. MCP-first: prioritize composite MCP tools immediately.

Default priority is Option 1.

## Current Workstream: Lazy Keychain Access (In Progress)

Goal: remove repeated keychain prompts during normal CLI/MCP usage while preserving secure credential behavior when auth is actually required.

### Product decisions (locked)

- Use token-first auth flow for both `skytab` and `skytab-mcp`.
- Allow command execution from a valid cached token even when keychain/credentials are currently unavailable.
- Keep `skytab auth login` as force-refresh so it remains an explicit auth check.
- Keep `skytab doctor` as deep diagnostics (it may still touch keychain).
- In `SKYTAB_CREDENTIAL_STORE=keyring` mode, enforce keyring strictly only when refresh/auth is needed.
- Defer legacy plaintext password migration work to auth-time (no eager startup migration).
- Add refresh serialization so concurrent calls do not trigger multi-prompt/multi-auth storms.
- Preserve existing MCP error `kind` values for compatibility while improving refresh-time error messages.
- Ship as one cohesive change: behavior + tests + README updates.

### Implementation plan

- Add base-url-only resolution path in config loading so runtime client creation does not require credential reads.
- Add lazy client construction in `ReadApi` and CLI request path.
- Resolve credentials only inside auth execution path (`/api/v1/auth/authenticate`).
- Add refresh lock + cache recheck after lock acquisition.
- On 401 retry, refresh once with stale-token detection so parallel retries reuse the newly refreshed token.
- Keep eager credential path available for diagnostics and explicit auth checks.
- Add targeted unit tests for lazy auth resolution behavior and refresh dedupe behavior.
- Update README notes to document token-first/lazy credential resolution.

### Acceptance criteria

- Warm cache: no keychain prompt for normal operational commands.
- Cold/expired cache: one credential resolution/auth path, then token cache repopulates.
- Parallel auth pressure: no prompt storm.
- `auth login` still forces a fresh authentication request.
- `doctor` remains deep and authoritative.
