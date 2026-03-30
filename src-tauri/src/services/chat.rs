use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result as AnyhowResult};
use async_trait::async_trait;
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use zeroclaw::{
    agent::{
        dispatcher::{NativeToolDispatcher, ToolDispatcher, XmlToolDispatcher},
        Agent,
    },
    memory::{self, Memory},
    observability::{self, Observer},
    providers::{traits::StreamOptions, ChatMessage, Provider},
    tools::{McpRegistry, McpToolWrapper, Tool, ToolResult},
};

use crate::{
    db,
    models::{
        chat::{
            ChatApprovalDecision, ChatApprovalRequestPayload, ChatContextPayload, ChatDonePayload,
            ChatTokenPayload, MessageRecord,
        },
        knowledge::KnowledgeDocumentRecord,
    },
    state::AppState,
};

const FALLBACK_CHUNK_DELAY_MS: u64 = 40;
const FALLBACK_CHUNK_TARGET_CHARS: usize = 24;
const MAX_KNOWLEDGE_DOCS_IN_CONTEXT: usize = 3;
const MAX_DOC_CHARS_IN_CONTEXT: usize = 700;
const MAX_TOTAL_CONTEXT_CHARS: usize = 2800;
const EXCERPT_RADIUS_CHARS: usize = 240;
const BUILTIN_AGENT_TOOLS: &[&str] = &[
    "shell",
    "file_read",
    "file_write",
    "file_edit",
    "glob_search",
    "content_search",
];

#[derive(Clone)]
struct ProjectContextSelection {
    system_context: String,
    project_name: String,
    scope_mode: String,
    knowledge_titles: Vec<String>,
}

struct ApprovalWrappingTool {
    inner: Box<dyn Tool>,
    app: AppHandle,
    state: AppState,
    session_id: String,
    skip_default_approval: bool,
    auto_approve: HashSet<String>,
    always_ask: HashSet<String>,
}

impl ApprovalWrappingTool {
    fn new(
        inner: Box<dyn Tool>,
        app: AppHandle,
        state: AppState,
        session_id: String,
        skip_default_approval: bool,
        auto_approve: HashSet<String>,
        always_ask: HashSet<String>,
    ) -> Self {
        Self {
            inner,
            app,
            state,
            session_id,
            skip_default_approval,
            auto_approve,
            always_ask,
        }
    }

    fn should_request_approval(&self, tool_name: &str) -> bool {
        if self.skip_default_approval {
            return false;
        }

        if self.always_ask.contains(tool_name) {
            return true;
        }

        if self.auto_approve.contains(tool_name) {
            return false;
        }

        if self
            .state
            .is_tool_allowed_for_session(&self.session_id, tool_name)
        {
            return false;
        }

        true
    }
}

#[async_trait]
impl Tool for ApprovalWrappingTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(&self, args: serde_json::Value) -> AnyhowResult<ToolResult> {
        let tool_name = self.inner.name().to_string();
        let mut effective_args = args;

        if self.should_request_approval(&tool_name) {
            let summary = summarize_args(&effective_args);
            let (payload, receiver) = self
                .state
                .register_approval_request(&self.session_id, &tool_name, &summary)
                .map_err(anyhow::Error::msg)?;

            emit_approval_request(&self.app, &payload).map_err(anyhow::Error::msg)?;
            let decision = receiver.await.unwrap_or(ChatApprovalDecision::No);

            if decision == ChatApprovalDecision::No {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Denied by user.".to_string()),
                });
            }

            if tool_name == "shell" {
                effective_args["approved"] = serde_json::Value::Bool(true);
            }
        } else if tool_name == "shell" {
            effective_args["approved"] = serde_json::Value::Bool(true);
        }

        self.inner.execute(effective_args).await
    }
}

pub async fn stream_response(
    app: AppHandle,
    state: AppState,
    session_id: String,
    content: String,
    agent_mode: bool,
) -> Result<(), String> {
    if agent_mode {
        return stream_agent_response(app, state, session_id, content).await;
    }

    stream_provider_response(app, state, session_id, content).await
}

async fn stream_provider_response(
    app: AppHandle,
    state: AppState,
    session_id: String,
    content: String,
) -> Result<(), String> {
    let db_path = state.db_path();
    let settings_path = state.settings_path();
    let records = db::list_messages(&db_path, &session_id)?;
    let (history, context_preview) =
        build_session_history(&db_path, &session_id, &content, &records)?;
    let runtime = super::runtime::build_runtime_session(&db_path, &settings_path)?;

    if let Some(context_preview) = context_preview {
        emit_context(
            &app,
            &ChatContextPayload {
                session_id: session_id.clone(),
                project_name: context_preview.project_name,
                scope_mode: context_preview.scope_mode,
                knowledge_titles: context_preview.knowledge_titles,
            },
        )?;
    }

    let response = if runtime.provider.supports_streaming() {
        let mut output = String::new();
        let mut stream = runtime.provider.stream_chat_with_history(
            &history,
            &runtime.model,
            runtime.temperature,
            StreamOptions::new(true),
        );

        while let Some(chunk) = stream.next().await {
            if state.take_cancellation(&session_id) {
                emit_done(&app, &session_id)?;
                return Ok(());
            }

            let chunk = chunk.map_err(super::runtime::sanitize_runtime_error)?;

            if !chunk.delta.is_empty() {
                output.push_str(&chunk.delta);
                emit_token(&app, &session_id, &chunk.delta)?;
            }

            if chunk.is_final {
                break;
            }
        }

        if output.trim().is_empty() {
            runtime
                .provider
                .chat_with_history(&history, &runtime.model, runtime.temperature)
                .await
                .map_err(super::runtime::sanitize_runtime_error)?
        } else {
            output
        }
    } else {
        let response = runtime
            .provider
            .chat_with_history(&history, &runtime.model, runtime.temperature)
            .await
            .map_err(super::runtime::sanitize_runtime_error)?;

        relay_fallback_chunks(&app, &state, &session_id, &response).await?;
        response
    };

    db::record_message(&db_path, &session_id, "assistant", &response)?;
    emit_done(&app, &session_id)?;

    Ok(())
}

