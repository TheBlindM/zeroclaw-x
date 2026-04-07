use chrono::{TimeZone, Utc};
use cron::Schedule;
use std::{
    collections::HashSet,
    path::Path,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{
    channel::ChannelRecord,
    chat::{MessageRecord, SessionRecord},
    cron::{CronJobRecord, CronRunRecord},
    knowledge::{KnowledgeDocumentRecord, SessionKnowledgeScopeRecord},
    mcp::McpServerRecord,
    project::ProjectRecord,
    skill::SkillRecord,
};

const MIGRATION_SQL: &str = include_str!("../../migrations/0001_init.sql");

fn connect(db_path: &Path) -> Result<Connection, String> {
    Connection::open(db_path).map_err(|error| error.to_string())
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

fn make_entity_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    format!("{prefix}-{nanos}")
}

fn normalize_project_status(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "archived" => "archived".to_string(),
        _ => "active".to_string(),
    }
}

fn normalize_scope_mode(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "manual" => "manual".to_string(),
        _ => "auto".to_string(),
    }
}

fn build_content_preview(content: &str) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= 220 {
        return normalized;
    }

    normalized.chars().take(220).collect::<String>() + "..."
}

fn map_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRecord> {
    Ok(SessionRecord {
        id: row.get(0)?,
        title: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get::<_, i64>(3)?.to_string(),
        message_count: row.get(4)?,
        last_message_preview: row.get(5)?,
        project_id: row.get(6)?,
        agent_mode: row.get::<_, i64>(7)? != 0,
    })
}

