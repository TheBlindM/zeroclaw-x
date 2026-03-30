use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use zeroclaw::{
    channels::{
        self, Channel as RuntimeChannel, DiscordChannel, SlackChannel, TelegramChannel,
        WebhookChannel,
    },
    config::{ChannelsConfig, DiscordConfig, SlackConfig, TelegramConfig, WebhookConfig},
};

use crate::{
    db,
    models::channel::{
        ChannelDraft, ChannelRecord, ChannelRuntimeStatusRecord, ChannelTestReport,
        ChannelTestResult,
    },
    state::AppState,
};

const CHANNELS_RUNTIME_EVENT: &str = "channels:runtime-status";

pub fn list_channels(state: &AppState) -> Result<Vec<ChannelRecord>, String> {
    db::list_channels(&state.db_path())
}

pub fn create_channel(state: &AppState, channel: ChannelDraft) -> Result<ChannelRecord, String> {
    let normalized = normalize_channel_draft(channel)?;
    db::upsert_channel(
        &state.db_path(),
        None,
        &normalized.name,
        &normalized.kind,
        &normalized.config_json,
        normalized.enabled,
    )
}

pub fn update_channel(
    state: &AppState,
    channel_id: &str,
    channel: ChannelDraft,
) -> Result<ChannelRecord, String> {
    let normalized = normalize_channel_draft(channel)?;
    db::upsert_channel(
        &state.db_path(),
        Some(channel_id),
        &normalized.name,
        &normalized.kind,
        &normalized.config_json,
        normalized.enabled,
    )
}

pub fn delete_channel(state: &AppState, channel_id: &str) -> Result<(), String> {
    db::delete_channel(&state.db_path(), channel_id)
}

pub async fn test_channel(state: &AppState, channel_id: &str) -> Result<ChannelTestResult, String> {
    let record = db::get_channel(&state.db_path(), channel_id)?
        .ok_or_else(|| "Channel not found.".to_string())?;
    let report = run_health_check(&record.kind, &record.config_json).await?;

    let updated = db::update_channel_health(
        &state.db_path(),
        &record.id,
        &report.checked_at,
        if report.ok { "healthy" } else { "error" },
        &report.message,
    )?;

    Ok(ChannelTestResult {
        channel: updated,
        report,
    })
}

pub fn get_runtime_status(state: &AppState) -> ChannelRuntimeStatusRecord {
    state.get_channel_runtime_status()
}

pub async fn start_runtime(
    app: AppHandle,
    state: AppState,
) -> Result<ChannelRuntimeStatusRecord, String> {
    if state.has_channel_runtime_task() {
        return Ok(state.get_channel_runtime_status());
    }

    let config = build_runtime_config_with_channels(&state)?;
    if !has_enabled_channels(&config.channels_config) {
        return Err(
            "Enable at least one configured channel before starting the runtime.".to_string(),
        );
    }

    let starting = make_runtime_status(true, "starting", "Launching the channels supervisor.");
    emit_runtime_status(&app, &state, starting.clone())?;

    let app_handle = app.clone();
    let state_clone = state.clone();
    let task = tauri::async_runtime::spawn(async move {
        let running = make_runtime_status(true, "running", "Channels supervisor is running.");
        let _ = emit_runtime_status(&app_handle, &state_clone, running);

        let result = channels::start_channels(config).await;
        state_clone.clear_channel_runtime_task();

        let final_status = match result {
            Ok(_) => make_runtime_status(false, "stopped", "Channels supervisor exited."),
            Err(error) => make_runtime_status(
                false,
                "error",
                &format!("Channels supervisor failed: {error}"),
            ),
        };

        let _ = emit_runtime_status(&app_handle, &state_clone, final_status);
    });

    state.store_channel_runtime_task(task);
    Ok(state.get_channel_runtime_status())
}

pub fn stop_runtime(
    app: AppHandle,
    state: &AppState,
) -> Result<ChannelRuntimeStatusRecord, String> {
    if let Some(task) = state.take_channel_runtime_task() {
        task.abort();
    }

    let status = make_runtime_status(false, "stopped", "Channels supervisor stopped by user.");
    emit_runtime_status(&app, state, status.clone())?;
    Ok(status)
}

fn normalize_channel_draft(channel: ChannelDraft) -> Result<ChannelDraft, String> {
    let kind = channel.kind.trim().to_lowercase();
    if channel.name.trim().is_empty() {
        return Err("Channel name cannot be empty.".to_string());
    }

    let config_json = normalize_config_json(&kind, &channel.config_json)?;
    validate_channel_kind(&kind)?;

    Ok(ChannelDraft {
        name: channel.name.trim().to_string(),
        kind,
        config_json,
        enabled: channel.enabled,
    })
}

async fn run_health_check(kind: &str, config_json: &str) -> Result<ChannelTestReport, String> {
    let checked_at = Utc::now().timestamp_millis().to_string();
    let channel = build_channel(kind, config_json)?;

    let result = tokio::time::timeout(Duration::from_secs(10), channel.health_check()).await;
    let (ok, message) = match result {
        Ok(true) => (true, "Health check passed.".to_string()),
        Ok(false) => (
            false,
            "Health check failed. Verify credentials, permissions, and network access.".to_string(),
        ),
        Err(_) => (
            false,
            "Health check timed out after 10 seconds.".to_string(),
        ),
    };

    Ok(ChannelTestReport {
        ok,
        kind: kind.to_string(),
        message,
        checked_at,
    })
}