async fn stream_agent_response(
    app: AppHandle,
    state: AppState,
    session_id: String,
    content: String,
) -> Result<(), String> {
    let db_path = state.db_path();
    let settings_path = state.settings_path();
    let settings = super::runtime::load_runtime_settings(&settings_path)?;
    let config = super::runtime::build_resolved_runtime_config(&db_path, settings)?;
    let records = db::list_messages(&db_path, &session_id)?;
    let (history_seed, effective_user_input, context_preview) =
        build_agent_turn_input(&db_path, &session_id, &content, &records)?;
    let runtime_session = super::runtime::build_runtime_session(&db_path, &settings_path)?;
    let mut agent = build_agent_for_session(
        &app,
        &state,
        &session_id,
        &config,
        runtime_session.provider,
        runtime_session.model.clone(),
        runtime_session.temperature,
    )
    .await?;

    agent.seed_history(&history_seed);

    if let Some(context_preview) = context_preview {
        emit_context(
            &app,
            &ChatContextPayload {
                session_id: session_id.clone(),
                project_name: context_preview.project_name,
                scope_mode: context_preview.scope_mode,
                knowledge_titles: context_preview.knowledge_titles,
            },
        )?;
    }

    let response = agent
        .turn(&effective_user_input)
        .await
        .map_err(super::runtime::sanitize_runtime_error)?;

    relay_fallback_chunks(&app, &state, &session_id, &response).await?;
    db::record_message(&db_path, &session_id, "assistant", &response)?;
    emit_done(&app, &session_id)?;

    Ok(())
}

async fn build_agent_for_session(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    config: &zeroclaw::Config,
    provider: Box<dyn Provider>,
    model_name: String,
    temperature: f64,
) -> Result<Agent, String> {
    let observer: Arc<dyn Observer> =
        Arc::from(observability::create_observer(&config.observability));
    let memory: Arc<dyn Memory> = Arc::from(
        memory::create_memory_with_storage_and_routes(
            &config.memory,
            &config.embedding_routes,
            Some(&config.storage.provider.config),
            &config.workspace_dir,
            config.api_key.as_deref(),
        )
        .map_err(|error| error.to_string())?,
    );
    let (session_tools, allowed_tool_names) =
        build_session_tools(&config.workspace_dir, &config.mcp).await?;
    let wrapped_tools = wrap_tools_for_session(
        session_tools,
        app.clone(),
        state.clone(),
        session_id,
        &config.autonomy,
    );
    let tool_dispatcher = select_tool_dispatcher(config, provider.as_ref());

    Agent::builder()
        .provider(provider)
        .tools(wrapped_tools)
        .memory(memory)
        .observer(observer)
        .tool_dispatcher(tool_dispatcher)
        .config(config.agent.clone())
        .model_name(model_name)
        .temperature(temperature)
        .workspace_dir(config.workspace_dir.clone())
        .classification_config(config.query_classification.clone())
        .identity_config(config.identity.clone())
        .skills_prompt_mode(config.skills.prompt_injection_mode)
        .auto_save(config.memory.auto_save)
        .allowed_tools(Some(allowed_tool_names))
        .build()
        .map_err(|error| error.to_string())
}

fn select_tool_dispatcher(
    config: &zeroclaw::Config,
    provider: &dyn Provider,
) -> Box<dyn ToolDispatcher> {
    match config.agent.tool_dispatcher.as_str() {
        "native" => Box::new(NativeToolDispatcher),
        "xml" => Box::new(XmlToolDispatcher),
        _ if provider.supports_native_tools() => Box::new(NativeToolDispatcher),
        _ => Box::new(XmlToolDispatcher),
    }
}

async fn build_session_tools(
    workspace_dir: &Path,
    mcp_config: &zeroclaw::config::McpConfig,
) -> Result<(Vec<Box<dyn Tool>>, Vec<String>), String> {
    let mut tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ShellExecutionTool::new(workspace_dir.to_path_buf())),
        Box::new(FileReadWorkspaceTool::new(workspace_dir.to_path_buf())),
        Box::new(FileWriteWorkspaceTool::new(workspace_dir.to_path_buf())),
        Box::new(FileEditWorkspaceTool::new(workspace_dir.to_path_buf())),
        Box::new(GlobSearchWorkspaceTool::new(workspace_dir.to_path_buf())),
        Box::new(ContentSearchWorkspaceTool::new(workspace_dir.to_path_buf())),
    ];
    let mut allowed_tool_names = BUILTIN_AGENT_TOOLS
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();

    if mcp_config.enabled && !mcp_config.servers.is_empty() {
        let registry = Arc::new(
            McpRegistry::connect_all(&mcp_config.servers)
                .await
                .map_err(|error| error.to_string())?,
        );
        let mut tool_names = registry.tool_names();
        tool_names.sort();

        for tool_name in tool_names {
            if let Some(definition) = registry.get_tool_def(&tool_name).await {
                tools.push(Box::new(McpToolWrapper::new(
                    tool_name.clone(),
                    definition,
                    Arc::clone(&registry),
                )));
                allowed_tool_names.push(tool_name);
            }
        }
    }

    Ok((tools, allowed_tool_names))
}

