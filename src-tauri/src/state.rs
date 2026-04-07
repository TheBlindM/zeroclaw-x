use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::async_runtime::JoinHandle;
use tokio::sync::oneshot;

use crate::{
    db,
    models::{
        channel::ChannelRuntimeStatusRecord,
        chat::{ChatApprovalDecision, ChatApprovalRequestPayload},
    },
};

struct PendingApproval {
    session_id: String,
    tool_name: String,
    sender: oneshot::Sender<ChatApprovalDecision>,
}

#[derive(Clone)]
pub struct AppState {
    db_path: Arc<PathBuf>,
    settings_path: Arc<PathBuf>,
    update_settings_path: Arc<PathBuf>,
    cancelled_sessions: Arc<Mutex<HashSet<String>>>,
    running_cron_jobs: Arc<Mutex<HashSet<String>>>,
    pending_approvals: Arc<Mutex<HashMap<String, PendingApproval>>>,
    session_tool_allowlists: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    channel_runtime_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    channel_runtime_status: Arc<Mutex<ChannelRuntimeStatusRecord>>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&app_data_dir).map_err(|error| error.to_string())?;
        let db_path = app_data_dir.join("zeroclawx.db");
        let settings_path = app_data_dir.join("runtime-settings.json");
        let update_settings_path = app_data_dir.join("update-settings.json");
        db::initialize(&db_path)?;

        Ok(Self {
            db_path: Arc::new(db_path),
            settings_path: Arc::new(settings_path),
            update_settings_path: Arc::new(update_settings_path),
            cancelled_sessions: Arc::new(Mutex::new(HashSet::new())),
            running_cron_jobs: Arc::new(Mutex::new(HashSet::new())),
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            session_tool_allowlists: Arc::new(Mutex::new(HashMap::new())),
            channel_runtime_task: Arc::new(Mutex::new(None)),
            channel_runtime_status: Arc::new(Mutex::new(ChannelRuntimeStatusRecord {
                running: false,
                state: "idle".to_string(),
                message: "Channels supervisor is idle.".to_string(),
                updated_at: current_timestamp(),
            })),
        })
    }

    pub fn db_path(&self) -> PathBuf {
        self.db_path.as_ref().clone()
    }

    pub fn settings_path(&self) -> PathBuf {
        self.settings_path.as_ref().clone()
    }

    pub fn update_settings_path(&self) -> PathBuf {
        self.update_settings_path.as_ref().clone()
    }

    pub fn try_start_cron_job(&self, job_id: &str) -> bool {
        self.running_cron_jobs
            .lock()
            .map(|mut running| running.insert(job_id.to_string()))
            .unwrap_or(false)
    }

    pub fn finish_cron_job(&self, job_id: &str) {
        if let Ok(mut running) = self.running_cron_jobs.lock() {
            running.remove(job_id);
        }
    }

    pub fn has_channel_runtime_task(&self) -> bool {
        self.channel_runtime_task
            .lock()
            .map(|task| task.is_some())
            .unwrap_or(false)
    }

    pub fn store_channel_runtime_task(&self, task: JoinHandle<()>) {
        if let Ok(mut slot) = self.channel_runtime_task.lock() {
            *slot = Some(task);
        }
    }

    pub fn take_channel_runtime_task(&self) -> Option<JoinHandle<()>> {
        self.channel_runtime_task
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }

    pub fn clear_channel_runtime_task(&self) {
        if let Ok(mut slot) = self.channel_runtime_task.lock() {
            *slot = None;
        }
    }

    pub fn get_channel_runtime_status(&self) -> ChannelRuntimeStatusRecord {
        self.channel_runtime_status
            .lock()
            .map(|status| status.clone())
            .unwrap_or(ChannelRuntimeStatusRecord {
                running: false,
                state: "error".to_string(),
                message: "Failed to read channel runtime status.".to_string(),
                updated_at: current_timestamp(),
            })
    }

    pub fn set_channel_runtime_status(&self, status: ChannelRuntimeStatusRecord) {
        if let Ok(mut slot) = self.channel_runtime_status.lock() {
            *slot = status;
        }
    }

    pub fn cancel_session(&self, session_id: &str) {
        if let Ok(mut cancelled) = self.cancelled_sessions.lock() {
            cancelled.insert(session_id.to_owned());
        }

        self.reject_session_approvals(session_id);
    }

    pub fn clear_cancellation(&self, session_id: &str) {
        if let Ok(mut cancelled) = self.cancelled_sessions.lock() {
            cancelled.remove(session_id);
        }
    }

    pub fn take_cancellation(&self, session_id: &str) -> bool {
        self.cancelled_sessions
            .lock()
            .map(|mut cancelled| cancelled.remove(session_id))
            .unwrap_or(false)
    }

    pub fn register_approval_request(
        &self,
        session_id: &str,
        tool_name: &str,
        arguments_summary: &str,
        requested_by: Option<&str>,
    ) -> Result<
        (
            ChatApprovalRequestPayload,
            oneshot::Receiver<ChatApprovalDecision>,
        ),
        String,
    > {
        let request_id = make_request_id("approval");
        let payload = ChatApprovalRequestPayload {
            request_id: request_id.clone(),
            session_id: session_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments_summary: arguments_summary.to_string(),
            requested_by: requested_by.map(ToString::to_string),
        };
        let (sender, receiver) = oneshot::channel();

        self.pending_approvals
            .lock()
            .map_err(|_| "Failed to register the approval request.".to_string())?
            .insert(
                request_id,
                PendingApproval {
                    session_id: session_id.to_string(),
                    tool_name: tool_name.to_string(),
                    sender,
                },
            );

        Ok((payload, receiver))
    }

    pub fn resolve_approval(
        &self,
        request_id: &str,
        decision: ChatApprovalDecision,
    ) -> Result<(), String> {
        let pending = self
            .pending_approvals
            .lock()
            .map_err(|_| "Failed to access pending approvals.".to_string())?
            .remove(request_id)
            .ok_or_else(|| "Approval request not found.".to_string())?;

        if decision == ChatApprovalDecision::Always {
            self.remember_tool_allowance(&pending.session_id, &pending.tool_name);
        }

        pending
            .sender
            .send(decision)
            .map_err(|_| "Approval request already closed.".to_string())
    }

    pub fn remember_tool_allowance(&self, session_id: &str, tool_name: &str) {
        if let Ok(mut allowlists) = self.session_tool_allowlists.lock() {
            allowlists
                .entry(session_id.to_string())
                .or_default()
                .insert(tool_name.to_string());
        }
    }

    pub fn is_tool_allowed_for_session(&self, session_id: &str, tool_name: &str) -> bool {
        self.session_tool_allowlists
            .lock()
            .ok()
            .and_then(|allowlists| allowlists.get(session_id).cloned())
            .is_some_and(|tools| tools.contains(tool_name))
    }

    pub fn clear_session_runtime_state(&self, session_id: &str) {
        self.reject_session_approvals(session_id);

        if let Ok(mut allowlists) = self.session_tool_allowlists.lock() {
            allowlists.remove(session_id);
        }

        if let Ok(mut cancelled) = self.cancelled_sessions.lock() {
            cancelled.remove(session_id);
        }
    }

    fn reject_session_approvals(&self, session_id: &str) {
        let mut senders = Vec::new();

        if let Ok(mut pending) = self.pending_approvals.lock() {
            let request_ids = pending
                .iter()
                .filter_map(|(request_id, approval)| {
                    if approval.session_id == session_id {
                        Some(request_id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            for request_id in request_ids {
                if let Some(approval) = pending.remove(&request_id) {
                    senders.push(approval.sender);
                }
            }
        }

        for sender in senders {
            let _ = sender.send(ChatApprovalDecision::No);
        }
    }
}

fn make_request_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    format!("{prefix}-{nanos}")
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
