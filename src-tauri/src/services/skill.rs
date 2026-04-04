use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use deunicode::deunicode;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::{
    db,
    models::skill::{
        SkillAssetImportReport, SkillDetailRecord, SkillDraft, SkillEntryDraft, SkillExportReport,
        SkillFileContentRecord, SkillFileEntryRecord, SkillRecord, SkillTemplateRecord,
    },
    state::AppState,
};

const STARTER_TEMPLATE_AUTHOR: &str = "ZeroClawX starter";
const IMPORTED_SKILL_FALLBACK_SLUG: &str = "imported-skill";
const STANDARD_SKILL_DIRECTORIES: &[&str] = &["scripts", "references", "assets"];

#[derive(Serialize, Deserialize)]
struct SkillTomlManifest {
    skill: SkillTomlMeta,
}

#[derive(Clone, Serialize, Deserialize)]
struct SkillTomlMeta {
    name: String,
    description: String,
    #[serde(default = "default_skill_version")]
    version: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

struct SkillTemplateSeed {
    template_id: &'static str,
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    author: &'static str,
    tags: &'static [&'static str],
    markdown: &'static str,
}

#[derive(Clone)]
struct ParsedSkillMetadata {
    slug: String,
    name: String,
    description: String,
    version: String,
    author: String,
    tags: Vec<String>,
}

#[derive(Clone)]
struct ParsedSkillDocument {
    metadata: ParsedSkillMetadata,
    markdown_content: String,
}

pub fn list_templates() -> Vec<SkillTemplateRecord> {
    starter_templates()
        .into_iter()
        .map(|template| SkillTemplateRecord {
            template_id: template.template_id.to_string(),
            slug: template.slug.to_string(),
            name: template.name.to_string(),
            description: template.description.to_string(),
            author: template.author.to_string(),
            tags_json: serde_json::to_string_pretty(&template.tags)
                .unwrap_or_else(|_| "[]".to_string()),
        })
        .collect()
}

pub fn list_skills(state: &AppState) -> Result<Vec<SkillRecord>, String> {
    db::list_skills(&state.db_path())
}

pub fn create_skill(state: &AppState, skill: &SkillDraft) -> Result<SkillRecord, String> {
    let document = draft_to_document(skill, None)?;
    let destination = skill_directory_from_db(&state.db_path(), &document.metadata.slug);

    ensure_skill_slug_available(&state.db_path(), &document.metadata.slug)?;
    if destination.exists() {
        return Err("Skill directory already exists on disk.".to_string());
    }

    materialize_skill_directory(&destination, &document)?;
    let record = upsert_skill_record(
        &state.db_path(),
        &document.metadata,
        "manual",
        "manual",
        skill.enabled,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn update_skill(
    state: &AppState,
    skill_id: &str,
    skill: &SkillDraft,
) -> Result<SkillRecord, String> {
    let existing =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let document = draft_to_document(skill, Some(existing.slug.as_str()))?;
    let destination = skill_directory(state, &existing.slug);

    materialize_skill_directory(&destination, &document)?;
    let record = upsert_skill_record(
        &state.db_path(),
        &document.metadata,
        &existing.source_kind,
        &existing.source_label,
        skill.enabled,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn get_skill_detail(state: &AppState, skill_id: &str) -> Result<SkillDetailRecord, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let directory = skill_directory(state, &skill.slug);
    skill_detail_from_record(&skill, &directory)
}

pub fn get_skill_file_content(
    state: &AppState,
    skill_id: &str,
    relative_path: &str,
) -> Result<SkillFileContentRecord, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let directory = skill_directory(state, &skill.slug);
    let path = resolve_skill_relative_path(&directory, relative_path)?;
    if !path.exists() || !path.is_file() {
        return Err("Skill file not found.".to_string());
    }

    let previewable = is_previewable_text_path(relative_path);
    if !previewable {
        return Err("This file type is not previewable in the built-in editor.".to_string());
    }

    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let content = String::from_utf8(bytes)
        .map_err(|_| "This file is not UTF-8 text and cannot be previewed here.".to_string())?;

    Ok(SkillFileContentRecord {
        relative_path: normalize_relative_path(relative_path)?,
        content,
        editable: is_editable_text_path(relative_path),
        previewable,
    })
}

pub fn save_skill_file_content(
    state: &AppState,
    skill_id: &str,
    relative_path: &str,
    content: &str,
) -> Result<SkillFileContentRecord, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let directory = skill_directory(state, &skill.slug);
    let path = resolve_skill_relative_path(&directory, relative_path)?;
    let normalized_relative_path = normalize_relative_path(relative_path)?;

    if !path.exists() || !path.is_file() {
        return Err("Skill file not found.".to_string());
    }

    if !is_editable_text_path(&normalized_relative_path) {
        return Err("This file cannot be edited in the built-in editor.".to_string());
    }

    let normalized_content = if normalized_relative_path.ends_with(".md")
        || normalized_relative_path.ends_with(".txt")
        || normalized_relative_path == "SKILL.md"
    {
        normalize_markdown(content)
    } else {
        content.to_string()
    };

    fs::write(&path, normalized_content.as_bytes()).map_err(|error| error.to_string())?;
    sync_runtime_skills(&state.db_path(), &state.settings_path())?;

    Ok(SkillFileContentRecord {
        relative_path: normalized_relative_path,
        content: normalized_content,
        editable: true,
        previewable: true,
    })
}

pub fn create_skill_entry(
    state: &AppState,
    skill_id: &str,
    draft: &SkillEntryDraft,
) -> Result<SkillDetailRecord, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let directory = skill_directory(state, &skill.slug);
    let parent_path = resolve_skill_relative_path(&directory, &draft.parent_path)?;
    if !parent_path.exists() || !parent_path.is_dir() {
        return Err("The selected parent folder does not exist.".to_string());
    }

    let entry_name = draft.name.trim();
    if entry_name.is_empty() {
        return Err("File or folder name is required.".to_string());
    }
    if entry_name.contains('/') || entry_name.contains('\\') {
        return Err(
            "Use the parent folder selector instead of path separators in the entry name."
                .to_string(),
        );
    }

    let destination = parent_path.join(entry_name);
    if destination.exists() {
        return Err("A file or folder with this name already exists.".to_string());
    }

    match draft.entry_kind.trim() {
        "directory" => fs::create_dir_all(&destination).map_err(|error| error.to_string())?,
        "file" => {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&destination, b"").map_err(|error| error.to_string())?;
        }
        _ => return Err("Entry kind must be \"file\" or \"directory\".".to_string()),
    }

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    skill_detail_from_record(&skill, &directory)
}

