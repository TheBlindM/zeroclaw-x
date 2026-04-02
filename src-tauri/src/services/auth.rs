use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{Duration, Utc};
use tauri::async_runtime;
use tracing::{error, info, warn};
use url::form_urlencoded::Serializer;
use zeroclaw::auth::{self, AuthService};

use crate::models::auth::{
    AuthLoginChallengeRecord, AuthLoginStatusRecord, AuthProfileRecord, AuthProfilesStateRecord,
};

static AUTH_LOGIN_STATUSES: OnceLock<Mutex<HashMap<String, AuthLoginStatusRecord>>> =
    OnceLock::new();

fn login_statuses() -> &'static Mutex<HashMap<String, AuthLoginStatusRecord>> {
    AUTH_LOGIN_STATUSES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub async fn list_auth_profiles(
    db_path: &Path,
    provider: &str,
) -> Result<AuthProfilesStateRecord, String> {
    let provider = normalize_provider(provider)?;
    info!(provider, "loading auth profiles");
    let auth_service = auth_service_from_db(db_path);
    let data = auth_service
        .load_profiles()
        .await
        .map_err(|error| error.to_string())?;
    let active_profile_id = data.active_profiles.get(&provider).cloned();

    let mut profiles = data
        .profiles
        .into_values()
        .filter(|profile| profile.provider == provider)
        .map(|profile| AuthProfileRecord {
            id: profile.id.clone(),
            provider: profile.provider.clone(),
            profile_name: profile.profile_name.clone(),
            kind: match profile.kind {
                zeroclaw::auth::profiles::AuthProfileKind::OAuth => "oauth".to_string(),
                zeroclaw::auth::profiles::AuthProfileKind::Token => "token".to_string(),
            },
            account_id: profile.account_id.clone(),
            expires_at: profile
                .token_set
                .as_ref()
                .and_then(|token_set| token_set.expires_at.map(|value| value.to_rfc3339())),
            created_at: profile.created_at.to_rfc3339(),
            updated_at: profile.updated_at.to_rfc3339(),
            is_active: active_profile_id
                .as_ref()
                .is_some_and(|active_id| active_id == &profile.id),
        })
        .collect::<Vec<_>>();

    profiles.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));

    Ok(AuthProfilesStateRecord {
        provider,
        active_profile_id,
        profiles,
    })
}

pub async fn start_auth_login(
    db_path: &Path,
    provider: &str,
    profile_name: &str,
) -> Result<AuthLoginChallengeRecord, String> {
    let provider = normalize_provider(provider)?;
    info!(provider, profile_name, "starting auth login");
    if !supports_auth_login(&provider) {
        warn!(provider, "auth login requested for unsupported provider");
        return Err(
            "Auth login is currently supported only for openai-codex and gemini.".to_string(),
        );
    }

    let profile_name = normalize_profile_name(profile_name);
    let runtime_root = runtime_root_from_db(db_path);
    fs::create_dir_all(&runtime_root).map_err(|error| error.to_string())?;

    let login_id = make_login_id();
    let client = reqwest::Client::new();

    let challenge = match provider.as_str() {
        "openai-codex" => {
            let pkce = auth::openai_oauth::generate_pkce_state();
            let authorize_url = auth::openai_oauth::build_authorize_url(&pkce);
            info!(
                provider,
                profile_name, authorize_url, "created OpenAI browser auth challenge"
            );
            let challenge = AuthLoginChallengeRecord {
                login_id: login_id.clone(),
                provider: provider.clone(),
                profile_name: profile_name.clone(),
                verification_uri: authorize_url.clone(),
                verification_uri_complete: Some(authorize_url.clone()),
                user_code: String::new(),
                expires_at: (Utc::now() + Duration::minutes(3)).to_rfc3339(),
                interval_seconds: 3,
                message: Some(
                    "Open the authorization page in your browser and complete the sign-in flow."
                        .to_string(),
                ),
            };

            store_login_status(AuthLoginStatusRecord {
                login_id: login_id.clone(),
                provider: provider.clone(),
                profile_name: profile_name.clone(),
                status: "pending".to_string(),
                message: challenge
                    .message
                    .clone()
                    .unwrap_or_else(|| "Waiting for browser authorization.".to_string()),
                completed_profile_id: None,
                completed_at: None,
            });

            spawn_openai_login_task(
                runtime_root,
                login_id.clone(),
                profile_name.clone(),
                client,
                pkce,
            );
            challenge
        }
        "gemini" => {
            let device = auth::gemini_oauth::start_device_code_flow(&client)
                .await
                .map_err(|error| error.to_string())?;
            info!(
                provider,
                profile_name,
                verification_uri = device.verification_uri,
                "created Gemini device-code auth challenge"
            );
            let challenge = AuthLoginChallengeRecord {
                login_id: login_id.clone(),
                provider: provider.clone(),
                profile_name: profile_name.clone(),
                verification_uri: device.verification_uri.clone(),
                verification_uri_complete: device
                    .verification_uri_complete
                    .clone()
                    .or_else(|| {
                        derive_verification_uri_complete(&device.verification_uri, &device.user_code)
                    }),
                user_code: device.user_code.clone(),
                expires_at: (Utc::now() + Duration::seconds(device.expires_in as i64)).to_rfc3339(),
                interval_seconds: device.interval,
                message: Some(
                    "Gemini device-code login requires GEMINI_OAUTH_CLIENT_ID and GEMINI_OAUTH_CLIENT_SECRET in the app environment.".to_string(),
                ),
            };

            store_login_status(AuthLoginStatusRecord {
                login_id: login_id.clone(),
                provider: provider.clone(),
                profile_name: profile_name.clone(),
                status: "pending".to_string(),
                message: challenge
                    .message
                    .clone()
                    .unwrap_or_else(|| "Waiting for device-code authorization.".to_string()),
                completed_profile_id: None,
                completed_at: None,
            });

            spawn_gemini_login_task(
                runtime_root,
                login_id.clone(),
                profile_name.clone(),
                client,
                device,
            );
            challenge
        }
        _ => {
            return Err(
                "Auth login is currently supported only for openai-codex and gemini.".to_string(),
            )
        }
    };

    Ok(challenge)
}

