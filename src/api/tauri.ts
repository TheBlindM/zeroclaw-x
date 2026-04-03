import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ChatTokenPayload {
  session_id: string;
  token: string;
}

export interface ChatDonePayload {
  session_id: string;
}

export interface ChatErrorPayload {
  session_id: string;
  error: string;
}

export interface ChatContextPayload {
  session_id: string;
  project_name: string;
  scope_mode: string;
  knowledge_titles: string[];
}

export interface ChatApprovalRequestPayload {
  request_id: string;
  session_id: string;
  tool_name: string;
  arguments_summary: string;
}

export type ChatApprovalDecision = "yes" | "no" | "always";

export interface SessionRecord {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  last_message_preview: string | null;
  project_id: string | null;
  agent_mode: boolean;
}

export interface MessageRecord {
  id: string;
  session_id: string;
  role: "system" | "user" | "assistant" | string;
  content: string;
  created_at: string;
}

export interface ProjectRecord {
  id: string;
  name: string;
  description: string;
  status: string;
  pinned: boolean;
  created_at: string;
  updated_at: string;
}

export interface KnowledgeDocumentRecord {
  id: string;
  project_id: string;
  title: string;
  source_path: string;
  content: string;
  content_preview: string;
  created_at: string;
  updated_at: string;
}

export interface SessionKnowledgeScopeRecord {
  session_id: string;
  mode: string;
  document_ids: string[];
}

export type RuntimeProxyScopeRecord = "environment" | "zeroclaw" | "services";
export type RuntimeAutonomyLevelRecord = "read_only" | "supervised" | "full";
export type RuntimeCredentialModeRecord = "api_key" | "auth_profile";

export interface RuntimeProviderEntryRecord {
  id: string;
  name: string;
  provider: string;
  model: string;
  provider_url: string;
  api_key: string;
  credential_mode: RuntimeCredentialModeRecord;
  auth_profile: string;
  temperature: number;
}

export interface RuntimeProviderGroupRecord {
  id: string;
  name: string;
  active_entry_id: string;
  entries: RuntimeProviderEntryRecord[];
}

export interface RuntimeProxySettingsRecord {
  enabled: boolean;
  scope: RuntimeProxyScopeRecord;
  http_proxy: string;
  https_proxy: string;
  all_proxy: string;
  no_proxy: string[];
  services: string[];
}

export interface RuntimeProxySupportRecord {
  supported_service_keys: string[];
  supported_selectors: string[];
}

export interface RuntimeAgentSettingsRecord {
  workspace_dir: string;
  compact_context: boolean;
  max_tool_iterations: number;
  max_history_messages: number;
  max_context_tokens: number;
  parallel_tools: boolean;
  tool_dispatcher: string;
}

export interface RuntimeAutonomySettingsRecord {
  level: RuntimeAutonomyLevelRecord;
  workspace_only: boolean;
  require_approval_for_medium_risk: boolean;
  block_high_risk_commands: boolean;
  allowed_commands: string[];
  allowed_roots: string[];
  shell_env_passthrough: string[];
  auto_approve: string[];
  always_ask: string[];
}

export interface RuntimeSettingsRecord {
  active_group_id: string;
  groups: RuntimeProviderGroupRecord[];
  active_entry_id: string;
  entries: RuntimeProviderEntryRecord[];
  provider: string;
  model: string;
  provider_url: string;
  api_key: string;
  credential_mode: RuntimeCredentialModeRecord;
  auth_profile: string;
  temperature: number;
  proxy: RuntimeProxySettingsRecord;
  agent: RuntimeAgentSettingsRecord;
  autonomy: RuntimeAutonomySettingsRecord;
}

export interface RuntimeProfileRecord {
  id: string;
  name: string;
  settings: RuntimeSettingsRecord;
}

export interface RuntimeProfilesState {
  active_profile_id: string;
  profiles: RuntimeProfileRecord[];
}

export interface RuntimeConnectionReport {
  ok: boolean;
  provider: string;
  model: string;
  message: string;
  preview: string | null;
}

export interface RuntimeProfilesExportReport {
  path: string;
  profile_count: number;
}

export interface RuntimeProfilesImportReport {
  path: string;
  imported_count: number;
  profiles: RuntimeProfilesState;
}

