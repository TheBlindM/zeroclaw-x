use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecord {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags_json: String,
    pub source_kind: String,
    pub source_label: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTemplateRecord {
    pub template_id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub tags_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetailRecord {
    pub skill: SkillRecord,
    pub markdown_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDraft {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags_json: String,
    pub markdown_content: String,
    pub enabled: bool,
}
