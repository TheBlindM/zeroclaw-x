use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use zeroclaw::{
    config::{
        runtime_proxy_config, set_runtime_proxy_config, AgentConfig, AutonomyConfig, Config,
        DelegateAgentConfig, DelegateToolConfig, McpConfig, McpServerConfig, McpTransport,
        ProxyConfig, ProxyScope,
    },
    providers::{self, ChatMessage, Provider, ProviderRuntimeOptions},
};

use crate::{
    db,
    models::{
        chat::MessageRecord,
        mcp::McpServerRecord,
        settings::{
            RuntimeAutonomyLevelRecord, RuntimeConnectionReport, RuntimeProfileRecord,
            RuntimeProfilesExportReport, RuntimeProfilesImportReport, RuntimeProfilesState,
            RuntimeProxyScopeRecord, RuntimeProxySettingsRecord, RuntimeProxySupportRecord,
            RuntimeSettingsRecord, RuntimeStatusRecord, RuntimeProviderEntryRecord,
            RuntimeProviderGroupRecord, RuntimeCredentialModeRecord,
        },
    },
};

const CONNECTION_TEST_PROMPT: &str = "Reply in one short sentence confirming the runtime connection works, and mention the provider or model if available.";
const RUNTIME_PROFILES_EXPORT_NAME: &str = "zeroclawx-runtime-profiles.json";
const PROXY_SETTINGS_FILE_NAME: &str = "proxy-settings.json";

pub struct RuntimeSession {
    pub provider: Box<dyn Provider>,
    pub provider_name: String,
    pub model: String,
    pub temperature: f64,
}

pub fn build_runtime_session(
    db_path: &PathBuf,
    settings_path: &Path,
) -> Result<RuntimeSession, String> {
    let settings = load_runtime_settings(settings_path)?;
    build_runtime_session_from_settings(db_path, settings)
}

pub fn build_agent_runtime_session(
    db_path: &PathBuf,
    settings_path: &Path,
) -> Result<RuntimeSession, String> {
    let settings = load_runtime_settings(settings_path)?;
    build_agent_runtime_session_from_settings(db_path, settings)
}

pub fn get_runtime_status(
    db_path: &PathBuf,
    settings_path: &Path,
) -> Result<RuntimeStatusRecord, String> {
    let profiles_state = load_runtime_profiles(settings_path)?;
    let active_profile = profiles_state
        .active_profile()
        .cloned()
        .unwrap_or_else(|| RuntimeProfilesState::default().profiles[0].clone());
    let config = build_resolved_runtime_config(db_path, active_profile.settings.clone())?;

    Ok(RuntimeStatusRecord {
        profile_id: active_profile.id,
        profile_name: active_profile.name,
        provider: config
            .default_provider
            .clone()
            .unwrap_or_else(|| "openrouter".to_string()),
        model: config
            .default_model
            .clone()
            .unwrap_or_else(|| "anthropic/claude-sonnet-4.6".to_string()),
        provider_url: config.api_url.unwrap_or_default(),
        temperature: config.default_temperature,
        api_key_configured: config
            .api_key
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty()),
        credential_mode: active_profile.settings.credential_mode,
        auth_profile: active_profile.settings.auth_profile,
        workspace_dir: config.workspace_dir.display().to_string(),
        tool_dispatcher: config.agent.tool_dispatcher.clone(),
        autonomy_level: autonomy_level_record_from_config(&config.autonomy),
        workspace_only: config.autonomy.workspace_only,
        parallel_tools: config.agent.parallel_tools,
    })
}

