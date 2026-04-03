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
    pub directory_path: String,
    pub manifest_path: String,
    pub source_path: Option<String>,
    pub file_tree: Vec<SkillFileEntryRecord>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFileEntryRecord {
    pub name: String,
    pub relative_path: String,
    pub kind: String,
    pub editable: bool,
    pub previewable: bool,
    pub size_bytes: Option<u64>,
    pub children: Vec<SkillFileEntryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFileContentRecord {
    pub relative_path: String,
    pub content: String,
    pub editable: bool,
    pub previewable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntryDraft {
    pub parent_path: String,
    pub name: String,
    pub entry_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAssetImportReport {
    pub imported_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExportReport {
    pub path: String,
}
