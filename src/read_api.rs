use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

use crate::cache::TokenCache;
use crate::client::SkyTabClient;
use crate::config::{
    Config, current_config_file_path, get_default_location_id, legacy_config_file_path,
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

    pub async fn doctor_report(&self) -> Result<DoctorReport> {
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
    let normalized = input.replace(['$', ','], "");
    normalized.parse::<f64>().unwrap_or(0.0)
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
