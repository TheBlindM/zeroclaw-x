import { defineStore } from "pinia";
import {
  activateRuntimeProfile,
  createRuntimeProfile,
  deleteRuntimeProfile,
  exportRuntimeProfiles,
  getProxySettings,
  getProxySupport,
  getRuntimeProfiles,
  getRuntimeStatus,
  importRuntimeProfiles,
  saveProxySettings,
  saveRuntimeSettings,
  testRuntimeSettings,
  updateRuntimeProfile,
  type RuntimeConnectionReport,
  type RuntimeDelegateAgentRecord,
  type RuntimeDelegateSettingsRecord,
  type RuntimeProviderEntryRecord,
  type RuntimeProviderGroupRecord,
  type RuntimeProfileRecord,
  type RuntimeProfilesExportReport,
  type RuntimeProfilesImportReport,
  type RuntimeProfilesState,
  type RuntimeProxySettingsRecord,
  type RuntimeProxySupportRecord,
  type RuntimeSettingsRecord,
  type RuntimeStatusRecord
} from "@/api/tauri";

export const defaultRuntimeProviderEntry = (
  overrides: Partial<RuntimeProviderEntryRecord> = {}
): RuntimeProviderEntryRecord => ({
  id: "primary",
  name: "Primary",
  provider: "openrouter",
  model: "anthropic/claude-sonnet-4.6",
  provider_url: "",
  api_key: "",
  credential_mode: "api_key",
  auth_profile: "",
  temperature: 0.7,
  ...overrides
});

export const defaultRuntimeProviderGroup = (
  overrides: Partial<RuntimeProviderGroupRecord> = {}
): RuntimeProviderGroupRecord => {
  const entry = defaultRuntimeProviderEntry();
  return {
    id: "general",
    name: "General",
    active_entry_id: entry.id,
    entries: [entry],
    ...overrides
  };
};

export const defaultRuntimeDelegateSettings = (): RuntimeDelegateSettingsRecord => ({
  timeout_secs: 120,
  agentic_timeout_secs: 300
});

export const defaultRuntimeDelegateAgent = (
  overrides: Partial<RuntimeDelegateAgentRecord> = {}
): RuntimeDelegateAgentRecord => ({
  enabled: true,
  name: "",
  runtime_group_id: "",
  runtime_entry_id: "",
  provider: "openrouter",
  model: "anthropic/claude-sonnet-4.6",
  system_prompt: null,
  api_key: null,
  temperature: null,
  max_depth: 2,
  agentic: true,
  allowed_tools: [],
  max_iterations: 8,
  timeout_secs: null,
  agentic_timeout_secs: null,
  skills_directory: null,
  memory_namespace: null,
  ...overrides
});

export const defaultRuntimeSettings = (): RuntimeSettingsRecord => ({
  active_group_id: "general",
  groups: [defaultRuntimeProviderGroup()],
  active_entry_id: "primary",
  entries: [defaultRuntimeProviderEntry()],
  provider: "openrouter",
  model: "anthropic/claude-sonnet-4.6",
  provider_url: "",
  api_key: "",
  credential_mode: "api_key",
  auth_profile: "",
  temperature: 0.7,
  proxy: {
    enabled: false,
    scope: "zeroclaw",
    http_proxy: "",
    https_proxy: "",
    all_proxy: "",
    no_proxy: [],
    services: []
  },
  delegate: defaultRuntimeDelegateSettings(),
  agents: [],
  agent: {
    enabled: true,
    runtime_group_id: "",
    runtime_entry_id: "",
    workspace_dir: "",
    compact_context: false,
    max_tool_iterations: 10,
    max_history_messages: 50,
    max_context_tokens: 32000,
    parallel_tools: false,
    tool_dispatcher: "auto"
  },
  autonomy: {
    level: "supervised",
    workspace_only: true,
    require_approval_for_medium_risk: true,
    block_high_risk_commands: true,
    allowed_commands: ["git", "npm", "cargo", "ls", "cat", "grep", "find", "echo", "pwd", "wc", "head", "tail", "date"],
    allowed_roots: [],
    shell_env_passthrough: [],
    auto_approve: ["file_read", "memory_recall"],
    always_ask: []
  }
});

export const defaultProxySettings = (): RuntimeProxySettingsRecord => ({
  enabled: false,
  scope: "zeroclaw",
  http_proxy: "",
  https_proxy: "",
  all_proxy: "",
  no_proxy: [],
  services: []
});

export const defaultProxySupport = (): RuntimeProxySupportRecord => ({
  supported_service_keys: [],
  supported_selectors: []
});

export const defaultRuntimeStatus = (): RuntimeStatusRecord => ({
  profile_id: "default",
  profile_name: "Default",
  provider: "openrouter",
  model: "anthropic/claude-sonnet-4.6",
  provider_url: "",
  temperature: 0.7,
  api_key_configured: false,
  credential_mode: "api_key",
  auth_profile: "",
  workspace_dir: "",
  tool_dispatcher: "auto",
  autonomy_level: "supervised",
  workspace_only: true,
  parallel_tools: false
});