fn map_project_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    Ok(ProjectRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        pinned: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn map_knowledge_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeDocumentRecord> {
    Ok(KnowledgeDocumentRecord {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        source_path: row.get(3)?,
        content: row.get(4)?,
        content_preview: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn list_sessions_query(filter_sql: &str) -> String {
    format!(
        "SELECT
            s.id,
            s.title,
            s.created_at,
            COALESCE(MAX(CAST(m.created_at AS INTEGER)), CAST(s.created_at AS INTEGER)) AS updated_at,
            COUNT(m.id) AS message_count,
            (
              SELECT content
              FROM messages latest
              WHERE latest.session_id = s.id
              ORDER BY CAST(latest.created_at AS INTEGER) DESC, latest.id DESC
              LIMIT 1
            ) AS last_message_preview,
            ps.project_id AS project_id,
            s.agent_mode AS agent_mode
         FROM sessions s
         LEFT JOIN messages m ON m.session_id = s.id
         LEFT JOIN project_sessions ps ON ps.session_id = s.id
         {filter_sql}
         GROUP BY s.id, s.title, s.created_at, ps.project_id, s.agent_mode
         ORDER BY updated_at DESC"
    )
}

fn ensure_sessions_agent_mode_column(connection: &Connection) -> Result<(), String> {
    let columns = get_table_columns(connection, "sessions")?;

    if columns.iter().any(|column| column == "agent_mode") {
        return Ok(());
    }

    connection
        .execute(
            "ALTER TABLE sessions ADD COLUMN agent_mode INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn get_table_columns(connection: &Connection, table_name: &str) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            params![table_name],
            |_| Ok(()),
        )
        .optional()
        .map(|row| row.is_some())
        .map_err(|error| error.to_string())
}

fn ensure_mcp_servers_columns(connection: &Connection) -> Result<(), String> {
    let columns = get_table_columns(connection, "mcp_servers")?;

    for (column_name, column_definition) in [
        ("command", "TEXT NOT NULL DEFAULT ''"),
        ("arguments_json", "TEXT NOT NULL DEFAULT '[]'"),
        ("url", "TEXT NOT NULL DEFAULT ''"),
        ("headers_json", "TEXT NOT NULL DEFAULT '{}'"),
        ("environment_json", "TEXT NOT NULL DEFAULT '{}'"),
        ("enabled", "INTEGER NOT NULL DEFAULT 1"),
        ("last_tested_at", "TEXT"),
        ("last_test_status", "TEXT"),
        ("last_test_message", "TEXT"),
        ("created_at", "TEXT NOT NULL DEFAULT '0'"),
        ("updated_at", "TEXT NOT NULL DEFAULT '0'"),
    ] {
        if columns.iter().any(|column| column == column_name) {
            continue;
        }

        connection
            .execute(
                &format!("ALTER TABLE mcp_servers ADD COLUMN {column_name} {column_definition}"),
                [],
            )
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn clear_session_knowledge_scope(connection: &Connection, session_id: &str) -> Result<(), String> {
    connection
        .execute(
            "DELETE FROM session_knowledge_scope WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "DELETE FROM session_knowledge_preferences WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn collect_unique_document_ids(document_ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut collected = Vec::new();

    for document_id in document_ids {
        let trimmed = document_id.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        collected.push(trimmed.to_string());
    }

    collected
}

pub fn initialize(db_path: &Path) -> Result<(), String> {
    let connection = connect(db_path)?;

    if table_exists(&connection, "mcp_servers")? {
        ensure_mcp_servers_columns(&connection)?;
    }

    connection
        .execute_batch(MIGRATION_SQL)
        .map_err(|error| error.to_string())?;
    ensure_sessions_agent_mode_column(&connection)?;
    ensure_mcp_servers_columns(&connection)
}

pub fn upsert_session(db_path: &Path, session_id: &str, title: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    let created_at = current_timestamp();

    connection
        .execute(
            "INSERT INTO sessions (id, title, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET title = excluded.title",
            params![session_id, title, created_at],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn rename_session(db_path: &Path, session_id: &str, title: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    connection
        .execute(
            "UPDATE sessions SET title = ?2 WHERE id = ?1",
            params![session_id, title],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn set_session_agent_mode(
    db_path: &Path,
    session_id: &str,
    agent_mode: bool,
) -> Result<(), String> {
    let connection = connect(db_path)?;
    let changed = connection
        .execute(
            "UPDATE sessions SET agent_mode = ?2 WHERE id = ?1",
            params![session_id, if agent_mode { 1 } else { 0 }],
        )
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("Session not found.".to_string());
    }

    Ok(())
}

pub fn delete_session(db_path: &Path, session_id: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    clear_session_knowledge_scope(&connection, session_id)?;

    connection
        .execute(
            "DELETE FROM project_sessions WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn record_message(
    db_path: &Path,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    let connection = connect(db_path)?;
    let created_at = current_timestamp();

    connection
        .execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, created_at],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn assign_session_project(
    db_path: &Path,
    session_id: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    let connection = connect(db_path)?;

    let session_exists: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if session_exists == 0 {
        return Err("Session not found.".to_string());
    }

    let current_project_id = connection
        .query_row(
            "SELECT project_id FROM project_sessions WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    if let Some(project_id) = project_id {
        let trimmed = project_id.trim();
        if trimmed.is_empty() {
            connection
                .execute(
                    "DELETE FROM project_sessions WHERE session_id = ?1",
                    params![session_id],
                )
                .map_err(|error| error.to_string())?;

            if current_project_id.is_some() {
                clear_session_knowledge_scope(&connection, session_id)?;
            }

            return Ok(());
        }

        let project_exists: i64 = connection
            .query_row(
                "SELECT COUNT(1) FROM projects WHERE id = ?1",
                params![trimmed],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;

        if project_exists == 0 {
            return Err("Project not found.".to_string());
        }

        connection
            .execute(
                "INSERT INTO project_sessions (session_id, project_id, linked_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(session_id) DO UPDATE SET project_id = excluded.project_id, linked_at = excluded.linked_at",
                params![session_id, trimmed, current_timestamp()],
            )
            .map_err(|error| error.to_string())?;

        if current_project_id.as_deref() != Some(trimmed) {
            clear_session_knowledge_scope(&connection, session_id)?;
        }

        return Ok(());
    }

    connection
        .execute(
            "DELETE FROM project_sessions WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| error.to_string())?;

    if current_project_id.is_some() {
        clear_session_knowledge_scope(&connection, session_id)?;
    }

    Ok(())
}

pub fn list_sessions(db_path: &Path) -> Result<Vec<SessionRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(&list_sessions_query(""))
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], map_session_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn list_project_sessions(
    db_path: &Path,
    project_id: &str,
) -> Result<Vec<SessionRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(&list_sessions_query("WHERE ps.project_id = ?1"))
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![project_id], map_session_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn list_messages(db_path: &Path, session_id: &str) -> Result<Vec<MessageRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, session_id, role, content, created_at
             FROM messages
             WHERE session_id = ?1
             ORDER BY CAST(created_at AS INTEGER) ASC, id ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![session_id], |row| {
            Ok(MessageRecord {
                id: row.get::<_, i64>(0)?.to_string(),
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn list_projects(db_path: &Path) -> Result<Vec<ProjectRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, description, status, pinned, created_at, updated_at
             FROM projects
             ORDER BY pinned DESC, CAST(updated_at AS INTEGER) DESC, CAST(created_at AS INTEGER) DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], map_project_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn create_project(
    db_path: &Path,
    name: &str,
    description: &str,
    status: &str,
    pinned: bool,
) -> Result<ProjectRecord, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Project name is required.".to_string());
    }

    let connection = connect(db_path)?;
    let now = current_timestamp();
    let record = ProjectRecord {
        id: make_entity_id("project"),
        name: trimmed_name.to_string(),
        description: description.trim().to_string(),
        status: normalize_project_status(status),
        pinned,
        created_at: now.clone(),
        updated_at: now,
    };

    connection
        .execute(
            "INSERT INTO projects (id, name, description, status, pinned, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.id,
                record.name,
                record.description,
                record.status,
                if record.pinned { 1 } else { 0 },
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(record)
}

pub fn update_project(
    db_path: &Path,
    project_id: &str,
    name: &str,
    description: &str,
    status: &str,
    pinned: bool,
) -> Result<ProjectRecord, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Project name is required.".to_string());
    }

    let connection = connect(db_path)?;
    let updated_at = current_timestamp();
    let normalized_status = normalize_project_status(status);
    let changed = connection
        .execute(
            "UPDATE projects
             SET name = ?2, description = ?3, status = ?4, pinned = ?5, updated_at = ?6
             WHERE id = ?1",
            params![
                project_id,
                trimmed_name,
                description.trim(),
                normalized_status,
                if pinned { 1 } else { 0 },
                updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("Project not found.".to_string());
    }

    let mut statement = connection
        .prepare(
            "SELECT id, name, description, status, pinned, created_at, updated_at
             FROM projects
             WHERE id = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![project_id], map_project_row)
        .map_err(|error| error.to_string())
}

pub fn delete_project(db_path: &Path, project_id: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    connection
        .execute(
            "DELETE FROM session_knowledge_scope
             WHERE document_id IN (
               SELECT id FROM knowledge_documents WHERE project_id = ?1
             )",
            params![project_id],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "DELETE FROM session_knowledge_preferences
             WHERE session_id IN (
               SELECT session_id FROM project_sessions WHERE project_id = ?1
             )",
            params![project_id],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "DELETE FROM project_sessions WHERE project_id = ?1",
            params![project_id],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "DELETE FROM knowledge_documents WHERE project_id = ?1",
            params![project_id],
        )
        .map_err(|error| error.to_string())?;

    let changed = connection
        .execute("DELETE FROM projects WHERE id = ?1", params![project_id])
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("Project not found.".to_string());
    }

    Ok(())
}

pub fn list_project_knowledge(
    db_path: &Path,
    project_id: &str,
) -> Result<Vec<KnowledgeDocumentRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, title, source_path, content, content_preview, created_at, updated_at
             FROM knowledge_documents
             WHERE project_id = ?1
             ORDER BY CAST(updated_at AS INTEGER) DESC, CAST(created_at AS INTEGER) DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![project_id], map_knowledge_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn create_project_knowledge_document(
    db_path: &Path,
    project_id: &str,
    title: &str,
    source_path: &str,
    content: &str,
) -> Result<KnowledgeDocumentRecord, String> {
    let trimmed_title = title.trim();
    if trimmed_title.is_empty() {
        return Err("Knowledge title is required.".to_string());
    }

    let connection = connect(db_path)?;
    let project_exists: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if project_exists == 0 {
        return Err("Project not found.".to_string());
    }

    let now = current_timestamp();
    let record = KnowledgeDocumentRecord {
        id: make_entity_id("knowledge"),
        project_id: project_id.to_string(),
        title: trimmed_title.to_string(),
        source_path: source_path.trim().to_string(),
        content: content.to_string(),
        content_preview: build_content_preview(content),
        created_at: now.clone(),
        updated_at: now,
    };

    connection
        .execute(
            "INSERT INTO knowledge_documents (id, project_id, title, source_path, content, content_preview, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.project_id,
                record.title,
                record.source_path,
                record.content,
                record.content_preview,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(record)
}

pub fn create_project_knowledge_note(
    db_path: &Path,
    project_id: &str,
    title: &str,
    content: &str,
) -> Result<KnowledgeDocumentRecord, String> {
    create_project_knowledge_document(db_path, project_id, title, "manual-note", content)
}

pub fn delete_knowledge_document(db_path: &Path, document_id: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    connection
        .execute(
            "DELETE FROM session_knowledge_scope WHERE document_id = ?1",
            params![document_id],
        )
        .map_err(|error| error.to_string())?;

    let changed = connection
        .execute(
            "DELETE FROM knowledge_documents WHERE id = ?1",
            params![document_id],
        )
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("Knowledge document not found.".to_string());
    }

    Ok(())
}

pub fn get_session_knowledge_scope(
    db_path: &Path,
    session_id: &str,
) -> Result<SessionKnowledgeScopeRecord, String> {
    let connection = connect(db_path)?;
    let mode = connection
        .query_row(
            "SELECT mode FROM session_knowledge_preferences WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| "auto".to_string());

    let mut statement = connection
        .prepare(
            "SELECT document_id
             FROM session_knowledge_scope
             WHERE session_id = ?1
             ORDER BY CAST(included_at AS INTEGER) ASC, document_id ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![session_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;

    let document_ids = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(SessionKnowledgeScopeRecord {
        session_id: session_id.to_string(),
        mode: normalize_scope_mode(&mode),
        document_ids,
    })
}

pub fn save_session_knowledge_scope(
    db_path: &Path,
    session_id: &str,
    mode: &str,
    document_ids: &[String],
) -> Result<SessionKnowledgeScopeRecord, String> {
    let connection = connect(db_path)?;
    let session_exists: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if session_exists == 0 {
        return Err("Session not found.".to_string());
    }

    let normalized_mode = normalize_scope_mode(mode);
    let cleaned_document_ids = collect_unique_document_ids(document_ids);
    let project_id = connection
        .query_row(
            "SELECT project_id FROM project_sessions WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    if !cleaned_document_ids.is_empty() && project_id.is_none() {
        return Err("Assign a project before selecting scoped knowledge.".to_string());
    }

    if let Some(project_id) = project_id.as_deref() {
        for document_id in &cleaned_document_ids {
            let valid_document_count: i64 = connection
                .query_row(
                    "SELECT COUNT(1)
                     FROM knowledge_documents
                     WHERE id = ?1 AND project_id = ?2",
                    params![document_id, project_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;

            if valid_document_count == 0 {
                return Err(
                    "One of the selected knowledge items no longer belongs to this project."
                        .to_string(),
                );
            }
        }
    }

    connection
        .execute(
            "INSERT INTO session_knowledge_preferences (session_id, mode, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(session_id) DO UPDATE SET mode = excluded.mode, updated_at = excluded.updated_at",
            params![session_id, normalized_mode, current_timestamp()],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "DELETE FROM session_knowledge_scope WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| error.to_string())?;

    if normalized_mode == "manual" {
        for document_id in &cleaned_document_ids {
            connection
                .execute(
                    "INSERT INTO session_knowledge_scope (session_id, document_id, included_at)
                     VALUES (?1, ?2, ?3)",
                    params![session_id, document_id, current_timestamp()],
                )
                .map_err(|error| error.to_string())?;
        }
    }

    Ok(SessionKnowledgeScopeRecord {
        session_id: session_id.to_string(),
        mode: normalized_mode,
        document_ids: cleaned_document_ids,
    })
}

pub fn get_project(db_path: &Path, project_id: &str) -> Result<Option<ProjectRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, description, status, pinned, created_at, updated_at
             FROM projects
             WHERE id = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![project_id], map_project_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn get_session_project_id(db_path: &Path, session_id: &str) -> Result<Option<String>, String> {
    let connection = connect(db_path)?;
    connection
        .query_row(
            "SELECT project_id FROM project_sessions WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn normalize_cron_schedule(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Cron schedule is required.".to_string());
    }

    Schedule::from_str(trimmed).map_err(|error| format!("Invalid cron schedule: {error}"))?;

    Ok(trimmed.to_string())
}

fn compute_next_run_at(schedule: &str, enabled: bool) -> Result<Option<String>, String> {
    if !enabled {
        return Ok(None);
    }

    let parsed =
        Schedule::from_str(schedule).map_err(|error| format!("Invalid cron schedule: {error}"))?;

    Ok(parsed
        .upcoming(Utc)
        .next()
        .map(|date_time| date_time.timestamp_millis().to_string()))
}

fn map_cron_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CronJobRecord> {
    Ok(CronJobRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        schedule: row.get(2)?,
        prompt: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        last_run_at: row.get(5)?,
        next_run_at: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn map_cron_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CronRunRecord> {
    Ok(CronRunRecord {
        id: row.get(0)?,
        job_id: row.get(1)?,
        status: row.get(2)?,
        output: row.get(3)?,
        started_at: row.get(4)?,
        finished_at: row.get(5)?,
    })
}

pub fn list_cron_jobs(db_path: &Path) -> Result<Vec<CronJobRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, schedule, prompt, enabled, last_run_at, next_run_at, created_at, updated_at
             FROM cron_jobs
             ORDER BY enabled DESC, CAST(updated_at AS INTEGER) DESC, CAST(created_at AS INTEGER) DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], map_cron_job_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn list_cron_runs(db_path: &Path, job_id: &str) -> Result<Vec<CronRunRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, job_id, status, output, started_at, finished_at
             FROM cron_runs
             WHERE job_id = ?1
             ORDER BY CAST(started_at AS INTEGER) DESC, id DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![job_id], map_cron_run_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn create_cron_job(
    db_path: &Path,
    name: &str,
    schedule: &str,
    prompt: &str,
    enabled: bool,
) -> Result<CronJobRecord, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Cron job name is required.".to_string());
    }

    let normalized_schedule = normalize_cron_schedule(schedule)?;
    let trimmed_prompt = prompt.trim();
    if trimmed_prompt.is_empty() {
        return Err("Cron job prompt is required.".to_string());
    }

    let connection = connect(db_path)?;
    let now = current_timestamp();
    let record = CronJobRecord {
        id: make_entity_id("cron-job"),
        name: trimmed_name.to_string(),
        schedule: normalized_schedule.clone(),
        prompt: trimmed_prompt.to_string(),
        enabled,
        last_run_at: None,
        next_run_at: compute_next_run_at(&normalized_schedule, enabled)?,
        created_at: now.clone(),
        updated_at: now,
    };

    connection
        .execute(
            "INSERT INTO cron_jobs (id, name, schedule, prompt, enabled, last_run_at, next_run_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.name,
                record.schedule,
                record.prompt,
                if record.enabled { 1 } else { 0 },
                record.last_run_at,
                record.next_run_at,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(record)
}

pub fn update_cron_job(
    db_path: &Path,
    job_id: &str,
    name: &str,
    schedule: &str,
    prompt: &str,
    enabled: bool,
) -> Result<CronJobRecord, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Cron job name is required.".to_string());
    }

    let normalized_schedule = normalize_cron_schedule(schedule)?;
    let trimmed_prompt = prompt.trim();
    if trimmed_prompt.is_empty() {
        return Err("Cron job prompt is required.".to_string());
    }

    let connection = connect(db_path)?;
    let (created_at, last_run_at): (String, Option<String>) = connection
        .query_row(
            "SELECT created_at, last_run_at FROM cron_jobs WHERE id = ?1",
            params![job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Cron job not found.".to_string())?;

    let updated_at = current_timestamp();
    let next_run_at = compute_next_run_at(&normalized_schedule, enabled)?;
    let changed = connection
        .execute(
            "UPDATE cron_jobs
             SET name = ?2, schedule = ?3, prompt = ?4, enabled = ?5, next_run_at = ?6, updated_at = ?7
             WHERE id = ?1",
            params![
                job_id,
                trimmed_name,
                normalized_schedule,
                trimmed_prompt,
                if enabled { 1 } else { 0 },
                next_run_at,
                updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("Cron job not found.".to_string());
    }

    Ok(CronJobRecord {
        id: job_id.to_string(),
        name: trimmed_name.to_string(),
        schedule: normalized_schedule,
        prompt: trimmed_prompt.to_string(),
        enabled,
        last_run_at,
        next_run_at,
        created_at,
        updated_at,
    })
}

pub fn delete_cron_job(db_path: &Path, job_id: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    connection
        .execute("DELETE FROM cron_runs WHERE job_id = ?1", params![job_id])
        .map_err(|error| error.to_string())?;

    let changed = connection
        .execute("DELETE FROM cron_jobs WHERE id = ?1", params![job_id])
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("Cron job not found.".to_string());
    }

    Ok(())
}

fn compute_next_run_at_after(
    schedule: &str,
    enabled: bool,
    after_timestamp_millis: i64,
) -> Result<Option<String>, String> {
    if !enabled {
        return Ok(None);
    }

    let parsed =
        Schedule::from_str(schedule).map_err(|error| format!("Invalid cron schedule: {error}"))?;
    let base_time = Utc
        .timestamp_millis_opt(after_timestamp_millis)
        .single()
        .unwrap_or_else(Utc::now);

    Ok(parsed
        .after(&base_time)
        .next()
        .map(|date_time| date_time.timestamp_millis().to_string()))
}

pub fn get_cron_job(db_path: &Path, job_id: &str) -> Result<Option<CronJobRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, schedule, prompt, enabled, last_run_at, next_run_at, created_at, updated_at
             FROM cron_jobs
             WHERE id = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![job_id], map_cron_job_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn list_due_cron_jobs(
    db_path: &Path,
    now_timestamp_millis: i64,
) -> Result<Vec<CronJobRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, schedule, prompt, enabled, last_run_at, next_run_at, created_at, updated_at
             FROM cron_jobs
             WHERE enabled = 1
               AND next_run_at IS NOT NULL
               AND CAST(next_run_at AS INTEGER) <= ?1
             ORDER BY CAST(next_run_at AS INTEGER) ASC, CAST(updated_at AS INTEGER) ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![now_timestamp_millis], map_cron_job_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn record_cron_job_run(
    db_path: &Path,
    job_id: &str,
    schedule: &str,
    enabled: bool,
    status: &str,
    output: &str,
    started_at: &str,
    finished_at: &str,
) -> Result<CronRunRecord, String> {
    let connection = connect(db_path)?;
    let run = CronRunRecord {
        id: make_entity_id("cron-run"),
        job_id: job_id.to_string(),
        status: status.trim().to_string(),
        output: output.to_string(),
        started_at: started_at.to_string(),
        finished_at: finished_at.to_string(),
    };

    connection
        .execute(
            "INSERT INTO cron_runs (id, job_id, status, output, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                run.id,
                run.job_id,
                run.status,
                run.output,
                run.started_at,
                run.finished_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    let finished_millis = finished_at
        .parse::<i64>()
        .unwrap_or_else(|_| Utc::now().timestamp_millis());
    let next_run_at = compute_next_run_at_after(schedule, enabled, finished_millis)?;

    connection
        .execute(
            "UPDATE cron_jobs
             SET last_run_at = ?2, next_run_at = ?3, updated_at = ?4
             WHERE id = ?1",
            params![job_id, finished_at, next_run_at, finished_at],
        )
        .map_err(|error| error.to_string())?;

    Ok(run)
}

fn map_mcp_server_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpServerRecord> {
    Ok(McpServerRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        transport: row.get(2)?,
        command: row.get(3)?,
        arguments_json: row.get(4)?,
        url: row.get(5)?,
        headers_json: row.get(6)?,
        environment_json: row.get(7)?,
        enabled: row.get::<_, i64>(8)? != 0,
        last_tested_at: row.get(9)?,
        last_test_status: row.get(10)?,
        last_test_message: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub fn list_mcp_servers(db_path: &Path) -> Result<Vec<McpServerRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, transport, command, arguments_json, url, headers_json, environment_json,
                    enabled, last_tested_at, last_test_status, last_test_message, created_at, updated_at
             FROM mcp_servers
             ORDER BY enabled DESC, CAST(updated_at AS INTEGER) DESC, CAST(created_at AS INTEGER) DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], map_mcp_server_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn get_mcp_server(db_path: &Path, server_id: &str) -> Result<Option<McpServerRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, transport, command, arguments_json, url, headers_json, environment_json,
                    enabled, last_tested_at, last_test_status, last_test_message, created_at, updated_at
             FROM mcp_servers
             WHERE id = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![server_id], map_mcp_server_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn create_mcp_server(
    db_path: &Path,
    name: &str,
    transport: &str,
    command: &str,
    arguments_json: &str,
    url: &str,
    headers_json: &str,
    environment_json: &str,
    enabled: bool,
) -> Result<McpServerRecord, String> {
    let connection = connect(db_path)?;
    let now = current_timestamp();
    let record = McpServerRecord {
        id: make_entity_id("mcp-server"),
        name: name.trim().to_string(),
        transport: transport.trim().to_string(),
        command: command.trim().to_string(),
        arguments_json: arguments_json.trim().to_string(),
        url: url.trim().to_string(),
        headers_json: headers_json.trim().to_string(),
        environment_json: environment_json.trim().to_string(),
        enabled,
        last_tested_at: None,
        last_test_status: None,
        last_test_message: None,
        created_at: now.clone(),
        updated_at: now,
    };

    connection
        .execute(
            "INSERT INTO mcp_servers (
                id, name, transport, command, arguments_json, url, headers_json, environment_json,
                enabled, last_tested_at, last_test_status, last_test_message, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                record.id,
                record.name,
                record.transport,
                record.command,
                record.arguments_json,
                record.url,
                record.headers_json,
                record.environment_json,
                if record.enabled { 1 } else { 0 },
                record.last_tested_at,
                record.last_test_status,
                record.last_test_message,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(record)
}

pub fn update_mcp_server(
    db_path: &Path,
    server_id: &str,
    name: &str,
    transport: &str,
    command: &str,
    arguments_json: &str,
    url: &str,
    headers_json: &str,
    environment_json: &str,
    enabled: bool,
) -> Result<McpServerRecord, String> {
    let connection = connect(db_path)?;
    let (created_at, last_tested_at, last_test_status, last_test_message): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT created_at, last_tested_at, last_test_status, last_test_message
             FROM mcp_servers
             WHERE id = ?1",
            params![server_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "MCP server not found.".to_string())?;

    let updated_at = current_timestamp();
    let changed = connection
        .execute(
            "UPDATE mcp_servers
             SET name = ?2,
                 transport = ?3,
                 command = ?4,
                 arguments_json = ?5,
                 url = ?6,
                 headers_json = ?7,
                 environment_json = ?8,
                 enabled = ?9,
                 updated_at = ?10
             WHERE id = ?1",
            params![
                server_id,
                name.trim(),
                transport.trim(),
                command.trim(),
                arguments_json.trim(),
                url.trim(),
                headers_json.trim(),
                environment_json.trim(),
                if enabled { 1 } else { 0 },
                updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("MCP server not found.".to_string());
    }

    Ok(McpServerRecord {
        id: server_id.to_string(),
        name: name.trim().to_string(),
        transport: transport.trim().to_string(),
        command: command.trim().to_string(),
        arguments_json: arguments_json.trim().to_string(),
        url: url.trim().to_string(),
        headers_json: headers_json.trim().to_string(),
        environment_json: environment_json.trim().to_string(),
        enabled,
        last_tested_at,
        last_test_status,
        last_test_message,
        created_at,
        updated_at,
    })
}

pub fn delete_mcp_server(db_path: &Path, server_id: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    let changed = connection
        .execute("DELETE FROM mcp_servers WHERE id = ?1", params![server_id])
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("MCP server not found.".to_string());
    }

    Ok(())
}

pub fn record_mcp_server_test_result(
    db_path: &Path,
    server_id: &str,
    status: &str,
    message: &str,
    tested_at: &str,
) -> Result<McpServerRecord, String> {
    let connection = connect(db_path)?;
    let updated_at = current_timestamp();
    let changed = connection
        .execute(
            "UPDATE mcp_servers
             SET last_tested_at = ?2,
                 last_test_status = ?3,
                 last_test_message = ?4,
                 updated_at = ?5
             WHERE id = ?1",
            params![
                server_id,
                tested_at,
                status.trim(),
                message.trim(),
                updated_at
            ],
        )
        .map_err(|error| error.to_string())?;

    if changed == 0 {
        return Err("MCP server not found.".to_string());
    }

    get_mcp_server(db_path, server_id)?.ok_or_else(|| "MCP server not found.".to_string())
}

fn map_skill_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillRecord> {
    Ok(SkillRecord {
        id: row.get(0)?,
        slug: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        version: row.get(4)?,
        author: row.get(5)?,
        tags_json: row.get(6)?,
        source_kind: row.get(7)?,
        source_label: row.get(8)?,
        enabled: row.get::<_, i64>(9)? != 0,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub fn list_skills(db_path: &Path) -> Result<Vec<SkillRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, slug, name, description, version, author, tags_json, source_kind, source_label,
                    enabled, created_at, updated_at
             FROM skills
             ORDER BY enabled DESC, CAST(updated_at AS INTEGER) DESC, CAST(created_at AS INTEGER) DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], map_skill_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn get_skill(db_path: &Path, skill_id: &str) -> Result<Option<SkillRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, slug, name, description, version, author, tags_json, source_kind, source_label,
                    enabled, created_at, updated_at
             FROM skills
             WHERE id = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![skill_id], map_skill_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn get_skill_by_slug(db_path: &Path, slug: &str) -> Result<Option<SkillRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, slug, name, description, version, author, tags_json, source_kind, source_label,
                    enabled, created_at, updated_at
             FROM skills
             WHERE slug = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![slug], map_skill_row)
        .optional()
        .map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_skill(
    db_path: &Path,
    slug: &str,
    name: &str,
    description: &str,
    version: &str,
    author: &str,
    tags_json: &str,
    source_kind: &str,
    source_label: &str,
    enabled: bool,
) -> Result<SkillRecord, String> {
    let connection = connect(db_path)?;
    let existing = get_skill_by_slug(db_path, slug)?;
    let updated_at = current_timestamp();

    if let Some(existing) = existing {
        connection
            .execute(
                "UPDATE skills
                 SET name = ?2,
                     description = ?3,
                     version = ?4,
                     author = ?5,
                     tags_json = ?6,
                     source_kind = ?7,
                     source_label = ?8,
                     enabled = ?9,
                     updated_at = ?10
                 WHERE id = ?1",
                params![
                    existing.id,
                    name.trim(),
                    description.trim(),
                    version.trim(),
                    author.trim(),
                    tags_json.trim(),
                    source_kind.trim(),
                    source_label.trim(),
                    if enabled { 1 } else { 0 },
                    updated_at,
                ],
            )
            .map_err(|error| error.to_string())?;

        return Ok(SkillRecord {
            id: existing.id,
            slug: slug.trim().to_string(),
            name: name.trim().to_string(),
            description: description.trim().to_string(),
            version: version.trim().to_string(),
            author: author.trim().to_string(),
            tags_json: tags_json.trim().to_string(),
            source_kind: source_kind.trim().to_string(),
            source_label: source_label.trim().to_string(),
            enabled,
            created_at: existing.created_at,
            updated_at,
        });
    }

    let record = SkillRecord {
        id: make_entity_id("skill"),
        slug: slug.trim().to_string(),
        name: name.trim().to_string(),
        description: description.trim().to_string(),
        version: version.trim().to_string(),
        author: author.trim().to_string(),
        tags_json: tags_json.trim().to_string(),
        source_kind: source_kind.trim().to_string(),
        source_label: source_label.trim().to_string(),
        enabled,
        created_at: updated_at.clone(),
        updated_at,
    };

    connection
        .execute(
            "INSERT INTO skills (
                id, slug, name, description, version, author, tags_json, source_kind, source_label,
                enabled, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.slug,
                record.name,
                record.description,
                record.version,
                record.author,
                record.tags_json,
                record.source_kind,
                record.source_label,
                if record.enabled { 1 } else { 0 },
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(record)
}

pub fn set_skill_enabled(
    db_path: &Path,
    skill_id: &str,
    enabled: bool,
) -> Result<SkillRecord, String> {
    let connection = connect(db_path)?;
    let existing = get_skill(db_path, skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;
    let updated_at = current_timestamp();

    connection
        .execute(
            "UPDATE skills
             SET enabled = ?2, updated_at = ?3
             WHERE id = ?1",
            params![skill_id, if enabled { 1 } else { 0 }, updated_at],
        )
        .map_err(|error| error.to_string())?;

    Ok(SkillRecord {
        enabled,
        updated_at,
        ..existing
    })
}

pub fn delete_skill(db_path: &Path, skill_id: &str) -> Result<SkillRecord, String> {
    let connection = connect(db_path)?;
    let existing = get_skill(db_path, skill_id)?.ok_or_else(|| "Skill not found.".to_string())?;

    connection
        .execute("DELETE FROM skills WHERE id = ?1", params![skill_id])
        .map_err(|error| error.to_string())?;

    Ok(existing)
}

fn map_channel_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChannelRecord> {
    Ok(ChannelRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        config_json: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        last_checked_at: row.get(5)?,
        last_health_status: row.get(6)?,
        last_health_message: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn list_channels(db_path: &Path) -> Result<Vec<ChannelRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, kind, config_json, enabled, last_checked_at, last_health_status,
                    last_health_message, created_at, updated_at
             FROM channels
             ORDER BY enabled DESC, CAST(updated_at AS INTEGER) DESC, CAST(created_at AS INTEGER) DESC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], map_channel_row)
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn get_channel(db_path: &Path, channel_id: &str) -> Result<Option<ChannelRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, kind, config_json, enabled, last_checked_at, last_health_status,
                    last_health_message, created_at, updated_at
             FROM channels
             WHERE id = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![channel_id], map_channel_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn get_channel_by_kind(db_path: &Path, kind: &str) -> Result<Option<ChannelRecord>, String> {
    let connection = connect(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, kind, config_json, enabled, last_checked_at, last_health_status,
                    last_health_message, created_at, updated_at
             FROM channels
             WHERE kind = ?1",
        )
        .map_err(|error| error.to_string())?;

    statement
        .query_row(params![kind], map_channel_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn upsert_channel(
    db_path: &Path,
    channel_id: Option<&str>,
    name: &str,
    kind: &str,
    config_json: &str,
    enabled: bool,
) -> Result<ChannelRecord, String> {
    let connection = connect(db_path)?;
    let updated_at = current_timestamp();

    let existing = if let Some(channel_id) = channel_id {
        get_channel(db_path, channel_id)?
    } else {
        get_channel_by_kind(db_path, kind)?
    };

    if let Some(existing) = existing {
        connection
            .execute(
                "UPDATE channels
                 SET name = ?2,
                     kind = ?3,
                     config_json = ?4,
                     enabled = ?5,
                     updated_at = ?6
                 WHERE id = ?1",
                params![
                    existing.id,
                    name.trim(),
                    kind.trim(),
                    config_json.trim(),
                    if enabled { 1 } else { 0 },
                    updated_at,
                ],
            )
            .map_err(|error| error.to_string())?;

        return Ok(ChannelRecord {
            id: existing.id,
            name: name.trim().to_string(),
            kind: kind.trim().to_string(),
            config_json: config_json.trim().to_string(),
            enabled,
            last_checked_at: existing.last_checked_at,
            last_health_status: existing.last_health_status,
            last_health_message: existing.last_health_message,
            created_at: existing.created_at,
            updated_at,
        });
    }

    let record = ChannelRecord {
        id: make_entity_id("channel"),
        name: name.trim().to_string(),
        kind: kind.trim().to_string(),
        config_json: config_json.trim().to_string(),
        enabled,
        last_checked_at: None,
        last_health_status: None,
        last_health_message: None,
        created_at: updated_at.clone(),
        updated_at,
    };

    connection
        .execute(
            "INSERT INTO channels (
                id, name, kind, config_json, enabled, last_checked_at, last_health_status,
                last_health_message, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.id,
                record.name,
                record.kind,
                record.config_json,
                if record.enabled { 1 } else { 0 },
                record.last_checked_at,
                record.last_health_status,
                record.last_health_message,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(record)
}

pub fn update_channel_health(
    db_path: &Path,
    channel_id: &str,
    checked_at: &str,
    status: &str,
    message: &str,
) -> Result<ChannelRecord, String> {
    let connection = connect(db_path)?;
    let existing =
        get_channel(db_path, channel_id)?.ok_or_else(|| "Channel not found.".to_string())?;
    let updated_at = current_timestamp();

    connection
        .execute(
            "UPDATE channels
             SET last_checked_at = ?2,
                 last_health_status = ?3,
                 last_health_message = ?4,
                 updated_at = ?5
             WHERE id = ?1",
            params![channel_id, checked_at, status, message, updated_at],
        )
        .map_err(|error| error.to_string())?;

    Ok(ChannelRecord {
        last_checked_at: Some(checked_at.to_string()),
        last_health_status: Some(status.to_string()),
        last_health_message: Some(message.to_string()),
        updated_at,
        ..existing
    })
}

pub fn delete_channel(db_path: &Path, channel_id: &str) -> Result<(), String> {
    let connection = connect(db_path)?;
    connection
        .execute("DELETE FROM channels WHERE id = ?1", params![channel_id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use rusqlite::Connection;

    fn make_test_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("{prefix}-{stamp}"));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[test]
    fn initialize_backfills_missing_mcp_columns() {
        let app_dir = make_test_dir("zeroclawx-mcp-schema");
        let db_path = app_dir.join("zeroclawx.db");
        let connection = Connection::open(&db_path).expect("db should open");

        connection
            .execute(
                "CREATE TABLE mcp_servers (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    transport TEXT NOT NULL
                 )",
                [],
            )
            .expect("legacy table should be created");

        drop(connection);

        super::initialize(&db_path).expect("initialize should backfill missing columns");

        let created = super::create_mcp_server(
            &db_path,
            "playwright",
            "stdio",
            "npx",
            r#"["-y","@playwright/mcp@latest"]"#,
            "",
            "{}",
            "{}",
            true,
        )
        .expect("server creation should succeed after backfill");

        assert_eq!(created.name, "playwright");
        assert_eq!(created.transport, "stdio");
        assert_eq!(created.command, "npx");
        assert_eq!(created.arguments_json, r#"["-y","@playwright/mcp@latest"]"#);

        let _ = fs::remove_dir_all(app_dir);
    }
}
