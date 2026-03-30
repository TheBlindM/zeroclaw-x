use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;
use url::Url;

use crate::models::update::{UpdateCheckReport, UpdateInstallReport, UpdateSettingsRecord};

pub fn load_update_settings(settings_path: &Path) -> Result<UpdateSettingsRecord, String> {
    if !settings_path.exists() {
        return Ok(UpdateSettingsRecord::default());
    }

    let raw = fs::read_to_string(settings_path).map_err(|error| error.to_string())?;
    let settings = serde_json::from_str::<UpdateSettingsRecord>(&raw)
        .map_err(|_| "Failed to parse update settings file.".to_string())?;

    Ok(settings.normalized())
}

pub fn save_update_settings(
    settings_path: &Path,
    settings: UpdateSettingsRecord,
) -> Result<UpdateSettingsRecord, String> {
    let normalized = settings.normalized();

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let serialized =
        serde_json::to_string_pretty(&normalized).map_err(|error| error.to_string())?;
    fs::write(settings_path, serialized).map_err(|error| error.to_string())?;

    Ok(normalized)
}

pub async fn check_for_update(
    app: &AppHandle,
    settings_path: &Path,
) -> Result<UpdateCheckReport, String> {
    let settings = load_update_settings(settings_path)?;
    check_for_update_with_settings(app, settings).await
}

pub async fn install_update(
    app: &AppHandle,
    settings_path: &Path,
) -> Result<UpdateInstallReport, String> {
    let settings = load_update_settings(settings_path)?;
    let checked_at = current_timestamp();
    let current_version = app.package_info().version.to_string();

    if !settings.enabled {
        return Ok(UpdateInstallReport {
            installed: false,
            version: None,
            checked_at,
            message: "Updates are disabled in desktop preferences.".to_string(),
        });
    }

    if !settings.is_configured() {
        return Ok(UpdateInstallReport {
            installed: false,
            version: None,
            checked_at,
            message: "Update source is not configured yet. Add at least one endpoint and a public key first.".to_string(),
        });
    }

    let urls = parse_update_endpoints(&settings)?;
    let updater = app
        .updater_builder()
        .pubkey(settings.pubkey.clone())
        .endpoints(urls)
        .map_err(|error| error.to_string())?
        .build()
        .map_err(|error| error.to_string())?;

    let update = updater.check().await.map_err(|error| error.to_string())?;
    let Some(update) = update else {
        return Ok(UpdateInstallReport {
            installed: false,
            version: None,
            checked_at,
            message: format!("ZeroClawX {current_version} is already up to date."),
        });
    };

    let version = update.version.clone();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;

    Ok(UpdateInstallReport {
        installed: true,
        version: Some(version.clone()),
        checked_at,
        message: format!(
            "Update {version} was downloaded and handed off to the installer. Follow any system prompts to finish the upgrade."
        ),
    })
}

async fn check_for_update_with_settings(
    app: &AppHandle,
    settings: UpdateSettingsRecord,
) -> Result<UpdateCheckReport, String> {
    let settings = settings.normalized();
    let checked_at = current_timestamp();
    let current_version = app.package_info().version.to_string();

    if !settings.enabled {
        return Ok(UpdateCheckReport {
            configured: settings.is_configured(),
            enabled: false,
            auto_check: settings.auto_check,
            checked_at,
            current_version,
            update_available: false,
            latest_version: None,
            notes: None,
            pub_date: None,
            download_url: None,
            endpoint_count: settings.endpoints.len(),
            message: "Updates are disabled in desktop preferences.".to_string(),
        });
    }

    if !settings.is_configured() {
        return Ok(UpdateCheckReport {
            configured: false,
            enabled: true,
            auto_check: settings.auto_check,
            checked_at,
            current_version,
            update_available: false,
            latest_version: None,
            notes: None,
            pub_date: None,
            download_url: None,
            endpoint_count: settings.endpoints.len(),
            message: "Update source is not configured yet. Add at least one endpoint and a public key first.".to_string(),
        });
    }

    let urls = parse_update_endpoints(&settings)?;
    let updater = app
        .updater_builder()
        .pubkey(settings.pubkey.clone())
        .endpoints(urls)
        .map_err(|error| error.to_string())?
        .build()
        .map_err(|error| error.to_string())?;

    let update = updater.check().await.map_err(|error| error.to_string())?;

    Ok(match update {
        Some(update) => UpdateCheckReport {
            configured: true,
            enabled: true,
            auto_check: settings.auto_check,
            checked_at,
            current_version: update.current_version,
            update_available: true,
            latest_version: Some(update.version.clone()),
            notes: update.body.clone(),
            pub_date: update.date.map(|value| value.to_string()),
            download_url: Some(update.download_url.to_string()),
            endpoint_count: settings.endpoints.len(),
            message: format!("Update {} is available.", update.version),
        },
        None => UpdateCheckReport {
            configured: true,
            enabled: true,
            auto_check: settings.auto_check,
            checked_at,
            current_version: current_version.clone(),
            update_available: false,
            latest_version: Some(current_version.clone()),
            notes: None,
            pub_date: None,
            download_url: None,
            endpoint_count: settings.endpoints.len(),
            message: format!("ZeroClawX {current_version} is already up to date."),
        },
    })
}

fn parse_update_endpoints(settings: &UpdateSettingsRecord) -> Result<Vec<Url>, String> {
    settings
        .endpoints
        .iter()
        .map(|value| {
            Url::parse(value).map_err(|error| format!("Invalid update endpoint `{value}`: {error}"))
        })
        .collect()
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
