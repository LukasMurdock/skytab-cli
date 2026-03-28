use chrono::DateTime;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

use crate::cli::OutputFormat;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvSchema {
    HourlySales,
    Payroll,
    TimeclockShifts,
    PaymentsTransactions,
    InsightsDailyBrief,
    InsightsLaborVsSales,
    InsightsPaymentMix,
}

const PREFERRED_ARRAY_KEYS: &[&str] = &[
    "rows",
    "transactions",
    "timeClockShifts",
    "buckets",
    "employees",
    "items",
    "checks",
];

pub fn write_structured_value(
    value: &Value,
    format: OutputFormat,
    output_path: Option<&Path>,
) -> Result<()> {
    write_structured_value_with_schema(value, format, output_path, None)
}

pub fn write_structured_value_with_schema(
    value: &Value,
    format: OutputFormat,
    output_path: Option<&Path>,
    csv_schema: Option<CsvSchema>,
) -> Result<()> {
    let rendered = render_structured_value_with_schema(value, format, csv_schema)?;
    write_text(&rendered, output_path)
}

pub fn write_text(text: &str, output_path: Option<&Path>) -> Result<()> {
    let output = ensure_trailing_newline(text);
    if let Some(path) = output_path {
        std::fs::write(path, output)?;
    } else {
        print!("{output}");
    }
    Ok(())
}

pub fn render_structured_value(value: &Value, format: OutputFormat) -> Result<String> {
    render_structured_value_with_schema(value, format, None)
}

pub fn render_structured_value_with_schema(
    value: &Value,
    format: OutputFormat,
    csv_schema: Option<CsvSchema>,
) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(value)?),
        OutputFormat::Csv => Ok(render_csv(value, csv_schema)),
        OutputFormat::Ndjson => render_ndjson(value),
    }
}

fn render_csv(value: &Value, csv_schema: Option<CsvSchema>) -> String {
    if let Some(schema) = csv_schema {
        return render_csv_with_schema(value, schema);
    }

    if let Some(rows) = extract_rows(value) {
        return rows_to_csv(rows);
    }

    rows_to_csv(std::slice::from_ref(value))
}

fn render_csv_with_schema(value: &Value, schema: CsvSchema) -> String {
    match schema {
        CsvSchema::HourlySales => render_hourly_sales_csv(value),
        CsvSchema::Payroll => render_payroll_csv(value),
        CsvSchema::TimeclockShifts => render_timeclock_shifts_csv(value),
        CsvSchema::PaymentsTransactions => render_payments_transactions_csv(value),
        CsvSchema::InsightsDailyBrief => render_insights_daily_brief_csv(value),
        CsvSchema::InsightsLaborVsSales => render_insights_labor_vs_sales_csv(value),
        CsvSchema::InsightsPaymentMix => render_insights_payment_mix_csv(value),
    }
}

fn render_hourly_sales_csv(value: &Value) -> String {
    let rows = value
        .get("rows")
        .and_then(Value::as_array)
        .or_else(|| value.as_array());

    let mut rendered_rows = Vec::new();
    if let Some(rows) = rows {
        for row in rows {
            let Some(columns) = row.as_array() else {
                continue;
            };
            rendered_rows.push(vec![
                columns.first().map(json_value_to_cell).unwrap_or_default(),
                columns.get(1).map(json_value_to_cell).unwrap_or_default(),
                columns.get(2).map(json_value_to_cell).unwrap_or_default(),
                columns.get(3).map(json_value_to_cell).unwrap_or_default(),
            ]);
        }
    }

    render_records_csv(&["date", "hour", "gross", "net"], rendered_rows)
}

