use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::cache::TokenCache;
use crate::client::SkyTabClient;
use crate::config::{
    Config, credential_storage_diagnostics, current_config_file_path, get_default_location_id,
    legacy_config_file_path,
};
use crate::error::{Result, SkyTabError};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthLoginResponse {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Location {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default, rename = "merchantId")]
    pub merchant_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LocationsWrappedResponse {
    #[serde(default)]
    locations: Vec<Location>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultLocationResponse {
    pub default_location_id: Option<i64>,
    pub configured: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HourlySalesResponse {
    pub rows: Vec<[String; 4]>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeClockShiftsResponse {
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
pub struct TicketDetailClosedResponse {
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActivitySummaryResponse {
    pub buckets: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReportRowsResponse {
    pub rows: Vec<Value>,
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
pub struct TillTransactionDetailData {
    pub name: String,
    pub headers: Value,
    pub items: Vec<TillTransactionItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TillTransactionItem {
    pub till_name: String,
    pub date_time: String,
    pub employee_name: String,
    pub drawer_action: String,
    pub amount: f64,
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
pub struct PayrollByEmployeeData {
    pub name: String,
    pub headers: Value,
    pub employees: Vec<PayrollEmployee>,
    pub totals: Option<PayrollEmployee>,
    pub custom: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayrollEmployee {
    pub employee_id: String,
    pub employee_name: String,
    pub normal_hours: f64,
    pub overtime_hours: f64,
    pub double_overtime_hours: f64,
    pub normal_pay: f64,
    pub overtime_pay: f64,
    pub double_overtime_pay: f64,
    pub total_gratuity: f64,
    pub total_pay: f64,
    pub adjusted_tips: f64,
    pub tip_reduction: f64,
    pub declared_tips: f64,
    pub gross_tips: f64,
    pub tip_share: f64,
    pub net_tips: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountPreferencesResponse {
    pub eod: String,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
    #[serde(default)]
    #[serde(rename = "weekStart")]
    pub week_start: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeClockShiftsResult {
    pub count: usize,
    #[serde(rename = "timeClockShifts")]
    pub time_clock_shifts: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionsResult {
    pub count: usize,
    pub transactions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentMixBucket {
    pub key: String,
    pub count: usize,
    pub amount: f64,
    pub share_of_count: f64,
    pub share_of_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyBriefInsight {
    pub period_start: String,
    pub period_end: String,
    pub location_ids: Vec<i64>,
    pub gross_sales: f64,
    pub net_sales: f64,
    pub labor_hours: f64,
    pub labor_pay: f64,
    pub labor_percent_of_net_sales: Option<f64>,
    pub sales_per_labor_hour: Option<f64>,
    pub transaction_count: usize,
    pub settled_count: usize,
    pub settled_amount: f64,
    pub settled_rate_percent: Option<f64>,
    pub top_payment_type: Option<String>,
    pub top_payment_type_amount: Option<f64>,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaborVsSalesInsight {
    pub period_start: String,
    pub period_end: String,
    pub location_ids: Vec<i64>,
    pub gross_sales: f64,
    pub net_sales: f64,
    pub labor_hours: f64,
    pub labor_pay: f64,
    pub labor_percent_of_net_sales: Option<f64>,
    pub sales_per_labor_hour: Option<f64>,
    pub labor_pay_per_labor_hour: Option<f64>,
    pub employee_count: usize,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentMixInsight {
    pub by_type: Vec<PaymentMixBucket>,
    pub by_tender: Vec<PaymentMixBucket>,
    pub period_start: String,
    pub period_end: String,
    pub location_ids: Vec<i64>,
    pub transaction_count: usize,
    pub total_amount: f64,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndOfDayInsight {
    pub daily_brief: DailyBriefInsight,
    pub labor_vs_sales: LaborVsSalesInsight,
    pub payment_mix: PaymentMixInsight,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeclockShiftSummary {
    pub shift_count: usize,
    pub open_shift_count: usize,
    pub total_hours: f64,
}

#[derive(Debug, Clone)]
struct InsightMetrics {
    period_start: String,
    period_end: String,
    location_ids: Vec<i64>,
    gross_sales: f64,
    net_sales: f64,
    labor_hours: f64,
    labor_pay: f64,
    employee_count: usize,
    transaction_count: usize,
    settled_count: usize,
    settled_amount: f64,
    total_payment_amount: f64,
    payment_type_mix: Vec<PaymentMixBucket>,
    payment_tender_mix: Vec<PaymentMixBucket>,
}

#[derive(Debug, Clone, Default)]
struct MixAccumulator {
    count: usize,
    amount: f64,
}

#[derive(Debug, Clone)]
struct PaymentAggregation {
    transaction_count: usize,
    settled_count: usize,
    settled_amount: f64,
    total_amount: f64,
    by_type: Vec<PaymentMixBucket>,
    by_tender: Vec<PaymentMixBucket>,
}

#[derive(Debug, Clone)]
pub struct ReadApi {
    base_url: Option<String>,
}

impl ReadApi {
    pub fn new(base_url: Option<String>) -> Self {
        Self { base_url }
    }

    pub async fn auth_login(&self) -> Result<AuthLoginResponse> {
        let client = self.build_client().await?;
        let token = client.token(true).await?;
        Ok(AuthLoginResponse { token })
    }

    pub async fn locations_list(&self) -> Result<Vec<Location>> {
        let client = self.build_client().await?;
        let raw_locations: Value = client
            .request_authed_json(Method::GET, "/api/v2/locations", &[], None)
            .await?;
        parse_locations_response(raw_locations)
    }

    pub async fn locations_show_default(&self) -> Result<DefaultLocationResponse> {
        let default_location_id = get_default_location_id().await?;
        Ok(DefaultLocationResponse {
            default_location_id,
            configured: default_location_id.is_some(),
        })
    }

    pub async fn accounts_preferences(
        &self,
        account_id: &str,
    ) -> Result<AccountPreferencesResponse> {
        let client = self.build_client().await?;
        let path = format!("/api/v1/accounts/{account_id}/preferences");
        client
            .request_authed_json(Method::GET, &path, &[], None)
            .await
    }

    pub async fn report_activity_summary(
        &self,
        start: String,
        end: String,
        location: Option<i64>,
    ) -> Result<ActivitySummaryResponse> {
        let location = self.resolve_single_location(location).await?;
        let client = self.build_client().await?;
        let (start, end) =
            normalize_range_for_locations_timezones(&client, &[location], start, end).await?;
        let payload = json!({
            "start": start,
            "end": end,
            "locations": [location.to_string()],
            "locale": "en-US",
            "intradayPeriodGroupGuids": [],
            "revenueCenterGuids": []
        });

        client
            .request_authed_json(
                Method::POST,
                "/api/v1/reports/echo-pro/activity-summary",
                &[],
                Some(payload),
            )
            .await
    }

    pub async fn report_discount_summary(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<ReportRowsResponse> {
        self.run_multi_location_report(
            "/api/v1/reports/echo-pro/discount-summary",
            start,
            end,
            location,
        )
        .await
    }

    pub async fn report_hourly_sales(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<HourlySalesResponse> {
        self.run_multi_location_report(
            "/api/v1/reports/echo-pro/hourly-sales",
            start,
            end,
            location,
        )
        .await
    }

    pub async fn report_ticket_detail_closed(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<TicketDetailClosedResponse> {
        self.run_multi_location_report(
            "/api/v1/reports/echo-pro/ticket-detail-closed",
            start,
            end,
            location,
        )
        .await
    }

    pub async fn report_sales_summary_by_item(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<ReportRowsResponse> {
        self.run_multi_location_report(
            "/api/v1/reports/echo-pro/sales-summary-by-item",
            start,
            end,
            location,
        )
        .await
    }

    pub async fn report_sales_summary_by_revenue_class(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<ReportRowsResponse> {
        self.run_multi_location_report(
            "/api/v1/reports/echo-pro/sales-summary-by-revenue-class",
            start,
            end,
            location,
        )
        .await
    }

    pub async fn report_till_transaction(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<TillTransactionDetailData> {
        let response: TillTransactionDetailResponse = self
            .run_multi_location_report(
                "/api/v1/reports/echo-pro/till-transaction-detail",
                start,
                end,
                location,
            )
            .await?;

        Ok(transform_till_transaction_response(response))
    }

    pub async fn report_payroll(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<PayrollByEmployeeData> {
        let response: PayrollByEmployeeResponse = self
            .run_multi_location_report(
                "/api/v1/reports/echo-pro/payroll-by-employee-new",
                start,
                end,
                location,
            )
            .await?;

        Ok(transform_payroll_response(response))
    }

    pub async fn timeclock_shifts(
        &self,
        location_id: Option<i64>,
        start: String,
        end: String,
        order: String,
        limit: usize,
    ) -> Result<TimeClockShiftsResult> {
        if limit == 0 {
            return Err(SkyTabError::InvalidArgument(
                "limit must be greater than zero".into(),
            ));
        }

        let location_id = self.resolve_single_location(location_id).await?;
        let client = self.build_client().await?;
        let (start, end) =
            normalize_range_for_location_timezone(&client, location_id, start, end).await?;
        fetch_timeclock_shifts(&client, location_id, start, end, order, limit).await
    }

    pub async fn payments_transactions(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
        order_type: Option<String>,
    ) -> Result<TransactionsResult> {
        let location = self.resolve_locations(location).await?;
        let client = self.build_client().await?;
        fetch_transactions(&client, start, end, location, order_type).await
    }

    pub async fn insight_daily_brief(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<DailyBriefInsight> {
        let metrics = self.collect_insight_metrics(start, end, location).await?;
        Ok(build_daily_brief_insight(&metrics))
    }

    pub async fn insight_labor_vs_sales(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<LaborVsSalesInsight> {
        let metrics = self.collect_insight_metrics(start, end, location).await?;
        Ok(build_labor_vs_sales_insight(&metrics))
    }

    pub async fn insight_payment_mix(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<PaymentMixInsight> {
        let metrics = self.collect_insight_metrics(start, end, location).await?;
        Ok(build_payment_mix_insight(&metrics))
    }

    pub async fn insight_end_of_day(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<EndOfDayInsight> {
        let metrics = self.collect_insight_metrics(start, end, location).await?;

        Ok(build_end_of_day_insight(&metrics))
    }

    pub async fn request_get(&self, path: String, query: Vec<(String, String)>) -> Result<Value> {
        if !path.starts_with('/') {
            return Err(SkyTabError::InvalidArgument(
                "path must start with '/'".into(),
            ));
        }

        let client = self.build_client().await?;
        client
            .request_authed_json(Method::GET, &path, &query, None)
            .await
    }

    async fn collect_insight_metrics(
        &self,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<InsightMetrics> {
        let location_ids = self.resolve_locations(location).await?;
        let client = self.build_client().await?;
        let (period_start, period_end) =
            normalize_range_for_locations_timezones(&client, &location_ids, start, end).await?;

        let hourly_sales_future = run_multi_location_report_with_client::<HourlySalesResponse>(
            &client,
            "/api/v1/reports/echo-pro/hourly-sales",
            &period_start,
            &period_end,
            &location_ids,
        );
        let payroll_future = run_multi_location_report_with_client::<PayrollByEmployeeResponse>(
            &client,
            "/api/v1/reports/echo-pro/payroll-by-employee-new",
            &period_start,
            &period_end,
            &location_ids,
        );
        let payments_future = fetch_transactions(
            &client,
            period_start.clone(),
            period_end.clone(),
            location_ids.clone(),
            None,
        );

        let (hourly_sales, payroll_response, payments_result) =
            tokio::try_join!(hourly_sales_future, payroll_future, payments_future)?;

        let payroll_data = transform_payroll_response(payroll_response);
        let payment_aggregation = summarize_payment_transactions(&payments_result.transactions);

        Ok(InsightMetrics {
            period_start,
            period_end,
            location_ids,
            gross_sales: sum_hourly_sales_column(&hourly_sales.rows, 2),
            net_sales: sum_hourly_sales_column(&hourly_sales.rows, 3),
            labor_hours: payroll_total_hours(&payroll_data),
            labor_pay: payroll_total_pay(&payroll_data),
            employee_count: payroll_data.employees.len(),
            transaction_count: payment_aggregation.transaction_count,
            settled_count: payment_aggregation.settled_count,
            settled_amount: payment_aggregation.settled_amount,
            total_payment_amount: payment_aggregation.total_amount,
            payment_type_mix: payment_aggregation.by_type,
            payment_tender_mix: payment_aggregation.by_tender,
        })
    }

    pub async fn doctor_report(&self) -> Result<DoctorReport> {
        let mut checks: Vec<DoctorCheck> = Vec::new();

        let env_username = std::env::var("SKYTAB_USERNAME").ok();
        let env_password = std::env::var("SKYTAB_PASSWORD").ok();
        let env_ok = env_username.is_some() && env_password.is_some();
        let partial_env = env_username.is_some() ^ env_password.is_some();

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

        let credential_diagnostics = credential_storage_diagnostics(self.base_url.clone()).await?;
        let has_persisted_credentials = credential_diagnostics.keyring_password_present
            || credential_diagnostics.config_password_present;

        checks.push(DoctorCheck {
            name: "env_credentials".to_string(),
            ok: if partial_env {
                false
            } else {
                env_ok || has_persisted_credentials
            },
            detail: format!(
                "SKYTAB_USERNAME={} SKYTAB_PASSWORD={} (partial_env={} persisted_credentials={})",
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
                if partial_env { "yes" } else { "no" },
                if has_persisted_credentials {
                    "available"
                } else {
                    "missing"
                }
            ),
        });

        checks.push(DoctorCheck {
            name: "credential_store".to_string(),
            ok: credential_diagnostics.mode != "keyring"
                || credential_diagnostics.keyring_accessible,
            detail: format!(
                "mode={} keyring_supported={} keyring_accessible={} keyring_password_present={} config_password_present={}",
                credential_diagnostics.mode,
                credential_diagnostics.keyring_supported,
                credential_diagnostics.keyring_accessible,
                credential_diagnostics.keyring_password_present,
                credential_diagnostics.config_password_present,
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

        let config = Config::from_sources(self.base_url.clone()).await;
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

        Ok(DoctorReport {
            ok: checks.iter().all(|check| check.ok),
            checks,
        })
    }

    async fn build_client(&self) -> Result<SkyTabClient> {
        let config = Config::from_sources(self.base_url.clone()).await?;
        Ok(SkyTabClient::new(config))
    }

    async fn resolve_single_location(&self, location: Option<i64>) -> Result<i64> {
        if let Some(location) = location {
            return Ok(location);
        }

        if let Some(default_location_id) = get_default_location_id().await? {
            return Ok(default_location_id);
        }

        if let Some(single_location_id) = self.resolve_sole_available_location().await? {
            return Ok(single_location_id);
        }

        Err(SkyTabError::InvalidArgument(
            "location is required; pass --location/--location-id, set a default with `locations set-default`, or rely on auto-selection when exactly one location is available"
                .into(),
        ))
    }

    async fn resolve_locations(&self, locations: Vec<i64>) -> Result<Vec<i64>> {
        if !locations.is_empty() {
            return Ok(locations);
        }

        if let Some(default_location_id) = get_default_location_id().await? {
            return Ok(vec![default_location_id]);
        }

        if let Some(single_location_id) = self.resolve_sole_available_location().await? {
            return Ok(vec![single_location_id]);
        }

        Err(SkyTabError::InvalidArgument(
            "at least one location is required; pass --location, set a default with `locations set-default`, or rely on auto-selection when exactly one location is available"
                .into(),
        ))
    }

    async fn resolve_sole_available_location(&self) -> Result<Option<i64>> {
        let client = self.build_client().await?;
        let raw_locations: Value = client
            .request_authed_json(Method::GET, "/api/v2/locations", &[], None)
            .await?;
        let locations = parse_locations_response(raw_locations)?;

        if locations.len() == 1 {
            return Ok(Some(locations[0].id));
        }

        Ok(None)
    }

    async fn run_multi_location_report<T>(
        &self,
        endpoint: &str,
        start: String,
        end: String,
        location: Vec<i64>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let location = self.resolve_locations(location).await?;
        let client = self.build_client().await?;
        let (start, end) =
            normalize_range_for_locations_timezones(&client, &location, start, end).await?;
        let payload = build_report_payload(start, end, location);

        client
            .request_authed_json(Method::POST, endpoint, &[], Some(payload))
            .await
    }
}

async fn run_multi_location_report_with_client<T>(
    client: &SkyTabClient,
    endpoint: &str,
    start: &str,
    end: &str,
    location_ids: &[i64],
) -> Result<T>
where
    T: DeserializeOwned,
{
    let payload = build_report_payload(start.to_string(), end.to_string(), location_ids.to_vec());
    client
        .request_authed_json(Method::POST, endpoint, &[], Some(payload))
        .await
}

fn sum_hourly_sales_column(rows: &[[String; 4]], index: usize) -> f64 {
    rows.iter()
        .map(|row| {
            row.get(index)
                .map(|value| parse_currency_number(value))
                .unwrap_or(0.0)
        })
        .sum()
}

fn payroll_total_hours(data: &PayrollByEmployeeData) -> f64 {
    if let Some(totals) = &data.totals {
        return totals.normal_hours + totals.overtime_hours + totals.double_overtime_hours;
    }

    data.employees
        .iter()
        .map(|employee| {
            employee.normal_hours + employee.overtime_hours + employee.double_overtime_hours
        })
        .sum()
}

fn payroll_total_pay(data: &PayrollByEmployeeData) -> f64 {
    if let Some(totals) = &data.totals {
        return totals.total_pay;
    }

    data.employees
        .iter()
        .map(|employee| employee.total_pay)
        .sum()
}

fn summarize_payment_transactions(transactions: &[Value]) -> PaymentAggregation {
    let mut transaction_count = 0usize;
    let mut settled_count = 0usize;
    let mut settled_amount = 0.0_f64;
    let mut total_amount = 0.0_f64;
    let mut by_type = BTreeMap::<String, MixAccumulator>::new();
    let mut by_tender = BTreeMap::<String, MixAccumulator>::new();

    for transaction in transactions {
        if !transaction.is_object() {
            continue;
        }

        transaction_count += 1;

        let amount = transaction_amount(transaction);
        total_amount += amount;

        let status = first_string_at_paths(transaction, &["status", "state", "paymentStatus"]);
        if is_settled_status(status.as_deref()) {
            settled_count += 1;
            settled_amount += amount;
        }

        let type_key = normalize_bucket_key(
            first_string_at_paths(transaction, &["type", "orderType", "transactionType"])
                .as_deref(),
            "UNKNOWN",
        );
        bump_mix_bucket(&mut by_type, type_key, amount);

        let tender_key = normalize_bucket_key(
            first_string_at_paths(
                transaction,
                &[
                    "cardBrand",
                    "cardType",
                    "card.brand",
                    "paymentMethod.cardBrand",
                    "paymentMethod.cardType",
                    "paymentMethod.type",
                    "method",
                ],
            )
            .as_deref(),
            "UNKNOWN",
        );
        bump_mix_bucket(&mut by_tender, tender_key, amount);
    }

    PaymentAggregation {
        transaction_count,
        settled_count,
        settled_amount,
        total_amount,
        by_type: finalize_mix_rows(by_type, transaction_count, total_amount),
        by_tender: finalize_mix_rows(by_tender, transaction_count, total_amount),
    }
}

fn bump_mix_bucket(buckets: &mut BTreeMap<String, MixAccumulator>, key: String, amount: f64) {
    let entry = buckets.entry(key).or_default();
    entry.count += 1;
    entry.amount += amount;
}

fn finalize_mix_rows(
    buckets: BTreeMap<String, MixAccumulator>,
    total_count: usize,
    total_amount: f64,
) -> Vec<PaymentMixBucket> {
    let mut rows = buckets
        .into_iter()
        .map(|(key, aggregate)| PaymentMixBucket {
            key,
            count: aggregate.count,
            amount: aggregate.amount,
            share_of_count: percentage(aggregate.count as f64, total_count as f64).unwrap_or(0.0),
            share_of_amount: percentage(aggregate.amount, total_amount).unwrap_or(0.0),
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        right
            .amount
            .partial_cmp(&left.amount)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.key.cmp(&right.key))
    });

    rows
}

fn transaction_amount(transaction: &Value) -> f64 {
    first_number_at_paths(
        transaction,
        &[
            "totalAmount",
            "totals.total",
            "amount",
            "paymentAmount",
            "netAmount",
            "totals.amount",
        ],
    )
    .unwrap_or(0.0)
}

fn normalize_bucket_key(value: Option<&str>, fallback: &str) -> String {
    match value.map(str::trim) {
        Some("") | None => fallback.to_string(),
        Some(value) => value.to_ascii_uppercase(),
    }
}

fn is_settled_status(status: Option<&str>) -> bool {
    let Some(status) = status else {
        return false;
    };
    let normalized = status.trim().to_ascii_uppercase();
    matches!(
        normalized.as_str(),
        "SETTLED" | "CAPTURED" | "SUCCEEDED" | "SUCCESS" | "PAID"
    )
}

fn first_string_at_paths(value: &Value, paths: &[&str]) -> Option<String> {
    for path in paths {
        if let Some(found) = value_at_path(value, path).and_then(Value::as_str) {
            let trimmed = found.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn first_number_at_paths(value: &Value, paths: &[&str]) -> Option<f64> {
    for path in paths {
        if let Some(found) = value_at_path(value, path) {
            if let Some(number) = found.as_f64() {
                return Some(number);
            }
            if let Some(string_value) = found.as_str() {
                return Some(parse_currency_number(string_value));
            }
        }
    }
    None
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

fn percentage(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator.abs() < f64::EPSILON {
        return None;
    }
    Some((numerator / denominator) * 100.0)
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator.abs() < f64::EPSILON {
        return None;
    }
    Some(numerator / denominator)
}

fn build_daily_brief_insight(metrics: &InsightMetrics) -> DailyBriefInsight {
    let highlights = build_daily_brief_highlights(metrics);
    let top_payment_type = metrics.payment_type_mix.first().cloned();

    DailyBriefInsight {
        period_start: metrics.period_start.clone(),
        period_end: metrics.period_end.clone(),
        location_ids: metrics.location_ids.clone(),
        gross_sales: metrics.gross_sales,
        net_sales: metrics.net_sales,
        labor_hours: metrics.labor_hours,
        labor_pay: metrics.labor_pay,
        labor_percent_of_net_sales: percentage(metrics.labor_pay, metrics.net_sales),
        sales_per_labor_hour: ratio(metrics.net_sales, metrics.labor_hours),
        transaction_count: metrics.transaction_count,
        settled_count: metrics.settled_count,
        settled_amount: metrics.settled_amount,
        settled_rate_percent: percentage(
            metrics.settled_count as f64,
            metrics.transaction_count as f64,
        ),
        top_payment_type: top_payment_type.as_ref().map(|item| item.key.clone()),
        top_payment_type_amount: top_payment_type.map(|item| item.amount),
        highlights,
    }
}

fn build_labor_vs_sales_insight(metrics: &InsightMetrics) -> LaborVsSalesInsight {
    let highlights = build_labor_vs_sales_highlights(metrics);

    LaborVsSalesInsight {
        period_start: metrics.period_start.clone(),
        period_end: metrics.period_end.clone(),
        location_ids: metrics.location_ids.clone(),
        gross_sales: metrics.gross_sales,
        net_sales: metrics.net_sales,
        labor_hours: metrics.labor_hours,
        labor_pay: metrics.labor_pay,
        labor_percent_of_net_sales: percentage(metrics.labor_pay, metrics.net_sales),
        sales_per_labor_hour: ratio(metrics.net_sales, metrics.labor_hours),
        labor_pay_per_labor_hour: ratio(metrics.labor_pay, metrics.labor_hours),
        employee_count: metrics.employee_count,
        highlights,
    }
}

fn build_payment_mix_insight(metrics: &InsightMetrics) -> PaymentMixInsight {
    let highlights = build_payment_mix_highlights(metrics);

    PaymentMixInsight {
        by_type: metrics.payment_type_mix.clone(),
        by_tender: metrics.payment_tender_mix.clone(),
        period_start: metrics.period_start.clone(),
        period_end: metrics.period_end.clone(),
        location_ids: metrics.location_ids.clone(),
        transaction_count: metrics.transaction_count,
        total_amount: metrics.total_payment_amount,
        highlights,
    }
}

fn build_end_of_day_insight(metrics: &InsightMetrics) -> EndOfDayInsight {
    EndOfDayInsight {
        daily_brief: build_daily_brief_insight(metrics),
        labor_vs_sales: build_labor_vs_sales_insight(metrics),
        payment_mix: build_payment_mix_insight(metrics),
    }
}

fn build_daily_brief_highlights(metrics: &InsightMetrics) -> Vec<String> {
    let mut highlights = Vec::new();

    if metrics.net_sales <= 0.0 {
        highlights.push("No net sales recorded for the selected range.".to_string());
    }

    if let Some(labor_pct) = percentage(metrics.labor_pay, metrics.net_sales) {
        if labor_pct >= 35.0 {
            highlights.push(format!("Labor is high at {:.1}% of net sales.", labor_pct));
        }
    }

    if metrics.transaction_count == 0 {
        highlights.push("No payment transactions were found.".to_string());
    } else if let Some(settled_rate) = percentage(
        metrics.settled_count as f64,
        metrics.transaction_count as f64,
    ) {
        if settled_rate < 95.0 {
            highlights.push(format!(
                "Settlement rate is {:.1}% ({} of {}).",
                settled_rate, metrics.settled_count, metrics.transaction_count
            ));
        }
    }

    if let Some(top_type) = metrics.payment_type_mix.first() {
        highlights.push(format!(
            "Top payment type is {} at ${:.2}.",
            top_type.key, top_type.amount
        ));
    }

    if highlights.is_empty() {
        highlights.push("No issues detected in this range.".to_string());
    }

    highlights
}

fn build_labor_vs_sales_highlights(metrics: &InsightMetrics) -> Vec<String> {
    let mut highlights = Vec::new();

    if metrics.labor_hours <= 0.0 {
        highlights.push("No labor hours found in payroll data.".to_string());
    }

    if let Some(labor_pct) = percentage(metrics.labor_pay, metrics.net_sales) {
        if labor_pct >= 35.0 {
            highlights.push(format!("Labor is high at {:.1}% of net sales.", labor_pct));
        } else if labor_pct <= 20.0 {
            highlights.push(format!("Labor is lean at {:.1}% of net sales.", labor_pct));
        }
    }

    if let Some(sales_per_hour) = ratio(metrics.net_sales, metrics.labor_hours) {
        highlights.push(format!(
            "Net sales per labor hour is ${:.2}.",
            sales_per_hour
        ));
    }

    if highlights.is_empty() {
        highlights.push("Labor and sales trends look stable.".to_string());
    }

    highlights
}

fn build_payment_mix_highlights(metrics: &InsightMetrics) -> Vec<String> {
    let mut highlights = Vec::new();

    if metrics.transaction_count == 0 {
        highlights.push("No payment transactions were found.".to_string());
        return highlights;
    }

    if let Some(top_type) = metrics.payment_type_mix.first() {
        highlights.push(format!(
            "{} leads payment type mix at {:.1}% of amount.",
            top_type.key, top_type.share_of_amount
        ));
    }

    if let Some(top_tender) = metrics.payment_tender_mix.first() {
        highlights.push(format!(
            "{} is the top tender at {:.1}% of amount.",
            top_tender.key, top_tender.share_of_amount
        ));
    }

    let unsettled_count = metrics
        .transaction_count
        .saturating_sub(metrics.settled_count);
    if unsettled_count > 0 {
        highlights.push(format!(
            "{} transactions are not settled yet.",
            unsettled_count
        ));
    }

    highlights
}

pub fn summarize_timeclock_shifts(shifts: &[Value]) -> TimeclockShiftSummary {
    let mut open_shift_count = 0usize;
    let mut total_hours = 0.0_f64;

    for shift in shifts {
        let clocked_out = shift
            .get("clockedOutAt")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if clocked_out.is_empty() {
            open_shift_count += 1;
        }

        if let Some(hours) = timeclock_shift_hours(shift) {
            total_hours += hours;
        }
    }

    TimeclockShiftSummary {
        shift_count: shifts.len(),
        open_shift_count,
        total_hours,
    }
}

fn timeclock_shift_hours(shift: &Value) -> Option<f64> {
    let clocked_in = shift
        .get("clockedInAt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let clocked_out = shift
        .get("clockedOutAt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    if !clocked_in.is_empty() && !clocked_out.is_empty() {
        let start = chrono::DateTime::parse_from_rfc3339(clocked_in).ok()?;
        let end = chrono::DateTime::parse_from_rfc3339(clocked_out).ok()?;
        let seconds = (end - start).num_seconds();
        if seconds > 0 {
            return Some(seconds as f64 / 3600.0);
        }
    }

    shift
        .get("clockedInSeconds")
        .and_then(Value::as_f64)
        .map(|seconds| seconds / 3600.0)
}

pub fn parse_query(parts: &[String]) -> Result<Vec<(String, String)>> {
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
        .request_authed_json(Method::GET, "/api/v2/locations", &[], None)
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
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    let wrapped_negative = trimmed.starts_with('(') && trimmed.ends_with(')');
    let inner = if wrapped_negative {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    let normalized = inner.replace(['$', ','], "");
    let parsed = normalized.parse::<f64>().unwrap_or(0.0);

    if wrapped_negative {
        -parsed.abs()
    } else {
        parsed
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{fs, path::PathBuf};

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("read_api")
            .join(name)
    }

    fn read_fixture(name: &str) -> String {
        let path = fixture_path(name);
        fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading fixture {}: {err}", path.display()))
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn sample_insight_metrics() -> InsightMetrics {
        InsightMetrics {
            period_start: "2026-03-01T00:00:00Z".to_string(),
            period_end: "2026-03-01T23:59:59Z".to_string(),
            location_ids: vec![43101562, 43101563],
            gross_sales: 1200.0,
            net_sales: 1100.0,
            labor_hours: 44.0,
            labor_pay: 400.0,
            employee_count: 9,
            transaction_count: 80,
            settled_count: 75,
            settled_amount: 980.0,
            total_payment_amount: 1040.0,
            payment_type_mix: vec![PaymentMixBucket {
                key: "SALE".to_string(),
                count: 70,
                amount: 980.0,
                share_of_count: 87.5,
                share_of_amount: 94.2307,
            }],
            payment_tender_mix: vec![PaymentMixBucket {
                key: "VISA".to_string(),
                count: 60,
                amount: 800.0,
                share_of_count: 75.0,
                share_of_amount: 76.923,
            }],
        }
    }

    #[test]
    fn parse_query_accepts_key_value_pairs() {
        let parts = vec!["a=1".to_string(), "b=two".to_string()];
        let parsed = parse_query(&parts).expect("query should parse");
        assert_eq!(
            parsed,
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "two".to_string())
            ]
        );
    }

    #[test]
    fn parse_query_rejects_invalid_values() {
        let parts = vec!["missing_equals".to_string()];
        let err = parse_query(&parts).expect_err("invalid query should fail");
        match err {
            SkyTabError::InvalidArgument(message) => {
                assert!(message.contains("invalid --query value"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn detects_date_only_values() {
        assert!(is_date_only("2026-03-28"));
        assert!(!is_date_only("2026-03-28T00:00:00Z"));
        assert!(!is_date_only("20260328"));
    }

    #[test]
    fn payroll_transform_parses_fixture_rows() {
        let fixture = read_fixture("payroll_report_response.json");
        let response: PayrollByEmployeeResponse =
            serde_json::from_str(&fixture).expect("fixture should parse");

        let transformed = transform_payroll_response(response);

        assert_eq!(transformed.employees.len(), 1);
        assert_eq!(transformed.employees[0].employee_id, "100");
        assert_eq!(transformed.employees[0].employee_name, "Alice Smith");
        assert_close(transformed.employees[0].total_pay, 157.75);

        let totals = transformed.totals.expect("totals row should be detected");
        assert_eq!(totals.employee_id, "TOTAL");
        assert_close(totals.normal_hours, 8.0);
        assert_close(totals.overtime_hours, 1.0);
        assert_close(totals.total_pay, 157.75);
    }

    #[test]
    fn till_transform_parses_fixture_rows() {
        let fixture = read_fixture("till_transaction_report_response.json");
        let response: TillTransactionDetailResponse =
            serde_json::from_str(&fixture).expect("fixture should parse");

        let transformed = transform_till_transaction_response(response);

        assert_eq!(transformed.items.len(), 2);
        assert_eq!(transformed.items[0].employee_name, "Alice Smith");
        assert_close(transformed.items[0].amount, 50.0);
        assert_eq!(transformed.items[1].employee_name, "Bob Stone");
        assert_close(transformed.items[1].amount, -10.5);
    }

    #[test]
    fn timeclock_summary_aggregates_fixture_rows() {
        let fixture = read_fixture("timeclock_shifts.json");
        let shifts: Vec<Value> = serde_json::from_str(&fixture).expect("fixture should parse");

        let summary = summarize_timeclock_shifts(&shifts);

        assert_eq!(summary.shift_count, 3);
        assert_eq!(summary.open_shift_count, 1);
        assert_close(summary.total_hours, 11.25);
    }

    #[test]
    fn parse_currency_number_supports_negative_parentheses() {
        assert_close(parse_currency_number("($12.34)"), -12.34);
        assert_close(parse_currency_number("$1,234.50"), 1234.50);
    }

    #[test]
    fn payment_transaction_summary_builds_mix_rows() {
        let transactions = vec![
            json!({
                "type": "SALE",
                "status": "SETTLED",
                "totalAmount": "10.00",
                "paymentMethod": { "cardBrand": "VISA" }
            }),
            json!({
                "type": "SALE",
                "status": "SETTLED",
                "totalAmount": "5.00",
                "paymentMethod": { "cardBrand": "VISA" }
            }),
            json!({
                "type": "REFUND",
                "status": "PENDING",
                "totalAmount": "2.00",
                "paymentMethod": { "cardBrand": "MASTERCARD" }
            }),
        ];

        let summary = summarize_payment_transactions(&transactions);

        assert_eq!(summary.transaction_count, 3);
        assert_eq!(summary.settled_count, 2);
        assert_close(summary.settled_amount, 15.0);
        assert_close(summary.total_amount, 17.0);

        assert_eq!(summary.by_type.len(), 2);
        assert_eq!(summary.by_type[0].key, "SALE");
        assert_eq!(summary.by_type[0].count, 2);
        assert_close(summary.by_type[0].amount, 15.0);

        assert_eq!(summary.by_tender.len(), 2);
        assert_eq!(summary.by_tender[0].key, "VISA");
        assert_eq!(summary.by_tender[0].count, 2);
        assert_close(summary.by_tender[0].amount, 15.0);
    }

    #[test]
    fn end_of_day_insight_contains_all_three_views() {
        let metrics = sample_insight_metrics();

        let insight = build_end_of_day_insight(&metrics);

        assert_close(insight.daily_brief.net_sales, 1100.0);
        assert_eq!(insight.labor_vs_sales.employee_count, 9);
        assert_eq!(insight.payment_mix.transaction_count, 80);
        assert_eq!(insight.payment_mix.by_type[0].key, "SALE");
        assert_eq!(insight.daily_brief.location_ids, vec![43101562, 43101563]);
    }

    #[tokio::test]
    async fn request_get_requires_absolute_path() {
        let api = ReadApi::new(None);
        let err = api
            .request_get("api/v2/locations".to_string(), Vec::new())
            .await
            .expect_err("relative path should fail");

        match err {
            SkyTabError::InvalidArgument(message) => {
                assert!(message.contains("path must start"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn timeclock_shifts_rejects_zero_limit() {
        let api = ReadApi::new(None);
        let err = api
            .timeclock_shifts(
                None,
                "2026-03-01".to_string(),
                "2026-03-01".to_string(),
                "clockedInAt asc".to_string(),
                0,
            )
            .await
            .expect_err("zero limit should fail");

        match err {
            SkyTabError::InvalidArgument(message) => {
                assert!(message.contains("greater than zero"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
