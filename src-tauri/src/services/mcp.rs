use std::{
    collections::{BTreeMap, HashMap},
    net::{TcpStream, ToSocketAddrs},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use chrono::Utc;
use url::Url;
use zeroclaw::{
    config::{McpServerConfig, McpTransport},
    tools::McpRegistry,
};

use crate::{
    db,
    models::mcp::{
        McpServerDraft, McpServerRecord, McpServerTestReport, McpServerTestResult,
        McpServerToolsResult, McpToolRecord,
    },
    state::AppState,
};

const STDIO_BOOT_TIMEOUT_MS: u64 = 800;
const TCP_CONNECT_TIMEOUT_MS: u64 = 2_500;

pub fn list_servers(state: &AppState) -> Result<Vec<McpServerRecord>, String> {
    db::list_mcp_servers(&state.db_path())
}

pub fn create_server(state: &AppState, draft: McpServerDraft) -> Result<McpServerRecord, String> {
    let draft = normalize_draft(draft)?;
    db::create_mcp_server(
        &state.db_path(),
        &draft.name,
        &draft.transport,
        &draft.command,
        &draft.arguments_json,
        &draft.url,
        &draft.headers_json,
        &draft.environment_json,
        draft.enabled,
    )
}

pub fn update_server(
    state: &AppState,
    server_id: &str,
    draft: McpServerDraft,
) -> Result<McpServerRecord, String> {
    let draft = normalize_draft(draft)?;
    db::update_mcp_server(
        &state.db_path(),
        server_id,
        &draft.name,
        &draft.transport,
        &draft.command,
        &draft.arguments_json,
        &draft.url,
        &draft.headers_json,
        &draft.environment_json,
        draft.enabled,
    )
}

pub fn delete_server(state: &AppState, server_id: &str) -> Result<(), String> {
    db::delete_mcp_server(&state.db_path(), server_id)
}

pub fn test_server(state: &AppState, server_id: &str) -> Result<McpServerTestResult, String> {
    let server = db::get_mcp_server(&state.db_path(), server_id)?
        .ok_or_else(|| "MCP server not found.".to_string())?;

    let report = if server.transport == "stdio" {
        test_stdio_server(&server)
    } else {
        test_remote_server(&server)
    }?;

    let status = if report.ok { "success" } else { "error" };
    let server = db::record_mcp_server_test_result(
        &state.db_path(),
        &server.id,
        status,
        &report.message,
        &report.checked_at,
    )?;

    Ok(McpServerTestResult { server, report })
}

pub async fn discover_server_tools(
    state: &AppState,
    server_id: &str,
) -> Result<McpServerToolsResult, String> {
    let server = db::get_mcp_server(&state.db_path(), server_id)?
        .ok_or_else(|| "MCP server not found.".to_string())?;
    let config = map_server_to_config(&server)?;
    let registry = McpRegistry::connect_all(&[config])
        .await
        .map_err(|error| error.to_string())?;
    let mut tool_names = registry.tool_names();
    tool_names.sort();

    let mut tools = Vec::new();
    for full_name in tool_names {
        if let Some(definition) = registry.get_tool_def(&full_name).await {
            let tool_name = full_name
                .split_once("__")
                .map(|(_, suffix)| suffix.to_string())
                .unwrap_or_else(|| full_name.clone());
            let description = definition
                .description
                .unwrap_or_else(|| "MCP tool".to_string());
            let input_schema_json = serde_json::to_string_pretty(&definition.input_schema)
                .map_err(|error| error.to_string())?;

            tools.push(McpToolRecord {
                full_name,
                tool_name,
                server_name: server.name.clone(),
                description,
                input_schema_json,
            });
        }
    }

    Ok(McpServerToolsResult {
        server,
        tools,
        discovered_at: Utc::now().timestamp_millis().to_string(),
    })
}

fn normalize_draft(mut draft: McpServerDraft) -> Result<McpServerDraft, String> {
    draft.name = draft.name.trim().to_string();
    if draft.name.is_empty() {
        return Err("MCP server name is required.".to_string());
    }

    draft.transport = normalize_transport(&draft.transport)?;
    draft.command = draft.command.trim().to_string();
    draft.arguments_json = normalize_string_list_json(&draft.arguments_json, "Arguments")?;
    draft.headers_json = normalize_string_map_json(&draft.headers_json, "Headers")?;
    draft.environment_json = normalize_string_map_json(&draft.environment_json, "Environment")?;
    draft.url = draft.url.trim().to_string();

    match draft.transport.as_str() {
        "stdio" => {
            if draft.command.is_empty() {
                return Err("A command is required for stdio MCP servers.".to_string());
            }
            draft.url.clear();
            draft.headers_json = "{}".to_string();
        }
        "sse" | "streamable_http" => {
            if draft.url.is_empty() {
                return Err("A URL is required for remote MCP servers.".to_string());
            }
            let parsed =
                Url::parse(&draft.url).map_err(|error| format!("Invalid MCP URL: {error}"))?;
            match parsed.scheme() {
                "http" | "https" => {}
                _ => return Err("MCP URLs must start with http:// or https://".to_string()),
            }
            draft.command.clear();
            draft.arguments_json = "[]".to_string();
            draft.environment_json = "{}".to_string();
        }
        _ => return Err("Unsupported MCP transport.".to_string()),
    }

    Ok(draft)
}

fn normalize_transport(value: &str) -> Result<String, String> {
    match value.trim().to_lowercase().as_str() {
        "stdio" => Ok("stdio".to_string()),
        "sse" => Ok("sse".to_string()),
        "streamable_http" | "streamable-http" | "http" => Ok("streamable_http".to_string()),
        _ => Err("MCP transport must be stdio, sse, or streamable_http.".to_string()),
    }
}

fn normalize_string_list_json(raw: &str, label: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok("[]".to_string());
    }

    let parsed = serde_json::from_str::<Vec<String>>(value)
        .map_err(|error| format!("{label} must be a JSON string array: {error}"))?;
    serde_json::to_string_pretty(&parsed).map_err(|error| error.to_string())
}