function applyProfilesState(target: {
  runtime: RuntimeSettingsRecord;
  profiles: RuntimeProfileRecord[];
  activeProfileId: string;
}, state: RuntimeProfilesState) {
  target.profiles = state.profiles;
  target.activeProfileId = state.active_profile_id;
  target.runtime =
    state.profiles.find((profile) => profile.id === state.active_profile_id)?.settings ?? defaultRuntimeSettings();
}

export const useSettingsStore = defineStore("settings", {
  state: () => ({
    runtime: defaultRuntimeSettings() as RuntimeSettingsRecord,
    proxySettings: defaultProxySettings() as RuntimeProxySettingsRecord,
    proxySupport: defaultProxySupport() as RuntimeProxySupportRecord,
    profiles: [] as RuntimeProfileRecord[],
    activeProfileId: "" as string,
    status: defaultRuntimeStatus() as RuntimeStatusRecord,
    testReport: null as RuntimeConnectionReport | null,
    isLoading: false,
    isSaving: false,
    isTesting: false,
    isRefreshingStatus: false,
    isImporting: false,
    isExporting: false,
    lastSavedAt: "" as string,
    error: "" as string,
    loaded: false,
    statusLoaded: false,
    proxyLoaded: false
  }),
  getters: {
    activeProfile(state) {
      return state.profiles.find((profile) => profile.id === state.activeProfileId) ?? null;
    }
  },
  actions: {
    async bootstrap() {
      if ((this.loaded && this.proxyLoaded) || this.isLoading) {
        return this.runtime;
      }

      this.isLoading = true;
      this.error = "";

      try {
        const profilesState = await getRuntimeProfiles();
        applyProfilesState(this, profilesState);
        this.proxySettings = await getProxySettings();
        this.proxySupport = await getProxySupport();
        this.proxyLoaded = true;
        this.loaded = true;
        await this.refreshStatus();
        return this.runtime;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async refreshStatus() {
      if (this.isRefreshingStatus) {
        return this.status;
      }

      this.isRefreshingStatus = true;

      try {
        this.status = await getRuntimeStatus();
        this.statusLoaded = true;
        return this.status;
      } finally {
        this.isRefreshingStatus = false;
      }
    },
    async save(settings: RuntimeSettingsRecord) {
      this.isSaving = true;
      this.error = "";

      try {
        const profilesState = await saveRuntimeSettings(settings);
        applyProfilesState(this, profilesState);
        this.loaded = true;
        this.lastSavedAt = new Date().toISOString();
        await this.refreshStatus();
        return this.runtime;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async saveProxy(settings: RuntimeProxySettingsRecord) {
      this.isSaving = true;
      this.error = "";

      try {
        this.proxySettings = await saveProxySettings(settings);
        this.proxyLoaded = true;
        this.lastSavedAt = new Date().toISOString();
        void this.refreshStatus().catch(() => undefined);
        return this.proxySettings;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async createProfile(name: string, settings: RuntimeSettingsRecord) {
      this.isSaving = true;
      this.error = "";

      try {
        const profilesState = await createRuntimeProfile(name, settings);
        applyProfilesState(this, profilesState);
        this.loaded = true;
        this.lastSavedAt = new Date().toISOString();
        await this.refreshStatus();
        return this.activeProfile;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async updateProfile(profileId: string, name: string, settings: RuntimeSettingsRecord) {
      this.isSaving = true;
      this.error = "";

      try {
        const profilesState = await updateRuntimeProfile(profileId, name, settings);
        applyProfilesState(this, profilesState);
        this.loaded = true;
        this.lastSavedAt = new Date().toISOString();
        await this.refreshStatus();
        return this.activeProfile;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async activateProfile(profileId: string) {
      this.isLoading = true;
      this.error = "";

      try {
        const profilesState = await activateRuntimeProfile(profileId);
        applyProfilesState(this, profilesState);
        this.loaded = true;
        await this.refreshStatus();
        return this.runtime;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isLoading = false;
      }
    },
    async deleteProfile(profileId: string) {
      this.isSaving = true;
      this.error = "";

      try {
        const profilesState = await deleteRuntimeProfile(profileId);
        applyProfilesState(this, profilesState);
        this.loaded = true;
        this.lastSavedAt = new Date().toISOString();
        await this.refreshStatus();
        return this.activeProfile;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isSaving = false;
      }
    },
    async importProfiles() {
      this.isImporting = true;
      this.error = "";

      try {
        const report = await importRuntimeProfiles();
        if (report) {
          applyProfilesState(this, report.profiles);
          this.loaded = true;
          this.lastSavedAt = new Date().toISOString();
          await this.refreshStatus();
        }
        return report as RuntimeProfilesImportReport | null;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isImporting = false;
      }
    },
    async exportProfiles() {
      this.isExporting = true;
      this.error = "";

      try {
        return (await exportRuntimeProfiles()) as RuntimeProfilesExportReport | null;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isExporting = false;
      }
    },
    async test(settings: RuntimeSettingsRecord) {
      this.isTesting = true;
      this.error = "";
      this.testReport = null;

      try {
        this.testReport = await testRuntimeSettings(settings);
        return this.testReport;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.isTesting = false;
      }
    }
  }
});
