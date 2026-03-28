use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::SkyTabError;
use crate::read_api::ReadApi;

pub const READ_ONLY_TOOL_NAMES: &[&str] = &[
    "skytab.accounts.preferences",
    "skytab.auth.login",
    "skytab.doctor",
    "skytab.insights.daily_brief",
    "skytab.insights.end_of_day",
    "skytab.insights.labor_vs_sales",
    "skytab.insights.payment_mix",
    "skytab.locations.list",
    "skytab.locations.show_default",
    "skytab.payments.transactions",
    "skytab.reports.activity_summary",
    "skytab.reports.discount_summary",
    "skytab.reports.hourly_sales",
    "skytab.reports.payroll",
    "skytab.reports.sales_summary_by_item",
    "skytab.reports.sales_summary_by_revenue_class",
    "skytab.reports.ticket_detail_closed",
    "skytab.reports.till_transaction",
    "skytab.request.get",
    "skytab.timeclock.shifts",
];

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AccountPreferencesArgs {
    #[schemars(description = "SkyTab account id")]
    account_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SingleLocationReportArgs {
    #[schemars(description = "Start datetime in RFC3339 or date-only YYYY-MM-DD")]
    start: String,
    #[schemars(description = "End datetime in RFC3339 or date-only YYYY-MM-DD")]
    end: String,
    #[schemars(description = "Location id. Falls back to default or sole account location")]
    location: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct MultiLocationReportArgs {
    #[schemars(description = "Start datetime in RFC3339 or date-only YYYY-MM-DD")]
    start: String,
    #[schemars(description = "End datetime in RFC3339 or date-only YYYY-MM-DD")]
    end: String,
    #[schemars(description = "Location ids. Falls back to default or sole account location")]
    #[serde(default)]
    location: Vec<i64>,
}

fn default_timeclock_order() -> String {
    "clockedInAt asc".to_string()
}

fn default_timeclock_limit() -> usize {
    100
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TimeclockShiftsArgs {
    #[schemars(description = "Location id. Falls back to default or sole account location")]
    location_id: Option<i64>,
    #[schemars(description = "Start datetime in RFC3339 or date-only YYYY-MM-DD")]
    start: String,
    #[schemars(description = "End datetime in RFC3339 or date-only YYYY-MM-DD")]
    end: String,
    #[schemars(description = "Sort order passed through to SkyTab API")]
    #[serde(default = "default_timeclock_order")]
    order: String,
    #[schemars(description = "Page size passed through to SkyTab API")]
    #[serde(default = "default_timeclock_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PaymentsTransactionsArgs {
    #[schemars(description = "Start datetime in RFC3339 or date-only YYYY-MM-DD")]
    start: String,
    #[schemars(description = "End datetime in RFC3339 or date-only YYYY-MM-DD")]
    end: String,
    #[schemars(description = "Location ids. Falls back to default or sole account location")]
    #[serde(default)]
    location: Vec<i64>,
    #[schemars(description = "Optional transaction type filter")]
    order_type: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct QueryPair {
    #[schemars(description = "Query parameter key")]
    key: String,
    #[schemars(description = "Query parameter value")]
    value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RequestGetArgs {
    #[schemars(description = "API path starting with /, for example /api/v2/locations")]
    path: String,
    #[schemars(description = "Query parameters")]
    #[serde(default)]
    query: Vec<QueryPair>,
}

#[derive(Clone)]
pub struct SkyTabMcpServer {
    api: ReadApi,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl SkyTabMcpServer {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            api: ReadApi::new(base_url),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "skytab.auth.login",
        description = "Authenticate using resolved SkyTab credentials"
    )]
    async fn auth_login(&self) -> std::result::Result<CallToolResult, McpError> {
        map_result(self.api.auth_login().await)
    }

    #[tool(
        name = "skytab.locations.list",
        description = "List locations for the authenticated SkyTab account"
    )]
    async fn locations_list(&self) -> std::result::Result<CallToolResult, McpError> {
        map_result(self.api.locations_list().await)
    }

    #[tool(
        name = "skytab.locations.show_default",
        description = "Show configured default location resolution"
    )]
    async fn locations_show_default(&self) -> std::result::Result<CallToolResult, McpError> {
        map_result(self.api.locations_show_default().await)
    }

    #[tool(
        name = "skytab.accounts.preferences",
        description = "Fetch account preferences by account id"
    )]
    async fn accounts_preferences(
        &self,
        Parameters(args): Parameters<AccountPreferencesArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(self.api.accounts_preferences(&args.account_id).await)
    }

    #[tool(
        name = "skytab.reports.activity_summary",
        description = "Run activity summary report"
    )]
    async fn reports_activity_summary(
        &self,
        Parameters(args): Parameters<SingleLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_activity_summary(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.reports.discount_summary",
        description = "Run discount summary report"
    )]
    async fn reports_discount_summary(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_discount_summary(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.reports.hourly_sales",
        description = "Run hourly sales report"
    )]
    async fn reports_hourly_sales(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_hourly_sales(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.reports.ticket_detail_closed",
        description = "Run ticket detail closed report"
    )]
    async fn reports_ticket_detail_closed(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_ticket_detail_closed(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.reports.sales_summary_by_item",
        description = "Run sales summary by item report"
    )]
    async fn reports_sales_summary_by_item(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_sales_summary_by_item(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.reports.sales_summary_by_revenue_class",
        description = "Run sales summary by revenue class report"
    )]
    async fn reports_sales_summary_by_revenue_class(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_sales_summary_by_revenue_class(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.reports.till_transaction",
        description = "Run till transaction report"
    )]
    async fn reports_till_transaction(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_till_transaction(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.reports.payroll",
        description = "Run payroll by employee report"
    )]
    async fn reports_payroll(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .report_payroll(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.insights.daily_brief",
        description = "Build a daily operations brief from sales, labor, and payments"
    )]
    async fn insights_daily_brief(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .insight_daily_brief(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.insights.end_of_day",
        description = "Build daily-brief, labor-vs-sales, and payment-mix in a single call"
    )]
    async fn insights_end_of_day(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .insight_end_of_day(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.insights.labor_vs_sales",
        description = "Compare labor cost and hours against sales"
    )]
    async fn insights_labor_vs_sales(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .insight_labor_vs_sales(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.insights.payment_mix",
        description = "Summarize payment mix by type and tender"
    )]
    async fn insights_payment_mix(
        &self,
        Parameters(args): Parameters<MultiLocationReportArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .insight_payment_mix(args.start, args.end, args.location)
                .await,
        )
    }

    #[tool(
        name = "skytab.timeclock.shifts",
        description = "List timeclock shifts with pagination aggregation"
    )]
    async fn timeclock_shifts(
        &self,
        Parameters(args): Parameters<TimeclockShiftsArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .timeclock_shifts(
                    args.location_id,
                    args.start,
                    args.end,
                    args.order,
                    args.limit,
                )
                .await,
        )
    }

    #[tool(
        name = "skytab.payments.transactions",
        description = "List internet payment transactions"
    )]
    async fn payments_transactions(
        &self,
        Parameters(args): Parameters<PaymentsTransactionsArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        map_result(
            self.api
                .payments_transactions(args.start, args.end, args.location, args.order_type)
                .await,
        )
    }

    #[tool(
        name = "skytab.request.get",
        description = "Execute a read-only GET request against SkyTab API"
    )]
    async fn request_get(
        &self,
        Parameters(args): Parameters<RequestGetArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let query = args.query.into_iter().map(|q| (q.key, q.value)).collect();
        map_result(self.api.request_get(args.path, query).await)
    }

    #[tool(
        name = "skytab.doctor",
        description = "Run local environment diagnostics"
    )]
    async fn doctor(&self) -> std::result::Result<CallToolResult, McpError> {
        map_result(self.api.doctor_report().await)
    }
}