fn normalize_string_map_json(raw: &str, label: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok("{}".to_string());
    }

    let parsed = serde_json::from_str::<BTreeMap<String, String>>(value)
        .map_err(|error| format!("{label} must be a JSON object of string values: {error}"))?;
    serde_json::to_string_pretty(&parsed).map_err(|error| error.to_string())
}

fn map_server_to_config(server: &McpServerRecord) -> Result<McpServerConfig, String> {
    let args = serde_json::from_str::<Vec<String>>(&server.arguments_json)
        .map_err(|error| format!("Invalid MCP arguments JSON: {error}"))?;
    let headers = serde_json::from_str::<HashMap<String, String>>(&server.headers_json)
        .map_err(|error| format!("Invalid MCP headers JSON: {error}"))?;
    let env = serde_json::from_str::<HashMap<String, String>>(&server.environment_json)
        .map_err(|error| format!("Invalid MCP environment JSON: {error}"))?;

    let transport = match server.transport.as_str() {
        "stdio" => McpTransport::Stdio,
        "sse" => McpTransport::Sse,
        "streamable_http" => McpTransport::Http,
        _ => return Err("Unsupported MCP transport.".to_string()),
    };

    Ok(McpServerConfig {
        name: server.name.clone(),
        transport,
        url: if server.url.trim().is_empty() {
            None
        } else {
            Some(server.url.clone())
        },
        command: server.command.clone(),
        args,
        env,
        headers,
        tool_timeout_secs: None,
    })
}

