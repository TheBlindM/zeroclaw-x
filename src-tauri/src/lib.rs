mod commands;
mod db;
mod models;
mod services;
mod state;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open ZeroClawX", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut tray_builder =
        TrayIconBuilder::with_id("main")
            .menu(&menu)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            });

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder.build(app)?;
    Ok(())
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("zeroclawx=info"));

    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

pub fn run() {
    init_logging();
    info!("starting ZeroClawX desktop app");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            info!(app_data_dir = %app_data_dir.display(), "initializing app state");

            let app_state = state::AppState::new(app_data_dir)?;
            app.manage(app_state.clone());
            services::cron::start_scheduler(app.handle().clone(), app_state);
            build_tray(app)?;
            info!("application setup completed");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::get_auth_login_status,
            commands::auth::list_auth_profiles,
            commands::auth::open_external_url,
            commands::auth::start_auth_login,
            commands::channel::create_channel,
            commands::channel::delete_channel,
            commands::channel::get_channel_runtime_status,
            commands::channel::list_channels,
            commands::channel::start_channel_runtime,
            commands::channel::stop_channel_runtime,
            commands::channel::test_channel,
            commands::channel::update_channel,
            commands::chat::assign_session_project,
            commands::chat::delete_session,
            commands::chat::list_messages,
            commands::chat::list_sessions,
            commands::chat::rename_session,
            commands::chat::respond_to_tool_approval,
            commands::chat::send_message,
            commands::chat::stop_message,
            commands::cron::create_cron_job,
            commands::cron::delete_cron_job,
            commands::cron::list_cron_jobs,
            commands::cron::list_cron_runs,
            commands::cron::run_cron_job_now,
            commands::cron::update_cron_job,
            commands::knowledge::create_project_knowledge_note,
            commands::knowledge::delete_knowledge_document,
            commands::knowledge::get_session_knowledge_scope,
            commands::knowledge::import_project_knowledge_files,
            commands::knowledge::list_project_knowledge,
            commands::knowledge::save_session_knowledge_scope,
            commands::mcp::create_mcp_server,
            commands::mcp::delete_mcp_server,
            commands::mcp::discover_mcp_server_tools,
            commands::mcp::list_mcp_servers,
            commands::mcp::test_mcp_server,
            commands::mcp::update_mcp_server,
            commands::project::create_project,
            commands::project::delete_project,
            commands::project::list_project_sessions,
            commands::project::list_projects,
            commands::project::update_project,
            commands::settings::activate_runtime_profile,
            commands::settings::create_runtime_profile,
            commands::settings::delete_runtime_profile,
            commands::settings::export_runtime_profiles,
            commands::settings::get_runtime_profiles,
            commands::settings::get_runtime_settings,
            commands::settings::get_runtime_status,
            commands::settings::import_runtime_profiles,
            commands::settings::pick_runtime_workspace,
            commands::settings::save_runtime_settings,
            commands::settings::test_runtime_profile,
            commands::settings::test_runtime_settings,
            commands::settings::update_runtime_profile,
            commands::skill::create_skill,
            commands::skill::delete_skill,
            commands::skill::get_skill_detail,
            commands::skill::import_skill_directory,
            commands::skill::install_skill_template,
            commands::skill::list_skill_templates,
            commands::skill::list_skills,
            commands::skill::set_skill_enabled,
            commands::update::check_app_update,
            commands::update::get_update_settings,
            commands::update::install_app_update,
            commands::update::save_update_settings
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            error!(?error, "error while running ZeroClawX");
            panic!("error while running ZeroClawX: {error}");
        });
}
