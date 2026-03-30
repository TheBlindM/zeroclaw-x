use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDocumentRecord {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub source_path: String,
    pub content: String,
    pub content_preview: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKnowledgeScopeRecord {
    pub session_id: String,
    pub mode: String,
    pub document_ids: Vec<String>,
}
