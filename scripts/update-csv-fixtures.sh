#!/usr/bin/env bash
set -euo pipefail

UPDATE_CSV_FIXTURES=1 cargo test --test csv_schema_golden -- --nocapture

echo "Updated CSV golden fixtures in tests/fixtures/csv"