pub async fn test_runtime_settings(
    db_path: &PathBuf,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeConnectionReport, String> {
    let runtime = build_runtime_session_from_settings(db_path, settings)?;
    let provider_name = runtime.provider_name.clone();
    let model = runtime.model.clone();

    let response = runtime
        .provider
        .simple_chat(CONNECTION_TEST_PROMPT, &model, runtime.temperature)
        .await
        .map_err(sanitize_runtime_error)?;

    Ok(RuntimeConnectionReport {
        ok: true,
        provider: provider_name,
        model,
        message: "Connection succeeded. The backend can reach the configured runtime.".to_string(),
        preview: Some(truncate_preview(&response)),
    })
}

pub fn load_proxy_settings(settings_path: &Path) -> Result<RuntimeProxySettingsRecord, String> {
    let legacy_proxy = load_runtime_settings(settings_path)?.proxy;
    let proxy_path = proxy_settings_path_from_settings_path(settings_path);
    load_or_migrate_proxy_settings(&proxy_path, legacy_proxy)
}

pub fn save_proxy_settings(
    settings_path: &Path,
    settings: RuntimeProxySettingsRecord,
) -> Result<RuntimeProxySettingsRecord, String> {
    let proxy_path = proxy_settings_path_from_settings_path(settings_path);
    save_proxy_settings_to_path(&proxy_path, settings)
}

pub fn get_proxy_support() -> RuntimeProxySupportRecord {
    RuntimeProxySupportRecord {
        supported_service_keys: ProxyConfig::supported_service_keys()
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        supported_selectors: ProxyConfig::supported_service_selectors()
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    }
}

pub fn export_runtime_profiles(
    app: &AppHandle,
    settings_path: &Path,
) -> Result<Option<RuntimeProfilesExportReport>, String> {
    let state = load_runtime_profiles(settings_path)?;
    let selected = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_title("Export runtime profiles")
        .set_file_name(RUNTIME_PROFILES_EXPORT_NAME)
        .blocking_save_file();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let path = selected
        .into_path()
        .map_err(|_| "Failed to resolve the export file path.".to_string())?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let serialized = serde_json::to_string_pretty(&state).map_err(|error| error.to_string())?;
    fs::write(&path, serialized).map_err(|error| error.to_string())?;

    Ok(Some(RuntimeProfilesExportReport {
        path: path.display().to_string(),
        profile_count: state.profiles.len(),
    }))
}

pub fn import_runtime_profiles(
    app: &AppHandle,
    settings_path: &Path,
) -> Result<Option<RuntimeProfilesImportReport>, String> {
    let selected = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_title("Import runtime profiles")
        .blocking_pick_file();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let path = selected
        .into_path()
        .map_err(|_| "Failed to resolve the selected import file path.".to_string())?;
    let raw = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let imported = parse_runtime_profiles_json(&raw)?;
    let imported_count = imported.profiles.len();
    let current = load_runtime_profiles(settings_path)?;
    let merged = merge_runtime_profiles(current, imported);
    let saved = save_runtime_profiles(settings_path, merged)?;

    Ok(Some(RuntimeProfilesImportReport {
        path: path.display().to_string(),
        imported_count,
        profiles: saved,
    }))
}

pub fn pick_runtime_workspace(app: &AppHandle) -> Result<Option<String>, String> {
    let selected = app
        .dialog()
        .file()
        .set_title("Select agent workspace")
        .blocking_pick_folder();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let path = selected
        .into_path()
        .map_err(|_| "Failed to resolve the selected workspace directory.".to_string())?;

    Ok(Some(path.display().to_string()))
}

pub fn load_runtime_profiles(settings_path: &Path) -> Result<RuntimeProfilesState, String> {
    if !settings_path.exists() {
        return Ok(RuntimeProfilesState::default());
    }

    let raw = fs::read_to_string(settings_path).map_err(|error| error.to_string())?;
    parse_runtime_profiles_json(&raw)
}

pub fn load_runtime_settings(settings_path: &Path) -> Result<RuntimeSettingsRecord, String> {
    let state = load_runtime_profiles(settings_path)?;
    Ok(state
        .active_profile()
        .map(|profile| profile.settings.clone())
        .unwrap_or_default()
        .normalized())
}

pub fn save_runtime_settings(
    settings_path: &Path,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeProfilesState, String> {
    let mut state = load_runtime_profiles(settings_path)?;
    if let Some(profile) = state.active_profile_mut() {
        profile.settings = settings.normalized();
    }
    save_runtime_profiles(settings_path, state)
}

pub fn create_runtime_profile(
    settings_path: &Path,
    name: &str,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeProfilesState, String> {
    let mut state = load_runtime_profiles(settings_path)?;
    let normalized_name = normalize_profile_name(name, state.profiles.len() + 1);
    let unique_name = generate_unique_profile_name(&normalized_name, &state.profiles);
    let profile = RuntimeProfileRecord {
        id: generate_unique_profile_id(&unique_name, &state.profiles),
        name: unique_name,
        settings: settings.normalized(),
    }
    .normalized();

    state.active_profile_id = profile.id.clone();
    state.profiles.push(profile);
    save_runtime_profiles(settings_path, state)
}

pub fn update_runtime_profile(
    settings_path: &Path,
    profile_id: &str,
    name: &str,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeProfilesState, String> {
    let mut state = load_runtime_profiles(settings_path)?;
    let profile_index = state
        .profiles
        .iter()
        .position(|profile| profile.id == profile_id)
        .ok_or_else(|| "Runtime profile not found.".to_string())?;

    let existing_profiles = state
        .profiles
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != profile_index)
        .map(|(_, profile)| profile.clone())
        .collect::<Vec<_>>();

    state.profiles[profile_index].name =
        generate_unique_profile_name(&normalize_profile_name(name, 1), &existing_profiles);
    state.profiles[profile_index].settings = settings.normalized();
    save_runtime_profiles(settings_path, state)
}

pub fn activate_runtime_profile(
    settings_path: &Path,
    profile_id: &str,
) -> Result<RuntimeProfilesState, String> {
    let mut state = load_runtime_profiles(settings_path)?;
    if !state
        .profiles
        .iter()
        .any(|profile| profile.id == profile_id)
    {
        return Err("Runtime profile not found.".to_string());
    }

    state.active_profile_id = profile_id.to_string();
    save_runtime_profiles(settings_path, state)
}

pub fn delete_runtime_profile(
    settings_path: &Path,
    profile_id: &str,
) -> Result<RuntimeProfilesState, String> {
    let mut state = load_runtime_profiles(settings_path)?;
    state.profiles.retain(|profile| profile.id != profile_id);

    if state.profiles.is_empty() {
        return save_runtime_profiles(settings_path, RuntimeProfilesState::default());
    }

    if state.active_profile_id == profile_id {
        state.active_profile_id = state.profiles[0].id.clone();
    }

    save_runtime_profiles(settings_path, state)
}

fn save_runtime_profiles(
    settings_path: &Path,
    state: RuntimeProfilesState,
) -> Result<RuntimeProfilesState, String> {
    let normalized = state.normalized();

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let serialized =
        serde_json::to_string_pretty(&normalized).map_err(|error| error.to_string())?;
    fs::write(settings_path, serialized).map_err(|error| error.to_string())?;

    Ok(normalized)
}

pub fn build_history(records: &[MessageRecord]) -> Vec<ChatMessage> {
    records
        .iter()
        .filter_map(|record| match record.role.as_str() {
            "system" => Some(ChatMessage::system(record.content.clone())),
            "user" => Some(ChatMessage::user(record.content.clone())),
            "assistant" => Some(ChatMessage::assistant(record.content.clone())),
            _ => None,
        })
        .collect()
}

pub fn sanitize_runtime_error(error: impl ToString) -> String {
    providers::sanitize_api_error(&error.to_string())
}

fn build_runtime_session_from_settings(
    db_path: &PathBuf,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeSession, String> {
    let normalized_settings = settings.normalized();
    let config = build_resolved_runtime_config(db_path, normalized_settings.clone())?;

    let provider_name = config
        .default_provider
        .clone()
        .unwrap_or_else(|| "openrouter".to_string());
    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| "anthropic/claude-sonnet-4.6".to_string());

    let mut options: ProviderRuntimeOptions =
        providers::provider_runtime_options_from_config(&config);
    if normalized_settings.credential_mode
        == crate::models::settings::RuntimeCredentialModeRecord::AuthProfile
        && supports_auth_profile_override(&provider_name)
        && !normalized_settings.auth_profile.is_empty()
    {
        options.auth_profile_override = Some(normalized_settings.auth_profile.clone());
    }
    let provider = providers::create_routed_provider_with_options(
        &provider_name,
        config.api_key.as_deref(),
        config.api_url.as_deref(),
        &config.reliability,
        &config.model_routes,
        &model,
        &options,
    )
    .map_err(|error| providers::sanitize_api_error(&error.to_string()))?;

    Ok(RuntimeSession {
        provider,
        provider_name,
        model,
        temperature: config.default_temperature,
    })
}

fn build_agent_runtime_session_from_settings(
    db_path: &PathBuf,
    settings: RuntimeSettingsRecord,
) -> Result<RuntimeSession, String> {
    let normalized_settings = settings.normalized();
    let config = build_resolved_runtime_config(db_path, normalized_settings.clone())?;
    let entry = resolve_runtime_entry_binding(
        &normalized_settings.groups,
        &normalized_settings.agent.runtime_group_id,
        &normalized_settings.agent.runtime_entry_id,
    )
    .unwrap_or_else(|| RuntimeProviderEntryRecord {
        id: "primary".to_string(),
        name: String::new(),
        provider: normalized_settings.provider.clone(),
        model: normalized_settings.model.clone(),
        provider_url: normalized_settings.provider_url.clone(),
        api_key: normalized_settings.api_key.clone(),
        credential_mode: normalized_settings.credential_mode,
        auth_profile: normalized_settings.auth_profile.clone(),
        temperature: normalized_settings.temperature,
    });

    let provider_name = entry.provider.clone();
    let model = entry.model.clone();
    let mut options: ProviderRuntimeOptions =
        providers::provider_runtime_options_from_config(&config);
    if entry.credential_mode == RuntimeCredentialModeRecord::AuthProfile
        && supports_auth_profile_override(&provider_name)
        && !entry.auth_profile.is_empty()
    {
        options.auth_profile_override = Some(entry.auth_profile.clone());
    }
    let provider = providers::create_routed_provider_with_options(
        &provider_name,
        non_empty_string(entry.api_key).as_deref().or(config.api_key.as_deref()),
        non_empty_string(entry.provider_url)
            .as_deref()
            .or(config.api_url.as_deref()),
        &config.reliability,
        &config.model_routes,
        &model,
        &options,
    )
    .map_err(|error| providers::sanitize_api_error(&error.to_string()))?;

    Ok(RuntimeSession {
        provider,
        provider_name,
        model,
        temperature: entry.temperature,
    })
}

pub(crate) fn build_resolved_runtime_config(
    db_path: &PathBuf,
    settings: RuntimeSettingsRecord,
) -> Result<Config, String> {
    let previous_proxy = runtime_proxy_config();
    let normalized_settings = settings.normalized();
    let runtime_proxy =
        load_runtime_proxy_settings(db_path.as_path(), normalized_settings.proxy.clone())?;
    let mut config = build_runtime_config(db_path, normalized_settings, runtime_proxy)?;

    if previous_proxy.scope == ProxyScope::Environment
        && !(config.proxy.enabled && config.proxy.scope == ProxyScope::Environment)
    {
        ProxyConfig::clear_process_env();
    }

    super::skill::sync_runtime_skills_to_workspace(db_path.as_path(), &config.workspace_dir)?;
    config.apply_env_overrides();
    config.mcp = build_runtime_mcp_config(db_path)?;
    config.validate().map_err(|error| error.to_string())?;
    apply_runtime_proxy_state(&config.proxy, &previous_proxy);

    Ok(config)
}

pub(crate) fn resolve_runtime_workspace_dir(
    db_path: &Path,
    settings_path: &Path,
) -> Result<PathBuf, String> {
    let settings = load_runtime_settings(settings_path)?;
    resolve_workspace_dir_from_settings(db_path, &settings)
}

pub(crate) fn resolve_workspace_dir_from_settings(
    db_path: &Path,
    settings: &RuntimeSettingsRecord,
) -> Result<PathBuf, String> {
    let app_data_dir = db_path
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "Failed to resolve app data directory for runtime.".to_string())?;
    let configured = settings.agent.workspace_dir.trim();

    if configured.is_empty() {
        return default_workspace_dir_from_db(db_path);
    }

    resolve_configured_path(configured, &app_data_dir)
}

fn build_runtime_config(
    db_path: &PathBuf,
    settings: RuntimeSettingsRecord,
    runtime_proxy: RuntimeProxySettingsRecord,
) -> Result<Config, String> {
    let runtime_root = default_runtime_root_from_db(db_path.as_path())?;
    let config_path = runtime_root.join("config.toml");
    let settings = settings.normalized();
    let workspace_dir = resolve_workspace_dir_from_settings(db_path.as_path(), &settings)?;
    let RuntimeSettingsRecord {
        active_group_id: _,
        groups: _,
        active_entry_id: _,
        entries: _,
        provider,
        model,
        provider_url,
        api_key,
        credential_mode,
        auth_profile: _,
        temperature,
        proxy: _,
        delegate,
        agents,
        agent,
        autonomy,
    } = settings;

    fs::create_dir_all(&runtime_root).map_err(|error| error.to_string())?;
    fs::create_dir_all(&workspace_dir).map_err(|error| error.to_string())?;

    let delegate_agents = build_delegate_agents(&settings.groups, agents);

    let mut config = Config::default();
    config.workspace_dir = workspace_dir;
    config.config_path = config_path;
    config.default_provider = Some(provider);
    config.default_model = Some(model);
    config.default_temperature = temperature;
    config.proxy = build_runtime_proxy_config(runtime_proxy);
    config.delegate = build_delegate_config(delegate);
    config.agents = delegate_agents;
    config.agent = build_agent_config(agent);
    config.autonomy = build_autonomy_config(autonomy);

    if !provider_url.is_empty() {
        config.api_url = Some(provider_url);
    }

    if credential_mode == crate::models::settings::RuntimeCredentialModeRecord::ApiKey
        && !api_key.is_empty()
    {
        config.api_key = Some(api_key);
    }

    if let Some(provider_url) = read_env("ZEROCLAW_PROVIDER_URL") {
        config.api_url = Some(provider_url);
    }

    if let Some(provider) = read_env("ZEROCLAW_PROVIDER") {
        config.default_provider = Some(provider);
    }

    if let Some(model) = read_env("ZEROCLAW_MODEL") {
        config.default_model = Some(model);
    }

    if credential_mode == crate::models::settings::RuntimeCredentialModeRecord::ApiKey {
        if let Some(api_key) = read_env("ZEROCLAW_API_KEY").or_else(|| read_env("API_KEY")) {
            config.api_key = Some(api_key);
        }
    }

    Ok(config)
}

fn supports_auth_profile_override(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "openai-codex" | "gemini"
    )
}

fn default_runtime_root_from_db(db_path: &Path) -> Result<PathBuf, String> {
    let app_data_dir = db_path
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "Failed to resolve app data directory for runtime.".to_string())?;
    Ok(app_data_dir.join("zeroclaw-runtime"))
}

