use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTokenPayload {
    pub session_id: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDonePayload {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatErrorPayload {
    pub session_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContextPayload {
    pub session_id: String,
    pub project_name: String,
    pub scope_mode: String,
    pub knowledge_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatApprovalRequestPayload {
    pub request_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub arguments_summary: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatApprovalDecision {
    Yes,
    No,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
    pub last_message_preview: Option<String>,
    pub project_id: Option<String>,
    pub agent_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