fn wrap_tools_for_session(
    tools_registry: Vec<Box<dyn Tool>>,
    app: AppHandle,
    state: AppState,
    session_id: &str,
    autonomy: &zeroclaw::config::AutonomyConfig,
) -> Vec<Box<dyn Tool>> {
    let auto_approve = autonomy
        .auto_approve
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let always_ask = autonomy.always_ask.iter().cloned().collect::<HashSet<_>>();

    tools_registry
        .into_iter()
        .map(|tool| {
            Box::new(ApprovalWrappingTool::new(
                tool,
                app.clone(),
                state.clone(),
                session_id.to_string(),
                autonomy_level_skips_default_approval(autonomy),
                auto_approve.clone(),
                always_ask.clone(),
            )) as Box<dyn Tool>
        })
        .collect()
}

struct ShellExecutionTool {
    workspace_dir: PathBuf,
}

impl ShellExecutionTool {
    fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for ShellExecutionTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Run a shell command inside the workspace for local builds, tests, diagnostics, and git operations."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "approved": { "type": "boolean" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> AnyhowResult<ToolResult> {
        let command = required_string_arg(&args, "command")?;
        let output = if cfg!(target_os = "windows") {
            Command::new("powershell")
                .args(["-NoLogo", "-NoProfile", "-Command", &command])
                .current_dir(&self.workspace_dir)
                .output()?
        } else {
            Command::new("sh")
                .args(["-lc", &command])
                .current_dir(&self.workspace_dir)
                .output()?
        };

        let rendered = render_command_output(&output.stdout, &output.stderr);

        Ok(ToolResult {
            success: output.status.success(),
            output: truncate_chars(&rendered, 12_000),
            error: if output.status.success() {
                None
            } else {
                Some(format!(
                    "Process exited with status {:?}.",
                    output.status.code()
                ))
            },
        })
    }
}

struct FileReadWorkspaceTool {
    workspace_dir: PathBuf,
}

impl FileReadWorkspaceTool {
    fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FileReadWorkspaceTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read a UTF-8 or text-like file from the workspace."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "max_chars": { "type": "integer", "minimum": 1 }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> AnyhowResult<ToolResult> {
        let path =
            resolve_workspace_path(&self.workspace_dir, &required_string_arg(&args, "path")?)?;
        let bytes = fs::read(&path)?;
        let content = String::from_utf8_lossy(&bytes).into_owned();
        let max_chars = optional_u64_arg(&args, "max_chars").unwrap_or(8_000) as usize;
        let relative = display_workspace_path(&self.workspace_dir, &path);

        Ok(ToolResult {
            success: true,
            output: format!("[{}]\n{}", relative, truncate_chars(&content, max_chars)),
            error: None,
        })
    }
}

struct FileWriteWorkspaceTool {
    workspace_dir: PathBuf,
}

impl FileWriteWorkspaceTool {
    fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FileWriteWorkspaceTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write or append text content to a workspace file."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
                "append": { "type": "boolean" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> AnyhowResult<ToolResult> {
        let path =
            resolve_workspace_path(&self.workspace_dir, &required_string_arg(&args, "path")?)?;
        let content = required_string_arg(&args, "content")?;
        let append = optional_bool_arg(&args, "append").unwrap_or(false);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        if append {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            file.write_all(content.as_bytes())?;
        } else {
            fs::write(&path, content.as_bytes())?;
        }

        Ok(ToolResult {
            success: true,
            output: format!(
                "Updated {}",
                display_workspace_path(&self.workspace_dir, &path)
            ),
            error: None,
        })
    }
}

struct FileEditWorkspaceTool {
    workspace_dir: PathBuf,
}

impl FileEditWorkspaceTool {
    fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FileEditWorkspaceTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Replace text inside a workspace file."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "find": { "type": "string" },
                "replace": { "type": "string" },
                "replace_all": { "type": "boolean" }
            },
            "required": ["path", "find", "replace"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> AnyhowResult<ToolResult> {
        let path =
            resolve_workspace_path(&self.workspace_dir, &required_string_arg(&args, "path")?)?;
        let find = required_string_arg(&args, "find")?;
        let replace = required_string_arg(&args, "replace")?;
        let replace_all = optional_bool_arg(&args, "replace_all").unwrap_or(false);
        let content = fs::read_to_string(&path)?;

        if !content.contains(&find) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Target text was not found in the file.".to_string()),
            });
        }

        let next_content = if replace_all {
            content.replace(&find, &replace)
        } else {
            content.replacen(&find, &replace, 1)
        };
        fs::write(&path, next_content.as_bytes())?;

        Ok(ToolResult {
            success: true,
            output: format!(
                "Edited {}",
                display_workspace_path(&self.workspace_dir, &path)
            ),
            error: None,
        })
    }
}

struct GlobSearchWorkspaceTool {
    workspace_dir: PathBuf,
}

