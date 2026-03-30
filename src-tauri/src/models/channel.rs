use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub config_json: String,
    pub enabled: bool,
    pub last_checked_at: Option<String>,
    pub last_health_status: Option<String>,
    pub last_health_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDraft {
    pub name: String,
    pub kind: String,
    pub config_json: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelTestReport {
    pub ok: bool,
    pub kind: String,
    pub message: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelTestResult {
    pub channel: ChannelRecord,
    pub report: ChannelTestReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRuntimeStatusRecord {
    pub running: bool,
    pub state: String,
    pub message: String,
    pub updated_at: String,
}