fn render_payroll_csv(value: &Value) -> String {
    const PAYROLL_COLUMNS: &[&str] = &[
        "employee_id",
        "employee_name",
        "normal_hours",
        "overtime_hours",
        "double_overtime_hours",
        "normal_pay",
        "overtime_pay",
        "double_overtime_pay",
        "total_gratuity",
        "total_pay",
        "adjusted_tips",
        "tip_reduction",
        "declared_tips",
        "gross_tips",
        "tip_share",
        "net_tips",
    ];

    let mut rows = Vec::new();

    if let Some(employees) = value.get("employees").and_then(Value::as_array) {
        for employee in employees {
            rows.push(build_payroll_csv_row(employee, "employee"));
        }
    } else if let Some(employees) = value.as_array() {
        for employee in employees {
            rows.push(build_payroll_csv_row(employee, "employee"));
        }
    }

    if let Some(totals) = value.get("totals").filter(|totals| totals.is_object()) {
        rows.push(build_payroll_csv_row(totals, "total"));
    }

    let mut headers = vec!["row_type"];
    headers.extend_from_slice(PAYROLL_COLUMNS);
    render_records_csv(&headers, rows)
}

fn build_payroll_csv_row(value: &Value, row_type: &str) -> Vec<String> {
    let mut row = vec![row_type.to_string()];
    for key in [
        "employee_id",
        "employee_name",
        "normal_hours",
        "overtime_hours",
        "double_overtime_hours",
        "normal_pay",
        "overtime_pay",
        "double_overtime_pay",
        "total_gratuity",
        "total_pay",
        "adjusted_tips",
        "tip_reduction",
        "declared_tips",
        "gross_tips",
        "tip_share",
        "net_tips",
    ] {
        row.push(
            value
                .get(key)
                .map(json_value_to_cell)
                .unwrap_or_else(String::new),
        );
    }
    row
}

fn render_timeclock_shifts_csv(value: &Value) -> String {
    let shifts = value
        .get("timeClockShifts")
        .and_then(Value::as_array)
        .or_else(|| value.as_array());

    let mut rows = Vec::new();
    if let Some(shifts) = shifts {
        for shift in shifts {
            if !shift.is_object() {
                continue;
            }

            let clocked_out = value_at_path(shift, "clockedOutAt")
                .and_then(Value::as_str)
                .unwrap_or("");

            let status = if clocked_out.is_empty() {
                "open".to_string()
            } else {
                "closed".to_string()
            };

            rows.push(vec![
                cell_from_paths(shift, &["guid", "id"]),
                cell_from_paths(shift, &["employee.name", "employeeName", "employee"]),
                cell_from_paths(shift, &["clockedInAt"]),
                cell_from_paths(shift, &["clockedOutAt"]),
                compute_shift_hours(shift)
                    .map(|hours| format!("{hours:.2}"))
                    .unwrap_or_default(),
                cell_from_paths(shift, &["payRate"]),
                cell_from_paths(shift, &["job.name", "jobName"]),
                status,
                cell_from_paths(shift, &["locationId", "location.id"]),
            ]);
        }
    }

    render_records_csv(
        &[
            "shift_guid",
            "employee_name",
            "clocked_in_at",
            "clocked_out_at",
            "hours",
            "pay_rate",
            "job_name",
            "status",
            "location_id",
        ],
        rows,
    )
}

fn compute_shift_hours(shift: &Value) -> Option<f64> {
    let clocked_in = value_at_path(shift, "clockedInAt")
        .and_then(Value::as_str)
        .unwrap_or("");
    let clocked_out = value_at_path(shift, "clockedOutAt")
        .and_then(Value::as_str)
        .unwrap_or("");

    if !clocked_in.is_empty() && !clocked_out.is_empty() {
        let start = DateTime::parse_from_rfc3339(clocked_in).ok()?;
        let end = DateTime::parse_from_rfc3339(clocked_out).ok()?;
        let seconds = (end - start).num_seconds();
        if seconds > 0 {
            return Some(seconds as f64 / 3600.0);
        }
    }

    value_at_path(shift, "clockedInSeconds")
        .and_then(Value::as_f64)
        .map(|seconds| seconds / 3600.0)
}