impl GlobSearchWorkspaceTool {
    fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for GlobSearchWorkspaceTool {
    fn name(&self) -> &str {
        "glob_search"
    }

    fn description(&self) -> &str {
        "List workspace files that match a simple wildcard pattern such as *.rs or src/*.ts."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> AnyhowResult<ToolResult> {
        let pattern = required_string_arg(&args, "pattern")?;
        let files = collect_workspace_files(&self.workspace_dir)?;
        let matches = files
            .into_iter()
            .filter_map(|path| {
                let relative = display_workspace_path(&self.workspace_dir, &path);
                let filename = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                if wildcard_match(&pattern, &relative) || wildcard_match(&pattern, filename) {
                    Some(relative)
                } else {
                    None
                }
            })
            .take(200)
            .collect::<Vec<_>>();

        Ok(ToolResult {
            success: true,
            output: if matches.is_empty() {
                format!("No files matched pattern {}", pattern)
            } else {
                matches.join("\n")
            },
            error: None,
        })
    }
}

struct ContentSearchWorkspaceTool {
    workspace_dir: PathBuf,
}

impl ContentSearchWorkspaceTool {
    fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for ContentSearchWorkspaceTool {
    fn name(&self) -> &str {
        "content_search"
    }

    fn description(&self) -> &str {
        "Search workspace text files for a query and return matching lines."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "case_sensitive": { "type": "boolean" }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> AnyhowResult<ToolResult> {
        let query = required_string_arg(&args, "query")?;
        let case_sensitive = optional_bool_arg(&args, "case_sensitive").unwrap_or(false);
        let query_cmp = if case_sensitive {
            query.clone()
        } else {
            query.to_lowercase()
        };
        let mut matches = Vec::new();

        for path in collect_workspace_files(&self.workspace_dir)? {
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            if bytes.iter().any(|byte| *byte == 0) {
                continue;
            }

            let content = String::from_utf8_lossy(&bytes);
            for (line_index, line) in content.lines().enumerate() {
                let haystack = if case_sensitive {
                    line.to_string()
                } else {
                    line.to_lowercase()
                };
                if haystack.contains(&query_cmp) {
                    matches.push(format!(
                        "{}:{} {}",
                        display_workspace_path(&self.workspace_dir, &path),
                        line_index + 1,
                        truncate_chars(line.trim(), 240)
                    ));
                }
                if matches.len() >= 80 {
                    break;
                }
            }
            if matches.len() >= 80 {
                break;
            }
        }

        Ok(ToolResult {
            success: true,
            output: if matches.is_empty() {
                format!("No matches found for {}", query)
            } else {
                matches.join("\n")
            },
            error: None,
        })
    }
}

fn autonomy_level_skips_default_approval(autonomy: &zeroclaw::config::AutonomyConfig) -> bool {
    serde_json::to_value(autonomy)
        .ok()
        .and_then(|value| {
            value
                .get("level")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .is_some_and(|level| matches!(level.as_str(), "full" | "readonly"))
}
fn required_string_arg(args: &serde_json::Value, key: &str) -> AnyhowResult<String> {
    args.get(key)
        .and_then(serde_json::Value::as_str)
        .map(|value| value.to_string())
        .ok_or_else(|| anyhow!("Missing string argument `{}`.", key))
}

fn optional_bool_arg(args: &serde_json::Value, key: &str) -> Option<bool> {
    args.get(key).and_then(serde_json::Value::as_bool)
}

fn optional_u64_arg(args: &serde_json::Value, key: &str) -> Option<u64> {
    args.get(key).and_then(serde_json::Value::as_u64)
}

fn render_command_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout_text = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr_text = String::from_utf8_lossy(stderr).trim().to_string();

    match (stdout_text.is_empty(), stderr_text.is_empty()) {
        (false, false) => format!("stdout:\n{}\n\nstderr:\n{}", stdout_text, stderr_text),
        (false, true) => stdout_text,
        (true, false) => format!("stderr:\n{}", stderr_text),
        (true, true) => "Command completed with no output.".to_string(),
    }
}

fn resolve_workspace_path(workspace_dir: &Path, raw_path: &str) -> AnyhowResult<PathBuf> {
    let workspace = normalize_path(workspace_dir);
    let candidate = if Path::new(raw_path).is_absolute() {
        normalize_path(Path::new(raw_path))
    } else {
        normalize_path(&workspace.join(raw_path))
    };

    if !candidate.starts_with(&workspace) {
        return Err(anyhow!("Path must stay inside the workspace."));
    }

    Ok(candidate)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }

    normalized
}

fn display_workspace_path(workspace_dir: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_workspace_files(workspace_dir: &Path) -> AnyhowResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_workspace_files_inner(workspace_dir, &mut files)?;
    Ok(files)
}

fn collect_workspace_files_inner(dir: &Path, files: &mut Vec<PathBuf>) -> AnyhowResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if file_type.is_dir() {
            if should_skip_directory(&name) {
                continue;
            }
            collect_workspace_files_inner(&path, files)?;
            continue;
        }

        if file_type.is_file() {
            files.push(path);
        }
    }

    Ok(())
}

fn should_skip_directory(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "dist" | "target" | ".cargo-target" | ".npm-cache"
    )
}

fn wildcard_match(pattern: &str, candidate: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let candidate = candidate.to_lowercase();

    if !pattern.contains('*') {
        return candidate.contains(&pattern);
    }

    let parts = pattern.split('*').collect::<Vec<_>>();
    let mut cursor = 0usize;
    let anchored_start = !pattern.starts_with('*');
    let anchored_end = !pattern.ends_with('*');

    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if index == 0 && anchored_start {
            if !candidate[cursor..].starts_with(part) {
                return false;
            }
            cursor += part.len();
            continue;
        }

        if let Some(found) = candidate[cursor..].find(part) {
            cursor += found + part.len();
        } else {
            return false;
        }
    }