pub fn get_auth_login_status(login_id: &str) -> Result<AuthLoginStatusRecord, String> {
    login_statuses()
        .lock()
        .map_err(|_| "Failed to read auth login status.".to_string())?
        .get(login_id)
        .cloned()
        .ok_or_else(|| "Auth login session not found.".to_string())
}

pub fn open_external_url(url: &str) -> Result<(), String> {
    info!(url, "opening external browser url");
    webbrowser::open(url)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn spawn_openai_login_task(
    runtime_root: PathBuf,
    login_id: String,
    profile_name: String,
    client: reqwest::Client,
    pkce: auth::openai_oauth::PkceState,
) {
    async_runtime::spawn(async move {
        info!(
            login_id,
            profile_name, "waiting for OpenAI browser auth callback"
        );
        let auth_service = AuthService::new(&runtime_root, true);
        let result = async {
            let code = auth::openai_oauth::receive_loopback_code(
                &pkce.state,
                std::time::Duration::from_secs(180),
            )
            .await?;
            let token_set =
                auth::openai_oauth::exchange_code_for_tokens(&client, &code, &pkce).await?;
            let account_id =
                auth::openai_oauth::extract_account_id_from_jwt(&token_set.access_token);
            let profile = auth_service
                .store_openai_tokens(&profile_name, token_set, account_id, true)
                .await?;
            Ok::<String, anyhow::Error>(profile.id)
        }
        .await;

        finalize_login_status(&login_id, "openai-codex", &profile_name, result);
    });
}

fn spawn_gemini_login_task(
    runtime_root: PathBuf,
    login_id: String,
    profile_name: String,
    client: reqwest::Client,
    device: auth::gemini_oauth::DeviceCodeStart,
) {
    async_runtime::spawn(async move {
        info!(
            login_id,
            profile_name, "polling Gemini device-code auth flow"
        );
        let auth_service = AuthService::new(&runtime_root, true);
        let result = async {
            let token_set = auth::gemini_oauth::poll_device_code_tokens(&client, &device).await?;
            let account_id = token_set
                .id_token
                .as_deref()
                .and_then(auth::gemini_oauth::extract_account_email_from_id_token);
            let profile = auth_service
                .store_gemini_tokens(&profile_name, token_set, account_id, true)
                .await?;
            Ok::<String, anyhow::Error>(profile.id)
        }
        .await;

        finalize_login_status(&login_id, "gemini", &profile_name, result);
    });
}

fn finalize_login_status(
    login_id: &str,
    provider: &str,
    profile_name: &str,
    result: Result<String, anyhow::Error>,
) {
    let status = match result {
        Ok(profile_id) => {
            info!(
                login_id,
                provider, profile_name, profile_id, "auth login succeeded"
            );
            AuthLoginStatusRecord {
                login_id: login_id.to_string(),
                provider: provider.to_string(),
                profile_name: profile_name.to_string(),
                status: "succeeded".to_string(),
                message: format!("Saved auth profile {profile_name}."),
                completed_profile_id: Some(profile_id),
                completed_at: Some(Utc::now().to_rfc3339()),
            }
        }
        Err(error) => {
            error!(login_id, provider, profile_name, error = %error, "auth login failed");
            AuthLoginStatusRecord {
                login_id: login_id.to_string(),
                provider: provider.to_string(),
                profile_name: profile_name.to_string(),
                status: "failed".to_string(),
                message: error.to_string(),
                completed_profile_id: None,
                completed_at: Some(Utc::now().to_rfc3339()),
            }
        }
    };

    store_login_status(status);
}

fn store_login_status(status: AuthLoginStatusRecord) {
    if let Ok(mut statuses) = login_statuses().lock() {
        statuses.insert(status.login_id.clone(), status);
    }
}

fn auth_service_from_db(db_path: &Path) -> AuthService {
    let runtime_root = runtime_root_from_db(db_path);
    AuthService::new(&runtime_root, true)
}

fn runtime_root_from_db(db_path: &Path) -> PathBuf {
    db_path.parent().map_or_else(
        || PathBuf::from("."),
        |parent| parent.join("zeroclaw-runtime"),
    )
}

fn normalize_provider(provider: &str) -> Result<String, String> {
    auth::normalize_provider(provider).map_err(|error| error.to_string())
}

fn supports_auth_login(provider: &str) -> bool {
    matches!(provider, "openai-codex" | "gemini")
}

fn normalize_profile_name(profile_name: &str) -> String {
    let trimmed = profile_name.trim();
    if trimmed.is_empty() {
        "default".to_string()
    } else {
        trimmed.to_string()
    }
}

fn derive_verification_uri_complete(verification_uri: &str, user_code: &str) -> Option<String> {
    if verification_uri.is_empty() || user_code.is_empty() {
        return None;
    }

    let mut serializer = Serializer::new(String::new());
    serializer.append_pair("user_code", user_code);
    let query = serializer.finish();
    let separator = if verification_uri.contains('?') {
        "&"
    } else {
        "?"
    };
    Some(format!("{verification_uri}{separator}{query}"))
}

fn make_login_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("auth-{millis}")
}
