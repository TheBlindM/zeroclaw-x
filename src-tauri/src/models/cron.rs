use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobRecord {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRunRecord {
    pub id: String,
    pub job_id: String,
    pub status: String,
    pub output: String,
    pub started_at: String,
    pub finished_at: String,
}