    if anchored_end {
        if let Some(last_part) = parts.iter().rev().find(|part| !part.is_empty()) {
            return candidate.ends_with(last_part);
        }
    }

    true
}
fn build_session_history(
    db_path: &std::path::Path,
    session_id: &str,
    user_input: &str,
    records: &[MessageRecord],
) -> Result<(Vec<ChatMessage>, Option<ProjectContextSelection>), String> {
    let mut history = super::runtime::build_history(records);
    let context_preview = resolve_project_context(db_path, session_id, user_input)?;

    if let Some(selection) = context_preview.as_ref() {
        history.insert(0, ChatMessage::system(selection.system_context.clone()));
    }

    Ok((history, context_preview))
}

fn build_agent_turn_input(
    db_path: &std::path::Path,
    session_id: &str,
    user_input: &str,
    records: &[MessageRecord],
) -> Result<(Vec<ChatMessage>, String, Option<ProjectContextSelection>), String> {
    let mut seed_records = records.to_vec();
    if seed_records
        .last()
        .is_some_and(|record| record.role == "user" && record.content == user_input)
    {
        seed_records.pop();
    }

    let history = super::runtime::build_history(&seed_records);
    let context_preview = resolve_project_context(db_path, session_id, user_input)?;
    let effective_user_input = if let Some(selection) = context_preview.as_ref() {
        format!(
            "Project context for this request:\n{}\n\nUser request:\n{}",
            selection.system_context, user_input
        )
    } else {
        user_input.to_string()
    };

    Ok((history, effective_user_input, context_preview))
}

fn resolve_project_context(
    db_path: &std::path::Path,
    session_id: &str,
    user_input: &str,
) -> Result<Option<ProjectContextSelection>, String> {
    let Some(project_id) = db::get_session_project_id(db_path, session_id)? else {
        return Ok(None);
    };

    let Some(project) = db::get_project(db_path, &project_id)? else {
        return Ok(None);
    };

    let knowledge_documents = db::list_project_knowledge(db_path, &project_id)?;
    let session_scope = db::get_session_knowledge_scope(db_path, session_id)?;
    let selected_documents = select_documents_for_session(
        &session_scope.mode,
        user_input,
        &knowledge_documents,
        &session_scope.document_ids,
    );

    Ok(build_project_context(
        &project.name,
        &project.description,
        &selected_documents,
        user_input,
        &session_scope.mode,
    ))
}

fn select_documents_for_session(
    scope_mode: &str,
    user_input: &str,
    knowledge_documents: &[KnowledgeDocumentRecord],
    scoped_document_ids: &[String],
) -> Vec<KnowledgeDocumentRecord> {
    if scope_mode == "manual" {
        return select_manual_knowledge_documents(knowledge_documents, scoped_document_ids);
    }

    select_relevant_knowledge_documents(user_input, knowledge_documents)
}

fn select_manual_knowledge_documents(
    knowledge_documents: &[KnowledgeDocumentRecord],
    scoped_document_ids: &[String],
) -> Vec<KnowledgeDocumentRecord> {
    let mut selected = Vec::new();

    for document_id in scoped_document_ids {
        if let Some(document) = knowledge_documents
            .iter()
            .find(|item| item.id == *document_id)
        {
            selected.push(document.clone());
        }
    }

    selected
}

fn build_project_context(
    project_name: &str,
    project_description: &str,
    knowledge_documents: &[KnowledgeDocumentRecord],
    user_input: &str,
    scope_mode: &str,
) -> Option<ProjectContextSelection> {
    let mut sections = vec![format!(
        "You are working inside the project context named \"{}\".",
        project_name.trim()
    )];

    let trimmed_description = project_description.trim();
    if !trimmed_description.is_empty() {
        sections.push(format!("Project brief:\n{}", trimmed_description));
    }

    let terms = tokenize_query(user_input);
    let mut rendered_titles = Vec::new();

    if !knowledge_documents.is_empty() {
        let mut used_chars = 0usize;
        let mut rendered_documents = Vec::new();

        for document in knowledge_documents
            .iter()
            .take(MAX_KNOWLEDGE_DOCS_IN_CONTEXT)
        {
            let excerpt =
                build_relevant_excerpt(&document.content, &terms, MAX_DOC_CHARS_IN_CONTEXT);
            if excerpt.is_empty() {
                continue;
            }

            let candidate = format!(
                "Title: {}\nSource: {}\nExcerpt:\n{}",
                document.title, document.source_path, excerpt
            );
            let candidate_len = candidate.chars().count();
            if used_chars + candidate_len > MAX_TOTAL_CONTEXT_CHARS {
                break;
            }
            used_chars += candidate_len;
            rendered_titles.push(document.title.clone());
            rendered_documents.push(candidate);
        }

        if !rendered_documents.is_empty() {
            let heading = if scope_mode == "manual" {
                "Use only the following manually selected project knowledge unless the user asks to broaden the scope."
            } else {
                "Use the following project knowledge excerpts only when they help answer the user's latest request. These excerpts were selected for relevance against the current message. If they conflict with newer user instructions, ask for clarification."
            };

            sections.push(format!(
                "{}\n\n{}",
                heading,
                rendered_documents
                    .iter()
                    .enumerate()
                    .map(|(index, value)| format!("[Knowledge {}]\n{}", index + 1, value))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ));
        }
    }

    let system_context = sections.join("\n\n");
    if system_context.trim().is_empty() {
        return None;
    }

    Some(ProjectContextSelection {
        system_context,
        project_name: project_name.trim().to_string(),
        scope_mode: scope_mode.to_string(),
        knowledge_titles: rendered_titles,
    })
}

