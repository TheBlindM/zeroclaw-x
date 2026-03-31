use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::{
    db,
    models::skill::{SkillDetailRecord, SkillDraft, SkillRecord, SkillTemplateRecord},
    state::AppState,
};

const STARTER_TEMPLATE_AUTHOR: &str = "ZeroClawX starter";
const IMPORTED_SKILL_FALLBACK_SLUG: &str = "imported-skill";

#[derive(Deserialize)]
struct SkillTomlManifest {
    skill: SkillTomlMeta,
}

#[derive(Deserialize)]
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

struct ParsedSkillMetadata {
    slug: String,
    name: String,
    description: String,
    version: String,
    author: String,
    tags: Vec<String>,
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
    let name = skill.name.trim();
    if name.is_empty() {
        return Err("Skill name is required.".to_string());
    }

    let description = skill.description.trim();
    if description.is_empty() {
        return Err("Skill description is required.".to_string());
    }

    let markdown_content = skill.markdown_content.trim();
    if markdown_content.is_empty() {
        return Err("Skill instructions are required.".to_string());
    }

    let slug_source = if skill.slug.trim().is_empty() {
        name
    } else {
        skill.slug.trim()
    };
    let slug = normalize_slug(slug_source);

    if db::get_skill_by_slug(&state.db_path(), &slug)?.is_some() {
        return Err("Skill slug already exists.".to_string());
    }

    let destination = library_root_from_db(&state.db_path())?.join(&slug);
    if destination.exists() {
        return Err("Skill directory already exists on disk.".to_string());
    }

    let version = if skill.version.trim().is_empty() {
        default_skill_version()
    } else {
        skill.version.trim().to_string()
    };
    let tags = parse_tags_json(&skill.tags_json)?;

    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    fs::write(destination.join("SKILL.md"), markdown_content).map_err(|error| error.to_string())?;

    let record = upsert_skill_record(
        &state.db_path(),
        &ParsedSkillMetadata {
            slug,
            name: name.to_string(),
            description: description.to_string(),
            version,
            author: skill.author.trim().to_string(),
            tags,
        },
        "manual",
        "manual",
        skill.enabled,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(record)
}

pub fn get_skill_detail(state: &AppState, skill_id: &str) -> Result<SkillDetailRecord, String> {
    let skill =
        db::get_skill(&state.db_path(), skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let markdown_content = read_skill_markdown(&skill_directory(state, &skill.slug))?;

    Ok(SkillDetailRecord {
        skill,
        markdown_content,
    })
}

pub fn install_template(state: &AppState, template_id: &str) -> Result<SkillRecord, String> {
    let template = starter_templates()
        .into_iter()
        .find(|template| template.template_id == template_id)
        .ok_or_else(|| "Skill template not found.".to_string())?;
    let destination = library_root_from_db(&state.db_path())?.join(template.slug);

    reset_directory(&destination)?;
    fs::write(destination.join("SKILL.md"), template.markdown)
        .map_err(|error| error.to_string())?;

    let metadata = parse_skill_directory(&destination, Some(template.slug.to_string()))?;
    let record = upsert_skill_record(
        &state.db_path(),
        &metadata,
        "template",
        template.template_id,
        true,
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
    let metadata = parse_skill_directory(&source, None)?;
    let destination = library_root_from_db(&state.db_path())?.join(&metadata.slug);

    reset_directory(&destination)?;
    copy_directory_recursive(&source, &destination)?;

    let refreshed = parse_skill_directory(&destination, Some(metadata.slug.clone()))?;
    let record = upsert_skill_record(
        &state.db_path(),
        &refreshed,
        "imported",
        &source.display().to_string(),
        true,
    )?;

    sync_runtime_skills(&state.db_path(), &state.settings_path())?;
    Ok(Some(record))
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

fn runtime_skills_root_from_workspace(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("skills")
}

fn skill_directory(state: &AppState, slug: &str) -> PathBuf {
    state
        .db_path()
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("skills-library")
        .join(slug)
}

fn parse_skill_directory(
    path: &Path,
    preferred_slug: Option<String>,
) -> Result<ParsedSkillMetadata, String> {
    let markdown_path = path.join("SKILL.md");
    let toml_path = path.join("SKILL.toml");

    if markdown_path.exists() {
        let markdown = fs::read_to_string(&markdown_path).map_err(|error| error.to_string())?;
        return parse_skill_markdown(
            &markdown,
            preferred_slug.or_else(|| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(str::to_string)
            }),
        );
    }

    if toml_path.exists() {
        let raw = fs::read_to_string(&toml_path).map_err(|error| error.to_string())?;
        let manifest = toml::from_str::<SkillTomlManifest>(&raw)
            .map_err(|error| format!("Failed to parse SKILL.toml: {error}"))?;
        let slug = normalize_slug(preferred_slug.as_deref().unwrap_or(&manifest.skill.name));

        return Ok(ParsedSkillMetadata {
            slug,
            name: manifest.skill.name,
            description: manifest.skill.description,
            version: manifest.skill.version,
            author: manifest.skill.author.unwrap_or_default(),
            tags: normalize_tags(manifest.skill.tags),
        });
    }

    Err("Skill directory must contain SKILL.md or SKILL.toml at its root.".to_string())
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

fn read_skill_markdown(skill_dir: &Path) -> Result<String, String> {
    let markdown_path = skill_dir.join("SKILL.md");
    if markdown_path.exists() {
        return fs::read_to_string(markdown_path).map_err(|error| error.to_string());
    }

    let toml_path = skill_dir.join("SKILL.toml");
    if toml_path.exists() {
        let raw = fs::read_to_string(&toml_path).map_err(|error| error.to_string())?;
        let manifest = toml::from_str::<SkillTomlManifest>(&raw)
            .map_err(|error| format!("Failed to parse SKILL.toml: {error}"))?;
        return Ok(format!(
            "# {}\n{}\n\nVersion: {}\nAuthor: {}\n",
            manifest.skill.name,
            manifest.skill.description,
            manifest.skill.version,
            manifest
                .skill
                .author
                .unwrap_or_else(|| "Unknown".to_string())
        ));
    }

    Err("Skill content not found on disk.".to_string())
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

fn slugify(value: &str) -> String {
    let mut slug = value
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