fn build_runtime_config_with_channels(
    state: &AppState,
) -> Result<zeroclaw::config::Config, String> {
    let settings = super::runtime::load_runtime_settings(&state.settings_path())?;
    let mut config = super::runtime::build_resolved_runtime_config(&state.db_path(), settings)?;
    config.channels_config = build_channels_config(&state.db_path())?;
    Ok(config)
}

fn build_channels_config(db_path: &std::path::Path) -> Result<ChannelsConfig, String> {
    let mut config = ChannelsConfig::default();
    config.cli = false;

    for record in db::list_channels(db_path)? {
        if !record.enabled {
            continue;
        }

        match record.kind.as_str() {
            "telegram" => {
                config.telegram = Some(parse_json_config::<TelegramConfig>(&record.config_json)?);
            }
            "discord" => {
                config.discord = Some(parse_json_config::<DiscordConfig>(&record.config_json)?);
            }
            "slack" => {
                config.slack = Some(parse_json_config::<SlackConfig>(&record.config_json)?);
            }
            "webhook" => {
                config.webhook = Some(parse_json_config::<WebhookConfig>(&record.config_json)?);
            }
            _ => {}
        }
    }

    Ok(config)
}

fn has_enabled_channels(config: &ChannelsConfig) -> bool {
    config.telegram.is_some()
        || config.discord.is_some()
        || config.slack.is_some()
        || config.webhook.is_some()
}

fn build_channel(kind: &str, config_json: &str) -> Result<Arc<dyn RuntimeChannel>, String> {
    match kind {
        "telegram" => {
            let config = parse_json_config::<TelegramConfig>(config_json)?;
            Ok(Arc::new(TelegramChannel::new(
                config.bot_token,
                config.allowed_users,
                config.mention_only,
            )))
        }
        "discord" => {
            let config = parse_json_config::<DiscordConfig>(config_json)?;
            Ok(Arc::new(DiscordChannel::new(
                config.bot_token,
                config.guild_id,
                config.allowed_users,
                config.listen_to_bots,
                config.mention_only,
            )))
        }
        "slack" => {
            let config = parse_json_config::<SlackConfig>(config_json)?;
            Ok(Arc::new(
                SlackChannel::new(
                    config.bot_token,
                    config.app_token,
                    config.channel_id,
                    Vec::new(),
                    config.allowed_users,
                )
                .with_group_reply_policy(config.mention_only, Vec::new()),
            ))
        }
        "webhook" => {
            let config = parse_json_config::<WebhookConfig>(config_json)?;
            Ok(Arc::new(WebhookChannel::new(
                config.port,
                config.listen_path,
                config.send_url,
                config.send_method,
                config.auth_header,
                config.secret,
            )))
        }
        _ => Err(
            "Unsupported channel kind. Supported kinds: telegram, discord, slack, webhook."
                .to_string(),
        ),
    }
}

fn normalize_config_json(kind: &str, config_json: &str) -> Result<String, String> {
    match kind {
        "telegram" => normalize_json_config::<TelegramConfig>(config_json),
        "discord" => normalize_json_config::<DiscordConfig>(config_json),
        "slack" => normalize_json_config::<SlackConfig>(config_json),
        "webhook" => normalize_json_config::<WebhookConfig>(config_json),
        _ => Err(
            "Unsupported channel kind. Supported kinds: telegram, discord, slack, webhook."
                .to_string(),
        ),
    }
}

fn validate_channel_kind(kind: &str) -> Result<(), String> {
    match kind {
        "telegram" | "discord" | "slack" | "webhook" => Ok(()),
        _ => Err(
            "Unsupported channel kind. Supported kinds: telegram, discord, slack, webhook."
                .to_string(),
        ),
    }
}

fn parse_json_config<T>(config_json: &str) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    serde_json::from_str(config_json)
        .map_err(|error| format!("Failed to parse channel config JSON: {error}"))
}

fn normalize_json_config<T>(config_json: &str) -> Result<String, String>
where
    T: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    let parsed = parse_json_config::<T>(config_json)?;
    serde_json::to_string_pretty(&parsed).map_err(|error| error.to_string())
}

fn make_runtime_status(running: bool, state: &str, message: &str) -> ChannelRuntimeStatusRecord {
    ChannelRuntimeStatusRecord {
        running,
        state: state.to_string(),
        message: message.to_string(),
        updated_at: Utc::now().timestamp_millis().to_string(),
    }
}

fn emit_runtime_status(
    app: &AppHandle,
    state: &AppState,
    status: ChannelRuntimeStatusRecord,
) -> Result<(), String> {
    state.set_channel_runtime_status(status.clone());
    app.emit(CHANNELS_RUNTIME_EVENT, &status)
        .map_err(|error| error.to_string())
}