fn select_relevant_knowledge_documents(
    user_input: &str,
    documents: &[KnowledgeDocumentRecord],
) -> Vec<KnowledgeDocumentRecord> {
    let terms = tokenize_query(user_input);
    if terms.is_empty() {
        return documents
            .iter()
            .take(MAX_KNOWLEDGE_DOCS_IN_CONTEXT)
            .cloned()
            .collect();
    }

    let mut scored = documents
        .iter()
        .map(|document| {
            let title = document.title.to_lowercase();
            let preview = document.content_preview.to_lowercase();
            let content = document.content.to_lowercase();
            let mut score = 0usize;

            for term in &terms {
                if title.contains(term) {
                    score += 6;
                }
                if preview.contains(term) {
                    score += 3;
                }
                if content.contains(term) {
                    score += 1;
                }
            }

            (score, document.clone())
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0));

    let selected = scored
        .into_iter()
        .filter(|(score, _)| *score > 0)
        .take(MAX_KNOWLEDGE_DOCS_IN_CONTEXT)
        .map(|(_, document)| document)
        .collect::<Vec<_>>();

    if selected.is_empty() {
        documents
            .iter()
            .take(MAX_KNOWLEDGE_DOCS_IN_CONTEXT)
            .cloned()
            .collect()
    } else {
        selected
    }
}

