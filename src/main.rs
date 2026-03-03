mod cache;
mod cli;
mod client;
mod config;
mod error;

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use clap::Parser;
use reqwest::Method;
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

use crate::cache::TokenCache;
use crate::cli::{
    AccountsSubcommand, AuthSubcommand, Cli, Commands, HttpMethod, LocationsSubcommand,
    PaymentsSubcommand, ReportsSubcommand, TimeclockSubcommand,
};
use crate::client::SkyTabClient;
use crate::config::{
    Config, clear_default_location_id, current_config_file_path, get_default_location_id,
    legacy_config_file_path, save_credentials, save_default_location_id,
};
use crate::error::{Result, SkyTabError};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Location {
    id: i64,
    name: String,
    #[serde(default)]
    timezone: Option<String>,
    #[serde(default, rename = "merchantId")]
    merchant_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LocationsWrappedResponse {
    #[serde(default)]
    locations: Vec<Location>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct HourlySalesResponse {
    rows: Vec<[String; 4]>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TimeClockShiftsResponse {
    meta: TimeClockMeta,
    #[serde(rename = "timeClockShifts")]
    time_clock_shifts: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TimeClockMeta {
    count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TransactionsResponse {
    transactions: Vec<Value>,
    count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TicketDetailClosedResponse {
    rows: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ActivitySummaryResponse {
    buckets: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ReportRowsResponse {
    rows: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TillTransactionDetailResponse {
    #[serde(default)]
    name: String,
    #[serde(default)]
    headers: Value,
    rows: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct TillTransactionDetailData {
    name: String,
    headers: Value,
    items: Vec<TillTransactionItem>,
}

#[derive(Debug, Clone, Serialize)]
struct TillTransactionItem {
    till_name: String,
    date_time: String,
    employee_name: String,
    drawer_action: String,
    amount: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PayrollByEmployeeResponse {
    #[serde(default)]
    name: String,
    #[serde(default)]
    headers: Value,
    rows: Vec<Value>,
    #[serde(default)]
    custom: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PayrollByEmployeeData {
    name: String,
    headers: Value,
    employees: Vec<PayrollEmployee>,
    totals: Option<PayrollEmployee>,
    custom: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PayrollEmployee {
    employee_id: String,
    employee_name: String,
    normal_hours: f64,
    overtime_hours: f64,
    double_overtime_hours: f64,
    normal_pay: f64,
    overtime_pay: f64,
    double_overtime_pay: f64,
    total_gratuity: f64,
    total_pay: f64,
    adjusted_tips: f64,
    tip_reduction: f64,
    declared_tips: f64,
    gross_tips: f64,
    tip_share: f64,
    net_tips: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AccountPreferencesResponse {
    eod: String,
    #[serde(rename = "timeZone")]
    time_zone: String,
    #[serde(default)]
    #[serde(rename = "weekStart")]
    week_start: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorCheck {
    name: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorReport {
    ok: bool,
    checks: Vec<DoctorCheck>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    if let Err(err) = run(cli).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Auth(args) => match args.command {
            AuthSubcommand::Login => {
                let client = build_client(cli.base_url.clone()).await?;
                let token = client.token(true).await?;
                print_output(cli.json, &json!({ "token": token }));
            }
            AuthSubcommand::SetCredentials {
                username,
                password,
                prompt_password,
                base_url,
            } => {
                let password = resolve_set_credentials_password(password, prompt_password)?;
                let path = save_credentials(username, password, base_url).await?;
                print_output(
                    cli.json,
                    &json!({
                        "ok": true,
                        "config_path": path,
                        "message": "credentials saved"
                    }),
                );
            }
        },
        Commands::Locations(args) => match args.command {
            LocationsSubcommand::List => {
                let client = build_client(cli.base_url.clone()).await?;
                let raw_locations: Value = client
                    .request_authed_json(Method::GET, "/api/v2/locations", &Vec::new(), None)
                    .await?;

                let locations = parse_locations_response(raw_locations)?;
                if cli.json {
                    print_output(true, &serde_json::to_value(locations)?);
                } else {
                    for l in locations {
                        println!(
                            "{}\t{}\t{}",
                            l.id,
                            l.name,
                            l.timezone.unwrap_or_else(|| "-".to_string())
                        );
                    }
                }
            }
            LocationsSubcommand::SetDefault { location_id } => {
                let client = build_client(cli.base_url.clone()).await?;
                let raw_locations: Value = client
                    .request_authed_json(Method::GET, "/api/v2/locations", &Vec::new(), None)
                    .await?;
                let locations = parse_locations_response(raw_locations)?;
                let exists = locations.iter().any(|location| location.id == location_id);
                if !exists {
                    return Err(SkyTabError::InvalidArgument(format!(
                        "location {} is not available for this account",
                        location_id
                    )));
                }

                let path = save_default_location_id(location_id).await?;
                print_output(
                    cli.json,
                    &json!({
                        "ok": true,
                        "default_location_id": location_id,
                        "config_path": path,
                        "message": "default location saved"
                    }),
                );
            }
            LocationsSubcommand::ShowDefault => {
                let default_location_id = get_default_location_id().await?;
                print_output(
                    cli.json,
                    &json!({
                        "default_location_id": default_location_id,
                        "configured": default_location_id.is_some()
                    }),
                );
            }
            LocationsSubcommand::ClearDefault => {
                let path = clear_default_location_id().await?;
                print_output(
                    cli.json,
                    &json!({
                        "ok": true,
                        "default_location_id": Value::Null,
                        "config_path": path,
                        "message": "default location cleared"
                    }),
                );
            }
        },
        Commands::Accounts(args) => match args.command {
            AccountsSubcommand::Preferences { account_id } => {
                let client = build_client(cli.base_url.clone()).await?;
                let path = format!("/api/v1/accounts/{account_id}/preferences");
                let preferences: AccountPreferencesResponse = client
                    .request_authed_json(Method::GET, &path, &Vec::new(), None)
                    .await?;

                print_output(cli.json, &serde_json::to_value(preferences)?);
            }
        },
        Commands::Reports(args) => match args.command {
            ReportsSubcommand::ActivitySummary {
                start,
                end,
                location,
            } => {
                let location = resolve_single_location(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &[location], start, end)
                        .await?;
                let payload = json!({
                    "start": start,
                    "end": end,
                    "locations": [location.to_string()],
                    "locale": "en-US",
                    "intradayPeriodGroupGuids": [],
                    "revenueCenterGuids": []
                });

                let response: ActivitySummaryResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/activity-summary",
                        &Vec::new(),
                        Some(payload),
                    )
                    .await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    println!("buckets: {}", response.buckets.len());
                }
            }
            ReportsSubcommand::DiscountSummary {
                start,
                end,
                location,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &location, start, end).await?;
                let response: ReportRowsResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/discount-summary",
                        &Vec::new(),
                        Some(build_report_payload(start, end, location)),
                    )
                    .await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::HourlySales {
                start,
                end,
                location,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &location, start, end).await?;
                let payload = build_report_payload(start, end, location);

                let report: HourlySalesResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/hourly-sales",
                        &Vec::new(),
                        Some(payload),
                    )
                    .await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(report)?);
                } else {
                    for row in report.rows {
                        println!("{}\t{}\t{}\t{}", row[0], row[1], row[2], row[3]);
                    }
                }
            }
            ReportsSubcommand::TicketDetailClosed {
                start,
                end,
                location,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &location, start, end).await?;
                let payload = build_report_payload(start, end, location);

                let response: TicketDetailClosedResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/ticket-detail-closed",
                        &Vec::new(),
                        Some(payload),
                    )
                    .await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::SalesSummaryByItem {
                start,
                end,
                location,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &location, start, end).await?;
                let response: ReportRowsResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/sales-summary-by-item",
                        &Vec::new(),
                        Some(build_report_payload(start, end, location)),
                    )
                    .await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::SalesSummaryByRevenueClass {
                start,
                end,
                location,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &location, start, end).await?;
                let response: ReportRowsResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/sales-summary-by-revenue-class",
                        &Vec::new(),
                        Some(build_report_payload(start, end, location)),
                    )
                    .await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::TillTransaction {
                start,
                end,
                location,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &location, start, end).await?;
                let response: TillTransactionDetailResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/till-transaction-detail",
                        &Vec::new(),
                        Some(build_report_payload(start, end, location)),
                    )
                    .await?;
                let transformed = transform_till_transaction_response(response);

                if cli.json {
                    print_output(true, &serde_json::to_value(transformed)?);
                } else {
                    print_till_transaction_human(&transformed);
                }
            }
            ReportsSubcommand::Payroll {
                start,
                end,
                location,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_locations_timezones(&client, &location, start, end).await?;
                let response: PayrollByEmployeeResponse = client
                    .request_authed_json(
                        Method::POST,
                        "/api/v1/reports/echo-pro/payroll-by-employee-new",
                        &Vec::new(),
                        Some(build_report_payload(start, end, location)),
                    )
                    .await?;
                let transformed = transform_payroll_response(response);

                if cli.json {
                    print_output(true, &serde_json::to_value(transformed)?);
                } else {
                    print_payroll_human(&transformed);
                }
            }
        },
        Commands::Timeclock(args) => match args.command {
            TimeclockSubcommand::Shifts {
                location_id,
                start,
                end,
                order,
                limit,
            } => {
                let location_id =
                    resolve_single_location(location_id, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let (start, end) =
                    normalize_range_for_location_timezone(&client, location_id, start, end).await?;
                let result =
                    fetch_timeclock_shifts(&client, location_id, start, end, order, limit).await?;
                if cli.json {
                    print_output(true, &serde_json::to_value(result)?);
                } else {
                    print_timeclock_shifts_human(&result.time_clock_shifts);
                }
            }
        },
        Commands::Payments(args) => match args.command {
            PaymentsSubcommand::Transactions {
                start,
                end,
                location,
                order_type,
            } => {
                let location = resolve_locations(location, cli.base_url.clone()).await?;
                let client = build_client(cli.base_url.clone()).await?;
                let result = fetch_transactions(&client, start, end, location, order_type).await?;
                if cli.json {
                    print_output(true, &serde_json::to_value(result)?);
                } else {
                    println!("count: {}", result.count);
                }
            }
        },
        Commands::Request(args) => {
            let client = build_client(cli.base_url.clone()).await?;
            let method = map_method(args.method);
            let query_pairs = parse_query(&args.query)?;
            let body = match args.body {
                Some(body) => Some(serde_json::from_str::<Value>(&body)?),
                None => None,
            };

            let response: Value = client
                .request_authed_json(method, &args.path, &query_pairs, body)
                .await?;
            print_output(true, &response);
        }
        Commands::Doctor => run_doctor(cli.base_url.clone(), cli.json).await?,
    }

    Ok(())
}

async fn run_doctor(base_url_override: Option<String>, json_mode: bool) -> Result<()> {
    let mut checks: Vec<DoctorCheck> = Vec::new();

    let env_username = std::env::var("SKYTAB_USERNAME").ok();
    let env_password = std::env::var("SKYTAB_PASSWORD").ok();
    let env_ok = env_username.is_some() && env_password.is_some();

    let config_path = current_config_file_path();
    let legacy_config_path = legacy_config_file_path();
    let config_exists = tokio::fs::metadata(&config_path).await.is_ok();
    let legacy_config_exists = tokio::fs::metadata(&legacy_config_path).await.is_ok();
    checks.push(DoctorCheck {
        name: "config_file".to_string(),
        ok: config_exists || legacy_config_exists,
        detail: format!(
            "current={} ({}) legacy={} ({})",
            config_path.display(),
            if config_exists { "exists" } else { "missing" },
            legacy_config_path.display(),
            if legacy_config_exists {
                "exists"
            } else {
                "missing"
            }
        ),
    });

    checks.push(DoctorCheck {
        name: "env_credentials".to_string(),
        ok: env_ok || config_exists || legacy_config_exists,
        detail: format!(
            "SKYTAB_USERNAME={} SKYTAB_PASSWORD={} (config fallback: {})",
            if env_username.is_some() {
                "set"
            } else {
                "missing"
            },
            if env_password.is_some() {
                "set"
            } else {
                "missing"
            },
            if config_exists || legacy_config_exists {
                "available"
            } else {
                "missing"
            }
        ),
    });

    let default_location = get_default_location_id().await?;
    checks.push(DoctorCheck {
        name: "default_location".to_string(),
        ok: true,
        detail: format!(
            "{}",
            default_location
                .map(|id| id.to_string())
                .unwrap_or_else(|| "not set".to_string())
        ),
    });

    let token_cache = TokenCache::new();
    let cache_path = token_cache.path();
    let legacy_cache_path = TokenCache::legacy_path();
    let cache_exists = tokio::fs::metadata(&cache_path).await.is_ok();
    let legacy_cache_exists = tokio::fs::metadata(&legacy_cache_path).await.is_ok();
    let cache_status = match token_cache.load_valid_token().await {
        Ok(Some(_)) => (true, "valid cached token".to_string()),
        Ok(None) => (true, "no valid cached token".to_string()),
        Err(err) => (false, format!("error reading cache: {err}")),
    };
    checks.push(DoctorCheck {
        name: "token_cache".to_string(),
        ok: cache_status.0,
        detail: format!(
            "{}; current={} ({}) legacy={} ({})",
            cache_status.1,
            cache_path.display(),
            if cache_exists { "exists" } else { "missing" },
            legacy_cache_path.display(),
            if legacy_cache_exists {
                "exists"
            } else {
                "missing"
            }
        ),
    });

    let config = Config::from_sources(base_url_override.clone()).await;
    match config {
        Ok(config) => {
            checks.push(DoctorCheck {
                name: "credentials_resolution".to_string(),
                ok: true,
                detail: format!(
                    "resolved username={} base_url={}",
                    redact_username(&config.username),
                    config.base_url
                ),
            });

            let client = SkyTabClient::new(config.clone());
            match client.token(true).await {
                Ok(_) => checks.push(DoctorCheck {
                    name: "auth".to_string(),
                    ok: true,
                    detail: "authentication succeeded".to_string(),
                }),
                Err(err) => checks.push(DoctorCheck {
                    name: "auth".to_string(),
                    ok: false,
                    detail: format!("authentication failed: {err}"),
                }),
            }
        }
        Err(err) => {
            checks.push(DoctorCheck {
                name: "credentials_resolution".to_string(),
                ok: false,
                detail: format!("unable to resolve credentials: {err}"),
            });
            checks.push(DoctorCheck {
                name: "auth".to_string(),
                ok: false,
                detail: "skipped auth because credentials are unresolved".to_string(),
            });
        }
    }

    let report = DoctorReport {
        ok: checks.iter().all(|check| check.ok),
        checks,
    };

    if json_mode {
        print_output(true, &serde_json::to_value(report)?);
    } else {
        println!("Doctor: {}", if report.ok { "OK" } else { "ISSUES FOUND" });
        for check in &report.checks {
            let status = if check.ok { "OK" } else { "FAIL" };
            println!("[{status}] {:<22} {}", check.name, check.detail);
        }
    }

    Ok(())
}

fn redact_username(username: &str) -> String {
    if let Some((local, domain)) = username.split_once('@') {
        if local.is_empty() {
            return format!("***@{domain}");
        }

        let first = local.chars().next().unwrap_or('*');
        return format!("{first}***@{domain}");
    }

    "***".to_string()
}

fn init_tracing(verbose: u8) {
    let filter = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        let level = match verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        };
        tracing_subscriber::EnvFilter::new(level)
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .init();
}

async fn build_client(base_url: Option<String>) -> Result<SkyTabClient> {
    let config = Config::from_sources(base_url).await?;
    Ok(SkyTabClient::new(config))
}

#[derive(Debug, Clone, Serialize)]
struct TimeClockShiftsResult {
    count: usize,
    #[serde(rename = "timeClockShifts")]
    time_clock_shifts: Vec<Value>,
}

async fn fetch_timeclock_shifts(
    client: &SkyTabClient,
    location_id: i64,
    start: String,
    end: String,
    order: String,
    limit: usize,
) -> Result<TimeClockShiftsResult> {
    let mut all: Vec<Value> = Vec::new();
    let mut offset = 0usize;

    loop {
        let filter = json!({
            "locationId": { "$eq": location_id },
            "voided": { "$eq": false },
            "isTracked": { "$eq": true },
            "$or": [
                {
                    "$and": [
                        { "clockedInAt": { "$gte": start } },
                        { "clockedInAt": { "$lte": end } }
                    ]
                },
                {
                    "$and": [
                        { "clockedOutAt": { "$gte": start } },
                        { "clockedOutAt": { "$lte": end } }
                    ]
                }
            ]
        })
        .to_string();

        let query = vec![
            ("filter".to_string(), filter),
            ("limit".to_string(), limit.to_string()),
            ("offset".to_string(), offset.to_string()),
            ("order".to_string(), order.clone()),
        ];

        let page: TimeClockShiftsResponse = client
            .request_authed_json(
                Method::GET,
                "/api/v2/echo-pro/time-clock-shifts",
                &query,
                None,
            )
            .await?;

        all.extend(page.time_clock_shifts);
        if all.len() >= page.meta.count {
            break;
        }

        offset += limit;
    }

    Ok(TimeClockShiftsResult {
        count: all.len(),
        time_clock_shifts: all,
    })
}

#[derive(Debug, Clone, Serialize)]
struct TransactionsResult {
    count: usize,
    transactions: Vec<Value>,
}

async fn fetch_transactions(
    client: &SkyTabClient,
    start: String,
    end: String,
    location_ids: Vec<i64>,
    order_type: Option<String>,
) -> Result<TransactionsResult> {
    let limit = 200usize;
    let mut offset = 0usize;
    let mut all = Vec::new();

    loop {
        let mut query = vec![
            ("limit".to_string(), limit.to_string()),
            ("offset".to_string(), offset.to_string()),
            ("start".to_string(), start.clone()),
            ("end".to_string(), end.clone()),
            ("searchTerm".to_string(), "".to_string()),
            ("sortBy".to_string(), "date".to_string()),
            ("sortDir".to_string(), "DESC".to_string()),
        ];
        if let Some(ref order_type) = order_type {
            query.push(("type[]".to_string(), order_type.clone()));
        }

        let body = json!({ "locationIds": location_ids });
        let page: TransactionsResponse = client
            .request_authed_json(
                Method::POST,
                "/api/v2/internet-payments/transactions",
                &query,
                Some(body),
            )
            .await?;

        all.extend(page.transactions);
        if all.len() >= page.count {
            break;
        }

        offset += limit;
    }

    Ok(TransactionsResult {
        count: all.len(),
        transactions: all,
    })
}

fn parse_query(parts: &[String]) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for part in parts {
        let (key, value) = part.split_once('=').ok_or_else(|| {
            SkyTabError::InvalidArgument(format!(
                "invalid --query value '{part}', expected key=value"
            ))
        })?;
        out.push((key.to_string(), value.to_string()));
    }
    Ok(out)
}

fn build_report_payload(start: String, end: String, locations: Vec<i64>) -> Value {
    json!({
        "start": start,
        "end": end,
        "locations": locations,
        "intradayPeriodGroupGuids": [],
        "revenueCenterGuids": [],
        "locale": "en-US"
    })
}

async fn resolve_single_location(location: Option<i64>, base_url: Option<String>) -> Result<i64> {
    if let Some(location) = location {
        return Ok(location);
    }

    if let Some(default_location_id) = get_default_location_id().await? {
        return Ok(default_location_id);
    }

    if let Some(single_location_id) = resolve_sole_available_location(base_url).await? {
        return Ok(single_location_id);
    }

    Err(SkyTabError::InvalidArgument(
        "location is required; pass --location/--location-id, set a default with `locations set-default`, or rely on auto-selection when exactly one location is available"
            .into(),
    ))
}

async fn resolve_locations(locations: Vec<i64>, base_url: Option<String>) -> Result<Vec<i64>> {
    if !locations.is_empty() {
        return Ok(locations);
    }

    if let Some(default_location_id) = get_default_location_id().await? {
        return Ok(vec![default_location_id]);
    }

    if let Some(single_location_id) = resolve_sole_available_location(base_url).await? {
        return Ok(vec![single_location_id]);
    }

    Err(SkyTabError::InvalidArgument(
        "at least one location is required; pass --location, set a default with `locations set-default`, or rely on auto-selection when exactly one location is available"
            .into(),
    ))
}

async fn resolve_sole_available_location(base_url: Option<String>) -> Result<Option<i64>> {
    let client = build_client(base_url).await?;
    let raw_locations: Value = client
        .request_authed_json(Method::GET, "/api/v2/locations", &Vec::new(), None)
        .await?;
    let locations = parse_locations_response(raw_locations)?;

    if locations.len() == 1 {
        return Ok(Some(locations[0].id));
    }

    Ok(None)
}

fn map_method(method: HttpMethod) -> Method {
    match method {
        HttpMethod::Get => Method::GET,
        HttpMethod::Post => Method::POST,
        HttpMethod::Put => Method::PUT,
        HttpMethod::Patch => Method::PATCH,
        HttpMethod::Delete => Method::DELETE,
    }
}

fn print_output(json_mode: bool, value: &Value) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "{}",
            serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
        );
    }
}

