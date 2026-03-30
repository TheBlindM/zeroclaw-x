use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateSettingsRecord {
    pub enabled: bool,
    pub auto_check: bool,
    pub endpoints: Vec<String>,
    pub pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckReport {
    pub configured: bool,
    pub enabled: bool,
    pub auto_check: bool,
    pub checked_at: String,
    pub current_version: String,
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub download_url: Option<String>,
    pub endpoint_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInstallReport {
    pub installed: bool,
    pub version: Option<String>,
    pub checked_at: String,
    pub message: String,
}

impl Default for UpdateSettingsRecord {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_check: true,
            endpoints: Vec::new(),
            pubkey: String::new(),
        }
    }
}

impl UpdateSettingsRecord {
    pub fn normalized(mut self) -> Self {
        self.pubkey = self.pubkey.trim().to_string();
        self.endpoints = normalize_list(self.endpoints);
        self
    }

    pub fn is_configured(&self) -> bool {
        !self.pubkey.is_empty() && !self.endpoints.is_empty()
    }
}

fn normalize_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .flat_map(|value| {
            value
                .lines()
                .flat_map(|line| {
                    line.split(',')
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