fn test_stdio_server(server: &McpServerRecord) -> Result<McpServerTestReport, String> {
    let arguments = serde_json::from_str::<Vec<String>>(&server.arguments_json)
        .map_err(|error| format!("Invalid MCP arguments JSON: {error}"))?;
    let environment = serde_json::from_str::<BTreeMap<String, String>>(&server.environment_json)
        .map_err(|error| format!("Invalid MCP environment JSON: {error}"))?;

    let checked_at = Utc::now().timestamp_millis().to_string();
    let mut child = Command::new(&server.command)
        .args(&arguments)
        .envs(
            environment
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start MCP server process: {error}"))?;

    thread::sleep(Duration::from_millis(STDIO_BOOT_TIMEOUT_MS));

    if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
        let output = child
            .wait_with_output()
            .map_err(|error| error.to_string())?;
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("Process exited before the MCP client could attach (status: {status}).")
        } else {
            format!("Process exited early: {stderr}")
        };

        return Ok(McpServerTestReport {
            ok: false,
            transport: server.transport.clone(),
            message,
            details: Some(format!(
                "command={} args={}",
                server.command, server.arguments_json
            )),
            checked_at,
        });
    }

    let _ = child.kill();
    let _ = child.wait();

    Ok(McpServerTestReport {
        ok: true,
        transport: server.transport.clone(),
        message: "The stdio MCP server started successfully and stayed alive during the handshake window.".to_string(),
        details: Some(format!("command={} args={}", server.command, server.arguments_json)),
        checked_at,
    })
}

fn test_remote_server(server: &McpServerRecord) -> Result<McpServerTestReport, String> {
    let parsed = Url::parse(&server.url).map_err(|error| format!("Invalid MCP URL: {error}"))?;
    let _headers = serde_json::from_str::<BTreeMap<String, String>>(&server.headers_json)
        .map_err(|error| format!("Invalid MCP headers JSON: {error}"))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| "The MCP URL is missing a host.".to_string())?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "The MCP URL is missing a usable port.".to_string())?;
    let checked_at = Utc::now().timestamp_millis().to_string();

    let address = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("Failed to resolve MCP host: {error}"))?
        .next()
        .ok_or_else(|| "Failed to resolve a socket address for the MCP host.".to_string())?;

    match TcpStream::connect_timeout(&address, Duration::from_millis(TCP_CONNECT_TIMEOUT_MS)) {
        Ok(_) => Ok(McpServerTestReport {
            ok: true,
            transport: server.transport.clone(),
            message: format!("Connected to {} over TCP.", server.url),
            details: Some(format!("host={} port={}", host, port)),
            checked_at,
        }),
        Err(error) => Ok(McpServerTestReport {
            ok: false,
            transport: server.transport.clone(),
            message: format!("Failed to connect to {}: {}", server.url, error),
            details: Some(format!("host={} port={}", host, port)),
            checked_at,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{models::mcp::McpServerDraft, state::AppState};

    fn make_test_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("{prefix}-{stamp}"));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn mock_server_script_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("scripts")
            .join("mock_mcp_stdio.mjs")
            .display()
            .to_string()
    }

    #[tokio::test]
    async fn discovers_mock_server_tools() {
        let app_dir = make_test_dir("zeroclawx-mcp-discovery");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");
        let server = super::create_server(
            &state,
            McpServerDraft {
                name: "mock-weather".to_string(),
                transport: "stdio".to_string(),
                command: "node".to_string(),
                arguments_json: serde_json::to_string(&vec![mock_server_script_path()]).unwrap(),
                url: String::new(),
                headers_json: "{}".to_string(),
                environment_json: "{}".to_string(),
                enabled: true,
            },
        )
        .expect("mock server should be created");

        let discovery = super::discover_server_tools(&state, &server.id)
            .await
            .expect("tool discovery should succeed");

        assert_eq!(discovery.tools.len(), 1);
        assert_eq!(discovery.tools[0].full_name, "mock-weather__echo_weather");
        assert_eq!(discovery.tools[0].tool_name, "echo_weather");
        assert!(discovery.tools[0].description.contains("weather"));
        assert!(discovery.tools[0].input_schema_json.contains("city"));

        let _ = fs::remove_dir_all(app_dir);
    }
}