fn parse_locations_response(raw: Value) -> Result<Vec<Location>> {
    if let Ok(locations) = serde_json::from_value::<Vec<Location>>(raw.clone()) {
        return Ok(locations);
    }

    if let Ok(wrapped) = serde_json::from_value::<LocationsWrappedResponse>(raw.clone()) {
        return Ok(wrapped.locations);
    }

    if let Some(data) = raw.get("data") {
        if let Ok(locations) = serde_json::from_value::<Vec<Location>>(data.clone()) {
            return Ok(locations);
        }
    }

    Err(SkyTabError::InvalidArgument(
        "unexpected /api/v2/locations response shape".into(),
    ))
}

async fn normalize_range_for_location_timezone(
    client: &SkyTabClient,
    location_id: i64,
    start: String,
    end: String,
) -> Result<(String, String)> {
    normalize_range_for_locations_timezones(client, &[location_id], start, end).await
}

async fn normalize_range_for_locations_timezones(
    client: &SkyTabClient,
    location_ids: &[i64],
    start: String,
    end: String,
) -> Result<(String, String)> {
    let start_is_date = is_date_only(&start);
    let end_is_date = is_date_only(&end);

    if !start_is_date && !end_is_date {
        return Ok((start, end));
    }

    let timezone = fetch_shared_timezone_for_locations(client, location_ids).await?;
    let start_out = if start_is_date {
        date_only_to_utc_boundary(&start, &timezone, true)?
    } else {
        start
    };
    let end_out = if end_is_date {
        date_only_to_utc_boundary(&end, &timezone, false)?
    } else {
        end
    };

    Ok((start_out, end_out))
}

