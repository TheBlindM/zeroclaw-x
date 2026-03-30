use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}
