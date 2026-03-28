use chrono::{DateTime, Utc};
use clap::Parser;
use reqwest::Method;
use rpassword::prompt_password;
use serde_json::{Value, json};

use skytab_cli::cli::{
    AccountsSubcommand, AuthSubcommand, Cli, Commands, HttpMethod, LocationsSubcommand,
    PaymentsSubcommand, ReportsSubcommand, TimeclockSubcommand,
};
use skytab_cli::client::SkyTabClient;
use skytab_cli::config::{
    Config, clear_default_location_id, save_credentials, save_default_location_id,
};
use skytab_cli::error::{Result, SkyTabError};
use skytab_cli::logging::init_tracing;
use skytab_cli::read_api::{
    DoctorReport, PayrollByEmployeeData, ReadApi, TillTransactionDetailData, parse_query,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose, false);

    if let Err(err) = run(cli).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let read_api = ReadApi::new(cli.base_url.clone());

    match cli.command {
        Commands::Auth(args) => match args.command {
            AuthSubcommand::Login => {
                let response = read_api.auth_login().await?;
                print_output(cli.json, &serde_json::to_value(response)?);
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
                let locations = read_api.locations_list().await?;
                if cli.json {
                    print_output(true, &serde_json::to_value(locations)?);
                } else {
                    for location in locations {
                        println!(
                            "{}\t{}\t{}",
                            location.id,
                            location.name,
                            location.timezone.unwrap_or_else(|| "-".to_string())
                        );
                    }
                }
            }
            LocationsSubcommand::SetDefault { location_id } => {
                let locations = read_api.locations_list().await?;
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
                let response = read_api.locations_show_default().await?;
                print_output(cli.json, &serde_json::to_value(response)?);
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
                let response = read_api.accounts_preferences(&account_id).await?;
                print_output(cli.json, &serde_json::to_value(response)?);
            }
        },
        Commands::Reports(args) => match args.command {
            ReportsSubcommand::ActivitySummary {
                start,
                end,
                location,
            } => {
                let response = read_api
                    .report_activity_summary(start, end, location)
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
                let response = read_api
                    .report_discount_summary(start, end, location)
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
                let response = read_api.report_hourly_sales(start, end, location).await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    println!("DATE\tHOUR\tGROSS\tNET");
                    for row in response.rows {
                        println!("{}\t{}\t{}\t{}", row[0], row[1], row[2], row[3]);
                    }
                }
            }
            ReportsSubcommand::TicketDetailClosed {
                start,
                end,
                location,
            } => {
                let response = read_api
                    .report_ticket_detail_closed(start, end, location)
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
                let response = read_api
                    .report_sales_summary_by_item(start, end, location)
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
                let response = read_api
                    .report_sales_summary_by_revenue_class(start, end, location)
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
                let response = read_api
                    .report_till_transaction(start, end, location)
                    .await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    print_till_transaction_human(&response);
                }
            }
            ReportsSubcommand::Payroll {
                start,
                end,
                location,
            } => {
                let response = read_api.report_payroll(start, end, location).await?;

                if cli.json {
                    print_output(true, &serde_json::to_value(response)?);
                } else {
                    print_payroll_human(&response);
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
                let result = read_api
                    .timeclock_shifts(location_id, start, end, order, limit)
                    .await?;
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
                let result = read_api
                    .payments_transactions(start, end, location, order_type)
                    .await?;
                if cli.json {
                    print_output(true, &serde_json::to_value(result)?);
                } else {
                    println!("count: {}", result.count);
                }
            }
        },
        Commands::Request(args) => {
            let query_pairs = parse_query(&args.query)?;
            let path = args.path;

            match args.method {
                HttpMethod::Get => {
                    let response = read_api.request_get(path, query_pairs).await?;
                    print_output(true, &response);
                }
                method => {
                    let client = build_client(cli.base_url.clone()).await?;
                    let body = match args.body {
                        Some(body) => Some(serde_json::from_str::<Value>(&body)?),
                        None => None,
                    };

                    let response: Value = client
                        .request_authed_json(map_method(method), &path, &query_pairs, body)
                        .await?;
                    print_output(true, &response);
                }
            }
        }
        Commands::Doctor => {
            let report = read_api.doctor_report().await?;
            print_doctor_report(&report, cli.json)?;
        }
    }

    Ok(())
}

fn print_doctor_report(report: &DoctorReport, json_mode: bool) -> Result<()> {
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

async fn build_client(base_url: Option<String>) -> Result<SkyTabClient> {
    let config = Config::from_sources(base_url).await?;
    Ok(SkyTabClient::new(config))
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
                    .map(|seconds| seconds / 3600.0)
            })
            .unwrap_or(0.0);
        total_hours += hours;

        let rate = shift
            .get("payRate")
            .and_then(Value::as_str)
            .map(|value| format!("${value}"))
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