fn default_workspace_dir_from_db(db_path: &Path) -> Result<PathBuf, String> {
    Ok(default_runtime_root_from_db(db_path)?.join("workspace"))
}

fn resolve_configured_path(raw: &str, base_dir: &Path) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Workspace path cannot be empty.".to_string());
    }

    if let Some(expanded_home) = expand_home_dir(trimmed) {
        return Ok(expanded_home);
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        Ok(candidate)
    } else {
        Ok(base_dir.join(candidate))
    }
}

fn expand_home_dir(raw: &str) -> Option<PathBuf> {
    let suffix = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\"))?;
    let home = read_env("USERPROFILE")
        .or_else(|| read_env("HOME"))
        .map(PathBuf::from)?;
    Some(home.join(suffix))
}

fn build_runtime_proxy_config(settings: RuntimeProxySettingsRecord) -> ProxyConfig {
    let settings = settings.normalized();

    ProxyConfig {
        enabled: settings.enabled,
        http_proxy: non_empty_string(settings.http_proxy),
        https_proxy: non_empty_string(settings.https_proxy),
        all_proxy: non_empty_string(settings.all_proxy),
        no_proxy: settings.no_proxy,
        scope: map_proxy_scope_record(settings.scope),
        services: settings.services,
    }
}

fn load_runtime_proxy_settings(
    db_path: &Path,
    legacy_proxy: RuntimeProxySettingsRecord,
) -> Result<RuntimeProxySettingsRecord, String> {
    let proxy_path = proxy_settings_path_from_db(db_path)?;
    load_or_migrate_proxy_settings(&proxy_path, legacy_proxy)
}