async fn fetch_shared_timezone_for_locations(
    client: &SkyTabClient,
    location_ids: &[i64],
) -> Result<String> {
    if location_ids.is_empty() {
        return Err(SkyTabError::InvalidArgument(
            "at least one location is required".into(),
        ));
    }

    let raw_locations: Value = client
        .request_authed_json(Method::GET, "/api/v2/locations", &Vec::new(), None)
        .await?;
    let locations = parse_locations_response(raw_locations)?;
    let mut timezones = BTreeSet::new();

    for location_id in location_ids {
        let location = locations
            .iter()
            .find(|location| location.id == *location_id)
            .ok_or_else(|| {
                SkyTabError::InvalidArgument(format!(
                    "location {} is not available for this account",
                    location_id
                ))
            })?;

        let timezone = location.timezone.clone().ok_or_else(|| {
            SkyTabError::InvalidArgument(format!(
                "location {} is missing timezone in SkyTab response",
                location_id
            ))
        })?;
        timezones.insert(timezone);
    }

    if timezones.len() > 1 {
        let joined = timezones.into_iter().collect::<Vec<_>>().join(", ");
        return Err(SkyTabError::InvalidArgument(format!(
            "date-only ranges across multiple timezones are ambiguous ({joined}); use RFC3339 timestamps instead"
        )));
    }

    timezones
        .into_iter()
        .next()
        .ok_or_else(|| SkyTabError::InvalidArgument("no timezone found for location(s)".into()))
}