export interface RuntimeStatusRecord {
  profile_id: string;
  profile_name: string;
  provider: string;
  model: string;
  provider_url: string;
  temperature: number;
  api_key_configured: boolean;
  credential_mode: RuntimeCredentialModeRecord;
  auth_profile: string;
  workspace_dir: string;
  tool_dispatcher: string;
  autonomy_level: RuntimeAutonomyLevelRecord;
  workspace_only: boolean;
  parallel_tools: boolean;
}

export interface AuthProfileRecord {
  id: string;
  provider: string;
  profile_name: string;
  kind: string;
  account_id: string | null;
  expires_at: string | null;
  created_at: string;
  updated_at: string;
  is_active: boolean;
}

export interface AuthProfilesStateRecord {
  provider: string;
  active_profile_id: string | null;
  profiles: AuthProfileRecord[];
}

export interface AuthLoginChallengeRecord {
  login_id: string;
  provider: string;
  profile_name: string;
  verification_uri: string;
  verification_uri_complete: string | null;
  user_code: string;
  expires_at: string;
  interval_seconds: number;
  message: string | null;
}

export interface AuthLoginStatusRecord {
  login_id: string;
  provider: string;
  profile_name: string;
  status: string;
  message: string;
  completed_profile_id: string | null;
  completed_at: string | null;
}

export interface UpdateSettingsRecord {
  enabled: boolean;
  auto_check: boolean;
  endpoints: string[];
  pubkey: string;
}

export interface UpdateCheckReport {
  configured: boolean;
  enabled: boolean;
  auto_check: boolean;
  checked_at: string;
  current_version: string;
  update_available: boolean;
  latest_version: string | null;
  notes: string | null;
  pub_date: string | null;
  download_url: string | null;
  endpoint_count: number;
  message: string;
}

export interface UpdateInstallReport {
  installed: boolean;
  version: string | null;
  checked_at: string;
  message: string;
}