#[tool_handler]
impl ServerHandler for SkyTabMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Read-only SkyTab MCP server exposing auth, locations, reports, insights, timeclock, payments, request.get, and doctor tools.".to_string(),
            )
    }
}

pub async fn serve_stdio(
    base_url: Option<String>,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = SkyTabMcpServer::new(base_url).serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}

fn map_result<T>(result: crate::error::Result<T>) -> std::result::Result<CallToolResult, McpError>
where
    T: serde::Serialize,
{
    match result {
        Ok(value) => {
            let value = serde_json::to_value(value).map_err(|err| {
                McpError::internal_error(
                    "failed to serialize tool result",
                    Some(json!({ "detail": err.to_string() })),
                )
            })?;
            Ok(CallToolResult::structured(value))
        }
        Err(err) => Ok(CallToolResult::structured_error(map_tool_error(err))),
    }
}

fn map_tool_error(err: SkyTabError) -> Value {
    match err {
        SkyTabError::MissingCredentials => json!({
            "kind": "missing_credentials",
            "message": "missing credentials; set SKYTAB_USERNAME/SKYTAB_PASSWORD or run `skytab auth set-credentials`"
        }),
        SkyTabError::MissingCredentialsForAuthRefresh => json!({
            "kind": "missing_credentials",
            "message": "cached auth token is missing or expired and credentials are unavailable; set SKYTAB_USERNAME/SKYTAB_PASSWORD or run `skytab auth set-credentials`"
        }),
        SkyTabError::PartialEnvCredentials => json!({
            "kind": "partial_env_credentials",
            "message": "set both SKYTAB_USERNAME and SKYTAB_PASSWORD"
        }),
        SkyTabError::PartialEnvCredentialsForAuthRefresh => json!({
            "kind": "partial_env_credentials",
            "message": "cached auth token is missing or expired and env credentials are incomplete; set both SKYTAB_USERNAME and SKYTAB_PASSWORD"
        }),
        SkyTabError::CredentialStore(message) => json!({
            "kind": "credential_store_error",
            "message": message
        }),
        SkyTabError::CredentialStoreForAuthRefresh(message) => json!({
            "kind": "credential_store_error",
            "message": format!(
                "cached auth token is missing or expired and credential store lookup failed: {}",
                message
            )
        }),
        SkyTabError::InvalidArgument(message) => json!({
            "kind": "invalid_argument",
            "message": message
        }),
        SkyTabError::Api { status, .. } => json!({
            "kind": "api_error",
            "message": "SkyTab API request failed",
            "status": status,
            "retryable": status == 429 || (500..=599).contains(&status)
        }),
        SkyTabError::Io(err) => json!({
            "kind": "io_error",
            "message": err.to_string()
        }),
        SkyTabError::Http(err) => json!({
            "kind": "http_error",
            "message": err.to_string()
        }),
        SkyTabError::Json(err) => json!({
            "kind": "json_error",
            "message": err.to_string()
        }),
        SkyTabError::TomlDe(err) => json!({
            "kind": "toml_decode_error",
            "message": err.to_string()
        }),
        SkyTabError::TomlSer(err) => json!({
            "kind": "toml_encode_error",
            "message": err.to_string()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_all_read_only_tools() {
        let server = SkyTabMcpServer::new(None);
        let tools = server.tool_router.list_all();
        let names = tools
            .iter()
            .map(|tool| tool.name.as_ref())
            .collect::<Vec<_>>();

        assert_eq!(names, READ_ONLY_TOOL_NAMES.to_vec());
    }

    #[test]
    fn request_get_tool_schema_includes_path() {
        let server = SkyTabMcpServer::new(None);
        let tool = server
            .tool_router
            .get("skytab.request.get")
            .expect("request.get tool should be registered");

        let path_type = tool
            .input_schema
            .get("properties")
            .and_then(|properties| properties.get("path"))
            .and_then(|path| path.get("type"))
            .and_then(Value::as_str)
            .expect("path type should be present");

        assert_eq!(path_type, "string");
    }

    #[test]
    fn insights_daily_brief_tool_schema_includes_start_and_end() {
        let server = SkyTabMcpServer::new(None);
        let tool = server
            .tool_router
            .get("skytab.insights.daily_brief")
            .expect("insights.daily_brief tool should be registered");

        let start_type = tool
            .input_schema
            .get("properties")
            .and_then(|properties| properties.get("start"))
            .and_then(|start| start.get("type"))
            .and_then(Value::as_str)
            .expect("start type should be present");
        let end_type = tool
            .input_schema
            .get("properties")
            .and_then(|properties| properties.get("end"))
            .and_then(|end| end.get("type"))
            .and_then(Value::as_str)
            .expect("end type should be present");

        assert_eq!(start_type, "string");
        assert_eq!(end_type, "string");
    }

    #[test]
    fn insights_end_of_day_tool_schema_includes_start_and_end() {
        let server = SkyTabMcpServer::new(None);
        let tool = server
            .tool_router
            .get("skytab.insights.end_of_day")
            .expect("insights.end_of_day tool should be registered");

        let start_type = tool
            .input_schema
            .get("properties")
            .and_then(|properties| properties.get("start"))
            .and_then(|start| start.get("type"))
            .and_then(Value::as_str)
            .expect("start type should be present");
        let end_type = tool
            .input_schema
            .get("properties")
            .and_then(|properties| properties.get("end"))
            .and_then(|end| end.get("type"))
            .and_then(Value::as_str)
            .expect("end type should be present");

        assert_eq!(start_type, "string");
        assert_eq!(end_type, "string");
    }
}