fn date_only_to_utc_boundary(date: &str, timezone: &str, is_start: bool) -> Result<String> {
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| SkyTabError::InvalidArgument(format!("invalid date format: {date}")))?;
    let tz: Tz = timezone
        .parse()
        .map_err(|_| SkyTabError::InvalidArgument(format!("invalid timezone: {timezone}")))?;

    let local_dt = if is_start {
        tz.with_ymd_and_hms(
            parsed_date.year(),
            parsed_date.month(),
            parsed_date.day(),
            0,
            0,
            0,
        )
        .single()
    } else {
        tz.with_ymd_and_hms(
            parsed_date.year(),
            parsed_date.month(),
            parsed_date.day(),
            23,
            59,
            59,
        )
        .single()
    }
    .ok_or_else(|| SkyTabError::InvalidArgument(format!("invalid local datetime for {date}")))?;

    Ok(local_dt
        .with_timezone(&Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

fn is_date_only(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value.chars().enumerate().all(|(idx, c)| {
            if idx == 4 || idx == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
}

fn print_timeclock_shifts_human(shifts: &[Value]) {
    println!(
        "{:<10} {:<18} {:<17} {:<17} {:>7} {:>8} {:<22} {:<6}",
        "SHIFT_ID", "EMPLOYEE", "CLOCKED_IN", "CLOCKED_OUT", "HOURS", "RATE", "JOB", "STATUS"
    );
    println!(
        "{:-<10} {:-<18} {:-<17} {:-<17} {:-<7} {:-<8} {:-<22} {:-<6}",
        "", "", "", "", "", "", "", ""
    );

    let mut total_hours = 0.0_f64;
    let mut open_shifts = 0usize;

    for shift in shifts {
        let shift_id = shift
            .get("guid")
            .and_then(Value::as_str)
            .map(short_id)
            .unwrap_or_else(|| "-".to_string());

        let employee = shift
            .get("employee")
            .and_then(|e| e.get("name"))
            .and_then(Value::as_str)
            .map(truncate_ellipsis)
            .unwrap_or_else(|| "-".to_string());

        let clocked_in_raw = shift
            .get("clockedInAt")
            .and_then(Value::as_str)
            .unwrap_or("");
        let clocked_out_raw = shift
            .get("clockedOutAt")
            .and_then(Value::as_str)
            .unwrap_or("");
        let clocked_in = format_iso_minute(clocked_in_raw).unwrap_or_else(|| "-".to_string());
        let clocked_out = if clocked_out_raw.is_empty() {
            "-".to_string()
        } else {
            format_iso_minute(clocked_out_raw).unwrap_or_else(|| "-".to_string())
        };

        let hours = calculate_hours(clocked_in_raw, clocked_out_raw)
            .or_else(|| {
                shift
                    .get("clockedInSeconds")
                    .and_then(Value::as_f64)
                    .map(|s| s / 3600.0)
            })
            .unwrap_or(0.0);
        total_hours += hours;

        let rate = shift
            .get("payRate")
            .and_then(Value::as_str)
            .map(|v| format!("${v}"))
            .unwrap_or_else(|| "-".to_string());

        let job = shift
            .get("job")
            .and_then(|j| j.get("name"))
            .and_then(Value::as_str)
            .map(truncate_ellipsis)
            .unwrap_or_else(|| "-".to_string());

        let is_open = clocked_out_raw.is_empty();
        if is_open {
            open_shifts += 1;
        }
        let status = if is_open { "OPEN" } else { "CLOSED" };

        println!(
            "{:<10} {:<18} {:<17} {:<17} {:>7.2} {:>8} {:<22} {:<6}",
            shift_id, employee, clocked_in, clocked_out, hours, rate, job, status
        );
    }

    println!();
    println!(
        "Total shifts: {} | Total hours: {:.2} | Open shifts: {}",
        shifts.len(),
        total_hours,
        open_shifts
    );
}

fn transform_till_transaction_response(
    response: TillTransactionDetailResponse,
) -> TillTransactionDetailData {
    let mut items = Vec::new();

    for row in response.rows {
        let Some(item) = parse_till_transaction_row(&row) else {
            continue;
        };
        items.push(item);
    }

    TillTransactionDetailData {
        name: response.name,
        headers: response.headers,
        items,
    }
}

fn parse_till_transaction_row(row: &Value) -> Option<TillTransactionItem> {
    let values = row.as_array()?;
    if values.len() < 5 {
        return None;
    }

    let s = |idx: usize| values.get(idx).and_then(Value::as_str).unwrap_or("").trim();

    Some(TillTransactionItem {
        till_name: s(0).to_string(),
        date_time: s(1).to_string(),
        employee_name: s(2).to_string(),
        drawer_action: s(3).to_string(),
        amount: parse_currency_number(s(4)),
    })
}

fn print_till_transaction_human(data: &TillTransactionDetailData) {
    println!(
        "{:<18} {:<18} {:<20} {:<18} {:>10}",
        "TILL", "DATE_TIME", "EMPLOYEE", "ACTION", "AMOUNT"
    );
    println!(
        "{:-<18} {:-<18} {:-<20} {:-<18} {:-<10}",
        "", "", "", "", ""
    );

    let mut total_amount = 0.0_f64;
    for item in &data.items {
        total_amount += item.amount;
        println!(
            "{:<18} {:<18} {:<20} {:<18} {:>10.2}",
            truncate_to(item.till_name.as_str(), 18),
            truncate_to(item.date_time.as_str(), 18),
            truncate_to(item.employee_name.as_str(), 20),
            truncate_to(item.drawer_action.as_str(), 18),
            item.amount
        );
    }

    println!();
    println!(
        "Rows: {} | Net amount: {:.2}",
        data.items.len(),
        total_amount
    );
}

fn transform_payroll_response(response: PayrollByEmployeeResponse) -> PayrollByEmployeeData {
    let mut employees = Vec::new();
    let mut totals = None;

    for row in response.rows {
        let Some(employee) = parse_payroll_row(&row) else {
            continue;
        };

        if employee.employee_id.eq_ignore_ascii_case("total")
            || employee.employee_name.eq_ignore_ascii_case("total")
        {
            totals = Some(employee);
        } else {
            employees.push(employee);
        }
    }

    PayrollByEmployeeData {
        name: response.name,
        headers: response.headers,
        employees,
        totals,
        custom: response.custom,
    }
}

fn parse_payroll_row(row: &Value) -> Option<PayrollEmployee> {
    let values = row.as_array()?;
    if values.len() < 16 {
        return None;
    }

    let s = |idx: usize| values.get(idx).and_then(Value::as_str).unwrap_or("").trim();

    Some(PayrollEmployee {
        employee_id: s(0).to_string(),
        employee_name: s(1).to_string(),
        normal_hours: parse_number(s(2)),
        overtime_hours: parse_number(s(3)),
        double_overtime_hours: parse_number(s(4)),
        normal_pay: parse_currency_number(s(5)),
        overtime_pay: parse_currency_number(s(6)),
        double_overtime_pay: parse_currency_number(s(7)),
        total_gratuity: parse_currency_number(s(8)),
        total_pay: parse_currency_number(s(9)),
        adjusted_tips: parse_currency_number(s(10)),
        tip_reduction: parse_currency_number(s(11)),
        declared_tips: parse_currency_number(s(12)),
        gross_tips: parse_currency_number(s(13)),
        tip_share: parse_currency_number(s(14)),
        net_tips: parse_currency_number(s(15)),
    })
}

fn parse_number(input: &str) -> f64 {
    input.parse::<f64>().unwrap_or(0.0)
}

fn parse_currency_number(input: &str) -> f64 {
    let normalized = input.replace(['$', ','], "");
    normalized.parse::<f64>().unwrap_or(0.0)
}

fn print_payroll_human(data: &PayrollByEmployeeData) {
    println!(
        "{:<20} {:>8} {:>8} {:>10} {:>10}",
        "EMPLOYEE", "NORMAL", "OT", "TOTAL_PAY", "NET_TIPS"
    );
    println!("{:-<20} {:-<8} {:-<8} {:-<10} {:-<10}", "", "", "", "", "");

    for employee in &data.employees {
        println!(
            "{:<20} {:>8.2} {:>8.2} {:>10.2} {:>10.2}",
            truncate_to(employee.employee_name.as_str(), 20),
            employee.normal_hours,
            employee.overtime_hours,
            employee.total_pay,
            employee.net_tips
        );
    }

    if let Some(totals) = &data.totals {
        println!("{:-<20} {:-<8} {:-<8} {:-<10} {:-<10}", "", "", "", "", "");
        println!(
            "{:<20} {:>8.2} {:>8.2} {:>10.2} {:>10.2}",
            "TOTAL", totals.normal_hours, totals.overtime_hours, totals.total_pay, totals.net_tips
        );
    }

    println!();
    println!("Employees: {}", data.employees.len());
}

fn truncate_to(input: &str, max_width: usize) -> String {
    if input.chars().count() <= max_width {
        return input.to_string();
    }

    input
        .chars()
        .take(max_width.saturating_sub(3))
        .collect::<String>()
        + "..."
}

fn short_id(value: &str) -> String {
    value.chars().take(8).collect()
}

fn truncate_ellipsis(value: &str) -> String {
    if value.chars().count() <= 22 {
        return value.to_string();
    }

    format!("{}...", value.chars().take(19).collect::<String>())
}

fn format_iso_minute(value: &str) -> Option<String> {
    let parsed = DateTime::parse_from_rfc3339(value).ok()?;
    Some(
        parsed
            .with_timezone(&Utc)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
    )
}

fn calculate_hours(clocked_in: &str, clocked_out: &str) -> Option<f64> {
    if clocked_in.is_empty() || clocked_out.is_empty() {
        return None;
    }

    let start = DateTime::parse_from_rfc3339(clocked_in).ok()?;
    let end = DateTime::parse_from_rfc3339(clocked_out).ok()?;
    let seconds = (end - start).num_seconds();
    if seconds <= 0 {
        return None;
    }

    Some(seconds as f64 / 3600.0)
}

fn resolve_set_credentials_password(password: Option<String>, prompt_mode: bool) -> Result<String> {
    match (password, prompt_mode) {
        (Some(_), true) => Err(SkyTabError::InvalidArgument(
            "use either --password or --prompt-password, not both".into(),
        )),
        (Some(pw), false) => Ok(pw),
        (None, true) | (None, false) => {
            let entered = prompt_password("SkyTab password: ")?;
            if entered.is_empty() {
                return Err(SkyTabError::InvalidArgument(
                    "password cannot be empty".into(),
                ));
            }
            Ok(entered)
        }
    }
}