fn render_payments_transactions_csv(value: &Value) -> String {
    let transactions = value
        .get("transactions")
        .and_then(Value::as_array)
        .or_else(|| value.as_array());

    let mut rows = Vec::new();
    if let Some(transactions) = transactions {
        for transaction in transactions {
            if !transaction.is_object() {
                continue;
            }

            rows.push(vec![
                cell_from_paths(
                    transaction,
                    &["id", "guid", "transactionId", "transactionGuid"],
                ),
                cell_from_paths(
                    transaction,
                    &[
                        "date",
                        "createdAt",
                        "created_at",
                        "processedAt",
                        "timestamp",
                    ],
                ),
                cell_from_paths(transaction, &["type", "orderType", "transactionType"]),
                cell_from_paths(transaction, &["status", "state", "paymentStatus"]),
                cell_from_paths(
                    transaction,
                    &["amount", "paymentAmount", "netAmount", "totals.amount"],
                ),
                cell_from_paths(transaction, &["tipAmount", "tip", "totals.tipAmount"]),
                cell_from_paths(transaction, &["taxAmount", "tax", "totals.taxAmount"]),
                cell_from_paths(transaction, &["totalAmount", "total", "totals.total"]),
                cell_from_paths(
                    transaction,
                    &["currency", "currencyCode", "totals.currency"],
                ),
                cell_from_paths(transaction, &["locationId", "location.id", "location.guid"]),
                cell_from_paths(transaction, &["orderId", "ticketId", "checkId", "order.id"]),
                cell_from_paths(
                    transaction,
                    &[
                        "reference",
                        "referenceNumber",
                        "authCode",
                        "authorizationCode",
                        "approvalCode",
                    ],
                ),
                cell_from_paths(
                    transaction,
                    &[
                        "cardBrand",
                        "cardType",
                        "card.brand",
                        "paymentMethod.cardBrand",
                        "paymentMethod.cardType",
                    ],
                ),
                cell_from_paths(
                    transaction,
                    &[
                        "cardLast4",
                        "last4",
                        "lastFour",
                        "card.last4",
                        "paymentMethod.last4",
                        "paymentMethod.lastFour",
                    ],
                ),
                serde_json::to_string(transaction).unwrap_or_default(),
            ]);
        }
    }

    render_records_csv(
        &[
            "transaction_id",
            "date",
            "type",
            "status",
            "amount",
            "tip_amount",
            "tax_amount",
            "total_amount",
            "currency",
            "location_id",
            "order_id",
            "reference",
            "card_brand",
            "card_last4",
            "raw_json",
        ],
        rows,
    )
}

fn render_insights_daily_brief_csv(value: &Value) -> String {
    let row = vec![
        cell_from_paths(value, &["period_start"]),
        cell_from_paths(value, &["period_end"]),
        join_numeric_array(value.get("location_ids")),
        cell_from_paths(value, &["gross_sales"]),
        cell_from_paths(value, &["net_sales"]),
        cell_from_paths(value, &["labor_hours"]),
        cell_from_paths(value, &["labor_pay"]),
        cell_from_paths(value, &["labor_percent_of_net_sales"]),
        cell_from_paths(value, &["sales_per_labor_hour"]),
        cell_from_paths(value, &["transaction_count"]),
        cell_from_paths(value, &["settled_count"]),
        cell_from_paths(value, &["settled_amount"]),
        cell_from_paths(value, &["settled_rate_percent"]),
        cell_from_paths(value, &["top_payment_type"]),
        cell_from_paths(value, &["top_payment_type_amount"]),
        join_string_array(value.get("highlights")),
    ];

    render_records_csv(
        &[
            "period_start",
            "period_end",
            "location_ids",
            "gross_sales",
            "net_sales",
            "labor_hours",
            "labor_pay",
            "labor_percent_of_net_sales",
            "sales_per_labor_hour",
            "transaction_count",
            "settled_count",
            "settled_amount",
            "settled_rate_percent",
            "top_payment_type",
            "top_payment_type_amount",
            "highlights",
        ],
        vec![row],
    )
}