fn tokenize_query(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric())
        .filter_map(|part| {
            let normalized = part.trim().to_lowercase();
            if normalized.chars().count() < 3 {
                return None;
            }
            Some(normalized)
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn build_relevant_excerpt(content: &str, terms: &[String], max_chars: usize) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if terms.is_empty() {
        return truncate_chars(trimmed, max_chars);
    }

    let lower = trimmed.to_lowercase();
    let mut best_index = None;

    for term in terms {
        if let Some(index) = lower.find(term) {
            best_index = match best_index {
                Some(current) if current < index => Some(current),
                _ => Some(index),
            };
        }
    }

    let Some(index) = best_index else {
        return truncate_chars(trimmed, max_chars);
    };

    let start_chars = lower[..index]
        .chars()
        .count()
        .saturating_sub(EXCERPT_RADIUS_CHARS / 2);
    let excerpt = trimmed
        .chars()
        .skip(start_chars)
        .take(max_chars)
        .collect::<String>();

    if start_chars > 0 {
        format!("...{}", excerpt)
    } else {
        excerpt
    }
}

fn summarize_args(args: &serde_json::Value) -> String {
    match args {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(key, value)| {
                let rendered = match value {
                    serde_json::Value::String(text) => truncate_chars(text, 80),
                    other => truncate_chars(&other.to_string(), 80),
                };
                format!("{key}: {rendered}")
            })
            .collect::<Vec<_>>()
            .join(", "),
        other => truncate_chars(&other.to_string(), 120),
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

async fn relay_fallback_chunks(
    app: &AppHandle,
    state: &AppState,
    session_id: &str,
    response: &str,
) -> Result<(), String> {
    for chunk in split_response_chunks(response, FALLBACK_CHUNK_TARGET_CHARS) {
        if state.take_cancellation(session_id) {
            emit_done(app, session_id)?;
            return Ok(());
        }

        emit_token(app, session_id, &chunk)?;
        sleep(Duration::from_millis(FALLBACK_CHUNK_DELAY_MS)).await;
    }

    Ok(())
}

fn split_response_chunks(response: &str, target_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut char_count = 0usize;

    for ch in response.chars() {
        current.push(ch);
        char_count += 1;

        let should_flush = char_count >= target_chars
            && (ch.is_whitespace() || matches!(ch, ',' | '.' | '!' | '?' | ';' | ':' | '\n'));
        if should_flush {
            chunks.push(std::mem::take(&mut current));
            char_count = 0;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn emit_approval_request(
    app: &AppHandle,
    payload: &ChatApprovalRequestPayload,
) -> Result<(), String> {
    app.emit("chat:approval-request", payload.clone())
        .map_err(|error| error.to_string())
}

fn emit_context(app: &AppHandle, payload: &ChatContextPayload) -> Result<(), String> {
    app.emit("chat:context", payload.clone())
        .map_err(|error| error.to_string())
}

fn emit_token(app: &AppHandle, session_id: &str, token: &str) -> Result<(), String> {
    app.emit(
        "chat:token",
        ChatTokenPayload {
            session_id: session_id.to_string(),
            token: token.to_string(),
        },
    )
    .map_err(|error| error.to_string())
}

fn emit_done(app: &AppHandle, session_id: &str) -> Result<(), String> {
    app.emit(
        "chat:done",
        ChatDonePayload {
            session_id: session_id.to_string(),
        },
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use zeroclaw::{
        agent::{dispatcher::NativeToolDispatcher, Agent},
        memory::{self, Memory},
        observability::{NoopObserver, Observer},
        providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ToolCall},
    };

    use crate::{
        models::{mcp::McpServerDraft, settings::RuntimeSettingsRecord},
        services::{mcp, runtime},
        state::AppState,
    };

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
    struct ScriptedProvider {
        responses: Mutex<Vec<ChatResponse>>,
        requests: Mutex<Vec<Vec<ChatMessage>>>,
    }

    struct SharedScriptedProvider(Arc<ScriptedProvider>);

    impl ScriptedProvider {
        fn new(responses: Vec<ChatResponse>) -> Self {
            Self {
                responses: Mutex::new(responses),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn requests(&self) -> Vec<Vec<ChatMessage>> {
            self.requests.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Provider for SharedScriptedProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> Result<String> {
            Ok("fallback".to_string())
        }

        async fn chat(
            &self,
            request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> Result<ChatResponse> {
            self.0
                .requests
                .lock()
                .unwrap()
                .push(request.messages.to_vec());

            let mut responses = self.0.responses.lock().unwrap();
            if responses.is_empty() {
                return Ok(ChatResponse {
                    text: Some("done".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                });
            }

            Ok(responses.remove(0))
        }
    }

    fn make_memory(workspace_dir: &std::path::Path) -> Arc<dyn Memory> {
        let config = zeroclaw::config::MemoryConfig {
            backend: "none".to_string(),
            ..zeroclaw::config::MemoryConfig::default()
        };
        Arc::from(memory::create_memory(&config, workspace_dir, None).unwrap())
    }

    fn make_observer() -> Arc<dyn Observer> {
        Arc::from(NoopObserver)
    }

    #[test]
    fn installing_skill_syncs_into_custom_workspace() {
        let app_dir = make_test_dir("zeroclawx-skill-workspace");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");
        let custom_workspace = app_dir.join("custom-workspace");

        let mut settings = RuntimeSettingsRecord::default();
        settings.agent.workspace_dir = custom_workspace.display().to_string();
        runtime::save_runtime_settings(&state.settings_path(), settings)
            .expect("runtime settings should save");

        let installed = crate::services::skill::install_template(&state, "code-review")
            .expect("skill template should install");
        let runtime_skill_path = custom_workspace
            .join("skills")
            .join(&installed.slug)
            .join("SKILL.md");
        assert!(
            runtime_skill_path.exists(),
            "enabled skills should sync into the configured workspace"
        );

        let runtime_skill = fs::read_to_string(&runtime_skill_path)
            .expect("synced skill markdown should be readable");
        assert!(runtime_skill.contains("# Code Review"));

        let default_workspace_copy = app_dir
            .join("zeroclaw-runtime")
            .join("workspace")
            .join("skills")
            .join(&installed.slug)
            .join("SKILL.md");
        assert!(
            !default_workspace_copy.exists(),
            "skills should not keep syncing into the default workspace once a custom workspace is configured"
        );

        let _ = fs::remove_dir_all(app_dir);
    }

    #[tokio::test]
    async fn workspace_tools_use_custom_workspace_directory() {
        let app_dir = make_test_dir("zeroclawx-chat-workspace-tools");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");
        let custom_workspace = app_dir.join("custom-workspace");

        let mut settings = RuntimeSettingsRecord::default();
        settings.agent.workspace_dir = custom_workspace.display().to_string();

        let config = runtime::build_resolved_runtime_config(&state.db_path(), settings)
            .expect("runtime config should resolve");
        assert_eq!(config.workspace_dir, custom_workspace);

        let (tools, allowed_tools) = super::build_session_tools(&config.workspace_dir, &config.mcp)
            .await
            .expect("session tools should build");
        assert!(allowed_tools.iter().any(|name| name == "file_write"));
        assert!(allowed_tools.iter().any(|name| name == "shell"));

        let mut shell_tool = None;
        let mut file_write_tool = None;

        for tool in tools {
            match tool.name() {
                "shell" => shell_tool = Some(tool),
                "file_write" => file_write_tool = Some(tool),
                _ => {}
            }
        }

        let file_write_tool = file_write_tool.expect("file_write tool should be present");
        let file_result = file_write_tool
            .execute(serde_json::json!({
                "path": "notes/agent.txt",
                "content": "workspace file write ok"
            }))
            .await
            .expect("file_write should execute");
        assert!(file_result.success, "file_write should succeed");

        let file_path = custom_workspace.join("notes").join("agent.txt");
        let file_content = fs::read_to_string(&file_path).expect("workspace file should exist");
        assert_eq!(file_content, "workspace file write ok");
        assert!(
            !app_dir
                .join("zeroclaw-runtime")
                .join("workspace")
                .join("notes")
                .join("agent.txt")
                .exists(),
            "file_write should target the configured workspace rather than the app default"
        );

        let shell_tool = shell_tool.expect("shell tool should be present");
        let shell_command = if cfg!(target_os = "windows") {
            "Set-Content -Path shell-tool.txt -Value 'workspace shell ok'"
        } else {
            "printf 'workspace shell ok' > shell-tool.txt"
        };
        let shell_result = shell_tool
            .execute(serde_json::json!({
                "command": shell_command,
                "approved": true
            }))
            .await
            .expect("shell tool should execute");
        assert!(
            shell_result.success,
            "shell should run in the configured workspace: {} {:?}",
            shell_result.output, shell_result.error
        );

        let shell_output_path = custom_workspace.join("shell-tool.txt");
        let shell_output =
            fs::read_to_string(&shell_output_path).expect("shell output file should exist");
        assert_eq!(shell_output.trim(), "workspace shell ok");

        let _ = fs::remove_dir_all(app_dir);
    }

    #[tokio::test]
    async fn agent_turn_uses_builtin_workspace_tool_in_custom_workspace() {
        let app_dir = make_test_dir("zeroclawx-chat-agent-workspace");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");
        let custom_workspace = app_dir.join("custom-workspace");

        let mut settings = RuntimeSettingsRecord::default();
        settings.agent.workspace_dir = custom_workspace.display().to_string();

        let config = runtime::build_resolved_runtime_config(&state.db_path(), settings)
            .expect("runtime config should resolve");
        let (tools, allowed_tools) = super::build_session_tools(&config.workspace_dir, &config.mcp)
            .await
            .expect("session tools should build");

        let provider = Arc::new(ScriptedProvider::new(vec![
            ChatResponse {
                text: Some(String::new()),
                tool_calls: vec![ToolCall {
                    id: "tool-call-workspace-1".to_string(),
                    name: "file_write".to_string(),
                    arguments: r#"{"path":"reports/status.txt","content":"workspace agent ok"}"#
                        .to_string(),
                }],
                usage: None,
                reasoning_content: None,
            },
            ChatResponse {
                text: Some("Created reports/status.txt in the configured workspace.".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ]));

        let mut agent = Agent::builder()
            .provider(Box::new(SharedScriptedProvider(Arc::clone(&provider))))
            .tools(tools)
            .memory(make_memory(&config.workspace_dir))
            .observer(make_observer())
            .tool_dispatcher(Box::new(NativeToolDispatcher))
            .workspace_dir(config.workspace_dir.clone())
            .allowed_tools(Some(allowed_tools))
            .build()
            .expect("agent should build");

        let response = agent
            .turn("Use file_write to create reports/status.txt and confirm the result.")
            .await
            .expect("agent turn should succeed");

        assert!(response.contains("reports/status.txt"));

        let written_file = custom_workspace.join("reports").join("status.txt");
        let written_content = fs::read_to_string(&written_file)
            .expect("agent should write the file into the custom workspace");
        assert_eq!(written_content, "workspace agent ok");
        assert!(
            !app_dir
                .join("zeroclaw-runtime")
                .join("workspace")
                .join("reports")
                .join("status.txt")
                .exists(),
            "agent tool calls should respect the configured workspace directory"
        );

        let requests = provider.requests();
        assert_eq!(requests.len(), 2, "provider should be called twice");
        assert!(
            requests[1].iter().any(|message| {
                message.role == "tool" && message.content.contains("Updated reports/status.txt")
            }),
            "second request should include the file_write tool result"
        );

        let _ = fs::remove_dir_all(app_dir);
    }

    #[tokio::test]
    async fn agent_turn_executes_mcp_tool_with_mock_provider() {
        let app_dir = make_test_dir("zeroclawx-chat-agent-mcp");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");

        mcp::create_server(
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

        let config = runtime::build_resolved_runtime_config(
            &state.db_path(),
            RuntimeSettingsRecord::default(),
        )
        .expect("runtime config should resolve");
        let (tools, allowed_tools) = super::build_session_tools(&config.workspace_dir, &config.mcp)
            .await
            .expect("session tools should build");

        let provider = Arc::new(ScriptedProvider::new(vec![
            ChatResponse {
                text: Some(String::new()),
                tool_calls: vec![ToolCall {
                    id: "tool-call-1".to_string(),
                    name: "mock-weather__echo_weather".to_string(),
                    arguments: r#"{"city":"Shanghai"}"#.to_string(),
                }],
                usage: None,
                reasoning_content: None,
            },
            ChatResponse {
                text: Some("The MCP tool reports clear skies for Shanghai.".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ]));

        let mut agent = Agent::builder()
            .provider(Box::new(SharedScriptedProvider(Arc::clone(&provider))))
            .tools(tools)
            .memory(make_memory(&config.workspace_dir))
            .observer(make_observer())
            .tool_dispatcher(Box::new(NativeToolDispatcher))
            .workspace_dir(config.workspace_dir.clone())
            .allowed_tools(Some(allowed_tools))
            .build()
            .expect("agent should build");

        let response = agent
            .turn("Use the MCP weather tool for Shanghai and summarize the result.")
            .await
            .expect("agent turn should succeed");

        assert!(response.contains("Shanghai"));
        assert!(response.contains("clear skies"));

        let requests = provider.requests();
        assert_eq!(requests.len(), 2, "provider should be called twice");
        assert!(
            requests[1]
                .iter()
                .any(|message| message.role == "tool" && message.content.contains("Shanghai")),
            "second request should include tool output from the MCP wrapper"
        );

        let _ = fs::remove_dir_all(app_dir);
    }
    #[tokio::test]
    async fn session_tools_include_and_execute_mcp_wrappers() {
        let app_dir = make_test_dir("zeroclawx-chat-mcp");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");

        mcp::create_server(
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

        let config = runtime::build_resolved_runtime_config(
            &state.db_path(),
            RuntimeSettingsRecord::default(),
        )
        .expect("runtime config should resolve");
        let (tools, allowed_tools) = super::build_session_tools(&config.workspace_dir, &config.mcp)
            .await
            .expect("session tools should build");

        assert!(
            allowed_tools
                .iter()
                .any(|name| name == "mock-weather__echo_weather"),
            "allowed tools should include the MCP wrapper"
        );

        let weather_tool = tools
            .into_iter()
            .find(|tool| tool.name() == "mock-weather__echo_weather")
            .expect("MCP wrapper should be present");
        let result = weather_tool
            .execute(serde_json::json!({ "city": "Shanghai" }))
            .await
            .expect("MCP wrapper execution should succeed");

        assert!(result.success, "tool execution should report success");
        assert!(result.output.contains("Shanghai"));
        assert!(result.output.contains("mock server"));

        let _ = fs::remove_dir_all(app_dir);
    }
}