export interface CronJobRecord {
  id: string;
  name: string;
  schedule: string;
  prompt: string;
  enabled: boolean;
  last_run_at: string | null;
  next_run_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CronRunRecord {
  id: string;
  job_id: string;
  status: string;
  output: string;
  started_at: string;
  finished_at: string;
}

export interface SendMessageOptions {
  sessionTitle?: string;
  projectId?: string | null;
  knowledgeMode?: string;
  knowledgeDocumentIds?: string[];
  agentMode?: boolean;
}

export function sendMessage(sessionId: string, content: string, options: SendMessageOptions = {}) {
  return invoke("send_message", {
    sessionId,
    content,
    sessionTitle: options.sessionTitle,
    projectId: options.projectId ?? null,
    knowledgeMode: options.knowledgeMode ?? "auto",
    knowledgeDocumentIds: options.knowledgeDocumentIds ?? [],
    agentMode: options.agentMode ?? false
  });
}

export function respondToToolApproval(requestId: string, decision: ChatApprovalDecision) {
  return invoke("respond_to_tool_approval", {
    requestId,
    decision
  });
}

export function stopMessage(sessionId: string) {
  return invoke("stop_message", {
    sessionId
  });
}

export function listSessions() {
  return invoke<SessionRecord[]>("list_sessions");
}

export function listMessages(sessionId: string) {
  return invoke<MessageRecord[]>("list_messages", {
    sessionId
  });
}

export function renameSession(sessionId: string, title: string) {
  return invoke("rename_session", {
    sessionId,
    title
  });
}

export function setSessionAgentMode(sessionId: string, agentMode: boolean) {
  return invoke("set_session_agent_mode", {
    sessionId,
    agentMode
  });
}

export function assignSessionProject(sessionId: string, projectId: string | null) {
  return invoke("assign_session_project", {
    sessionId,
    projectId
  });
}

export function deleteSession(sessionId: string) {
  return invoke("delete_session", {
    sessionId
  });
}

export function listProjects() {
  return invoke<ProjectRecord[]>("list_projects");
}

export function listProjectSessions(projectId: string) {
  return invoke<SessionRecord[]>("list_project_sessions", {
    projectId
  });
}

export function createProject(name: string, description: string, status: string, pinned: boolean) {
  return invoke<ProjectRecord>("create_project", {
    name,
    description,
    status,
    pinned
  });
}

export function updateProject(projectId: string, name: string, description: string, status: string, pinned: boolean) {
  return invoke<ProjectRecord>("update_project", {
    projectId,
    name,
    description,
    status,
    pinned
  });
}

export function deleteProject(projectId: string) {
  return invoke("delete_project", {
    projectId
  });
}

export function listProjectKnowledge(projectId: string) {
  return invoke<KnowledgeDocumentRecord[]>("list_project_knowledge", {
    projectId
  });
}

export function getSessionKnowledgeScope(sessionId: string) {
  return invoke<SessionKnowledgeScopeRecord>("get_session_knowledge_scope", {
    sessionId
  });
}

export function saveSessionKnowledgeScope(sessionId: string, mode: string, documentIds: string[]) {
  return invoke<SessionKnowledgeScopeRecord>("save_session_knowledge_scope", {
    sessionId,
    mode,
    documentIds
  });
}

export function createProjectKnowledgeNote(projectId: string, title: string, content: string) {
  return invoke<KnowledgeDocumentRecord>("create_project_knowledge_note", {
    projectId,
    title,
    content
  });
}

export function importProjectKnowledgeFiles(projectId: string) {
  return invoke<KnowledgeDocumentRecord[]>("import_project_knowledge_files", {
    projectId
  });
}

export function deleteKnowledgeDocument(documentId: string) {
  return invoke("delete_knowledge_document", {
    documentId
  });
}

export function getRuntimeProfiles() {
  return invoke<RuntimeProfilesState>("get_runtime_profiles");
}

export function getRuntimeSettings() {
  return invoke<RuntimeSettingsRecord>("get_runtime_settings");
}

export function getRuntimeStatus() {
  return invoke<RuntimeStatusRecord>("get_runtime_status");
}

export function getProxySettings() {
  return invoke<RuntimeProxySettingsRecord>("get_proxy_settings");
}

export function saveProxySettings(settings: RuntimeProxySettingsRecord) {
  return invoke<RuntimeProxySettingsRecord>("save_proxy_settings", {
    settings
  });
}

export function getProxySupport() {
  return invoke<RuntimeProxySupportRecord>("get_proxy_support");
}

export function saveRuntimeSettings(settings: RuntimeSettingsRecord) {
  return invoke<RuntimeProfilesState>("save_runtime_settings", {
    settings
  });
}

export function createRuntimeProfile(name: string, settings: RuntimeSettingsRecord) {
  return invoke<RuntimeProfilesState>("create_runtime_profile", {
    name,
    settings
  });
}

export function updateRuntimeProfile(profileId: string, name: string, settings: RuntimeSettingsRecord) {
  return invoke<RuntimeProfilesState>("update_runtime_profile", {
    profileId,
    name,
    settings
  });
}

export function activateRuntimeProfile(profileId: string) {
  return invoke<RuntimeProfilesState>("activate_runtime_profile", {
    profileId
  });
}

export function deleteRuntimeProfile(profileId: string) {
  return invoke<RuntimeProfilesState>("delete_runtime_profile", {
    profileId
  });
}

export function exportRuntimeProfiles() {
  return invoke<RuntimeProfilesExportReport | null>("export_runtime_profiles");
}

export function importRuntimeProfiles() {
  return invoke<RuntimeProfilesImportReport | null>("import_runtime_profiles");
}

export function pickRuntimeWorkspace() {
  return invoke<string | null>("pick_runtime_workspace");
}

export function testRuntimeSettings(settings: RuntimeSettingsRecord) {
  return invoke<RuntimeConnectionReport>("test_runtime_settings", {
    settings
  });
}

export function testRuntimeProfile(profile: RuntimeProfileRecord) {
  return invoke<RuntimeConnectionReport>("test_runtime_profile", {
    profile
  });
}

export function listAuthProfiles(provider: string) {
  return invoke<AuthProfilesStateRecord>("list_auth_profiles", {
    provider
  });
}

export function startAuthLogin(provider: string, profileName: string) {
  return invoke<AuthLoginChallengeRecord>("start_auth_login", {
    provider,
    profileName
  });
}

export function openExternalUrl(url: string) {
  return invoke<void>("open_external_url", {
    url
  });
}

export function getAuthLoginStatus(loginId: string) {
  return invoke<AuthLoginStatusRecord>("get_auth_login_status", {
    loginId
  });
}

export function getUpdateSettings() {
  return invoke<UpdateSettingsRecord>("get_update_settings");
}

export function saveUpdateSettings(settings: UpdateSettingsRecord) {
  return invoke<UpdateSettingsRecord>("save_update_settings", {
    settings
  });
}

export function checkAppUpdate() {
  return invoke<UpdateCheckReport>("check_app_update");
}

export function installAppUpdate() {
  return invoke<UpdateInstallReport>("install_app_update");
}

export function onChatApprovalRequest(listener: (payload: ChatApprovalRequestPayload) => void) {
  return listen<ChatApprovalRequestPayload>("chat:approval-request", (event) => listener(event.payload));
}

export function onChatContext(listener: (payload: ChatContextPayload) => void) {
  return listen<ChatContextPayload>("chat:context", (event) => listener(event.payload));
}

export function onChatToken(listener: (payload: ChatTokenPayload) => void) {
  return listen<ChatTokenPayload>("chat:token", (event) => listener(event.payload));
}

export function onChatDone(listener: (payload: ChatDonePayload) => void) {
  return listen<ChatDonePayload>("chat:done", (event) => listener(event.payload));
}

export function onChatError(listener: (payload: ChatErrorPayload) => void) {
  return listen<ChatErrorPayload>("chat:error", (event) => listener(event.payload));
}

export function listCronJobs() {
  return invoke<CronJobRecord[]>("list_cron_jobs");
}

export function listCronRuns(jobId: string) {
  return invoke<CronRunRecord[]>("list_cron_runs", {
    jobId
  });
}

export function createCronJob(name: string, schedule: string, prompt: string, enabled: boolean) {
  return invoke<CronJobRecord>("create_cron_job", {
    name,
    schedule,
    prompt,
    enabled
  });
}

export function updateCronJob(jobId: string, name: string, schedule: string, prompt: string, enabled: boolean) {
  return invoke<CronJobRecord>("update_cron_job", {
    jobId,
    name,
    schedule,
    prompt,
    enabled
  });
}

export function deleteCronJob(jobId: string) {
  return invoke("delete_cron_job", {
    jobId
  });
}

export function runCronJobNow(jobId: string) {
  return invoke<CronRunRecord>("run_cron_job_now", {
    jobId
  });
}

export function onCronJobUpdated(listener: (payload: CronJobRecord) => void) {
  return listen<CronJobRecord>("cron:job-updated", (event) => listener(event.payload));
}

export function onCronRunRecorded(listener: (payload: CronRunRecord) => void) {
  return listen<CronRunRecord>("cron:run-recorded", (event) => listener(event.payload));
}

export interface ChannelRecord {
  id: string;
  name: string;
  kind: string;
  config_json: string;
  enabled: boolean;
  last_checked_at: string | null;
  last_health_status: string | null;
  last_health_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface ChannelDraft {
  name: string;
  kind: string;
  config_json: string;
  enabled: boolean;
}

export interface ChannelTestReport {
  ok: boolean;
  kind: string;
  message: string;
  checked_at: string;
}

export interface ChannelTestResult {
  channel: ChannelRecord;
  report: ChannelTestReport;
}

export interface ChannelRuntimeStatusRecord {
  running: boolean;
  state: string;
  message: string;
  updated_at: string;
}

export function listChannels() {
  return invoke<ChannelRecord[]>("list_channels");
}

export function createChannel(channel: ChannelDraft) {
  return invoke<ChannelRecord>("create_channel", {
    channel
  });
}

export function updateChannel(channelId: string, channel: ChannelDraft) {
  return invoke<ChannelRecord>("update_channel", {
    channelId,
    channel
  });
}

export function deleteChannel(channelId: string) {
  return invoke("delete_channel", {
    channelId
  });
}

export function testChannel(channelId: string) {
  return invoke<ChannelTestResult>("test_channel", {
    channelId
  });
}

export function getChannelRuntimeStatus() {
  return invoke<ChannelRuntimeStatusRecord>("get_channel_runtime_status");
}

export function startChannelRuntime() {
  return invoke<ChannelRuntimeStatusRecord>("start_channel_runtime");
}

export function stopChannelRuntime() {
  return invoke<ChannelRuntimeStatusRecord>("stop_channel_runtime");
}

export function onChannelRuntimeStatus(listener: (payload: ChannelRuntimeStatusRecord) => void) {
  return listen<ChannelRuntimeStatusRecord>("channels:runtime-status", (event) => listener(event.payload));
}
export interface SkillRecord {
  id: string;
  slug: string;
  name: string;
  description: string;
  version: string;
  author: string;
  tags_json: string;
  source_kind: string;
  source_label: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface SkillTemplateRecord {
  template_id: string;
  slug: string;
  name: string;
  description: string;
  author: string;
  tags_json: string;
}

export interface SkillDetailRecord {
  skill: SkillRecord;
  markdown_content: string;
  directory_path: string;
  manifest_path: string;
  source_path: string | null;
}

export interface SkillDraft {
  slug: string;
  name: string;
  description: string;
  version: string;
  author: string;
  tags_json: string;
  markdown_content: string;
  enabled: boolean;
}

export interface SkillExportReport {
  path: string;
}

export function listSkillTemplates() {
  return invoke<SkillTemplateRecord[]>("list_skill_templates");
}

export function listSkills() {
  return invoke<SkillRecord[]>("list_skills");
}

export function createSkill(skill: SkillDraft) {
  return invoke<SkillRecord>("create_skill", {
    skill
  });
}

export function updateSkill(skillId: string, skill: SkillDraft) {
  return invoke<SkillRecord>("update_skill", {
    skillId,
    skill
  });
}

export function getSkillDetail(skillId: string) {
  return invoke<SkillDetailRecord>("get_skill_detail", {
    skillId
  });
}

export function installSkillTemplate(templateId: string) {
  return invoke<SkillRecord>("install_skill_template", {
    templateId
  });
}

export function importSkillDirectory() {
  return invoke<SkillRecord | null>("import_skill_directory");
}

export function duplicateSkill(skillId: string) {
  return invoke<SkillRecord>("duplicate_skill", {
    skillId
  });
}

export function refreshSkill(skillId: string) {
  return invoke<SkillRecord>("refresh_skill", {
    skillId
  });
}

export function exportSkill(skillId: string) {
  return invoke<SkillExportReport | null>("export_skill", {
    skillId
  });
}

export function openSkillDirectory(skillId: string) {
  return invoke<string>("open_skill_directory", {
    skillId
  });
}

export function setSkillEnabled(skillId: string, enabled: boolean) {
  return invoke<SkillRecord>("set_skill_enabled", {
    skillId,
    enabled
  });
}

export function deleteSkill(skillId: string) {
  return invoke<SkillRecord>("delete_skill", {
    skillId
  });
}

export interface McpServerRecord {
  id: string;
  name: string;
  transport: string;
  command: string;
  arguments_json: string;
  url: string;
  headers_json: string;
  environment_json: string;
  enabled: boolean;
  last_tested_at: string | null;
  last_test_status: string | null;
  last_test_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface McpServerDraft {
  name: string;
  transport: string;
  command: string;
  arguments_json: string;
  url: string;
  headers_json: string;
  environment_json: string;
  enabled: boolean;
}

export interface McpServerTestReport {
  ok: boolean;
  transport: string;
  message: string;
  details: string | null;
  checked_at: string;
}

export interface McpServerTestResult {
  server: McpServerRecord;
  report: McpServerTestReport;
}

export interface McpToolRecord {
  full_name: string;
  tool_name: string;
  server_name: string;
  description: string;
  input_schema_json: string;
}

export interface McpServerToolsResult {
  server: McpServerRecord;
  tools: McpToolRecord[];
  discovered_at: string;
}

export function listMcpServers() {
  return invoke<McpServerRecord[]>("list_mcp_servers");
}

export function createMcpServer(server: McpServerDraft) {
  return invoke<McpServerRecord>("create_mcp_server", {
    server
  });
}

export function updateMcpServer(serverId: string, server: McpServerDraft) {
  return invoke<McpServerRecord>("update_mcp_server", {
    serverId,
    server
  });
}

export function deleteMcpServer(serverId: string) {
  return invoke("delete_mcp_server", {
    serverId
  });
}

export function testMcpServer(serverId: string) {
  return invoke<McpServerTestResult>("test_mcp_server", {
    serverId
  });
}

export function discoverMcpServerTools(serverId: string) {
  return invoke<McpServerToolsResult>("discover_mcp_server_tools", {
    serverId
  });
}