fn render_insights_labor_vs_sales_csv(value: &Value) -> String {
    let row = vec![
        cell_from_paths(value, &["period_start"]),
        cell_from_paths(value, &["period_end"]),
        join_numeric_array(value.get("location_ids")),
        cell_from_paths(value, &["gross_sales"]),
        cell_from_paths(value, &["net_sales"]),
        cell_from_paths(value, &["labor_hours"]),
        cell_from_paths(value, &["labor_pay"]),
        cell_from_paths(value, &["labor_percent_of_net_sales"]),
        cell_from_paths(value, &["sales_per_labor_hour"]),
        cell_from_paths(value, &["labor_pay_per_labor_hour"]),
        cell_from_paths(value, &["employee_count"]),
        join_string_array(value.get("highlights")),
    ];

    render_records_csv(
        &[
            "period_start",
            "period_end",
            "location_ids",
            "gross_sales",
            "net_sales",
            "labor_hours",
            "labor_pay",
            "labor_percent_of_net_sales",
            "sales_per_labor_hour",
            "labor_pay_per_labor_hour",
            "employee_count",
            "highlights",
        ],
        vec![row],
    )
}

fn render_insights_payment_mix_csv(value: &Value) -> String {
    let period_start = cell_from_paths(value, &["period_start"]);
    let period_end = cell_from_paths(value, &["period_end"]);
    let location_ids = join_numeric_array(value.get("location_ids"));
    let transaction_count = cell_from_paths(value, &["transaction_count"]);
    let total_amount = cell_from_paths(value, &["total_amount"]);
    let highlights = join_string_array(value.get("highlights"));

    let mut rows = Vec::new();

    for (row_type, key) in [("type", "by_type"), ("tender", "by_tender")] {
        if let Some(items) = value.get(key).and_then(Value::as_array) {
            for item in items {
                rows.push(vec![
                    row_type.to_string(),
                    cell_from_paths(item, &["key"]),
                    cell_from_paths(item, &["count"]),
                    cell_from_paths(item, &["amount"]),
                    cell_from_paths(item, &["share_of_count"]),
                    cell_from_paths(item, &["share_of_amount"]),
                    period_start.clone(),
                    period_end.clone(),
                    location_ids.clone(),
                    transaction_count.clone(),
                    total_amount.clone(),
                    highlights.clone(),
                ]);
            }
        }
    }

    if rows.is_empty() {
        rows.push(vec![
            "summary".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            period_start,
            period_end,
            location_ids,
            transaction_count,
            total_amount,
            highlights,
        ]);
    }

    render_records_csv(
        &[
            "row_type",
            "key",
            "count",
            "amount",
            "share_of_count",
            "share_of_amount",
            "period_start",
            "period_end",
            "location_ids",
            "transaction_count",
            "total_amount",
            "highlights",
        ],
        rows,
    )
}

fn join_numeric_array(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_i64)
                .map(|number| number.to_string())
                .collect::<Vec<_>>()
                .join("|")
        })
        .unwrap_or_default()
}

fn join_string_array(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .unwrap_or_default()
}

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    if current.is_null() {
        return None;
    }
    Some(current)
}

fn cell_from_paths(value: &Value, paths: &[&str]) -> String {
    for path in paths {
        if let Some(found) = value_at_path(value, path) {
            return json_value_to_cell(found);
        }
    }
    String::new()
}

fn render_records_csv(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(csv_line(headers.iter().copied()));
    for row in rows {
        lines.push(csv_line(row));
    }
    lines.join("\n")
}

fn render_ndjson(value: &Value) -> Result<String> {
    if let Some(rows) = extract_rows(value) {
        let mut lines = Vec::with_capacity(rows.len());
        for row in rows {
            lines.push(serde_json::to_string(row)?);
        }
        return Ok(lines.join("\n"));
    }

    Ok(serde_json::to_string(value)?)
}

fn extract_rows(value: &Value) -> Option<&[Value]> {
    match value {
        Value::Array(rows) => Some(rows),
        Value::Object(obj) => {
            for key in PREFERRED_ARRAY_KEYS {
                if let Some(Value::Array(rows)) = obj.get(*key) {
                    return Some(rows);
                }
            }

            obj.values().find_map(|candidate| {
                if let Value::Array(rows) = candidate {
                    Some(rows.as_slice())
                } else {
                    None
                }
            })
        }
        _ => None,
    }
}