fn load_or_migrate_proxy_settings(
    proxy_path: &Path,
    legacy_proxy: RuntimeProxySettingsRecord,
) -> Result<RuntimeProxySettingsRecord, String> {
    if proxy_path.exists() {
        let raw = fs::read_to_string(proxy_path).map_err(|error| error.to_string())?;
        let parsed = serde_json::from_str::<RuntimeProxySettingsRecord>(&raw)
            .map_err(|error| error.to_string())?;
        return Ok(parsed.normalized());
    }

    Ok(legacy_proxy.normalized())
}

fn save_proxy_settings_to_path(
    proxy_path: &Path,
    settings: RuntimeProxySettingsRecord,
) -> Result<RuntimeProxySettingsRecord, String> {
    let normalized = settings.normalized();
    build_runtime_proxy_config(normalized.clone())
        .validate()
        .map_err(|error| error.to_string())?;

    if let Some(parent) = proxy_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let serialized =
        serde_json::to_string_pretty(&normalized).map_err(|error| error.to_string())?;
    fs::write(proxy_path, serialized).map_err(|error| error.to_string())?;

    Ok(normalized)
}

fn proxy_settings_path_from_settings_path(settings_path: &Path) -> PathBuf {
    settings_path
        .parent()
        .map(|parent| parent.join(PROXY_SETTINGS_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(PROXY_SETTINGS_FILE_NAME))
}

fn proxy_settings_path_from_db(db_path: &Path) -> Result<PathBuf, String> {
    db_path
        .parent()
        .map(|parent| parent.join(PROXY_SETTINGS_FILE_NAME))
        .ok_or_else(|| "Failed to resolve proxy settings path.".to_string())
}

fn build_agent_config(
    settings: crate::models::settings::RuntimeAgentSettingsRecord,
) -> AgentConfig {
    let settings = settings.normalized();

    AgentConfig {
        compact_context: settings.compact_context,
        max_tool_iterations: settings.max_tool_iterations,
        max_history_messages: settings.max_history_messages,
        max_context_tokens: settings.max_context_tokens,
        parallel_tools: settings.parallel_tools,
        tool_dispatcher: settings.tool_dispatcher,
        ..AgentConfig::default()
    }
}

fn build_delegate_config(
    settings: crate::models::settings::RuntimeDelegateSettingsRecord,
) -> DelegateToolConfig {
    let settings = settings.normalized();

    DelegateToolConfig {
        timeout_secs: settings.timeout_secs,
        agentic_timeout_secs: settings.agentic_timeout_secs,
    }
}

fn build_delegate_agents(
    groups: &[RuntimeProviderGroupRecord],
    settings: Vec<crate::models::settings::RuntimeDelegateAgentRecord>,
) -> HashMap<String, DelegateAgentConfig> {
    settings
        .into_iter()
        .filter_map(|record| {
            let normalized = record.normalized();
            if !normalized.enabled {
                return None;
            }

            let runtime_entry = resolve_runtime_entry_binding(
                groups,
                &normalized.runtime_group_id,
                &normalized.runtime_entry_id,
            );
            let provider = runtime_entry
                .as_ref()
                .map(|entry| entry.provider.clone())
                .unwrap_or_else(|| normalized.provider.clone());
            let model = runtime_entry
                .as_ref()
                .map(|entry| entry.model.clone())
                .unwrap_or_else(|| normalized.model.clone());
            let api_key = runtime_entry
                .as_ref()
                .and_then(|entry| non_empty_string(entry.api_key.clone()))
                .or(normalized.api_key.clone());
            let provider_api_url = runtime_entry
                .as_ref()
                .and_then(|entry| non_empty_string(entry.provider_url.clone()));
            let auth_profile_override = runtime_entry.as_ref().and_then(|entry| {
                if entry.credential_mode == RuntimeCredentialModeRecord::AuthProfile {
                    non_empty_string(entry.auth_profile.clone())
                } else {
                    None
                }
            });
            let temperature = runtime_entry
                .as_ref()
                .map(|entry| entry.temperature)
                .or(normalized.temperature);

            Some((
                normalized.name.clone(),
                DelegateAgentConfig {
                    provider,
                    model,
                    provider_api_url,
                    system_prompt: normalized.system_prompt,
                    api_key,
                    auth_profile_override,
                    temperature,
                    max_depth: normalized.max_depth,
                    agentic: normalized.agentic,
                    allowed_tools: normalized.allowed_tools,
                    max_iterations: normalized.max_iterations,
                    timeout_secs: normalized.timeout_secs,
                    agentic_timeout_secs: normalized.agentic_timeout_secs,
                    skills_directory: normalized.skills_directory,
                    memory_namespace: normalized.memory_namespace,
                },
            ))
        })
        .collect()
}

fn resolve_runtime_entry_binding(
    groups: &[RuntimeProviderGroupRecord],
    group_id: &str,
    entry_id: &str,
) -> Option<RuntimeProviderEntryRecord> {
    let fallback_group = groups.first()?;
    let group = groups
        .iter()
        .find(|group| group.id == group_id.trim())
        .unwrap_or(fallback_group);
    group
        .entries
        .iter()
        .find(|entry| entry.id == entry_id.trim())
        .or_else(|| group.active_entry())
        .or_else(|| group.entries.first())
        .cloned()
}

fn build_autonomy_config(
    settings: crate::models::settings::RuntimeAutonomySettingsRecord,
) -> AutonomyConfig {
    let settings = settings.normalized();
    let mut config = AutonomyConfig::default();

    let level_json = match settings.level {
        RuntimeAutonomyLevelRecord::ReadOnly => "\"readonly\"",
        RuntimeAutonomyLevelRecord::Supervised => "\"supervised\"",
        RuntimeAutonomyLevelRecord::Full => "\"full\"",
    };

    if let Ok(level) = serde_json::from_str(level_json) {
        config.level = level;
    }

    config.workspace_only = settings.workspace_only;
    config.allowed_commands = settings.allowed_commands;
    config.require_approval_for_medium_risk = settings.require_approval_for_medium_risk;
    config.block_high_risk_commands = settings.block_high_risk_commands;
    config.shell_env_passthrough = settings.shell_env_passthrough;
    config.auto_approve = settings.auto_approve;
    config.always_ask = settings.always_ask;
    config.allowed_roots = settings.allowed_roots;

    config
}

fn autonomy_level_record_from_config(config: &AutonomyConfig) -> RuntimeAutonomyLevelRecord {
    let serialized =
        serde_json::to_string(&config.level).unwrap_or_else(|_| "\"supervised\"".to_string());
    match serialized.trim_matches('"') {
        "readonly" => RuntimeAutonomyLevelRecord::ReadOnly,
        "full" => RuntimeAutonomyLevelRecord::Full,
        _ => RuntimeAutonomyLevelRecord::Supervised,
    }
}

fn map_proxy_scope_record(scope: RuntimeProxyScopeRecord) -> ProxyScope {
    match scope {
        RuntimeProxyScopeRecord::Environment => ProxyScope::Environment,
        RuntimeProxyScopeRecord::Zeroclaw => ProxyScope::Zeroclaw,
        RuntimeProxyScopeRecord::Services => ProxyScope::Services,
    }
}

fn apply_runtime_proxy_state(next_proxy: &ProxyConfig, previous_proxy: &ProxyConfig) {
    if next_proxy.enabled && next_proxy.scope == ProxyScope::Environment {
        next_proxy.apply_to_process_env();
    } else if previous_proxy.scope == ProxyScope::Environment {
        ProxyConfig::clear_process_env();
    }

    set_runtime_proxy_config(next_proxy.clone());
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn build_runtime_mcp_config(db_path: &Path) -> Result<McpConfig, String> {
    let mut servers = Vec::new();

    for record in db::list_mcp_servers(db_path)? {
        if !record.enabled {
            continue;
        }

        match map_mcp_server_record(&record) {
            Ok(server) => servers.push(server),
            Err(error) => eprintln!(
                "Skipping MCP server `{}` during runtime config assembly: {}",
                record.name, error
            ),
        }
    }

    Ok(McpConfig {
        enabled: !servers.is_empty(),
        deferred_loading: false,
        servers,
    })
}

fn map_mcp_server_record(record: &McpServerRecord) -> Result<McpServerConfig, String> {
    let args = serde_json::from_str::<Vec<String>>(&record.arguments_json).map_err(|error| {
        format!(
            "Invalid arguments JSON for MCP server `{}`: {error}",
            record.name
        )
    })?;
    let headers =
        serde_json::from_str::<HashMap<String, String>>(&record.headers_json).map_err(|error| {
            format!(
                "Invalid headers JSON for MCP server `{}`: {error}",
                record.name
            )
        })?;
    let env = serde_json::from_str::<HashMap<String, String>>(&record.environment_json).map_err(
        |error| {
            format!(
                "Invalid environment JSON for MCP server `{}`: {error}",
                record.name
            )
        },
    )?;

    let transport = match record.transport.as_str() {
        "stdio" => McpTransport::Stdio,
        "sse" => McpTransport::Sse,
        "streamable_http" => McpTransport::Http,
        other => {
            return Err(format!(
                "Unsupported transport `{other}` for MCP server `{}`.",
                record.name
            ))
        }
    };

    Ok(McpServerConfig {
        name: record.name.clone(),
        transport,
        url: if record.url.trim().is_empty() {
            None
        } else {
            Some(record.url.clone())
        },
        command: record.command.clone(),
        args,
        env,
        headers,
        tool_timeout_secs: None,
    })
}

fn parse_runtime_profiles_json(raw: &str) -> Result<RuntimeProfilesState, String> {
    if let Ok(state) = serde_json::from_str::<RuntimeProfilesState>(raw) {
        return Ok(state.normalized());
    }

    if let Ok(legacy) = serde_json::from_str::<RuntimeSettingsRecord>(raw) {
        return Ok(RuntimeProfilesState {
            active_profile_id: "default".to_string(),
            profiles: vec![RuntimeProfileRecord {
                id: "default".to_string(),
                name: "Default".to_string(),
                settings: legacy.normalized(),
            }],
        });
    }

    Err("Failed to parse runtime settings file.".to_string())
}

fn merge_runtime_profiles(
    current: RuntimeProfilesState,
    imported: RuntimeProfilesState,
) -> RuntimeProfilesState {
    let mut merged = current.normalized();
    let imported_active_original_id = imported.active_profile_id.clone();
    let mut imported_active_resolved_id = None;

    for imported_profile in imported.profiles {
        let normalized = imported_profile.normalized();
        let unique_name = generate_unique_profile_name(&normalized.name, &merged.profiles);
        let unique_id = generate_unique_profile_id(&normalized.name, &merged.profiles);

        if normalized.id == imported_active_original_id {
            imported_active_resolved_id = Some(unique_id.clone());
        }

        merged.profiles.push(RuntimeProfileRecord {
            id: unique_id,
            name: unique_name,
            settings: normalized.settings,
        });
    }

    if let Some(active_id) = imported_active_resolved_id {
        merged.active_profile_id = active_id;
    }

    merged.normalized()
}

fn generate_unique_profile_name(value: &str, existing_profiles: &[RuntimeProfileRecord]) -> String {
    let base = normalize_profile_name(value, existing_profiles.len() + 1);
    if !existing_profiles.iter().any(|profile| profile.name == base) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base} {suffix}");
        if !existing_profiles
            .iter()
            .any(|profile| profile.name == candidate)
        {
            return candidate;
        }
        suffix += 1;
    }
}

fn generate_unique_profile_id(
    base_name: &str,
    existing_profiles: &[RuntimeProfileRecord],
) -> String {
    let base = slugify_profile_name(base_name);
    if !existing_profiles.iter().any(|profile| profile.id == base) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !existing_profiles
            .iter()
            .any(|profile| profile.id == candidate)
        {
            return candidate;
        }
        suffix += 1;
    }
}

fn read_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn truncate_preview(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= 180 {
        return normalized;
    }

    normalized.chars().take(180).collect::<String>() + "..."
}

fn normalize_profile_name(value: &str, fallback_index: usize) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return format!("Profile {fallback_index}");
    }

    trimmed.to_string()
}

fn slugify_profile_name(name: &str) -> String {
    let slug = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        "profile".to_string()
    } else {
        slug
    }
}
