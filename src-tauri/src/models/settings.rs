use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProxyScopeRecord {
    Environment,
    #[default]
    Zeroclaw,
    Services,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAutonomyLevelRecord {
    ReadOnly,
    #[default]
    Supervised,
    Full,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCredentialModeRecord {
    #[default]
    ApiKey,
    AuthProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeProxySettingsRecord {
    pub enabled: bool,
    pub scope: RuntimeProxyScopeRecord,
    pub http_proxy: String,
    pub https_proxy: String,
    pub all_proxy: String,
    pub no_proxy: Vec<String>,
    pub services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeDelegateSettingsRecord {
    pub timeout_secs: u64,
    pub agentic_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeDelegateAgentRecord {
    pub enabled: bool,
    pub name: String,
    pub runtime_group_id: String,
    pub runtime_entry_id: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub api_key: Option<String>,
    pub temperature: Option<f64>,
    pub max_depth: u32,
    pub agentic: bool,
    pub allowed_tools: Vec<String>,
    pub max_iterations: usize,
    pub timeout_secs: Option<u64>,
    pub agentic_timeout_secs: Option<u64>,
    pub skills_directory: Option<String>,
    pub memory_namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeAgentSettingsRecord {
    pub enabled: bool,
    pub runtime_group_id: String,
    pub runtime_entry_id: String,
    pub workspace_dir: String,
    pub compact_context: bool,
    pub max_tool_iterations: usize,
    pub max_history_messages: usize,
    pub max_context_tokens: usize,
    pub parallel_tools: bool,
    pub tool_dispatcher: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeAutonomySettingsRecord {
    pub level: RuntimeAutonomyLevelRecord,
    pub workspace_only: bool,
    pub require_approval_for_medium_risk: bool,
    pub block_high_risk_commands: bool,
    pub allowed_commands: Vec<String>,
    pub allowed_roots: Vec<String>,
    pub shell_env_passthrough: Vec<String>,
    pub auto_approve: Vec<String>,
    pub always_ask: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeProviderEntryRecord {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub provider_url: String,
    pub api_key: String,
    pub credential_mode: RuntimeCredentialModeRecord,
    pub auth_profile: String,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeProviderGroupRecord {
    pub id: String,
    pub name: String,
    pub active_entry_id: String,
    pub entries: Vec<RuntimeProviderEntryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeSettingsRecord {
    pub active_group_id: String,
    pub groups: Vec<RuntimeProviderGroupRecord>,
    pub active_entry_id: String,
    pub entries: Vec<RuntimeProviderEntryRecord>,
    pub provider: String,
    pub model: String,
    pub provider_url: String,
    pub api_key: String,
    pub credential_mode: RuntimeCredentialModeRecord,
    pub auth_profile: String,
    pub temperature: f64,
    pub proxy: RuntimeProxySettingsRecord,
    pub delegate: RuntimeDelegateSettingsRecord,
    pub agents: Vec<RuntimeDelegateAgentRecord>,
    pub agent: RuntimeAgentSettingsRecord,
    pub autonomy: RuntimeAutonomySettingsRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfileRecord {
    pub id: String,
    pub name: String,
    pub settings: RuntimeSettingsRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfilesState {
    pub active_profile_id: String,
    pub profiles: Vec<RuntimeProfileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConnectionReport {
    pub ok: bool,
    pub provider: String,
    pub model: String,
    pub message: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProxySupportRecord {
    pub supported_service_keys: Vec<String>,
    pub supported_selectors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfilesExportReport {
    pub path: String,
    pub profile_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfilesImportReport {
    pub path: String,
    pub imported_count: usize,
    pub profiles: RuntimeProfilesState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatusRecord {
    pub profile_id: String,
    pub profile_name: String,
    pub provider: String,
    pub model: String,
    pub provider_url: String,
    pub temperature: f64,
    pub api_key_configured: bool,
    pub credential_mode: RuntimeCredentialModeRecord,
    pub auth_profile: String,
    pub workspace_dir: String,
    pub tool_dispatcher: String,
    pub autonomy_level: RuntimeAutonomyLevelRecord,
    pub workspace_only: bool,
    pub parallel_tools: bool,
}

impl Default for RuntimeProxySettingsRecord {
    fn default() -> Self {
        Self {
            enabled: false,
            scope: RuntimeProxyScopeRecord::Zeroclaw,
            http_proxy: String::new(),
            https_proxy: String::new(),
            all_proxy: String::new(),
            no_proxy: Vec::new(),
            services: Vec::new(),
        }
    }
}

impl Default for RuntimeDelegateSettingsRecord {
    fn default() -> Self {
        Self {
            timeout_secs: 120,
            agentic_timeout_secs: 300,
        }
    }
}

impl Default for RuntimeDelegateAgentRecord {
    fn default() -> Self {
        Self {
            enabled: true,
            name: String::new(),
            runtime_group_id: String::new(),
            runtime_entry_id: String::new(),
            provider: "openrouter".to_string(),
            model: "anthropic/claude-sonnet-4.6".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            max_depth: 2,
            agentic: true,
            allowed_tools: Vec::new(),
            max_iterations: 8,
            timeout_secs: None,
            agentic_timeout_secs: None,
            skills_directory: None,
            memory_namespace: None,
        }
    }
}

impl Default for RuntimeAgentSettingsRecord {
    fn default() -> Self {
        Self {
            enabled: true,
            runtime_group_id: String::new(),
            runtime_entry_id: String::new(),
            workspace_dir: String::new(),
            compact_context: false,
            max_tool_iterations: 10,
            max_history_messages: 50,
            max_context_tokens: 32_000,
            parallel_tools: false,
            tool_dispatcher: "auto".to_string(),
        }
    }
}

impl Default for RuntimeAutonomySettingsRecord {
    fn default() -> Self {
        Self {
            level: RuntimeAutonomyLevelRecord::Supervised,
            workspace_only: true,
            require_approval_for_medium_risk: true,
            block_high_risk_commands: true,
            allowed_commands: vec![
                "git".to_string(),
                "npm".to_string(),
                "cargo".to_string(),
                "ls".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "wc".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "date".to_string(),
            ],
            allowed_roots: Vec::new(),
            shell_env_passthrough: Vec::new(),
            auto_approve: vec!["file_read".to_string(), "memory_recall".to_string()],
            always_ask: Vec::new(),
        }
    }
}

impl Default for RuntimeSettingsRecord {
    fn default() -> Self {
        let entry = RuntimeProviderEntryRecord::default();
        let group = RuntimeProviderGroupRecord {
            id: "default-group".to_string(),
            name: "Default group".to_string(),
            active_entry_id: entry.id.clone(),
            entries: vec![entry.clone()],
        };
        Self {
            active_group_id: group.id.clone(),
            groups: vec![group],
            active_entry_id: entry.id.clone(),
            entries: vec![entry.clone()],
            provider: entry.provider,
            model: entry.model,
            provider_url: entry.provider_url,
            api_key: entry.api_key,
            credential_mode: entry.credential_mode,
            auth_profile: entry.auth_profile,
            temperature: entry.temperature,
            proxy: RuntimeProxySettingsRecord::default(),
            delegate: RuntimeDelegateSettingsRecord::default(),
            agents: Vec::new(),
            agent: RuntimeAgentSettingsRecord::default(),
            autonomy: RuntimeAutonomySettingsRecord::default(),
        }
    }
}

impl Default for RuntimeProfilesState {
    fn default() -> Self {
        Self {
            active_profile_id: "default".to_string(),
            profiles: vec![RuntimeProfileRecord {
                id: "default".to_string(),
                name: "Default".to_string(),
                settings: RuntimeSettingsRecord::default(),
            }],
        }
    }
}

impl Default for RuntimeProviderEntryRecord {
    fn default() -> Self {
        Self {
            id: "primary".to_string(),
            name: "Primary".to_string(),
            provider: "openrouter".to_string(),
            model: "anthropic/claude-sonnet-4.6".to_string(),
            provider_url: String::new(),
            api_key: String::new(),
            credential_mode: RuntimeCredentialModeRecord::ApiKey,
            auth_profile: String::new(),
            temperature: 0.7,
        }
    }
}

impl Default for RuntimeProviderGroupRecord {
    fn default() -> Self {
        let entry = RuntimeProviderEntryRecord::default();
        Self {
            id: "default-group".to_string(),
            name: "Default group".to_string(),
            active_entry_id: entry.id.clone(),
            entries: vec![entry],
        }
    }
}

impl RuntimeProviderEntryRecord {
    pub fn normalized(mut self) -> Self {
        self.id = slugify_entry_name(&self.id);
        if self.id.is_empty() {
            self.id = Self::default().id;
        }

        self.provider = self.provider.trim().to_string();
        if self.provider.is_empty() {
            self.provider = Self::default().provider;
        }

        self.model = self.model.trim().to_string();
        if self.model.is_empty() {
            self.model = Self::default().model;
        }

        self.provider_url = self.provider_url.trim().to_string();
        self.api_key = self.api_key.trim().to_string();
        self.auth_profile = self.auth_profile.trim().to_string();
        self.name = normalize_entry_name(
            &self.name,
            &self.provider,
            &self.model,
            self.credential_mode,
        );

        if !self.temperature.is_finite() {
            self.temperature = Self::default().temperature;
        }
        self.temperature = self.temperature.clamp(0.0, 2.0);

        self
    }
}

impl RuntimeProviderGroupRecord {
    pub fn normalized(mut self) -> Self {
        self.entries = self
            .entries
            .into_iter()
            .map(RuntimeProviderEntryRecord::normalized)
            .collect();

        if self.entries.is_empty() {
            self.entries = vec![RuntimeProviderEntryRecord::default()];
        } else {
            let mut seen_ids = HashSet::new();
            for (index, entry) in self.entries.iter_mut().enumerate() {
                if entry.id.is_empty() {
                    entry.id = format!("entry-{}", index + 1);
                }

                if !seen_ids.insert(entry.id.clone()) {
                    entry.id = make_unique_entry_id(&entry.id, &seen_ids);
                    seen_ids.insert(entry.id.clone());
                }
            }
        }

        self.id = slugify_group_name(&self.id);
        if self.id.is_empty() {
            self.id = slugify_group_name(&self.name);
        }
        if self.id.is_empty() {
            self.id = "group".to_string();
        }

        let primary_entry = self.entries.first().cloned().unwrap_or_default();
        self.name = normalize_group_name(&self.name, &primary_entry.provider, &primary_entry.model);
        self.active_entry_id = self.active_entry_id.trim().to_string();
        if self.active_entry_id.is_empty()
            || !self
                .entries
                .iter()
                .any(|entry| entry.id == self.active_entry_id)
        {
            self.active_entry_id = self.entries[0].id.clone();
        }

        self
    }

    pub fn active_entry(&self) -> Option<&RuntimeProviderEntryRecord> {
        self.entries
            .iter()
            .find(|entry| entry.id == self.active_entry_id)
    }
}

impl RuntimeSettingsRecord {
    pub fn normalized(mut self) -> Self {
        self.active_group_id = self.active_group_id.trim().to_string();
        self.active_entry_id = self.active_entry_id.trim().to_string();
        let legacy_entry = RuntimeProviderEntryRecord {
            id: "primary".to_string(),
            name: normalize_entry_name("", &self.provider, &self.model, self.credential_mode),
            provider: self.provider.trim().to_string(),
            model: self.model.trim().to_string(),
            provider_url: self.provider_url.trim().to_string(),
            api_key: self.api_key.trim().to_string(),
            credential_mode: self.credential_mode,
            auth_profile: self.auth_profile.trim().to_string(),
            temperature: self.temperature,
        }
        .normalized();

        self.groups = self
            .groups
            .into_iter()
            .map(RuntimeProviderGroupRecord::normalized)
            .collect();

        if self.groups.is_empty() {
            let entries = std::mem::take(&mut self.entries)
                .into_iter()
                .map(RuntimeProviderEntryRecord::normalized)
                .collect::<Vec<_>>();
            let group_entries = if entries.is_empty() {
                vec![legacy_entry]
            } else {
                entries
            };
            let group_name =
                normalize_group_name("", &group_entries[0].provider, &group_entries[0].model);
            self.groups = vec![RuntimeProviderGroupRecord {
                id: slugify_group_name(&group_name),
                name: group_name,
                active_entry_id: self.active_entry_id.clone(),
                entries: group_entries,
            }
            .normalized()];
        } else {
            let mut seen_group_ids = HashSet::new();
            for (index, group) in self.groups.iter_mut().enumerate() {
                if group.id.is_empty() {
                    group.id = format!("group-{}", index + 1);
                }

                if !seen_group_ids.insert(group.id.clone()) {
                    group.id = make_unique_group_id(&group.id, &seen_group_ids);
                    seen_group_ids.insert(group.id.clone());
                }
            }
        }

        if self.active_group_id.is_empty()
            || !self
                .groups
                .iter()
                .any(|group| group.id == self.active_group_id)
        {
            self.active_group_id = self.groups[0].id.clone();
        }

        self.proxy = self.proxy.normalized();
        self.delegate = self.delegate.normalized();
        self.agent = self.agent.normalized();
        self.resolve_main_agent_binding();
        self.agents = self
            .agents
            .into_iter()
            .map(RuntimeDelegateAgentRecord::normalized)
            .filter(|agent| !agent.name.is_empty())
            .collect();
        self.resolve_sub_agent_bindings();
        self.autonomy = self.autonomy.normalized();
        self.sync_legacy_fields_from_active_group_entry();

        self
    }

    pub fn active_group(&self) -> Option<&RuntimeProviderGroupRecord> {
        self.groups
            .iter()
            .find(|group| group.id == self.active_group_id)
    }

    fn sync_legacy_fields_from_active_group_entry(&mut self) {
        let fallback_group = RuntimeProviderGroupRecord::default();
        let group = self.active_group().cloned().unwrap_or(fallback_group);
        let entry = group
            .active_entry()
            .cloned()
            .unwrap_or_else(RuntimeProviderEntryRecord::default);

        self.active_entry_id = group.active_entry_id;
        self.entries = group.entries;
        self.provider = entry.provider;
        self.model = entry.model;
        self.provider_url = entry.provider_url;
        self.api_key = entry.api_key;
        self.credential_mode = entry.credential_mode;
        self.auth_profile = entry.auth_profile;
        self.temperature = entry.temperature;
    }

    fn resolve_main_agent_binding(&mut self) {
        let (runtime_group_id, runtime_entry_id) = self.resolve_runtime_binding(
            &self.agent.runtime_group_id,
            &self.agent.runtime_entry_id,
        );
        self.agent.runtime_group_id = runtime_group_id;
        self.agent.runtime_entry_id = runtime_entry_id;
    }

    fn resolve_sub_agent_bindings(&mut self) {
        let groups = self.groups.clone();
        let active_group_id = self.active_group_id.clone();
        for agent in &mut self.agents {
            let (runtime_group_id, runtime_entry_id) = resolve_runtime_binding_from_groups(
                &groups,
                &active_group_id,
                &agent.runtime_group_id,
                &agent.runtime_entry_id,
            );
            agent.runtime_group_id = runtime_group_id;
            agent.runtime_entry_id = runtime_entry_id;
        }
    }

    fn resolve_runtime_binding(&self, group_id: &str, entry_id: &str) -> (String, String) {
        resolve_runtime_binding_from_groups(
            &self.groups,
            &self.active_group_id,
            group_id,
            entry_id,
        )
    }
}

impl RuntimeProxySettingsRecord {
    pub fn normalized(mut self) -> Self {
        self.http_proxy = self.http_proxy.trim().to_string();
        self.https_proxy = self.https_proxy.trim().to_string();
        self.all_proxy = self.all_proxy.trim().to_string();
        self.no_proxy = normalize_proxy_list(self.no_proxy);
        self.services = normalize_proxy_list(self.services)
            .into_iter()
            .map(|value| value.to_ascii_lowercase())
            .collect();

        self
    }
}

impl RuntimeDelegateSettingsRecord {
    pub fn normalized(mut self) -> Self {
        self.timeout_secs = normalize_positive_u64(self.timeout_secs, 120);
        self.agentic_timeout_secs = normalize_positive_u64(self.agentic_timeout_secs, 300);
        self
    }
}

impl RuntimeDelegateAgentRecord {
    pub fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self.runtime_group_id = self.runtime_group_id.trim().to_string();
        self.runtime_entry_id = self.runtime_entry_id.trim().to_string();
        self.provider = self.provider.trim().to_string();
        self.model = self.model.trim().to_string();
        self.system_prompt = normalize_optional_string(self.system_prompt);
        self.api_key = normalize_optional_string(self.api_key);
        self.temperature = self.temperature.and_then(|value| {
            if value.is_finite() {
                Some(value.clamp(0.0, 2.0))
            } else {
                None
            }
        });
        self.max_depth = normalize_positive_u32(self.max_depth, 2);
        self.allowed_tools = normalize_string_list(self.allowed_tools)
            .into_iter()
            .map(|value| value.to_ascii_lowercase())
            .collect();
        self.max_iterations = normalize_positive_usize(self.max_iterations, 8);
        self.timeout_secs = self
            .timeout_secs
            .map(|value| normalize_positive_u64(value, 120));
        self.agentic_timeout_secs = self
            .agentic_timeout_secs
            .map(|value| normalize_positive_u64(value, 300));
        self.skills_directory = normalize_optional_string(self.skills_directory);
        self.memory_namespace = normalize_optional_string(self.memory_namespace);
        self
    }
}

impl RuntimeAgentSettingsRecord {
    pub fn normalized(mut self) -> Self {
        self.runtime_group_id = self.runtime_group_id.trim().to_string();
        self.runtime_entry_id = self.runtime_entry_id.trim().to_string();
        self.workspace_dir = self.workspace_dir.trim().to_string();
        self.max_tool_iterations = normalize_positive_usize(self.max_tool_iterations, 10);
        self.max_history_messages = normalize_positive_usize(self.max_history_messages, 50);
        self.max_context_tokens = normalize_positive_usize(self.max_context_tokens, 32_000);
        self.tool_dispatcher = normalize_dispatcher(self.tool_dispatcher);

        self
    }
}

impl RuntimeAutonomySettingsRecord {
    pub fn normalized(mut self) -> Self {
        self.allowed_commands = normalize_string_list(self.allowed_commands)
            .into_iter()
            .map(|value| value.to_ascii_lowercase())
            .collect();
        if self.allowed_commands.is_empty() {
            self.allowed_commands = Self::default().allowed_commands;
        }

        self.allowed_roots = normalize_string_list(self.allowed_roots);
        self.shell_env_passthrough = normalize_string_list(self.shell_env_passthrough)
            .into_iter()
            .map(|value| value.to_ascii_uppercase())
            .collect();
        self.auto_approve = normalize_string_list(self.auto_approve);
        self.always_ask = normalize_string_list(self.always_ask);

        self
    }
}

impl RuntimeProfileRecord {
    pub fn normalized(mut self) -> Self {
        self.id = self.id.trim().to_string();
        if self.id.is_empty() {
            self.id = "profile".to_string();
        }

        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            self.name = "Unnamed profile".to_string();
        }

        self.settings = self.settings.normalized();
        self
    }
}

impl RuntimeProfilesState {
    pub fn normalized(mut self) -> Self {
        self.active_profile_id = self.active_profile_id.trim().to_string();
        self.profiles = self
            .profiles
            .into_iter()
            .map(RuntimeProfileRecord::normalized)
            .collect();

        if self.profiles.is_empty() {
            return Self::default();
        }

        if self.active_profile_id.is_empty()
            || !self
                .profiles
                .iter()
                .any(|profile| profile.id == self.active_profile_id)
        {
            self.active_profile_id = self.profiles[0].id.clone();
        }

        self
    }

    pub fn active_profile(&self) -> Option<&RuntimeProfileRecord> {
        self.profiles
            .iter()
            .find(|profile| profile.id == self.active_profile_id)
    }

    pub fn active_profile_mut(&mut self) -> Option<&mut RuntimeProfileRecord> {
        self.profiles
            .iter_mut()
            .find(|profile| profile.id == self.active_profile_id)
    }
}

fn normalize_proxy_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn normalize_entry_name(
    value: &str,
    provider: &str,
    model: &str,
    credential_mode: RuntimeCredentialModeRecord,
) -> String {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    let provider = provider.trim();
    let model = model.trim();
    if !provider.is_empty() && !model.is_empty() {
        return format!("{provider} · {model}");
    }

    if !provider.is_empty() {
        return provider.to_string();
    }

    if credential_mode == RuntimeCredentialModeRecord::AuthProfile {
        return "Auth entry".to_string();
    }

    "Entry".to_string()
}

fn normalize_group_name(value: &str, provider: &str, model: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    let provider_key = provider.trim().to_ascii_lowercase();
    let model_key = model.trim().to_ascii_lowercase();
    if provider_key.contains("openai-codex")
        || model_key.contains("gpt-5")
        || model_key.contains("codex")
    {
        return "Codex".to_string();
    }
    if provider_key.contains("gemini") || model_key.contains("gemini") {
        return "Gemini".to_string();
    }
    if provider_key.contains("anthropic") || model_key.contains("claude") {
        return "Claude".to_string();
    }
    if provider_key.contains("openai") || model_key.contains("gpt-4") {
        return "OpenAI".to_string();
    }
    if provider_key.contains("ollama") {
        return "Ollama".to_string();
    }
    "General".to_string()
}

fn resolve_runtime_binding_from_groups(
    groups: &[RuntimeProviderGroupRecord],
    active_group_id: &str,
    group_id: &str,
    entry_id: &str,
) -> (String, String) {
    let fallback_group = groups
        .iter()
        .find(|group| group.id == active_group_id)
        .or_else(|| groups.first())
        .cloned()
        .unwrap_or_default();
    let group = groups
        .iter()
        .find(|group| group.id == group_id.trim())
        .cloned()
        .unwrap_or_else(|| fallback_group.clone());
    let entry = group
        .entries
        .iter()
        .find(|entry| entry.id == entry_id.trim())
        .cloned()
        .or_else(|| group.active_entry().cloned())
        .or_else(|| group.entries.first().cloned())
        .unwrap_or_default();

    (group.id, entry.id)
}

fn slugify_entry_name(name: &str) -> String {
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
        String::new()
    } else {
        slug
    }
}

fn slugify_group_name(name: &str) -> String {
    slugify_entry_name(name)
}

fn make_unique_entry_id(base: &str, seen_ids: &HashSet<String>) -> String {
    let base = if base.is_empty() {
        "entry".to_string()
    } else {
        slugify_entry_name(base)
    };

    if !seen_ids.contains(&base) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !seen_ids.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn make_unique_group_id(base: &str, seen_ids: &HashSet<String>) -> String {
    let base = if base.is_empty() {
        "group".to_string()
    } else {
        slugify_group_name(base)
    };

    if !seen_ids.contains(&base) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !seen_ids.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .flat_map(|value| {
            value
                .split(|character| character == ',' || character == '\n' || character == '\r')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn normalize_positive_usize(value: usize, fallback: usize) -> usize {
    if value == 0 {
        fallback
    } else {
        value
    }
}

fn normalize_positive_u32(value: u32, fallback: u32) -> u32 {
    if value == 0 {
        fallback
    } else {
        value
    }
}

fn normalize_positive_u64(value: u64, fallback: u64) -> u64 {
    if value == 0 {
        fallback
    } else {
        value
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let trimmed = entry.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_dispatcher(value: String) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "native" => "native".to_string(),
        "xml" => "xml".to_string(),
        _ => "auto".to_string(),
    }
}