fn rows_to_csv(rows: &[Value]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    if rows.iter().all(|row| matches!(row, Value::Object(_))) {
        return object_rows_to_csv(rows);
    }

    if rows.iter().all(|row| matches!(row, Value::Array(_))) {
        return array_rows_to_csv(rows);
    }

    scalar_rows_to_csv(rows)
}

fn object_rows_to_csv(rows: &[Value]) -> String {
    let mut header_set: BTreeSet<String> = BTreeSet::new();
    for row in rows {
        if let Value::Object(map) = row {
            for key in map.keys() {
                header_set.insert(key.clone());
            }
        }
    }

    let headers: Vec<String> = header_set.into_iter().collect();
    if headers.is_empty() {
        return String::new();
    }

    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(csv_line(headers.iter().map(std::string::String::as_str)));

    for row in rows {
        let Some(map) = row.as_object() else {
            continue;
        };

        lines.push(csv_line(headers.iter().map(|header| {
            map.get(header)
                .map(json_value_to_cell)
                .unwrap_or_else(String::new)
        })));
    }

    lines.join("\n")
}

fn array_rows_to_csv(rows: &[Value]) -> String {
    let max_width = rows
        .iter()
        .filter_map(Value::as_array)
        .map(std::vec::Vec::len)
        .max()
        .unwrap_or(0);

    if max_width == 0 {
        return String::new();
    }

    let headers = (1..=max_width).map(|idx| format!("col_{idx}"));
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(csv_line(headers));

    for row in rows {
        let Some(values) = row.as_array() else {
            continue;
        };

        let mut cells = values.iter().map(json_value_to_cell).collect::<Vec<_>>();
        while cells.len() < max_width {
            cells.push(String::new());
        }
        lines.push(csv_line(cells));
    }

    lines.join("\n")
}

fn scalar_rows_to_csv(rows: &[Value]) -> String {
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(csv_line(["value"]));
    for row in rows {
        lines.push(csv_line([json_value_to_cell(row)]));
    }
    lines.join("\n")
}

fn csv_line<T, I>(cells: I) -> String
where
    T: AsRef<str>,
    I: IntoIterator<Item = T>,
{
    cells
        .into_iter()
        .map(|cell| escape_csv_cell(cell.as_ref()))
        .collect::<Vec<_>>()
        .join(",")
}

