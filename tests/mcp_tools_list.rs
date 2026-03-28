use rmcp::{
    ServiceExt,
    model::{CallToolResult, ClientJsonRpcMessage, ServerJsonRpcMessage, ServerResult},
    transport::{IntoTransport, Transport},
};
use skytab_cli::mcp_server::{READ_ONLY_TOOL_NAMES, SkyTabMcpServer};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct ScopedEnvVar {
    key: &'static str,
    previous: Option<String>,
}

impl ScopedEnvVar {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, previous }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

fn msg(raw: &str) -> ClientJsonRpcMessage {
    serde_json::from_str(raw).expect("invalid test message JSON")
}

fn init_request() -> ClientJsonRpcMessage {
    msg(r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "0.0.1" }
        }
    }"#)
}

fn initialized_notification() -> ClientJsonRpcMessage {
    msg(r#"{ "jsonrpc": "2.0", "method": "notifications/initialized" }"#)
}

fn list_tools_request(id: u64) -> ClientJsonRpcMessage {
    msg(&format!(
        r#"{{ "jsonrpc": "2.0", "id": {id}, "method": "tools/list" }}"#
    ))
}

fn call_tool_request(
    id: u64,
    tool_name: &str,
    arguments: serde_json::Value,
) -> ClientJsonRpcMessage {
    msg(&format!(
        r#"{{ "jsonrpc": "2.0", "id": {id}, "method": "tools/call", "params": {{ "name": "{tool_name}", "arguments": {arguments} }} }}"#
    ))
}

async fn start_initialized_client() -> impl Transport<rmcp::RoleClient> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let _server = tokio::spawn(async move {
        let server = SkyTabMcpServer::new(None)
            .serve(server_transport)
            .await
            .expect("server should start");
        server.waiting().await.expect("server should run");
    });
    let mut client = IntoTransport::<rmcp::RoleClient, _, _>::into_transport(client_transport);

    client.send(init_request()).await.expect("send initialize");
    let _init_response = client.receive().await.expect("receive initialize response");

    client
        .send(initialized_notification())
        .await
        .expect("send initialized notification");

    client
}

fn expect_call_tool_result(message: ServerJsonRpcMessage) -> CallToolResult {
    match message {
        ServerJsonRpcMessage::Response(response) => match response.result {
            ServerResult::CallToolResult(result) => result,
            other => panic!("expected CallToolResult, got: {other:?}"),
        },
        other => panic!("expected response message, got: {other:?}"),
    }
}

#[tokio::test]
async fn tools_list_exposes_all_read_only_tools() {
    let mut client = start_initialized_client().await;

    client
        .send(list_tools_request(2))
        .await
        .expect("send tools/list");

    let response = client.receive().await.expect("receive tools/list response");
    let tool_names = match response {
        ServerJsonRpcMessage::Response(response) => match response.result {
            ServerResult::ListToolsResult(list) => list
                .tools
                .into_iter()
                .map(|tool| tool.name.to_string())
                .collect::<Vec<_>>(),
            other => panic!("expected ListToolsResult, got: {other:?}"),
        },
        other => panic!("expected response message, got: {other:?}"),
    };

    assert_eq!(
        tool_names,
        READ_ONLY_TOOL_NAMES
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn request_get_returns_structured_error_for_invalid_path() {
    let mut client = start_initialized_client().await;
    let args = serde_json::json!({
        "path": "api/v2/locations",
        "query": []
    });

    client
        .send(call_tool_request(3, "skytab.request.get", args))
        .await
        .expect("send tools/call request");

    let result =
        expect_call_tool_result(client.receive().await.expect("receive tools/call response"));

    assert_eq!(result.is_error, Some(true));
    let structured = result
        .structured_content
        .expect("expected structured error payload");
    assert_eq!(structured["kind"], "invalid_argument");
    assert_eq!(structured["message"], "path must start with '/'");
}

#[tokio::test]
async fn timeclock_shifts_returns_structured_error_for_zero_limit() {
    let mut client = start_initialized_client().await;
    let args = serde_json::json!({
        "start": "2026-03-01",
        "end": "2026-03-01",
        "order": "clockedInAt asc",
        "limit": 0
    });

    client
        .send(call_tool_request(4, "skytab.timeclock.shifts", args))
        .await
        .expect("send tools/call request");

    let result =
        expect_call_tool_result(client.receive().await.expect("receive tools/call response"));

    assert_eq!(result.is_error, Some(true));
    let structured = result
        .structured_content
        .expect("expected structured error payload");
    assert_eq!(structured["kind"], "invalid_argument");
    assert_eq!(structured["message"], "limit must be greater than zero");
}

#[tokio::test]
async fn auth_login_partial_env_error_is_contextual_and_compatible() {
    let _env_lock = ENV_LOCK.lock().expect("env lock should be available");
    let _username = ScopedEnvVar::set("SKYTAB_USERNAME", Some("partial@example.com"));
    let _password = ScopedEnvVar::set("SKYTAB_PASSWORD", None);

    let mut client = start_initialized_client().await;

    client
        .send(call_tool_request(
            5,
            "skytab.auth.login",
            serde_json::json!({}),
        ))
        .await
        .expect("send tools/call request");

    let result =
        expect_call_tool_result(client.receive().await.expect("receive tools/call response"));

    assert_eq!(result.is_error, Some(true));
    let structured = result
        .structured_content
        .expect("expected structured error payload");
    assert_eq!(structured["kind"], "partial_env_credentials");
    assert_eq!(
        structured["message"],
        "cached auth token is missing or expired and env credentials are incomplete; set both SKYTAB_USERNAME and SKYTAB_PASSWORD"
    );
}
