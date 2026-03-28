use chrono::{DateTime, Utc};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, shells};
use reqwest::Method;
use rpassword::prompt_password;
use serde_json::{Value, json};

use skytab_cli::cli::{
    AccountsSubcommand, AuthSubcommand, Cli, Commands, CompletionShell, DateRangeArgs, HttpMethod,
    InsightsSubcommand, LocationsSubcommand, PaymentsSubcommand, ReportsSubcommand,
    TimeclockSubcommand, UpdateArgs,
};
use skytab_cli::client::SkyTabClient;
use skytab_cli::config::{
    clear_default_location_id, resolve_base_url_from_sources, save_credentials,
    save_default_location_id,
};
use skytab_cli::error::{Result, SkyTabError};
use skytab_cli::logging::init_tracing;
use skytab_cli::output;
use skytab_cli::read_api::{
    DailyBriefInsight, DoctorReport, LaborVsSalesInsight, PaymentMixBucket, PaymentMixInsight,
    PayrollByEmployeeData, ReadApi, TillTransactionDetailData, parse_query,
};
use skytab_cli::update::run_update;

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

    match cli.command.clone() {
        Commands::Auth(args) => match args.command {
            AuthSubcommand::Login => {
                let response = read_api.auth_login().await?;
                emit_value(&cli, &serde_json::to_value(response)?)?;
            }
            AuthSubcommand::SetCredentials {
                username,
                password,
                prompt_password,
                base_url,
            } => {
                let password = resolve_set_credentials_password(password, prompt_password)?;
                let path = save_credentials(username, password, base_url).await?;
                emit_value(
                    &cli,
                    &json!({
                        "ok": true,
                        "config_path": path,
                        "message": "credentials saved"
                    }),
                )?;
            }
        },
        Commands::Locations(args) => match args.command {
            LocationsSubcommand::List => {
                let locations = read_api.locations_list().await?;
                if wants_structured_output(&cli) {
                    emit_value(&cli, &serde_json::to_value(locations)?)?;
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
                emit_value(
                    &cli,
                    &json!({
                        "ok": true,
                        "default_location_id": location_id,
                        "config_path": path,
                        "message": "default location saved"
                    }),
                )?;
            }
            LocationsSubcommand::ShowDefault => {
                let response = read_api.locations_show_default().await?;
                emit_value(&cli, &serde_json::to_value(response)?)?;
            }
            LocationsSubcommand::ClearDefault => {
                let path = clear_default_location_id().await?;
                emit_value(
                    &cli,
                    &json!({
                        "ok": true,
                        "default_location_id": Value::Null,
                        "config_path": path,
                        "message": "default location cleared"
                    }),
                )?;
            }
        },
        Commands::Accounts(args) => match args.command {
            AccountsSubcommand::Preferences { account_id } => {
                let response = read_api.accounts_preferences(&account_id).await?;
                emit_value(&cli, &serde_json::to_value(response)?)?;
            }
        },
        Commands::Reports(args) => match args.command {
            ReportsSubcommand::ActivitySummary { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api
                    .report_activity_summary(start, end, location)
                    .await?;
                if wants_structured_output(&cli) {
                    emit_value(&cli, &serde_json::to_value(response)?)?;
                } else {
                    println!("buckets: {}", response.buckets.len());
                }
            }
            ReportsSubcommand::DiscountSummary { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api
                    .report_discount_summary(start, end, location)
                    .await?;
                if wants_structured_output(&cli) {
                    emit_value(&cli, &serde_json::to_value(response)?)?;
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::HourlySales { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api.report_hourly_sales(start, end, location).await?;

                if wants_structured_output(&cli) {
                    emit_value_with_schema(
                        &cli,
                        &serde_json::to_value(response)?,
                        Some(output::CsvSchema::HourlySales),
                    )?;
                } else {
                    println!("DATE\tHOUR\tGROSS\tNET");
                    for row in response.rows {
                        println!("{}\t{}\t{}\t{}", row[0], row[1], row[2], row[3]);
                    }
                }
            }
            ReportsSubcommand::TicketDetailClosed { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api
                    .report_ticket_detail_closed(start, end, location)
                    .await?;
                if wants_structured_output(&cli) {
                    emit_value(&cli, &serde_json::to_value(response)?)?;
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::SalesSummaryByItem { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api
                    .report_sales_summary_by_item(start, end, location)
                    .await?;
                if wants_structured_output(&cli) {
                    emit_value(&cli, &serde_json::to_value(response)?)?;
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::SalesSummaryByRevenueClass { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api
                    .report_sales_summary_by_revenue_class(start, end, location)
                    .await?;
                if wants_structured_output(&cli) {
                    emit_value(&cli, &serde_json::to_value(response)?)?;
                } else {
                    println!("rows: {}", response.rows.len());
                }
            }
            ReportsSubcommand::TillTransaction { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api
                    .report_till_transaction(start, end, location)
                    .await?;

                if wants_structured_output(&cli) {
                    emit_value(&cli, &serde_json::to_value(response)?)?;
                } else {
                    print_till_transaction_human(&response);
                }
            }
            ReportsSubcommand::Payroll { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api.report_payroll(start, end, location).await?;

                if wants_structured_output(&cli) {
                    emit_value_with_schema(
                        &cli,
                        &serde_json::to_value(response)?,
                        Some(output::CsvSchema::Payroll),
                    )?;
                } else {
                    print_payroll_human(&response);
                }
            }
        },
        Commands::Insights(args) => match args.command {
            InsightsSubcommand::DailyBrief { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api.insight_daily_brief(start, end, location).await?;

                if wants_structured_output(&cli) {
                    emit_value_with_schema(
                        &cli,
                        &serde_json::to_value(response)?,
                        Some(output::CsvSchema::InsightsDailyBrief),
                    )?;
                } else {
                    print_daily_brief_human(&response);
                }
            }
            InsightsSubcommand::LaborVsSales { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api
                    .insight_labor_vs_sales(start, end, location)
                    .await?;

                if wants_structured_output(&cli) {
                    emit_value_with_schema(
                        &cli,
                        &serde_json::to_value(response)?,
                        Some(output::CsvSchema::InsightsLaborVsSales),
                    )?;
                } else {
                    print_labor_vs_sales_human(&response);
                }
            }
            InsightsSubcommand::PaymentMix { range, location } => {
                let (start, end) = resolve_date_range(range)?;
                let response = read_api.insight_payment_mix(start, end, location).await?;

                if wants_structured_output(&cli) {
                    emit_value_with_schema(
                        &cli,
                        &serde_json::to_value(response)?,
                        Some(output::CsvSchema::InsightsPaymentMix),
                    )?;
                } else {
                    print_payment_mix_human(&response);
                }
            }
        },
        Commands::Timeclock(args) => match args.command {
            TimeclockSubcommand::Shifts {
                location_id,
                range,
                order,
                limit,
            } => {
                let (start, end) = resolve_date_range(range)?;
                let result = read_api
                    .timeclock_shifts(location_id, start, end, order, limit)
                    .await?;
                if wants_structured_output(&cli) {
                    emit_value_with_schema(
                        &cli,
                        &serde_json::to_value(result)?,
                        Some(output::CsvSchema::TimeclockShifts),
                    )?;
                } else {
                    print_timeclock_shifts_human(&result.time_clock_shifts);
                }
            }
        },
        Commands::Payments(args) => match args.command {
            PaymentsSubcommand::Transactions {
                range,
                location,
                order_type,
            } => {
                let (start, end) = resolve_date_range(range)?;
                let result = read_api
                    .payments_transactions(start, end, location, order_type)
                    .await?;
                if wants_structured_output(&cli) {
                    emit_value_with_schema(
                        &cli,
                        &serde_json::to_value(result)?,
                        Some(output::CsvSchema::PaymentsTransactions),
                    )?;
                } else {
                    println!("count: {}", result.count);
                }
            }
        },
        Commands::Request(args) => {
            ensure_mutating_request_allowed(&args.method, args.allow_write)?;
            let query_pairs = parse_query(&args.query)?;
            let path = args.path;

            match args.method {
                HttpMethod::Get => {
                    let response = read_api.request_get(path, query_pairs).await?;
                    emit_request_value(&cli, &response)?;
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
                    emit_request_value(&cli, &response)?;
                }
            }
        }
        Commands::Completion(args) => {
            if cli.json || cli.format.is_some() {
                return Err(SkyTabError::InvalidArgument(
                    "completion command does not support --json or --format".into(),
                ));
            }

            let script = render_completion_script(args.shell)?;
            output::write_text(&script, cli.output.as_deref())?;
        }
        Commands::Doctor => {
            let report = read_api.doctor_report().await?;
            if wants_structured_output(&cli) {
                emit_value(&cli, &serde_json::to_value(report)?)?;
            } else {
                print_doctor_report(&report);
            }
        }
        Commands::Update(args) => {
            run_update_command(&cli, args).await?;
        }
    }

    Ok(())
}

fn print_doctor_report(report: &DoctorReport) {
    println!("Doctor: {}", if report.ok { "OK" } else { "ISSUES FOUND" });
    for check in &report.checks {
        let status = if check.ok { "OK" } else { "FAIL" };
        println!("[{status}] {:<22} {}", check.name, check.detail);
    }
}

async fn run_update_command(cli: &Cli, args: UpdateArgs) -> Result<()> {
    let report = run_update(args).await?;
    if wants_structured_output(cli) {
        emit_value(cli, &serde_json::to_value(report)?)?;
        return Ok(());
    }

    if report.check_only {
        if report.update_available {
            println!(
                "Update available: {} -> {} ({})",
                report.current_version, report.target_version, report.target_triple
            );
        } else {
            println!("Already up to date ({})", report.current_version);
        }
        return Ok(());
    }

    if report.updated {
        println!(
            "Updated skytab: {} -> {}",
            report.current_version, report.target_version
        );
        if !report.installed_paths.is_empty() {
            println!("Installed:");
            for path in report.installed_paths {
                println!("- {path}");
            }
        }
    } else {
        println!("Already up to date ({})", report.current_version);
    }

    Ok(())
}

async fn build_client(base_url: Option<String>) -> Result<SkyTabClient> {
    let resolved_base_url = resolve_base_url_from_sources(base_url).await?;
    Ok(SkyTabClient::new_lazy(resolved_base_url))
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

fn ensure_mutating_request_allowed(method: &HttpMethod, allow_write: bool) -> Result<()> {
    if is_mutating_method(method) && !allow_write {
        return Err(SkyTabError::InvalidArgument(
            "mutating request blocked; pass --allow-write to execute POST/PUT/PATCH/DELETE".into(),
        ));
    }

    Ok(())
}

fn is_mutating_method(method: &HttpMethod) -> bool {
    matches!(
        method,
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Delete
    )
}

fn resolve_date_range(range: DateRangeArgs) -> Result<(String, String)> {
    if let Some(date_range) = range.date_range {
        return resolve_named_date_range(&date_range);
    }

    match (range.start, range.end) {
        (Some(start), Some(end)) => Ok((start, end)),
        (None, None) => Ok(("today".to_string(), "today".to_string())),
        _ => Err(SkyTabError::InvalidArgument(
            "both --start and --end are required when using explicit range".into(),
        )),
    }
}

fn resolve_named_date_range(value: &str) -> Result<(String, String)> {
    let normalized = value.trim().to_ascii_lowercase();

    if normalized == "today" || normalized == "yesterday" {
        return Ok((normalized.clone(), normalized));
    }

    if let Some(days) = parse_trailing_days_token(&normalized) {
        if days == 0 {
            return Err(SkyTabError::InvalidArgument(
                "invalid --date-range value `0days`; use today, yesterday, or Ndays where N >= 1"
                    .into(),
            ));
        }

        return Ok((format!("{days}days"), "today".to_string()));
    }

    Err(SkyTabError::InvalidArgument(format!(
        "invalid --date-range value `{value}`; expected today, yesterday, or Ndays (for example 7days)"
    )))
}

fn parse_trailing_days_token(value: &str) -> Option<u32> {
    let number = value
        .strip_suffix("days")
        .or_else(|| value.strip_suffix("day"))?;

    if number.is_empty() {
        return None;
    }

    number.parse::<u32>().ok()
}

fn render_completion_script(shell: CompletionShell) -> Result<String> {
    let mut command = Cli::command();
    let mut output_buffer = Vec::<u8>::new();

    match shell {
        CompletionShell::Bash => generate(shells::Bash, &mut command, "skytab", &mut output_buffer),
        CompletionShell::Zsh => generate(shells::Zsh, &mut command, "skytab", &mut output_buffer),
        CompletionShell::Fish => generate(shells::Fish, &mut command, "skytab", &mut output_buffer),
    }

    String::from_utf8(output_buffer)
        .map_err(|_| SkyTabError::InvalidArgument("completion output is not valid UTF-8".into()))
}

fn wants_structured_output(cli: &Cli) -> bool {
    cli.json || cli.format.is_some() || cli.output.is_some()
}

fn emit_value(cli: &Cli, value: &Value) -> Result<()> {
    emit_value_with_schema(cli, value, None)
}

fn emit_value_with_schema(
    cli: &Cli,
    value: &Value,
    csv_schema: Option<output::CsvSchema>,
) -> Result<()> {
    if let Some(format) = cli.format {
        return output::write_structured_value_with_schema(
            value,
            format,
            cli.output.as_deref(),
            csv_schema,
        );
    }

    let rendered = if cli.json || cli.output.is_some() {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    } else {
        serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
    };
    output::write_text(&rendered, cli.output.as_deref())
}

fn emit_request_value(cli: &Cli, value: &Value) -> Result<()> {
    if let Some(format) = cli.format {
        return output::write_structured_value(value, format, cli.output.as_deref());
    }

    let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    output::write_text(&rendered, cli.output.as_deref())
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

fn print_daily_brief_human(data: &DailyBriefInsight) {
    println!("DAILY BRIEF");
    println!(
        "Range: {} -> {} | Locations: {}",
        data.period_start,
        data.period_end,
        format_location_ids(&data.location_ids)
    );
    println!();
    println!(
        "Sales: Gross ${:.2} | Net ${:.2}",
        data.gross_sales, data.net_sales
    );
    println!(
        "Labor: Hours {:.2} | Pay ${:.2} | Labor % of Net {} | Sales/Labor Hr {}",
        data.labor_hours,
        data.labor_pay,
        format_optional_percent(data.labor_percent_of_net_sales),
        format_optional_currency(data.sales_per_labor_hour)
    );
    println!(
        "Payments: Tx {} | Settled {} ({}) | Settled Amount ${:.2}",
        data.transaction_count,
        data.settled_count,
        format_optional_percent(data.settled_rate_percent),
        data.settled_amount
    );

    if let Some(top_type) = &data.top_payment_type {
        println!(
            "Top payment type: {} (${:.2})",
            top_type,
            data.top_payment_type_amount.unwrap_or(0.0)
        );
    }

    println!();
    println!("Highlights:");
    for highlight in &data.highlights {
        println!("- {highlight}");
    }
}

fn print_labor_vs_sales_human(data: &LaborVsSalesInsight) {
    println!("LABOR VS SALES");
    println!(
        "Range: {} -> {} | Locations: {}",
        data.period_start,
        data.period_end,
        format_location_ids(&data.location_ids)
    );
    println!();
    println!(
        "Net sales ${:.2} | Gross sales ${:.2}",
        data.net_sales, data.gross_sales
    );
    println!(
        "Labor pay ${:.2} | Labor hours {:.2} | Employees {}",
        data.labor_pay, data.labor_hours, data.employee_count
    );
    println!(
        "Labor % of net {} | Sales per labor hour {} | Labor pay per labor hour {}",
        format_optional_percent(data.labor_percent_of_net_sales),
        format_optional_currency(data.sales_per_labor_hour),
        format_optional_currency(data.labor_pay_per_labor_hour)
    );

    println!();
    println!("Highlights:");
    for highlight in &data.highlights {
        println!("- {highlight}");
    }
}

fn print_payment_mix_human(data: &PaymentMixInsight) {
    println!("PAYMENT MIX");
    println!(
        "Range: {} -> {} | Locations: {}",
        data.period_start,
        data.period_end,
        format_location_ids(&data.location_ids)
    );
    println!(
        "Transactions: {} | Total amount: ${:.2}",
        data.transaction_count, data.total_amount
    );
    println!();

    print_payment_mix_section("By Type", &data.by_type, 8);
    println!();
    print_payment_mix_section("By Tender", &data.by_tender, 8);

    if !data.highlights.is_empty() {
        println!();
        println!("Highlights:");
        for highlight in &data.highlights {
            println!("- {highlight}");
        }
    }
}

fn print_payment_mix_section(title: &str, rows: &[PaymentMixBucket], limit: usize) {
    println!("{title}");
    println!(
        "{:<16} {:>8} {:>12} {:>10}",
        "KEY", "COUNT", "AMOUNT", "AMOUNT%"
    );
    println!("{:-<16} {:-<8} {:-<12} {:-<10}", "", "", "", "");

    if rows.is_empty() {
        println!("{:<16} {:>8} {:>12} {:>10}", "-", 0, "0.00", "0.0%");
        return;
    }

    for row in rows.iter().take(limit) {
        println!(
            "{:<16} {:>8} {:>12.2} {:>9.1}%",
            truncate_to(row.key.as_str(), 16),
            row.count,
            row.amount,
            row.share_of_amount
        );
    }
}

fn format_location_ids(location_ids: &[i64]) -> String {
    if location_ids.is_empty() {
        return "-".to_string();
    }

    location_ids
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_optional_percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "-".to_string())
}

fn format_optional_currency(value: Option<f64>) -> String {
    value
        .map(|value| format!("${value:.2}"))
        .unwrap_or_else(|| "-".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_methods_require_allow_write() {
        let blocked_post = ensure_mutating_request_allowed(&HttpMethod::Post, false)
            .expect_err("post should be blocked without allow-write");
        let blocked_put = ensure_mutating_request_allowed(&HttpMethod::Put, false)
            .expect_err("put should be blocked without allow-write");
        let blocked_patch = ensure_mutating_request_allowed(&HttpMethod::Patch, false)
            .expect_err("patch should be blocked without allow-write");
        let blocked_delete = ensure_mutating_request_allowed(&HttpMethod::Delete, false)
            .expect_err("delete should be blocked without allow-write");

        for blocked in [blocked_post, blocked_put, blocked_patch, blocked_delete] {
            match blocked {
                SkyTabError::InvalidArgument(message) => {
                    assert!(message.contains("--allow-write"));
                }
                other => panic!("unexpected error: {other}"),
            }
        }
    }

    #[test]
    fn allow_write_permits_mutating_methods() {
        ensure_mutating_request_allowed(&HttpMethod::Post, true)
            .expect("post should be allowed with --allow-write");
        ensure_mutating_request_allowed(&HttpMethod::Put, true)
            .expect("put should be allowed with --allow-write");
        ensure_mutating_request_allowed(&HttpMethod::Patch, true)
            .expect("patch should be allowed with --allow-write");
        ensure_mutating_request_allowed(&HttpMethod::Delete, true)
            .expect("delete should be allowed with --allow-write");
    }

    #[test]
    fn get_request_never_requires_allow_write() {
        ensure_mutating_request_allowed(&HttpMethod::Get, false)
            .expect("get should never require --allow-write");
    }

    #[test]
    fn resolve_date_range_defaults_to_today() {
        let (start, end) = resolve_date_range(DateRangeArgs {
            start: None,
            end: None,
            date_range: None,
        })
        .expect("empty date range should default to today");

        assert_eq!(start, "today");
        assert_eq!(end, "today");
    }

    #[test]
    fn resolve_named_date_range_supports_rolling_days() {
        let (start, end) = resolve_named_date_range("7days").expect("rolling range should parse");
        assert_eq!(start, "7days");
        assert_eq!(end, "today");
    }

    #[test]
    fn resolve_named_date_range_rejects_invalid_values() {
        let err = resolve_named_date_range("lastweek").expect_err("invalid range should fail");

        match err {
            SkyTabError::InvalidArgument(message) => {
                assert!(message.contains("--date-range"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
