use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProfileRecord {
    pub id: String,
    pub provider: String,
    pub profile_name: String,
    pub kind: String,
    pub account_id: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProfilesStateRecord {
    pub provider: String,
    pub active_profile_id: Option<String>,
    pub profiles: Vec<AuthProfileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthLoginChallengeRecord {
    pub login_id: String,
    pub provider: String,
    pub profile_name: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub user_code: String,
    pub expires_at: String,
    pub interval_seconds: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthLoginStatusRecord {
    pub login_id: String,
    pub provider: String,
    pub profile_name: String,
    pub status: String,
    pub message: String,
    pub completed_profile_id: Option<String>,
    pub completed_at: Option<String>,
}