pub fn delete_skill_entry(
    state: &AppState,
    skill_id: &str,
    relative_path: &str,
) -> Result<SkillDetailRecord, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let directory = skill_directory(state, &skill.slug);
    let normalized_relative_path = normalize_relative_path(relative_path)?;
    if normalized_relative_path == "SKILL.md" || normalized_relative_path == "SKILL.toml" {
        return Err("Core skill files cannot be deleted.".to_string());
    }
    let path = resolve_skill_relative_path(&directory, &normalized_relative_path)?;
    if !path.exists() {
        return Err("Skill file or folder not found.".to_string());
    }

    if path.is_dir() {
        fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
    } else {
        fs::remove_file(&path).map_err(|error| error.to_string())?;
    }

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    skill_detail_from_record(&skill, &directory)
}

pub fn install_template(state: &AppState, template_id: &str) -> Result<SkillRecord, String> {
    let template = starter_templates()
        .into_iter()
        .find(|template| template.template_id == template_id)
        .ok_or_else(|| "Skill template not found.".to_string())?;
    let existing = ensure_template_install_allowed(&state.db_path(), &template)?;
    let document = template_document(&template, template.slug.to_string());
    let destination = skill_directory_from_db(&state.db_path(), template.slug);
    let enabled = existing
        .as_ref()
        .map(|record| record.enabled)
        .unwrap_or(true);

    materialize_skill_directory(&destination, &document)?;
    let record = upsert_skill_record(
        &state.db_path(),
        &document.metadata,
        "template",
        template.template_id,
        enabled,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn import_skill_directory(
    app: &AppHandle,
    state: &AppState,
) -> Result<Option<SkillRecord>, String> {
    let selected = app
        .dialog()
        .file()
        .set_title("Import skill directory")
        .blocking_pick_folder();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let source = selected
        .into_path()
        .map_err(|_| "Failed to resolve the selected skill directory.".to_string())?;
    let document = load_skill_document(&source, None)?;
    let destination = skill_directory_from_db(&state.db_path(), &document.metadata.slug);

    ensure_skill_slug_available(&state.db_path(), &document.metadata.slug)?;
    if destination.exists() {
        return Err("Skill directory already exists on disk.".to_string());
    }

    copy_directory_recursive(&source, &destination)?;
    materialize_skill_directory(&destination, &document)?;

    let record = upsert_skill_record(
        &state.db_path(),
        &document.metadata,
        "imported",
        &source.display().to_string(),
        true,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(Some(record))
}

pub fn duplicate_skill(state: &AppState, skill_id: &str) -> Result<SkillRecord, String> {
    let existing =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let source = skill_directory(state, &existing.slug);
    let existing_document = load_skill_document(&source, Some(existing.slug.clone()))?;
    let duplicated_name = next_duplicate_name(&state.db_path(), &existing.name)?;
    let duplicated_slug = next_duplicate_slug(&state.db_path(), &existing.slug)?;
    let duplicated_markdown = maybe_replace_markdown_title(
        &existing_document.markdown_content,
        &existing.name,
        &duplicated_name,
    );
    let duplicated_document = ParsedSkillDocument {
        metadata: ParsedSkillMetadata {
            slug: duplicated_slug.clone(),
            name: duplicated_name,
            description: existing.description.clone(),
            version: existing.version.clone(),
            author: existing.author.clone(),
            tags: parse_tags_json(&existing.tags_json)?,
        },
        markdown_content: duplicated_markdown,
    };
    let destination = skill_directory_from_db(&state.db_path(), &duplicated_slug);

    copy_directory_recursive(&source, &destination)?;
    materialize_skill_directory(&destination, &duplicated_document)?;

    let record = upsert_skill_record(
        &state.db_path(),
        &duplicated_document.metadata,
        "manual",
        "manual",
        existing.enabled,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn refresh_skill(state: &AppState, skill_id: &str) -> Result<SkillRecord, String> {
    let existing =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let destination = skill_directory(state, &existing.slug);

    let document = match existing.source_kind.as_str() {
        "imported" => {
            let source = PathBuf::from(existing.source_label.trim());
            if !source.exists() {
                return Err("The original imported skill directory no longer exists.".to_string());
            }
            let document = load_skill_document(&source, Some(existing.slug.clone()))?;
            reset_directory(&destination)?;
            copy_directory_recursive(&source, &destination)?;
            document
        }
        "template" => {
            let template = starter_templates()
                .into_iter()
                .find(|template| template.template_id == existing.source_label)
                .ok_or_else(|| "The original skill template is no longer available.".to_string())?;
            reset_directory(&destination)?;
            template_document(&template, existing.slug.clone())
        }
        _ => load_skill_document(&destination, Some(existing.slug.clone()))?,
    };

    materialize_skill_directory(&destination, &document)?;
    let record = upsert_skill_record(
        &state.db_path(),
        &document.metadata,
        &existing.source_kind,
        &existing.source_label,
        existing.enabled,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn export_skill(
    app: &AppHandle,
    state: &AppState,
    skill_id: &str,
) -> Result<Option<SkillExportReport>, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let source = skill_directory(state, &skill.slug);

    let selected = app
        .dialog()
        .file()
        .set_title("Export skill directory")
        .blocking_pick_folder();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let parent = selected
        .into_path()
        .map_err(|_| "Failed to resolve the selected export directory.".to_string())?;
    let destination = parent.join(&skill.slug);

    if destination.exists() {
        return Err(
            "The selected export location already contains a folder with this skill slug."
                .to_string(),
        );
    }

    copy_directory_recursive(&source, &destination)?;

    Ok(Some(SkillExportReport {
        path: destination.display().to_string(),
    }))
}

pub fn import_skill_assets(
    app: &AppHandle,
    state: &AppState,
    skill_id: &str,
) -> Result<Option<SkillAssetImportReport>, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let destination_root = skill_directory(state, &skill.slug).join("assets");
    fs::create_dir_all(&destination_root).map_err(|error| error.to_string())?;

    let selected = app
        .dialog()
        .file()
        .set_title("Import asset files")
        .blocking_pick_files();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let paths = selected
        .into_iter()
        .map(|entry| {
            entry
                .into_path()
                .map_err(|_| "Failed to resolve one of the selected asset paths.".to_string())
        })
        .collect::<Result<Vec<PathBuf>, _>>()?;

    let mut imported_paths = Vec::new();
    for source_path in paths {
        if !source_path.is_file() {
            continue;
        }
        let file_name = source_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "Failed to resolve one of the selected asset names.".to_string())?;
        let destination_path = next_available_copy_path(&destination_root, file_name);
        fs::copy(&source_path, &destination_path)
            .map_err(|error| format!("Failed to import {}: {error}", source_path.display()))?;
        let relative_path = destination_path
            .strip_prefix(skill_directory(state, &skill.slug))
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        imported_paths.push(relative_path);
    }

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(Some(SkillAssetImportReport { imported_paths }))
}

pub fn open_skill_directory(state: &AppState, skill_id: &str) -> Result<String, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let directory = skill_directory(state, &skill.slug);

    if !directory.exists() {
        return Err("Skill directory not found on disk.".to_string());
    }

    open_path_in_file_manager(&directory)?;
    Ok(directory.display().to_string())
}

pub fn set_skill_enabled(
    state: &AppState,
    skill_id: &str,
    enabled: bool,
) -> Result<SkillRecord, String> {
    let record = db::set_skill_enabled(&state.db_path(), skill_id, enabled)?;
    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn delete_skill(state: &AppState, skill_id: &str) -> Result<SkillRecord, String> {
    let record = db::delete_skill(&state.db_path(), skill_id)?;
    let path = skill_directory(state, &record.slug);
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
    }
    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn sync_runtime_skills(db_path: &Path, settings_path: &Path) -> Result<(), String> {
    let workspace_dir = super::runtime::resolve_runtime_workspace_dir(db_path, settings_path)?;
    sync_runtime_skills_to_workspace(db_path, &workspace_dir)
}

pub fn sync_runtime_skills_to_workspace(
    db_path: &Path,
    workspace_dir: &Path,
) -> Result<(), String> {
    let runtime_skills = runtime_skills_root_from_workspace(workspace_dir);
    if runtime_skills.exists() {
        fs::remove_dir_all(&runtime_skills).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&runtime_skills).map_err(|error| error.to_string())?;

    let library_root = library_root_from_db(db_path)?;
    for skill in db::list_skills(db_path)? {
        if !skill.enabled {
            continue;
        }

        let source = library_root.join(&skill.slug);
        if !source.exists() {
            continue;
        }

        let destination = runtime_skills.join(&skill.slug);
        copy_directory_recursive(&source, &destination)?;
    }

    Ok(())
}

fn ensure_skill_slug_available(db_path: &Path, slug: &str) -> Result<(), String> {
    if db::get_skill_by_slug(db_path, slug)?.is_some() {
        return Err(
            "Skill slug already exists. Rename, duplicate, or delete the existing skill first."
                .to_string(),
        );
    }

    Ok(())
}

fn ensure_template_install_allowed(
    db_path: &Path,
    template: &SkillTemplateSeed,
) -> Result<Option<SkillRecord>, String> {
    let existing = db::get_skill_by_slug(db_path, template.slug)?;

    if let Some(record) = &existing {
        if record.source_kind != "template" || record.source_label != template.template_id {
            return Err(
                "A different skill already uses this template slug. Delete, duplicate, or rename the existing skill before installing the template."
                    .to_string(),
            );
        }
    }

    Ok(existing)
}

fn upsert_skill_record(
    db_path: &Path,
    metadata: &ParsedSkillMetadata,
    source_kind: &str,
    source_label: &str,
    enabled: bool,
) -> Result<SkillRecord, String> {
    db::upsert_skill(
        db_path,
        &metadata.slug,
        &metadata.name,
        &metadata.description,
        &metadata.version,
        &metadata.author,
        &serde_json::to_string_pretty(&metadata.tags).map_err(|error| error.to_string())?,
        source_kind,
        source_label,
        enabled,
    )
}

fn starter_templates() -> Vec<SkillTemplateSeed> {
    vec![
        SkillTemplateSeed {
            template_id: "code-review",
            slug: "code-review",
            name: "Code Review",
            description: "Review a change set for bugs, regressions, and missing tests before merging.",
            author: STARTER_TEMPLATE_AUTHOR,
            tags: &["quality", "review", "engineering"],
            markdown: "# Code Review\nReview a code change for bugs, regressions, edge cases, and missing tests.\n\nWhen invoked:\n- Focus on correctness and risk before style.\n- Call out user-visible regressions clearly.\n- Suggest targeted tests for the riskiest paths.\n",
        },
        SkillTemplateSeed {
            template_id: "repo-summary",
            slug: "repo-summary",
            name: "Repository Summary",
            description: "Summarize a codebase quickly for onboarding, handoffs, and architecture reviews.",
            author: STARTER_TEMPLATE_AUTHOR,
            tags: &["summary", "onboarding", "architecture"],
            markdown: "# Repository Summary\nSummarize the repository for fast onboarding.\n\nWhen invoked:\n- Explain the product purpose first.\n- Map the main directories and ownership boundaries.\n- Call out risky subsystems and key workflows.\n",
        },
        SkillTemplateSeed {
            template_id: "release-notes",
            slug: "release-notes",
            name: "Release Notes",
            description: "Turn a batch of changes into concise release notes for internal or customer-facing updates.",
            author: STARTER_TEMPLATE_AUTHOR,
            tags: &["release", "product", "communication"],
            markdown: "# Release Notes\nTurn recent changes into polished release notes.\n\nWhen invoked:\n- Group changes by user impact.\n- Keep language concise and concrete.\n- Flag breaking changes and follow-up actions.\n",
        },
    ]
}

fn library_root_from_db(db_path: &Path) -> Result<PathBuf, String> {
    let root = db_path
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "Failed to resolve app data directory for skills.".to_string())?
        .join("skills-library");
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    Ok(root)
}

fn skill_directory_from_db(db_path: &Path, slug: &str) -> PathBuf {
    db_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("skills-library")
        .join(slug)
}

fn runtime_skills_root_from_workspace(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("skills")
}

fn skill_directory(state: &AppState, slug: &str) -> PathBuf {
    skill_directory_from_db(&state.db_path(), slug)
}

fn imported_source_path(skill: &SkillRecord) -> Option<String> {
    (skill.source_kind == "imported").then(|| skill.source_label.clone())
}

fn skill_detail_from_record(
    skill: &SkillRecord,
    directory: &Path,
) -> Result<SkillDetailRecord, String> {
    let document = load_skill_document(directory, Some(skill.slug.clone()))?;
    Ok(SkillDetailRecord {
        skill: skill.clone(),
        markdown_content: document.markdown_content,
        directory_path: directory.display().to_string(),
        manifest_path: directory.join("SKILL.toml").display().to_string(),
        source_path: imported_source_path(skill),
        file_tree: load_skill_file_tree(directory)?,
    })
}

fn template_document(template: &SkillTemplateSeed, slug: String) -> ParsedSkillDocument {
    ParsedSkillDocument {
        metadata: ParsedSkillMetadata {
            slug,
            name: template.name.to_string(),
            description: template.description.to_string(),
            version: default_skill_version(),
            author: template.author.to_string(),
            tags: normalize_tags(template.tags.iter().map(|tag| (*tag).to_string()).collect()),
        },
        markdown_content: normalize_markdown(template.markdown),
    }
}

fn draft_to_document(
    skill: &SkillDraft,
    fixed_slug: Option<&str>,
) -> Result<ParsedSkillDocument, String> {
    let name = skill.name.trim();
    if name.is_empty() {
        return Err("Skill name is required.".to_string());
    }

    let description = skill.description.trim();
    if description.is_empty() {
        return Err("Skill description is required.".to_string());
    }

    let markdown_content = normalize_markdown(&skill.markdown_content);
    if markdown_content.trim().is_empty() {
        return Err("Skill instructions are required.".to_string());
    }

    let slug = match fixed_slug {
        Some(existing_slug) => {
            if !skill.slug.trim().is_empty() && normalize_slug(skill.slug.trim()) != existing_slug {
                return Err("Renaming skill slugs is not supported yet.".to_string());
            }
            existing_slug.to_string()
        }
        None => {
            let slug_source = if skill.slug.trim().is_empty() {
                name
            } else {
                skill.slug.trim()
            };
            normalize_slug(slug_source)
        }
    };

    let version = if skill.version.trim().is_empty() {
        default_skill_version()
    } else {
        skill.version.trim().to_string()
    };

    Ok(ParsedSkillDocument {
        metadata: ParsedSkillMetadata {
            slug,
            name: name.to_string(),
            description: description.to_string(),
            version,
            author: skill.author.trim().to_string(),
            tags: parse_tags_json(&skill.tags_json)?,
        },
        markdown_content,
    })
}

fn load_skill_document(
    path: &Path,
    preferred_slug: Option<String>,
) -> Result<ParsedSkillDocument, String> {
    let markdown_path = path.join("SKILL.md");
    let toml_path = path.join("SKILL.toml");
    let markdown = if markdown_path.exists() {
        Some(fs::read_to_string(&markdown_path).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let metadata = if toml_path.exists() {
        Some(parse_skill_manifest(&toml_path, preferred_slug.clone())?)
    } else if let Some(markdown) = &markdown {
        Some(parse_skill_markdown(markdown, preferred_slug)?)
    } else {
        None
    };

    let Some(metadata) = metadata else {
        return Err("Skill directory must contain SKILL.md or SKILL.toml at its root.".to_string());
    };

    Ok(ParsedSkillDocument {
        markdown_content: markdown
            .as_deref()
            .map(normalize_markdown)
            .unwrap_or_else(|| render_markdown_from_metadata(&metadata)),
        metadata,
    })
}

fn parse_skill_manifest(
    path: &Path,
    preferred_slug: Option<String>,
) -> Result<ParsedSkillMetadata, String> {
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let manifest = toml::from_str::<SkillTomlManifest>(&raw)
        .map_err(|error| format!("Failed to parse SKILL.toml: {error}"))?;
    let slug = normalize_slug(preferred_slug.as_deref().unwrap_or(&manifest.skill.name));

    Ok(ParsedSkillMetadata {
        slug,
        name: manifest.skill.name,
        description: manifest.skill.description,
        version: manifest.skill.version,
        author: manifest.skill.author.unwrap_or_default(),
        tags: normalize_tags(manifest.skill.tags),
    })
}

fn parse_skill_markdown(
    markdown: &str,
    preferred_slug: Option<String>,
) -> Result<ParsedSkillMetadata, String> {
    let mut title = None;
    let mut description = None;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if title.is_none() && trimmed.starts_with('#') {
            title = Some(trimmed.trim_start_matches('#').trim().to_string());
            continue;
        }

        if description.is_none() && !trimmed.is_empty() && !trimmed.starts_with('#') {
            description = Some(trimmed.to_string());
        }

        if title.is_some() && description.is_some() {
            break;
        }
    }

    let name = title.unwrap_or_else(|| "Imported Skill".to_string());
    Ok(ParsedSkillMetadata {
        slug: normalize_slug(preferred_slug.as_deref().unwrap_or(&name)),
        name,
        description: description.unwrap_or_else(|| "No description provided.".to_string()),
        version: default_skill_version(),
        author: String::new(),
        tags: Vec::new(),
    })
}

fn materialize_skill_directory(path: &Path, document: &ParsedSkillDocument) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    write_skill_markdown(path, &document.markdown_content)?;
    write_skill_manifest(path, &document.metadata)?;
    ensure_standard_skill_directories(path)?;
    Ok(())
}

fn write_skill_markdown(path: &Path, markdown: &str) -> Result<(), String> {
    fs::write(path.join("SKILL.md"), normalize_markdown(markdown))
        .map_err(|error| error.to_string())
}

fn write_skill_manifest(path: &Path, metadata: &ParsedSkillMetadata) -> Result<(), String> {
    let manifest = SkillTomlManifest {
        skill: SkillTomlMeta {
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            version: metadata.version.clone(),
            author: (!metadata.author.trim().is_empty()).then(|| metadata.author.clone()),
            tags: metadata.tags.clone(),
        },
    };
    let serialized = toml::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(path.join("SKILL.toml"), serialized).map_err(|error| error.to_string())
}

fn render_markdown_from_metadata(metadata: &ParsedSkillMetadata) -> String {
    let mut rendered = format!(
        "# {}\n{}\n",
        metadata.name.trim(),
        metadata.description.trim()
    );
    if !metadata.author.trim().is_empty() || !metadata.version.trim().is_empty() {
        rendered.push('\n');
        if !metadata.version.trim().is_empty() {
            rendered.push_str(&format!("Version: {}\n", metadata.version.trim()));
        }
        if !metadata.author.trim().is_empty() {
            rendered.push_str(&format!("Author: {}\n", metadata.author.trim()));
        }
    }
    rendered
}

fn next_duplicate_name(db_path: &Path, base_name: &str) -> Result<String, String> {
    let existing_names = db::list_skills(db_path)?
        .into_iter()
        .map(|skill| skill.name)
        .collect::<BTreeSet<_>>();
    let base = base_name.trim();
    let mut index = 1;

    loop {
        let candidate = if index == 1 {
            format!("{base} Copy")
        } else {
            format!("{base} Copy {index}")
        };

        if !existing_names.contains(&candidate) {
            return Ok(candidate);
        }

        index += 1;
    }
}

fn next_duplicate_slug(db_path: &Path, base_slug: &str) -> Result<String, String> {
    let base = normalize_slug(base_slug);
    let mut index = 1;

    loop {
        let candidate = if index == 1 {
            format!("{base}-copy")
        } else {
            format!("{base}-copy-{index}")
        };

        if db::get_skill_by_slug(db_path, &candidate)?.is_none() {
            return Ok(candidate);
        }

        index += 1;
    }
}

fn maybe_replace_markdown_title(markdown: &str, old_name: &str, new_name: &str) -> String {
    let mut lines = normalize_markdown(markdown)
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();

    if let Some(first) = lines.first_mut() {
        if first.trim() == format!("# {}", old_name.trim()) {
            *first = format!("# {}", new_name.trim());
        }
    }

    normalize_markdown(&lines.join("\n"))
}

fn open_path_in_file_manager(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");

    command.arg(path);
    command
        .status()
        .map_err(|error| error.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err("Failed to open the skill directory in the system file manager.".to_string())
            }
        })
}

fn reset_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(path).map_err(|error| error.to_string())
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;

    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else if source_path.is_file() {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::copy(&source_path, &destination_path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut unique = BTreeSet::new();

    for tag in tags {
        let normalized = tag.trim().to_lowercase();
        if !normalized.is_empty() {
            unique.insert(normalized);
        }
    }

    unique.into_iter().collect()
}

fn parse_tags_json(tags_json: &str) -> Result<Vec<String>, String> {
    if tags_json.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed = serde_json::from_str::<Vec<String>>(tags_json)
        .map_err(|error| format!("Failed to parse skill tags JSON: {error}"))?;

    Ok(normalize_tags(parsed))
}

fn normalize_slug(value: &str) -> String {
    let slug = slugify(value);
    if slug.is_empty() {
        IMPORTED_SKILL_FALLBACK_SLUG.to_string()
    } else {
        slug
    }
}

fn normalize_markdown(markdown: &str) -> String {
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

fn ensure_standard_skill_directories(path: &Path) -> Result<(), String> {
    for directory in STANDARD_SKILL_DIRECTORIES {
        fs::create_dir_all(path.join(directory)).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn load_skill_file_tree(path: &Path) -> Result<Vec<SkillFileEntryRecord>, String> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let relative_path = name.clone();
        let full_path = entry.path();
        let metadata = entry.metadata().map_err(|error| error.to_string())?;

        let children = if file_type.is_dir() {
            load_skill_file_tree_recursive(path, &full_path)?
        } else {
            Vec::new()
        };

        entries.push(SkillFileEntryRecord {
            name,
            relative_path: relative_path.clone(),
            kind: if file_type.is_dir() {
                "directory".to_string()
            } else {
                "file".to_string()
            },
            editable: file_type.is_file() && is_editable_text_path(&relative_path),
            previewable: file_type.is_file() && is_previewable_text_path(&relative_path),
            size_bytes: file_type.is_file().then_some(metadata.len()),
            children,
        });
    }

    sort_skill_file_entries(&mut entries);
    Ok(entries)
}

fn load_skill_file_tree_recursive(
    root: &Path,
    directory: &Path,
) -> Result<Vec<SkillFileEntryRecord>, String> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let full_path = entry.path();
        let relative_path = full_path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = entry.metadata().map_err(|error| error.to_string())?;

        let children = if file_type.is_dir() {
            load_skill_file_tree_recursive(root, &full_path)?
        } else {
            Vec::new()
        };

        entries.push(SkillFileEntryRecord {
            name: entry.file_name().to_string_lossy().to_string(),
            relative_path: relative_path.clone(),
            kind: if file_type.is_dir() {
                "directory".to_string()
            } else {
                "file".to_string()
            },
            editable: file_type.is_file() && is_editable_text_path(&relative_path),
            previewable: file_type.is_file() && is_previewable_text_path(&relative_path),
            size_bytes: file_type.is_file().then_some(metadata.len()),
            children,
        });
    }

    sort_skill_file_entries(&mut entries);
    Ok(entries)
}

fn sort_skill_file_entries(entries: &mut [SkillFileEntryRecord]) {
    entries.sort_by(|left, right| {
        skill_entry_order(&left.relative_path, &left.kind)
            .cmp(&skill_entry_order(&right.relative_path, &right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}

fn skill_entry_order(relative_path: &str, kind: &str) -> (u8, u8) {
    let root_name = relative_path.split('/').next().unwrap_or(relative_path);
    let bucket = match root_name {
        "SKILL.md" => 0,
        "SKILL.toml" => 1,
        "scripts" => 2,
        "references" => 3,
        "assets" => 4,
        "agents" => 5,
        _ => 6,
    };
    let kind_order = if kind == "directory" { 0 } else { 1 };
    (bucket, kind_order)
}

fn normalize_relative_path(relative_path: &str) -> Result<String, String> {
    let path = relative_path.trim();
    if path.is_empty() {
        return Err("A relative path is required.".to_string());
    }

    let mut normalized = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => normalized.push(part),
            _ => {
                return Err(
                    "Only relative paths inside the skill directory are allowed.".to_string(),
                )
            }
        }
    }

    let normalized_string = normalized.to_string_lossy().replace('\\', "/");
    if normalized_string.is_empty() {
        Err("A relative path is required.".to_string())
    } else {
        Ok(normalized_string)
    }
}

fn resolve_skill_relative_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    if relative_path.trim().is_empty() {
        return Ok(root.to_path_buf());
    }

    Ok(root.join(normalize_relative_path(relative_path)?))
}

fn is_previewable_text_path(relative_path: &str) -> bool {
    matches!(
        Path::new(relative_path)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some(
            "md" | "markdown"
                | "txt"
                | "json"
                | "toml"
                | "yaml"
                | "yml"
                | "rs"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "vue"
                | "py"
                | "sh"
                | "bash"
                | "zsh"
                | "css"
                | "html"
        )
    ) || relative_path == "SKILL.md"
        || relative_path == "SKILL.toml"
}

fn is_editable_text_path(relative_path: &str) -> bool {
    if relative_path == "SKILL.toml" {
        return false;
    }

    is_previewable_text_path(relative_path)
}

fn next_available_copy_path(root: &Path, file_name: &str) -> PathBuf {
    let candidate = root.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    let extension = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();

    let mut index = 2;
    loop {
        let candidate = root.join(format!("{stem}-{index}{extension}"));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn slugify(value: &str) -> String {
    let transliterated = deunicode(value);
    let mut slug = transliterated
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }

    slug.trim_matches('-').to_string()
}

fn default_skill_version() -> String {
    "0.1.0".to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::state::AppState;

    fn make_test_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}"))
    }

    #[test]
    fn create_and_update_skill_materialize_manifest_and_markdown() {
        let app_dir = make_test_dir("zeroclawx-skill-create-update");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");
        let created = create_skill(
            &state,
            &SkillDraft {
                slug: String::new(),
                name: "Repository Triage".to_string(),
                description: "Summarize a repository.".to_string(),
                version: String::new(),
                author: "Team Zero".to_string(),
                tags_json: "[\"review\", \"triage\"]".to_string(),
                markdown_content: "# Repository Triage\nSummarize a repository.\n".to_string(),
                enabled: true,
            },
        )
        .expect("skill should create");

        let skill_dir = skill_directory(&state, &created.slug);
        assert!(skill_dir.join("SKILL.md").exists());
        assert!(skill_dir.join("SKILL.toml").exists());

        let updated = update_skill(
            &state,
            &created.id,
            &SkillDraft {
                slug: created.slug.clone(),
                name: "Repository Triage".to_string(),
                description: "Summarize a repository with action items.".to_string(),
                version: "1.2.3".to_string(),
                author: "Team Zero".to_string(),
                tags_json: "[\"triage\", \"action\"]".to_string(),
                markdown_content:
                    "# Repository Triage\nSummarize a repository with action items.\n".to_string(),
                enabled: false,
            },
        )
        .expect("skill should update");

        let detail = get_skill_detail(&state, &created.id).expect("detail should load");
        let manifest =
            fs::read_to_string(skill_dir.join("SKILL.toml")).expect("manifest should read");
        assert!(manifest.contains("version = \"1.2.3\""));
        assert!(detail.markdown_content.contains("action items"));
        assert_eq!(updated.enabled, false);
        assert_eq!(detail.skill.version, "1.2.3");

        let rename_attempt = update_skill(
            &state,
            &created.id,
            &SkillDraft {
                slug: "renamed-skill".to_string(),
                name: "Repository Triage".to_string(),
                description: "Summarize a repository with action items.".to_string(),
                version: "1.2.3".to_string(),
                author: "Team Zero".to_string(),
                tags_json: "[\"triage\", \"action\"]".to_string(),
                markdown_content:
                    "# Repository Triage\nSummarize a repository with action items.\n".to_string(),
                enabled: false,
            },
        )
        .expect_err("slug rename should be rejected");
        assert!(rename_attempt.contains("Renaming skill slugs"));

        let _ = fs::remove_dir_all(app_dir);
    }

    #[test]
    fn template_install_rejects_slug_conflicts_and_duplicate_creates_manual_copy() {
        let app_dir = make_test_dir("zeroclawx-skill-template-conflict");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");

        create_skill(
            &state,
            &SkillDraft {
                slug: "code-review".to_string(),
                name: "Code Review".to_string(),
                description: "Custom code review workflow.".to_string(),
                version: String::new(),
                author: String::new(),
                tags_json: "[]".to_string(),
                markdown_content: "# Code Review\nCustom code review workflow.\n".to_string(),
                enabled: true,
            },
        )
        .expect("manual skill should create");

        let install_error =
            install_template(&state, "code-review").expect_err("template should not overwrite");
        assert!(install_error.contains("already uses this template slug"));

        let created = create_skill(
            &state,
            &SkillDraft {
                slug: "repo-summary".to_string(),
                name: "Repository Summary".to_string(),
                description: "Summarize the codebase quickly.".to_string(),
                version: String::new(),
                author: String::new(),
                tags_json: "[]".to_string(),
                markdown_content: "# Repository Summary\nSummarize the codebase quickly.\n"
                    .to_string(),
                enabled: true,
            },
        )
        .expect("source skill should create");

        let duplicated = duplicate_skill(&state, &created.id).expect("duplicate should create");
        let duplicated_detail =
            get_skill_detail(&state, &duplicated.id).expect("duplicate detail should load");
        assert_eq!(duplicated.source_kind, "manual");
        assert_ne!(duplicated.slug, created.slug);
        assert!(duplicated_detail
            .markdown_content
            .contains("# Repository Summary Copy"));

        let _ = fs::remove_dir_all(app_dir);
    }

    #[test]
    fn refresh_imported_skill_reloads_source_directory() {
        let app_dir = make_test_dir("zeroclawx-skill-refresh-imported");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");
        let source_dir = app_dir.join("external-skill");
        fs::create_dir_all(&source_dir).expect("source dir should create");
        fs::write(
            source_dir.join("SKILL.md"),
            "# Imported Skill\nOriginal description.\n",
        )
        .expect("source markdown should write");

        let imported_document =
            load_skill_document(&source_dir, Some("imported-skill".to_string()))
                .expect("document should parse");
        let imported_dir = skill_directory(&state, &imported_document.metadata.slug);
        copy_directory_recursive(&source_dir, &imported_dir).expect("source should copy");
        materialize_skill_directory(&imported_dir, &imported_document)
            .expect("imported skill should normalize");
        let imported = upsert_skill_record(
            &state.db_path(),
            &imported_document.metadata,
            "imported",
            &source_dir.display().to_string(),
            true,
        )
        .expect("skill record should save");

        fs::write(
            source_dir.join("SKILL.md"),
            "# Imported Skill\nUpdated description from source.\n",
        )
        .expect("source markdown should update");

        let refreshed = refresh_skill(&state, &imported.id).expect("refresh should succeed");
        let detail = get_skill_detail(&state, &imported.id).expect("detail should load");
        assert_eq!(refreshed.slug, imported.slug);
        assert!(detail
            .markdown_content
            .contains("Updated description from source"));
        assert!(imported_dir.join("SKILL.toml").exists());

        let _ = fs::remove_dir_all(app_dir);
    }

    #[test]
    fn create_skill_transliterates_non_ascii_name_into_unique_slug() {
        let app_dir = make_test_dir("zeroclawx-skill-unicode-slug");
        let state = AppState::new(app_dir.clone()).expect("state should initialize");

        let created = create_skill(
            &state,
            &SkillDraft {
                slug: String::new(),
                name: "我的技能".to_string(),
                description: "第一个中文技能".to_string(),
                version: String::new(),
                author: String::new(),
                tags_json: "[]".to_string(),
                markdown_content: "# 我的技能\n第一个中文技能\n".to_string(),
                enabled: true,
            },
        )
        .expect("unicode skill should create");

        assert_ne!(created.slug, IMPORTED_SKILL_FALLBACK_SLUG);
        assert!(!created.slug.trim().is_empty());

        let second = create_skill(
            &state,
            &SkillDraft {
                slug: String::new(),
                name: "另一个技能".to_string(),
                description: "第二个中文技能".to_string(),
                version: String::new(),
                author: String::new(),
                tags_json: "[]".to_string(),
                markdown_content: "# 另一个技能\n第二个中文技能\n".to_string(),
                enabled: true,
            },
        )
        .expect("second unicode skill should also create");

        assert_ne!(second.slug, IMPORTED_SKILL_FALLBACK_SLUG);
        assert_ne!(second.slug, created.slug);

        let _ = fs::remove_dir_all(app_dir);
    }
}
