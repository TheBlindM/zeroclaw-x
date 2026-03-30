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
pub struct RuntimeAgentSettingsRecord {
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
pub struct RuntimeSettingsRecord {
    pub provider: String,
    pub model: String,
    pub provider_url: String,
    pub api_key: String,
    pub temperature: f64,
    pub proxy: RuntimeProxySettingsRecord,
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

impl Default for RuntimeAgentSettingsRecord {
    fn default() -> Self {
        Self {
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
        Self {
            provider: "openrouter".to_string(),
            model: "anthropic/claude-sonnet-4.6".to_string(),
            provider_url: String::new(),
            api_key: String::new(),
            temperature: 0.7,
            proxy: RuntimeProxySettingsRecord::default(),
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

impl RuntimeSettingsRecord {
    pub fn normalized(mut self) -> Self {
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

        if !self.temperature.is_finite() {
            self.temperature = Self::default().temperature;
        }
        self.temperature = self.temperature.clamp(0.0, 2.0);
        self.proxy = self.proxy.normalized();
        self.agent = self.agent.normalized();
        self.autonomy = self.autonomy.normalized();

        self
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

impl RuntimeAgentSettingsRecord {
    pub fn normalized(mut self) -> Self {
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

fn normalize_dispatcher(value: String) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "native" => "native".to_string(),
        "xml" => "xml".to_string(),
        _ => "auto".to_string(),
    }
}