fn escape_csv_cell(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn json_value_to_cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(bool_value) => bool_value.to_string(),
        Value::Number(number_value) => number_value.to_string(),
        Value::String(string_value) => string_value.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn ensure_trailing_newline(value: &str) -> String {
    if value.ends_with('\n') {
        value.to_string()
    } else {
        format!("{value}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ndjson_prefers_rows_in_wrapped_objects() {
        let wrapped = json!({
            "count": 2,
            "transactions": [
                {"id": 1, "status": "ok"},
                {"id": 2, "status": "ok"}
            ]
        });

        let rendered = render_structured_value(&wrapped, OutputFormat::Ndjson)
            .expect("ndjson should render successfully");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"id\":1"));
        assert!(lines[1].contains("\"id\":2"));
    }

    #[test]
    fn csv_renders_object_rows() {
        let wrapped = json!({
            "rows": [
                {"name": "Alice", "hours": 8},
                {"name": "Bob", "hours": 6}
            ]
        });

        let rendered = render_structured_value(&wrapped, OutputFormat::Csv)
            .expect("csv should render successfully");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "hours,name");
        assert_eq!(lines[1], "8,Alice");
        assert_eq!(lines[2], "6,Bob");
    }

    #[test]
    fn csv_renders_array_rows() {
        let wrapped = json!({
            "rows": [
                ["2026-03-01", "09", "100.00"],
                ["2026-03-01", "10", "120.00"]
            ]
        });

        let rendered = render_structured_value(&wrapped, OutputFormat::Csv)
            .expect("csv should render successfully");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "col_1,col_2,col_3");
        assert_eq!(lines[1], "2026-03-01,09,100.00");
        assert_eq!(lines[2], "2026-03-01,10,120.00");
    }

    #[test]
    fn hourly_sales_schema_has_stable_columns() {
        let wrapped = json!({
            "rows": [
                ["2026-03-01", "09", "100.00", "90.00"],
                ["2026-03-01", "10", "120.00", "110.00"]
            ]
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::HourlySales),
        )
        .expect("hourly sales csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "date,hour,gross,net");
        assert_eq!(lines[1], "2026-03-01,09,100.00,90.00");
    }

    #[test]
    fn payroll_schema_includes_row_type_and_totals() {
        let wrapped = json!({
            "employees": [
                {
                    "employee_id": "1",
                    "employee_name": "Alice",
                    "normal_hours": 8.0,
                    "overtime_hours": 1.0,
                    "double_overtime_hours": 0.0,
                    "normal_pay": 100.0,
                    "overtime_pay": 20.0,
                    "double_overtime_pay": 0.0,
                    "total_gratuity": 5.0,
                    "total_pay": 125.0,
                    "adjusted_tips": 2.0,
                    "tip_reduction": 0.0,
                    "declared_tips": 10.0,
                    "gross_tips": 12.0,
                    "tip_share": 1.0,
                    "net_tips": 11.0
                }
            ],
            "totals": {
                "employee_id": "TOTAL",
                "employee_name": "TOTAL",
                "normal_hours": 8.0,
                "overtime_hours": 1.0,
                "double_overtime_hours": 0.0,
                "normal_pay": 100.0,
                "overtime_pay": 20.0,
                "double_overtime_pay": 0.0,
                "total_gratuity": 5.0,
                "total_pay": 125.0,
                "adjusted_tips": 2.0,
                "tip_reduction": 0.0,
                "declared_tips": 10.0,
                "gross_tips": 12.0,
                "tip_share": 1.0,
                "net_tips": 11.0
            }
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::Payroll),
        )
        .expect("payroll csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert!(lines[0].starts_with("row_type,employee_id,employee_name"));
        assert!(lines[1].starts_with("employee,1,Alice"));
        assert!(lines[2].starts_with("total,TOTAL,TOTAL"));
    }

    #[test]
    fn payroll_schema_skips_null_totals() {
        let wrapped = json!({
            "employees": [
                {
                    "employee_id": "1",
                    "employee_name": "Alice"
                }
            ],
            "totals": null
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::Payroll),
        )
        .expect("payroll csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("employee,1,Alice"));
    }

    #[test]
    fn timeclock_schema_extracts_expected_columns() {
        let wrapped = json!({
            "timeClockShifts": [
                {
                    "guid": "shift-1",
                    "employee": { "name": "Alice" },
                    "clockedInAt": "2026-03-01T09:00:00Z",
                    "clockedOutAt": "2026-03-01T11:00:00Z",
                    "payRate": "15.00",
                    "job": { "name": "Server" },
                    "locationId": 123
                }
            ]
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::TimeclockShifts),
        )
        .expect("timeclock csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(
            lines[0],
            "shift_guid,employee_name,clocked_in_at,clocked_out_at,hours,pay_rate,job_name,status,location_id"
        );
        assert!(lines[1].starts_with(
            "shift-1,Alice,2026-03-01T09:00:00Z,2026-03-01T11:00:00Z,2.00,15.00,Server,closed,123"
        ));
    }

    #[test]
    fn payments_schema_uses_fixed_columns() {
        let wrapped = json!({
            "transactions": [
                {
                    "id": "txn_1",
                    "createdAt": "2026-03-01T00:00:00Z",
                    "type": "SALE",
                    "status": "SETTLED",
                    "amount": 12.34,
                    "locationId": 456,
                    "paymentMethod": {"cardBrand": "VISA", "last4": "4242"}
                }
            ]
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::PaymentsTransactions),
        )
        .expect("payments csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(
            lines[0],
            "transaction_id,date,type,status,amount,tip_amount,tax_amount,total_amount,currency,location_id,order_id,reference,card_brand,card_last4,raw_json"
        );
        assert!(lines[1].starts_with("txn_1,2026-03-01T00:00:00Z,SALE,SETTLED,12.34"));
    }

    #[test]
    fn insights_daily_brief_schema_has_fixed_columns() {
        let wrapped = json!({
            "period_start": "2026-03-01T00:00:00Z",
            "period_end": "2026-03-01T23:59:59Z",
            "location_ids": [43101562],
            "gross_sales": 1200.0,
            "net_sales": 1100.0,
            "labor_hours": 45.5,
            "labor_pay": 400.0,
            "labor_percent_of_net_sales": 36.36,
            "sales_per_labor_hour": 24.17,
            "transaction_count": 80,
            "settled_count": 76,
            "settled_amount": 1040.0,
            "settled_rate_percent": 95.0,
            "top_payment_type": "SALE",
            "top_payment_type_amount": 990.0,
            "highlights": ["Labor is high", "Settlement rate dipped"]
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::InsightsDailyBrief),
        )
        .expect("insights daily brief csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(
            lines[0],
            "period_start,period_end,location_ids,gross_sales,net_sales,labor_hours,labor_pay,labor_percent_of_net_sales,sales_per_labor_hour,transaction_count,settled_count,settled_amount,settled_rate_percent,top_payment_type,top_payment_type_amount,highlights"
        );
        assert!(lines[1].contains("43101562"));
        assert!(lines[1].contains("Labor is high | Settlement rate dipped"));
    }

    #[test]
    fn insights_labor_vs_sales_schema_has_fixed_columns() {
        let wrapped = json!({
            "period_start": "2026-03-01T00:00:00Z",
            "period_end": "2026-03-01T23:59:59Z",
            "location_ids": [43101562, 43101563],
            "gross_sales": 2200.0,
            "net_sales": 2100.0,
            "labor_hours": 85.0,
            "labor_pay": 750.0,
            "labor_percent_of_net_sales": 35.71,
            "sales_per_labor_hour": 24.71,
            "labor_pay_per_labor_hour": 8.82,
            "employee_count": 11,
            "highlights": ["Labor is high"]
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::InsightsLaborVsSales),
        )
        .expect("insights labor vs sales csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(
            lines[0],
            "period_start,period_end,location_ids,gross_sales,net_sales,labor_hours,labor_pay,labor_percent_of_net_sales,sales_per_labor_hour,labor_pay_per_labor_hour,employee_count,highlights"
        );
        assert!(lines[1].contains("43101562|43101563"));
    }

    #[test]
    fn insights_payment_mix_schema_has_fixed_columns() {
        let wrapped = json!({
            "period_start": "2026-03-01T00:00:00Z",
            "period_end": "2026-03-01T23:59:59Z",
            "location_ids": [43101562],
            "transaction_count": 5,
            "total_amount": 300.0,
            "highlights": ["SALE leads"],
            "by_type": [
                {
                    "key": "SALE",
                    "count": 4,
                    "amount": 280.0,
                    "share_of_count": 80.0,
                    "share_of_amount": 93.33
                }
            ],
            "by_tender": [
                {
                    "key": "VISA",
                    "count": 3,
                    "amount": 210.0,
                    "share_of_count": 60.0,
                    "share_of_amount": 70.0
                }
            ]
        });

        let rendered = render_structured_value_with_schema(
            &wrapped,
            OutputFormat::Csv,
            Some(CsvSchema::InsightsPaymentMix),
        )
        .expect("insights payment mix csv should render");
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(
            lines[0],
            "row_type,key,count,amount,share_of_count,share_of_amount,period_start,period_end,location_ids,transaction_count,total_amount,highlights"
        );
        assert!(lines[1].starts_with("type,SALE,4,280.0,80.0,93.33"));
        assert!(lines[2].starts_with("tender,VISA,3,210.0,60.0,70.0"));
    }
}
