use std::{fs, path::PathBuf};

use serde_json::Value;
use skytab_cli::cli::OutputFormat;
use skytab_cli::output::{CsvSchema, render_structured_value_with_schema};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("csv")
}

fn read_fixture(name: &str) -> String {
    let path = fixture_dir().join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn normalize(text: String) -> String {
    text.replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_string()
}

fn expected_fixture_path(case_name: &str) -> PathBuf {
    fixture_dir().join(format!("{case_name}.expected.csv"))
}

fn update_mode_enabled() -> bool {
    matches!(std::env::var("UPDATE_CSV_FIXTURES").as_deref(), Ok("1"))
}

fn assert_schema_matches_fixture(case_name: &str, schema: CsvSchema) {
    let input = read_fixture(&format!("{case_name}.input.json"));
    let input_value: Value = serde_json::from_str(&input)
        .unwrap_or_else(|err| panic!("invalid fixture JSON for {case_name}: {err}"));

    let actual = render_structured_value_with_schema(&input_value, OutputFormat::Csv, Some(schema))
        .unwrap_or_else(|err| panic!("failed rendering csv for {case_name}: {err}"));
    let normalized_actual = normalize(actual);

    let expected_path = expected_fixture_path(case_name);
    if update_mode_enabled() {
        let with_newline = format!("{normalized_actual}\n");
        fs::write(&expected_path, with_newline).unwrap_or_else(|err| {
            panic!(
                "failed writing updated fixture {}: {err}",
                expected_path.display()
            )
        });
    }

    let expected = fs::read_to_string(&expected_path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", expected_path.display()));

    assert_eq!(
        normalized_actual,
        normalize(expected),
        "csv output mismatch for fixture case: {case_name}"
    );
}

#[test]
fn hourly_sales_csv_matches_golden_fixture() {
    assert_schema_matches_fixture("hourly_sales", CsvSchema::HourlySales);
}

#[test]
fn payroll_csv_matches_golden_fixture() {
    assert_schema_matches_fixture("payroll", CsvSchema::Payroll);
}

#[test]
fn timeclock_csv_matches_golden_fixture() {
    assert_schema_matches_fixture("timeclock_shifts", CsvSchema::TimeclockShifts);
}

#[test]
fn payments_csv_matches_golden_fixture() {
    assert_schema_matches_fixture("payments_transactions", CsvSchema::PaymentsTransactions);
}

#[test]
fn insights_daily_brief_csv_matches_golden_fixture() {
    assert_schema_matches_fixture("insights_daily_brief", CsvSchema::InsightsDailyBrief);
}

#[test]
fn insights_labor_vs_sales_csv_matches_golden_fixture() {
    assert_schema_matches_fixture("insights_labor_vs_sales", CsvSchema::InsightsLaborVsSales);
}

#[test]
fn insights_payment_mix_csv_matches_golden_fixture() {
    assert_schema_matches_fixture("insights_payment_mix", CsvSchema::InsightsPaymentMix);
}
