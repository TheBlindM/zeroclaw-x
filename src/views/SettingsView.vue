<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  getAuthLoginStatus,
  listAuthProfiles,
  openExternalUrl,
  pickRuntimeWorkspace,
  startAuthLogin,
  type AuthLoginChallengeRecord,
  type AuthLoginStatusRecord,
  type AuthProfileRecord,
  type RuntimeCredentialModeRecord,
  type RuntimeSettingsRecord,
  type UpdateSettingsRecord
} from "@/api/tauri";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { useAppStore } from "@/stores/app";
import { defaultRuntimeSettings, useSettingsStore } from "@/stores/settings";
import { defaultUpdateSettings, useUpdateStore } from "@/stores/update";

interface RuntimePreset {
  id: string;
  labelKey: string;
  provider: string;
  model: string;
  providerUrl: string;
  credentialMode: RuntimeCredentialModeRecord;
  temperature: number;
  noteKey: string;
}

interface ProviderSuggestion {
  id: string;
  labelKey: string;
  provider: string;
  providerUrl: string;
  credentialMode: RuntimeCredentialModeRecord;
  defaultModel: string;
  noteKey: string;
}

type SettingsTabKey = "general" | "runtime" | "agent" | "updates";

interface SettingsTabDefinition {
  id: SettingsTabKey;
  labelKey: string;
  descriptionKey: string;
}

const runtimePresets: RuntimePreset[] = [
  {
    id: "openrouter",
    labelKey: "settings.presetLabels.openrouter",
    provider: "openrouter",
    model: "anthropic/claude-sonnet-4.6",
    providerUrl: "",
    credentialMode: "api_key",
    temperature: 0.7,
    noteKey: "settings.presetNotes.openrouter"
  },
  {
    id: "openai",
    labelKey: "settings.presetLabels.openai",
    provider: "openai",
    model: "gpt-4o-mini",
    providerUrl: "",
    credentialMode: "api_key",
    temperature: 0.7,
    noteKey: "settings.presetNotes.openai"
  },
  {
    id: "openai-codex",
    labelKey: "settings.presetLabels.openaiCodex",
    provider: "openai-codex",
    model: "gpt-5.4",
    providerUrl: "",
    credentialMode: "auth_profile",
    temperature: 0.7,
    noteKey: "settings.presetNotes.openaiCodex"
  },
  {
    id: "gemini",
    labelKey: "settings.presetLabels.gemini",
    provider: "gemini",
    model: "gemini-2.5-pro",
    providerUrl: "",
    credentialMode: "auth_profile",
    temperature: 0.7,
    noteKey: "settings.presetNotes.gemini"
  },
  {
    id: "anthropic",
    labelKey: "settings.presetLabels.anthropic",
    provider: "anthropic",
    model: "claude-sonnet-4-5",
    providerUrl: "",
    credentialMode: "api_key",
    temperature: 0.7,
    noteKey: "settings.presetNotes.anthropic"
  },
  {
    id: "ollama",
    labelKey: "settings.presetLabels.ollama",
    provider: "ollama",
    model: "qwen2.5-coder:7b",
    providerUrl: "http://127.0.0.1:11434",
    credentialMode: "api_key",
    temperature: 0.3,
    noteKey: "settings.presetNotes.ollama"
  },
  {
    id: "custom-endpoint",
    labelKey: "settings.presetLabels.custom",
    provider: "custom:https://your-endpoint.example/v1",
    model: "gpt-4o-mini",
    providerUrl: "https://your-endpoint.example/v1",
    credentialMode: "api_key",
    temperature: 0.7,
    noteKey: "settings.presetNotes.custom"
  }
];

const providerSuggestions: ProviderSuggestion[] = runtimePresets.map((preset) => ({
  id: preset.id,
  labelKey: preset.labelKey,
  provider: preset.provider,
  providerUrl: preset.providerUrl,
  credentialMode: preset.credentialMode,
  defaultModel: preset.model,
  noteKey: preset.noteKey
}));

const settingsTabs: SettingsTabDefinition[] = [
  {
    id: "general",
    labelKey: "settings.tabs.general.label",
    descriptionKey: "settings.tabs.general.description"
  },
  {
    id: "runtime",
    labelKey: "settings.tabs.runtime.label",
    descriptionKey: "settings.tabs.runtime.description"
  },
  {
    id: "agent",
    labelKey: "settings.tabs.agent.label",
    descriptionKey: "settings.tabs.agent.description"
  },
  {
    id: "updates",
    labelKey: "settings.tabs.updates.label",
    descriptionKey: "settings.tabs.updates.description"
  }
];

const modelSuggestionsByProvider: Record<string, string[]> = {
  openrouter: [
    "anthropic/claude-sonnet-4.6",
    "anthropic/claude-3.7-sonnet",
    "openai/gpt-4o-mini",
    "google/gemini-2.5-pro",
    "meta-llama/llama-3.3-70b-instruct"
  ],
  "openai-codex": [
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.3-codex",
    "gpt-5.2-codex",
    "gpt-5.2",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini"
  ],
  gemini: ["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"],
  openai: ["gpt-4o", "gpt-4o-mini", "gpt-4.1", "gpt-4.1-mini"],
  anthropic: ["claude-sonnet-4-5", "claude-3-7-sonnet-latest", "claude-3-5-haiku-latest"],
  ollama: ["qwen2.5-coder:7b", "qwen2.5:14b", "llama3.1:8b", "deepseek-r1:8b"],
  "custom-endpoint": ["gpt-4o-mini", "gpt-4.1-mini", "claude-sonnet-4-5", "qwen2.5-coder:7b"]
};

const appStore = useAppStore();
const settingsStore = useSettingsStore();
const updateStore = useUpdateStore();
const { t } = useI18n();
const { activeProfile, profiles } = storeToRefs(settingsStore);
const activeSettingsTab = ref<SettingsTabKey>("general");
const showApiKey = ref(false);
const showProxySettings = ref(false);
const showAutonomyAdvanced = ref(false);
const saveMessage = ref("");
const testMessage = ref("");
const updateMessage = ref("");
const authProfiles = ref<AuthProfileRecord[]>([]);
const authProfilesLoading = ref(false);
const authLoginChallenge = ref<AuthLoginChallengeRecord | null>(null);
const authLoginStatus = ref<AuthLoginStatusRecord | null>(null);
const authPollingHandle = ref<number | null>(null);

const form = reactive(defaultRuntimeSettings());
const updateForm = reactive(defaultUpdateSettings());

const currentProviderKey = computed(() => resolveProviderKey(form.provider));
const currentProviderSupportsAuth = computed(() => currentProviderKey.value === "openai-codex" || currentProviderKey.value === "gemini");
const currentProviderRequiresAuthProfile = computed(() => currentProviderKey.value === "openai-codex");
const currentProviderSupportsApiKey = computed(() => currentProviderKey.value !== "openai-codex");
const effectiveCredentialMode = computed<RuntimeCredentialModeRecord>(() => {
  if (currentProviderRequiresAuthProfile.value) {
    return "auth_profile";
  }

  if (!currentProviderSupportsAuth.value) {
    return "api_key";
  }

  return form.credential_mode;
});
const showAuthSettings = computed(() => currentProviderSupportsAuth.value && effectiveCredentialMode.value === "auth_profile");
const selectedTheme = computed({
  get: () => appStore.theme,
  set: (value) => {
    if (value === "dark" || value === "light") {
      appStore.applyTheme(value);
    }
  }
});
const selectedLocale = computed({
  get: () => appStore.locale,
  set: (value) => {
    if (value === "zh" || value === "en") {
      appStore.setLocale(value);
    }
  }
});
const proxyNoProxyText = computed({
  get: () => form.proxy.no_proxy.join(", "),
  set: (value: string) => {
    form.proxy.no_proxy = splitCommaSeparated(value);
  }
});
const proxyServicesText = computed({
  get: () => form.proxy.services.join(", "),
  set: (value: string) => {
    form.proxy.services = splitCommaSeparated(value);
  }
});
const autonomyAllowedCommandsText = computed({
  get: () => form.autonomy.allowed_commands.join(", "),
  set: (value: string) => {
    form.autonomy.allowed_commands = splitDelimitedList(value);
  }
});
const autonomyAllowedRootsText = computed({
  get: () => form.autonomy.allowed_roots.join("\n"),
  set: (value: string) => {
    form.autonomy.allowed_roots = splitDelimitedList(value);
  }
});
const autonomyEnvText = computed({
  get: () => form.autonomy.shell_env_passthrough.join(", "),
  set: (value: string) => {
    form.autonomy.shell_env_passthrough = splitDelimitedList(value);
  }
});
const autonomyAutoApproveText = computed({
  get: () => form.autonomy.auto_approve.join(", "),
  set: (value: string) => {
    form.autonomy.auto_approve = splitDelimitedList(value);
  }
});
const autonomyAlwaysAskText = computed({
  get: () => form.autonomy.always_ask.join(", "),
  set: (value: string) => {
    form.autonomy.always_ask = splitDelimitedList(value);
  }
});
const updateEndpointsText = computed({
  get: () => updateForm.endpoints.join("\n"),
  set: (value: string) => {
    updateForm.endpoints = splitDelimitedList(value);
  }
});
const visibleProviderSuggestions = computed(() => {
  const query = form.provider.trim().toLowerCase();
  if (!query) {
    return providerSuggestions.slice(0, 5);
  }

  return providerSuggestions
    .filter((suggestion) => {
      const haystack = `${resolvePresetLabel(suggestion.labelKey)} ${suggestion.provider} ${t(suggestion.noteKey)}`.toLowerCase();
      return haystack.includes(query);
    })
    .slice(0, 5);
});
const visibleModelSuggestions = computed(() => {
  const providerKey = currentProviderKey.value;
  const query = form.model.trim().toLowerCase();
  const suggestions = modelSuggestionsByProvider[providerKey] ?? [];

  if (!query) {
    return suggestions.slice(0, 6);
  }

  return suggestions.filter((model) => model.toLowerCase().includes(query)).slice(0, 6);
});
const providerContextHint = computed(() => {
  switch (currentProviderKey.value) {
    case "ollama":
      return t("settings.contextHints.ollama");
    case "custom-endpoint":
      return t("settings.contextHints.custom");
    case "openrouter":
      return t("settings.contextHints.openrouter");
    default:
      return t("settings.contextHints.default");
  }
});
const proxyScopeHint = computed(() => {
  switch (form.proxy.scope) {
    case "environment":
      return t("settings.proxyScopeHints.environment");
    case "services":
      return t("settings.proxyScopeHints.services");
    default:
      return t("settings.proxyScopeHints.zeroclaw");
  }
});
const proxySummary = computed(() => {
  if (!form.proxy.enabled) {
    return t("settings.proxyStatusDisabled");
  }

  const configuredTargets = [form.proxy.http_proxy, form.proxy.https_proxy, form.proxy.all_proxy].filter(
    (value) => value.trim().length > 0
  ).length;
  const scopeLabel = t(`settings.proxyScopes.${form.proxy.scope}`);

  if (form.proxy.scope === "services") {
    return t("settings.proxyStatusServices", {
      scope: scopeLabel,
      targets: configuredTargets,
      count: form.proxy.services.length
    });
  }

  return t("settings.proxyStatusEnabled", {
    scope: scopeLabel,
    targets: configuredTargets
  });
});
const workspaceSummary = computed(() => form.agent.workspace_dir.trim() || settingsStore.status.workspace_dir || t("settings.workspaceDirectoryDefault"));
const toolDispatcherHint = computed(() => {
  switch (form.agent.tool_dispatcher) {
    case "native":
      return t("settings.toolDispatcherHints.native");
    case "xml":
      return t("settings.toolDispatcherHints.xml");
    default:
      return t("settings.toolDispatcherHints.auto");
  }
});
const authLoginHint = computed(() => {
  switch (currentProviderKey.value) {
    case "gemini":
      return t("settings.authLoginHints.gemini");
    default:
      return t("settings.authLoginHints.openaiCodex");
  }
});
const autonomyLevelHint = computed(() => {
  switch (form.autonomy.level) {
    case "read_only":
      return t("settings.autonomyLevelHints.read_only");
    case "full":
      return t("settings.autonomyLevelHints.full");
    default:
      return t("settings.autonomyLevelHints.supervised");
  }
});
const agentSummary = computed(() =>
  t("settings.agentWorkspaceSummary", {
    workspace: workspaceSummary.value,
    autonomy: t(`settings.autonomyLevels.${form.autonomy.level}`),
    dispatcher: t(`settings.toolDispatchers.${form.agent.tool_dispatcher}`)
  })
);
const canRunUpdater = computed(
  () => updateForm.enabled && updateForm.endpoints.length > 0 && updateForm.pubkey.trim().length > 0
);
const activeSettingsTabMeta = computed(
  () => settingsTabs.find((tab) => tab.id === activeSettingsTab.value) ?? settingsTabs[0]
);

watch(
  () => settingsStore.runtime,
  (runtime) => {
    applyForm(runtime);
  },
  { deep: true, immediate: true }
);

watch(
  () => updateStore.settings,
  (settings) => {
    applyUpdateForm(settings);
  },
  { deep: true, immediate: true }
);

watch(
  () => form.proxy.enabled,
  (enabled) => {
    if (enabled) {
      showProxySettings.value = true;
    }
  },
  { immediate: true }
);

watch(
  () => [
    form.autonomy.level,
    form.autonomy.allowed_roots.length,
    form.autonomy.shell_env_passthrough.length,
    form.autonomy.always_ask.length
  ] as const,
  ([level, allowedRootsCount, envCount, alwaysAskCount]) => {
    if (level === "full" || allowedRootsCount > 0 || envCount > 0 || alwaysAskCount > 0) {
      showAutonomyAdvanced.value = true;
    }
  },
  { immediate: true }
);

watch(
  [currentProviderKey, effectiveCredentialMode],
  async ([providerKey, credentialMode]) => {
    if (providerKey === "openai-codex") {
      form.credential_mode = "auth_profile";
    } else if (providerKey !== "gemini") {
      form.credential_mode = "api_key";
    }

    if (providerKey !== "openai-codex" && providerKey !== "gemini") {
      stopAuthLoginPolling();
      authProfiles.value = [];
      authLoginChallenge.value = null;
      authLoginStatus.value = null;
      return;
    }

    if (credentialMode === "auth_profile") {
      await refreshAuthProfiles();
      return;
    }

    stopAuthLoginPolling();
    authProfiles.value = [];
    authLoginChallenge.value = null;
    authLoginStatus.value = null;
  },
  { immediate: true }
);

onMounted(async () => {
  if (!settingsStore.loaded) {
    try {
      await settingsStore.bootstrap();
    } catch {
      // keep the default form values and show the store error below
    }
  }

  if (!updateStore.loaded) {
    try {
      await updateStore.bootstrap();
    } catch {
      // keep the default update settings and show the store error below
    }
  }
});

onBeforeUnmount(() => {
  stopAuthLoginPolling();
});

function resolveProfileName(name: string | null | undefined) {
  if (!name) {
    return t("settings.profileNames.current");
  }

  if (name === "Default" || name === "默认") {
    return t("settings.profileNames.default");
  }

  return name;
}

function resolvePresetLabel(labelKey: string) {
  return t(labelKey);
}

async function handleSave() {
  saveMessage.value = "";

  try {
    await settingsStore.save(buildRuntimeSettingsPayload());
    saveMessage.value = t("settings.feedback.savedToProfile", {
      name: resolveProfileName(activeProfile.value?.name)
    });
  } catch {
    saveMessage.value = t("settings.feedback.saveFailed");
  }
}

async function handleTest() {
  testMessage.value = "";

  try {
    const report = await settingsStore.test(buildRuntimeSettingsPayload());
    testMessage.value = report.message;
  } catch {
    testMessage.value = t("settings.feedback.testFailed");
  }
}

async function handlePickWorkspace() {
  saveMessage.value = "";

  try {
    const selected = await pickRuntimeWorkspace();
    if (!selected) {
      saveMessage.value = t("settings.feedback.workspacePickCancelled");
      return;
    }

    form.agent.workspace_dir = selected;
    saveMessage.value = t("settings.feedback.workspacePicked", { path: selected });
  } catch {
    saveMessage.value = t("settings.feedback.workspacePickFailed");
  }
}

async function refreshAuthProfiles() {
  if (!showAuthSettings.value) {
    authProfiles.value = [];
    return;
  }

  authProfilesLoading.value = true;

  try {
    const state = await listAuthProfiles(form.provider);
    authProfiles.value = state.profiles;
  } catch {
    authProfiles.value = [];
    saveMessage.value = t("settings.feedback.authProfilesLoadFailed");
  } finally {
    authProfilesLoading.value = false;
  }
}

function stopAuthLoginPolling() {
  if (authPollingHandle.value !== null) {
    window.clearInterval(authPollingHandle.value);
    authPollingHandle.value = null;
  }
}

function describeUnknownError(error: unknown) {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === "object") {
    if ("message" in error && typeof error.message === "string") {
      return error.message;
    }

    if ("cause" in error && typeof error.cause === "string") {
      return error.cause;
    }

    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }

  return String(error);
}

function startAuthLoginPolling(loginId: string, intervalSeconds: number) {
  stopAuthLoginPolling();
  authPollingHandle.value = window.setInterval(() => {
    void pollAuthLoginStatus(loginId);
  }, Math.max(intervalSeconds, 2) * 1000);
}

async function pollAuthLoginStatus(loginId?: string) {
  const target = loginId ?? authLoginChallenge.value?.login_id;
  if (!target) {
    return;
  }

  try {
    const status = await getAuthLoginStatus(target);
    authLoginStatus.value = status;

    if (status.status === "pending") {
      return;
    }

    stopAuthLoginPolling();
    await refreshAuthProfiles();

    if (status.status === "succeeded") {
      form.auth_profile = status.profile_name;
      saveMessage.value = t("settings.feedback.authLoginSucceeded", { name: status.profile_name });
      return;
    }

    saveMessage.value = t("settings.feedback.authLoginFailed", { message: status.message });
  } catch {
    stopAuthLoginPolling();
    saveMessage.value = t("settings.feedback.authLoginStatusFailed");
  }
}

async function handleStartAuthLogin() {
  if (!showAuthSettings.value) {
    return;
  }

  saveMessage.value = "";
  authLoginStatus.value = null;

  try {
    const challenge = await startAuthLogin(form.provider, form.auth_profile.trim() || "default");
    authLoginChallenge.value = challenge;
    form.auth_profile = challenge.profile_name;
    const loginUrl = challenge.verification_uri_complete || challenge.verification_uri;

    if (loginUrl) {
      try {
        await openExternalUrl(loginUrl);
      } catch (error) {
        saveMessage.value = t("settings.feedback.authLoginOpenFailed", {
          message: describeUnknownError(error)
        });
      }
    }

    if (!saveMessage.value) {
      saveMessage.value = t("settings.feedback.authLoginStarted", {
        provider: challenge.provider,
        code: challenge.user_code
      });
    }
    startAuthLoginPolling(challenge.login_id, challenge.interval_seconds);
    void pollAuthLoginStatus(challenge.login_id);
  } catch (error) {
    saveMessage.value = t("settings.feedback.authLoginStartFailed", {
      message: describeUnknownError(error)
    });
  }
}

function handleSelectAuthProfile(profileName: string) {
  form.auth_profile = profileName;
  saveMessage.value = t("settings.feedback.authProfileSelected", { name: profileName });
}

function applyPreset(preset: RuntimePreset) {
  Object.assign(form, {
    provider: preset.provider,
    model: preset.model,
    provider_url: preset.providerUrl,
    credential_mode: preset.credentialMode,
    temperature: preset.temperature
  });
  saveMessage.value = t("settings.feedback.presetLoaded", { name: resolvePresetLabel(preset.labelKey) });
  testMessage.value = "";
  settingsStore.testReport = null;
}

function applyProviderSuggestion(suggestion: ProviderSuggestion) {
  form.provider = suggestion.provider;
  form.provider_url = suggestion.providerUrl;
  form.credential_mode = suggestion.credentialMode;
  form.model = suggestion.defaultModel;
  saveMessage.value = t("settings.feedback.providerSuggestionLoaded", { name: resolvePresetLabel(suggestion.labelKey) });
  testMessage.value = "";
  settingsStore.testReport = null;
}

function applyModelSuggestion(model: string) {
  form.model = model;
  saveMessage.value = t("settings.feedback.modelSuggestionLoaded", { name: model });
  testMessage.value = "";
  settingsStore.testReport = null;
}

async function handleActivateProfile(profileId: string) {
  if (profileId === settingsStore.activeProfileId) {
    return;
  }

  saveMessage.value = "";
  testMessage.value = "";
  settingsStore.testReport = null;

  try {
    await settingsStore.activateProfile(profileId);
    saveMessage.value = t("settings.feedback.switchedProfile", {
      name: resolveProfileName(activeProfile.value?.name)
    });
  } catch {
    saveMessage.value = t("settings.feedback.switchProfileFailed");
  }
}

async function handleCreateProfile() {
  const defaultName = t("settings.profileNames.generated", { count: profiles.value.length + 1 });
  const name = window.prompt(t("settings.prompts.newProfileName"), defaultName)?.trim();
  if (!name) {
    return;
  }

  try {
    await settingsStore.createProfile(name, buildRuntimeSettingsPayload());
    saveMessage.value = t("settings.feedback.createdProfile", {
      name: resolveProfileName(activeProfile.value?.name ?? name)
    });
  } catch {
    saveMessage.value = t("settings.feedback.createProfileFailed");
  }
}

async function handleRenameProfile() {
  if (!activeProfile.value) {
    return;
  }

  const nextName = window.prompt(t("settings.prompts.renameProfile"), activeProfile.value.name)?.trim();
  if (!nextName) {
    return;
  }

  try {
    await settingsStore.updateProfile(activeProfile.value.id, nextName, buildRuntimeSettingsPayload());
    saveMessage.value = t("settings.feedback.renamedProfile", { name: nextName });
  } catch {
    saveMessage.value = t("settings.feedback.renameProfileFailed");
  }
}

async function handleDeleteProfile() {
  if (!activeProfile.value) {
    return;
  }

  const confirmed = window.confirm(t("settings.prompts.deleteProfile", { name: activeProfile.value.name }));
  if (!confirmed) {
    return;
  }

  try {
    const deletedName = activeProfile.value.name;
    await settingsStore.deleteProfile(activeProfile.value.id);
    saveMessage.value = t("settings.feedback.deletedProfile", { name: deletedName });
    testMessage.value = "";
    settingsStore.testReport = null;
  } catch {
    saveMessage.value = t("settings.feedback.deleteProfileFailed");
  }
}

async function handleImportProfiles() {
  saveMessage.value = "";
  testMessage.value = "";
  settingsStore.testReport = null;

  try {
    const report = await settingsStore.importProfiles();
    if (!report) {
      saveMessage.value = t("settings.feedback.importCancelled");
      return;
    }

    saveMessage.value = t("settings.feedback.importedProfiles", {
      count: report.imported_count,
      path: report.path
    });
  } catch {
    saveMessage.value = t("settings.feedback.importFailed");
  }
}

async function handleExportProfiles() {
  saveMessage.value = "";

  try {
    const report = await settingsStore.exportProfiles();
    if (!report) {
      saveMessage.value = t("settings.feedback.exportCancelled");
      return;
    }

    saveMessage.value = t("settings.feedback.exportedProfiles", {
      count: report.profile_count,
      path: report.path
    });
  } catch {
    saveMessage.value = t("settings.feedback.exportFailed");
  }
}

function resetForm() {
  applyForm(settingsStore.runtime);
  stopAuthLoginPolling();
  authLoginChallenge.value = null;
  authLoginStatus.value = null;
  saveMessage.value = t("settings.feedback.resetUnsaved");
  testMessage.value = "";
  settingsStore.testReport = null;
}

async function handleSaveUpdateSettings() {
  updateMessage.value = "";

  try {
    await updateStore.saveSettings(buildUpdateSettingsPayload());
    updateMessage.value = t("settings.feedback.updateSettingsSaved");
  } catch {
    updateMessage.value = t("settings.feedback.updateSettingsSaveFailed");
  }
}

async function handleCheckForUpdates() {
  updateMessage.value = "";

  try {
    await updateStore.saveSettings(buildUpdateSettingsPayload());
    const report = await updateStore.checkForUpdates();
    updateMessage.value = report.message;
  } catch {
    updateMessage.value = t("settings.feedback.updateCheckFailed");
  }
}

async function handleInstallUpdate() {
  updateMessage.value = "";

  try {
    await updateStore.saveSettings(buildUpdateSettingsPayload());
    const report = await updateStore.installLatestUpdate();
    updateMessage.value = report.message;
  } catch {
    updateMessage.value = t("settings.feedback.updateInstallFailed");
  }
}

function resetUpdateForm() {
  applyUpdateForm(updateStore.settings);
  updateMessage.value = t("settings.feedback.updateReset");
}

function resolveProviderKey(value: string) {
  const normalized = value.trim().toLowerCase();
  if (normalized.startsWith("custom:")) {
    return "custom-endpoint";
  }

  if (normalized.includes("openai-codex") || normalized.includes("openai_codex") || normalized === "codex") {
    return "openai-codex";
  }

  if (normalized.includes("gemini") || normalized.includes("google")) {
    return "gemini";
  }

  if (normalized.includes("openrouter")) {
    return "openrouter";
  }

  if (normalized.includes("openai")) {
    return "openai";
  }

  if (normalized.includes("anthropic")) {
    return "anthropic";
  }

  if (normalized.includes("ollama")) {
    return "ollama";
  }

  return "custom-endpoint";
}

function applyForm(runtime: RuntimeSettingsRecord) {
  const normalized = cloneRuntimeSettings(runtime);
  Object.assign(form, normalized);
}

function applyUpdateForm(settings: UpdateSettingsRecord) {
  const normalized = cloneUpdateSettings(settings);
  Object.assign(updateForm, normalized);
}

function cloneRuntimeSettings(runtime: RuntimeSettingsRecord): RuntimeSettingsRecord {
  const defaults = defaultRuntimeSettings();

  return {
    provider: runtime.provider,
    model: runtime.model,
    provider_url: runtime.provider_url,
    api_key: runtime.api_key,
    credential_mode: runtime.credential_mode ?? defaults.credential_mode,
    auth_profile: runtime.auth_profile ?? defaults.auth_profile,
    temperature: runtime.temperature,
    proxy: {
      ...defaults.proxy,
      ...(runtime.proxy ?? defaults.proxy),
      no_proxy: [...(runtime.proxy?.no_proxy ?? [])],
      services: [...(runtime.proxy?.services ?? [])]
    },
    agent: {
      ...defaults.agent,
      ...(runtime.agent ?? defaults.agent)
    },
    autonomy: {
      ...defaults.autonomy,
      ...(runtime.autonomy ?? defaults.autonomy),
      allowed_commands: [...(runtime.autonomy?.allowed_commands ?? defaults.autonomy.allowed_commands)],
      allowed_roots: [...(runtime.autonomy?.allowed_roots ?? defaults.autonomy.allowed_roots)],
      shell_env_passthrough: [...(runtime.autonomy?.shell_env_passthrough ?? defaults.autonomy.shell_env_passthrough)],
      auto_approve: [...(runtime.autonomy?.auto_approve ?? defaults.autonomy.auto_approve)],
      always_ask: [...(runtime.autonomy?.always_ask ?? defaults.autonomy.always_ask)]
    }
  };
}

function cloneUpdateSettings(settings: UpdateSettingsRecord): UpdateSettingsRecord {
  const defaults = defaultUpdateSettings();

  return {
    enabled: settings.enabled,
    auto_check: settings.auto_check,
    endpoints: [...(settings.endpoints ?? defaults.endpoints)],
    pubkey: settings.pubkey ?? defaults.pubkey
  };
}

function buildRuntimeSettingsPayload(): RuntimeSettingsRecord {
  return cloneRuntimeSettings({
    provider: form.provider,
    model: form.model,
    provider_url: form.provider_url,
    api_key: form.api_key,
    credential_mode: effectiveCredentialMode.value,
    auth_profile: form.auth_profile,
    temperature: Number(form.temperature),
    proxy: {
      enabled: form.proxy.enabled,
      scope: form.proxy.scope,
      http_proxy: form.proxy.http_proxy,
      https_proxy: form.proxy.https_proxy,
      all_proxy: form.proxy.all_proxy,
      no_proxy: [...form.proxy.no_proxy],
      services: [...form.proxy.services]
    },
    agent: {
      workspace_dir: form.agent.workspace_dir,
      compact_context: form.agent.compact_context,
      max_tool_iterations: Number(form.agent.max_tool_iterations),
      max_history_messages: Number(form.agent.max_history_messages),
      max_context_tokens: Number(form.agent.max_context_tokens),
      parallel_tools: form.agent.parallel_tools,
      tool_dispatcher: form.agent.tool_dispatcher
    },
    autonomy: {
      level: form.autonomy.level,
      workspace_only: form.autonomy.workspace_only,
      require_approval_for_medium_risk: form.autonomy.require_approval_for_medium_risk,
      block_high_risk_commands: form.autonomy.block_high_risk_commands,
      allowed_commands: [...form.autonomy.allowed_commands],
      allowed_roots: [...form.autonomy.allowed_roots],
      shell_env_passthrough: [...form.autonomy.shell_env_passthrough],
      auto_approve: [...form.autonomy.auto_approve],
      always_ask: [...form.autonomy.always_ask]
    }
  });
}

function buildUpdateSettingsPayload(): UpdateSettingsRecord {
  return cloneUpdateSettings({
    enabled: updateForm.enabled,
    auto_check: updateForm.auto_check,
    endpoints: [...updateForm.endpoints],
    pubkey: updateForm.pubkey
  });
}

function splitCommaSeparated(value: string) {
  return value
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function splitDelimitedList(value: string) {
  return value
    .split(/\r?\n|,/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}
</script>

<template>
  <div class="stack settings-page">
    <section class="panel settings-panel">
      <div class="stack" style="gap: 14px;">
        <div class="stack" style="gap: 6px; max-width: 760px;">
          <strong>{{ t(activeSettingsTabMeta.labelKey) }}</strong>
          <span class="muted">{{ t(activeSettingsTabMeta.descriptionKey) }}</span>
        </div>
        <div class="settings-tabs" role="tablist">
          <button
            v-for="tab in settingsTabs"
            :key="tab.id"
            class="settings-tab"
            type="button"
            role="tab"
            :aria-selected="activeSettingsTab === tab.id"
            :data-active="activeSettingsTab === tab.id"
            @click="activeSettingsTab = tab.id"
          >
            {{ t(tab.labelKey) }}
          </button>
        </div>
      </div>
    </section>

    <template v-if="activeSettingsTab === 'general'">
      <section class="panel settings-panel">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
          <div class="stack" style="gap: 6px; max-width: 720px;">
            <strong>{{ t("settings.runtimeTitle") }}</strong>
            <span class="muted">{{ t("settings.runtimeDescription") }}</span>
          </div>
          <div class="settings-preferences-inline">
            <label class="settings-field settings-field--compact">
              <span class="settings-field__label">{{ t("settings.theme") }}</span>
              <select v-model="selectedTheme" class="select">
                <option value="dark">{{ t("settings.darkTheme") }}</option>
                <option value="light">{{ t("settings.lightTheme") }}</option>
              </select>
            </label>
            <label class="settings-field settings-field--compact">
              <span class="settings-field__label">{{ t("settings.language") }}</span>
              <select v-model="selectedLocale" class="select">
                <option value="zh">{{ t("settings.languageChinese") }}</option>
                <option value="en">{{ t("settings.languageEnglish") }}</option>
              </select>
            </label>
          </div>
        </div>
      </section>

      <section class="panel settings-panel">
        <div class="stack" style="gap: 8px;">
          <strong>{{ t("settings.howItApplies") }}</strong>
          <span class="muted">{{ t("settings.howItAppliesDescription") }}</span>
          <span class="muted">{{ t("settings.howItAppliesDescription2") }}</span>
        </div>
      </section>
    </template>

    <template v-else-if="activeSettingsTab === 'runtime'">
      <section class="panel settings-panel">
        <div class="stack" style="gap: 12px;">
          <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap;">
            <div class="stack" style="gap: 6px; max-width: 720px;">
              <strong>{{ t("settings.profilesTitle") }}</strong>
              <span class="muted">{{ t("settings.profilesDescription") }}</span>
            </div>
            <div class="row settings-action-row">
              <Button variant="secondary" :disabled="settingsStore.isImporting || settingsStore.isSaving || settingsStore.isLoading" @click="handleImportProfiles">
                {{ settingsStore.isImporting ? t("settings.importing") : t("settings.importJson") }}
              </Button>
              <Button variant="secondary" :disabled="settingsStore.isExporting || settingsStore.isSaving || settingsStore.isLoading" @click="handleExportProfiles">
                {{ settingsStore.isExporting ? t("settings.exporting") : t("settings.exportJson") }}
              </Button>
              <Button variant="secondary" :disabled="settingsStore.isSaving || settingsStore.isLoading" @click="handleCreateProfile">{{ t("settings.newProfile") }}</Button>
              <Button variant="secondary" :disabled="!activeProfile || settingsStore.isSaving || settingsStore.isLoading" @click="handleRenameProfile">{{ t("settings.rename") }}</Button>
              <Button variant="secondary" :disabled="!activeProfile || settingsStore.isSaving || settingsStore.isLoading" @click="handleDeleteProfile">{{ t("settings.delete") }}</Button>
            </div>
          </div>
          <div class="settings-inline-note">
            {{ t("settings.importNote") }}
          </div>
          <div class="profile-grid">
            <button
              v-for="profile in profiles"
              :key="profile.id"
              class="profile-card"
              :data-active="profile.id === settingsStore.activeProfileId"
              type="button"
              @click="handleActivateProfile(profile.id)"
            >
              <div class="stack" style="gap: 6px;">
                <strong>{{ resolveProfileName(profile.name) }}</strong>
                <span class="muted">{{ profile.settings.provider }}</span>
                <span class="muted">{{ profile.settings.model }}</span>
              </div>
              <span class="profile-card__badge">{{ profile.id === settingsStore.activeProfileId ? t("settings.profileActive") : t("settings.profileSwitch") }}</span>
            </button>
          </div>
        </div>
      </section>

      <section class="panel settings-panel">
        <div class="stack" style="gap: 12px;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("settings.presetsTitle") }}</strong>
            <span class="muted">{{ t("settings.presetsDescription") }}</span>
          </div>
          <div class="preset-grid">
            <button
              v-for="preset in runtimePresets"
              :key="preset.id"
              class="preset-card"
              type="button"
              @click="applyPreset(preset)"
            >
              <div class="stack" style="gap: 6px;">
                <strong>{{ resolvePresetLabel(preset.labelKey) }}</strong>
                <span class="muted">{{ preset.provider }}</span>
                <span class="muted">{{ preset.model }}</span>
              </div>
              <p class="preset-card__note">{{ t(preset.noteKey) }}</p>
            </button>
          </div>
        </div>
      </section>

      <section class="panel settings-panel">
        <div class="row" style="justify-content: space-between; align-items: flex-start; margin-bottom: 12px; flex-wrap: wrap;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ t("settings.editingProfile", { name: resolveProfileName(activeProfile?.name) }) }}</strong>
            <span class="muted">{{ t("settings.editingProfileDescription") }}</span>
          </div>
          <span class="profile-inline-badge">{{ resolveProfileName(settingsStore.status.profile_name) }}</span>
        </div>

        <datalist id="runtime-provider-options">
          <option v-for="suggestion in providerSuggestions" :key="suggestion.id" :value="suggestion.provider">{{ resolvePresetLabel(suggestion.labelKey) }}</option>
        </datalist>
        <datalist id="runtime-model-options">
          <option v-for="model in visibleModelSuggestions" :key="model" :value="model">{{ model }}</option>
        </datalist>
        <datalist id="runtime-auth-profile-options">
          <option v-for="profile in authProfiles" :key="profile.id" :value="profile.profile_name">{{ profile.profile_name }}</option>
        </datalist>

        <div class="settings-grid">
          <label class="settings-field">
            <span class="settings-field__label">{{ t("settings.provider") }}</span>
            <input v-model="form.provider" list="runtime-provider-options" class="field" :placeholder="t('settings.providerPlaceholder')" />
            <span class="muted settings-field__hint">{{ t("settings.providerHint") }}</span>
            <div v-if="visibleProviderSuggestions.length" class="suggestion-row">
              <button
                v-for="suggestion in visibleProviderSuggestions"
                :key="suggestion.id"
                class="suggestion-chip"
                type="button"
                @click="applyProviderSuggestion(suggestion)"
              >
                <span class="suggestion-chip__label">{{ resolvePresetLabel(suggestion.labelKey) }}</span>
                <span class="suggestion-chip__meta">{{ suggestion.provider }}</span>
              </button>
            </div>
          </label>

          <label class="settings-field">
            <span class="settings-field__label">{{ t("settings.model") }}</span>
            <input v-model="form.model" list="runtime-model-options" class="field" :placeholder="t('settings.modelPlaceholder')" />
            <span class="muted settings-field__hint">{{ t("settings.modelHint") }}</span>
            <div v-if="visibleModelSuggestions.length" class="suggestion-row">
              <button
                v-for="model in visibleModelSuggestions"
                :key="model"
                class="suggestion-chip"
                type="button"
                @click="applyModelSuggestion(model)"
              >
                <span class="suggestion-chip__label">{{ model }}</span>
                <span class="suggestion-chip__meta">{{ t("settings.suggested") }}</span>
              </button>
            </div>
          </label>

          <label class="settings-field settings-field--wide">
            <span class="settings-field__label">{{ t("settings.providerUrl") }}</span>
            <input v-model="form.provider_url" class="field" :placeholder="t('settings.providerUrlPlaceholder')" />
            <span class="muted settings-field__hint">{{ t("settings.providerUrlHint") }}</span>
            <span class="settings-context-note">{{ providerContextHint }}</span>
          </label>

          <label v-if="currentProviderSupportsAuth && !currentProviderRequiresAuthProfile" class="settings-field">
            <span class="settings-field__label">{{ t("settings.credentialMode") }}</span>
            <select v-model="form.credential_mode" class="select">
              <option value="api_key">{{ t("settings.credentialModes.apiKey") }}</option>
              <option value="auth_profile">{{ t("settings.credentialModes.authProfile") }}</option>
            </select>
            <span class="muted settings-field__hint">{{ t("settings.credentialModeHint") }}</span>
          </label>

          <div v-else-if="currentProviderRequiresAuthProfile" class="settings-subsection settings-field--wide">
            <div class="stack" style="gap: 6px;">
              <strong>{{ t("settings.credentialModes.authProfile") }}</strong>
              <span class="muted">{{ t("settings.authProfileRequiredHint") }}</span>
            </div>
          </div>

          <label v-if="showAuthSettings" class="settings-field settings-field--wide">
            <span class="settings-field__label">{{ t("settings.authProfile") }}</span>
            <div class="row settings-secret-row">
              <input
                v-model="form.auth_profile"
                list="runtime-auth-profile-options"
                class="field"
                :placeholder="t('settings.authProfilePlaceholder')"
              />
              <Button variant="secondary" :disabled="authProfilesLoading" @click="refreshAuthProfiles">
                {{ authProfilesLoading ? t("settings.authProfilesLoading") : t("settings.refreshAuthProfiles") }}
              </Button>
              <Button variant="secondary" @click="handleStartAuthLogin">{{ t("settings.startAuthLogin") }}</Button>
            </div>
            <span class="muted settings-field__hint">{{ t("settings.authProfileHint") }}</span>
            <span class="settings-context-note">{{ authLoginHint }}</span>
            <div v-if="authProfiles.length" class="suggestion-row">
              <button
                v-for="profile in authProfiles"
                :key="profile.id"
                class="suggestion-chip"
                type="button"
                @click="handleSelectAuthProfile(profile.profile_name)"
              >
                <span class="suggestion-chip__label">{{ profile.profile_name }}</span>
                <span class="suggestion-chip__meta">
                  {{ profile.is_active ? t("settings.authProfileActive") : profile.kind }}
                </span>
              </button>
            </div>
            <span v-else-if="!authProfilesLoading" class="muted settings-field__hint">{{ t("settings.authProfilesEmpty") }}</span>
          </label>

          <div v-if="authLoginChallenge" class="settings-subsection settings-field--wide">
            <div class="stack" style="gap: 6px;">
              <strong>{{ t("settings.authLoginTitle") }}</strong>
              <span v-if="authLoginChallenge.user_code" class="muted">{{ t("settings.authLoginCode", { code: authLoginChallenge.user_code }) }}</span>
              <span class="muted">{{ t("settings.authLoginExpiresAt", { value: formatTimestamp(authLoginChallenge.expires_at, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
              <a :href="authLoginChallenge.verification_uri_complete || authLoginChallenge.verification_uri" target="_blank" rel="noreferrer">
                {{ t("settings.openAuthLoginPage") }}
              </a>
              <span v-if="authLoginChallenge.message" class="muted">{{ authLoginChallenge.message }}</span>
              <span v-if="authLoginStatus" class="muted">
                {{ t("settings.authLoginStatusLabel", { status: authLoginStatus.status, message: authLoginStatus.message }) }}
              </span>
            </div>
          </div>

          <label v-if="currentProviderSupportsApiKey && effectiveCredentialMode === 'api_key'" class="settings-field settings-field--wide">
            <span class="settings-field__label">{{ t("settings.apiKey") }}</span>
            <div class="row settings-secret-row">
              <input
                v-model="form.api_key"
                :type="showApiKey ? 'text' : 'password'"
                class="field"
                :placeholder="t('settings.apiKeyPlaceholder')"
              />
              <Button variant="ghost" @click="showApiKey = !showApiKey">{{ showApiKey ? t("settings.hide") : t("settings.show") }}</Button>
            </div>
            <span class="muted settings-field__hint">{{ t("settings.apiKeyHint") }}</span>
          </label>

          <label class="settings-field">
            <span class="settings-field__label">{{ t("settings.temperature") }}</span>
            <input v-model.number="form.temperature" class="field" type="number" min="0" max="2" step="0.1" />
            <span class="muted settings-field__hint">{{ t("settings.temperatureHint") }}</span>
          </label>
        </div>
      </section>

      <section class="panel settings-panel">
        <button class="settings-collapsible" type="button" :aria-expanded="showProxySettings" @click="showProxySettings = !showProxySettings">
          <div class="stack" style="gap: 6px; text-align: left;">
            <strong>{{ t("settings.proxyTitle") }}</strong>
            <span class="muted">{{ t("settings.proxyDescription") }}</span>
            <span class="settings-context-note">{{ proxySummary }}</span>
          </div>
          <span class="profile-inline-badge">{{ showProxySettings ? t("settings.hide") : t("settings.show") }}</span>
        </button>

        <div v-if="showProxySettings" class="stack settings-collapsible__body">
          <div class="settings-grid">
            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.proxyEnabled") }}</span>
              <label class="settings-checkbox">
                <input v-model="form.proxy.enabled" type="checkbox" />
                <span>{{ t("settings.proxyEnabledHint") }}</span>
              </label>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.proxyScope") }}</span>
              <select v-model="form.proxy.scope" class="select">
                <option value="zeroclaw">{{ t("settings.proxyScopes.zeroclaw") }}</option>
                <option value="services">{{ t("settings.proxyScopes.services") }}</option>
                <option value="environment">{{ t("settings.proxyScopes.environment") }}</option>
              </select>
              <span class="muted settings-field__hint">{{ proxyScopeHint }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.allProxy") }}</span>
              <input v-model="form.proxy.all_proxy" class="field" :placeholder="t('settings.proxyUrlPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.allProxyHint") }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.httpProxy") }}</span>
              <input v-model="form.proxy.http_proxy" class="field" :placeholder="t('settings.proxyUrlPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.httpProxyHint") }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.httpsProxy") }}</span>
              <input v-model="form.proxy.https_proxy" class="field" :placeholder="t('settings.proxyUrlPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.httpsProxyHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.noProxy") }}</span>
              <input v-model="proxyNoProxyText" class="field" :placeholder="t('settings.noProxyPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.noProxyHint") }}</span>
            </label>

            <label v-if="form.proxy.scope === 'services'" class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.proxyServices") }}</span>
              <input v-model="proxyServicesText" class="field" :placeholder="t('settings.proxyServicesPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.proxyServicesHint") }}</span>
            </label>
          </div>
        </div>
      </section>

      <section class="panel settings-panel settings-runtime-actions-panel">
        <div class="row settings-runtime-actions">
          <div class="stack settings-runtime-actions__meta">
            <strong>{{ t("settings.saveProfile") }}</strong>
            <span class="muted">{{ t("settings.editingProfileDescription") }}</span>
            <span v-if="saveMessage" class="muted">{{ saveMessage }}</span>
            <span v-if="settingsStore.error" class="settings-error">{{ settingsStore.error }}</span>
            <span v-if="settingsStore.lastSavedAt" class="muted">{{ t("settings.lastSavedAt", { value: formatTimestamp(settingsStore.lastSavedAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
          </div>
          <div class="row settings-action-row settings-runtime-actions__buttons">
            <Button variant="secondary" :disabled="settingsStore.isSaving || settingsStore.isTesting" @click="resetForm">{{ t("settings.reset") }}</Button>
            <Button variant="secondary" :disabled="settingsStore.isSaving || settingsStore.isLoading || settingsStore.isTesting" @click="handleTest">
              {{ settingsStore.isTesting ? t("settings.testing") : t("settings.testConnection") }}
            </Button>
            <Button :disabled="settingsStore.isSaving || settingsStore.isLoading || settingsStore.isTesting" @click="handleSave">
              {{ t("settings.saveProfile") }}
            </Button>
          </div>
        </div>
      </section>

      <section class="panel settings-panel">
        <div class="stack" style="gap: 8px;">
          <strong>{{ t("settings.connectionTest") }}</strong>
          <span class="muted">{{ t("settings.connectionTestDescription") }}</span>
          <span v-if="testMessage" :class="settingsStore.testReport?.ok ? 'muted' : 'settings-error'">{{ testMessage }}</span>
          <div v-if="settingsStore.testReport" class="settings-test-card" :data-ok="settingsStore.testReport.ok">
            <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px;">
              <div class="stack" style="gap: 4px;">
                <strong>{{ settingsStore.testReport.ok ? t("settings.runtimeReachable") : t("settings.runtimeFailed") }}</strong>
                <span class="muted">{{ t("settings.providerLabel", { value: settingsStore.testReport.provider }) }}</span>
                <span class="muted">{{ t("settings.modelLabel", { value: settingsStore.testReport.model }) }}</span>
              </div>
            </div>
            <p class="settings-test-card__message">{{ settingsStore.testReport.message }}</p>
            <div v-if="settingsStore.testReport.preview" class="code-block">{{ settingsStore.testReport.preview }}</div>
          </div>
        </div>
      </section>
    </template>

    <template v-else-if="activeSettingsTab === 'agent'">
      <section class="panel settings-panel">
        <div class="stack" style="gap: 12px;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("settings.agentWorkspaceTitle") }}</strong>
            <span class="muted">{{ t("settings.agentWorkspaceDescription") }}</span>
            <span class="settings-context-note">{{ agentSummary }}</span>
          </div>

          <div class="settings-grid">
            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.workspaceDirectory") }}</span>
              <div class="row settings-secret-row">
                <input v-model="form.agent.workspace_dir" class="field" :placeholder="t('settings.workspaceDirectoryPlaceholder')" />
                <Button variant="secondary" @click="handlePickWorkspace">{{ t("settings.chooseFolder") }}</Button>
              </div>
              <span class="muted settings-field__hint">{{ t("settings.workspaceDirectoryHint") }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.toolDispatcher") }}</span>
              <select v-model="form.agent.tool_dispatcher" class="select">
                <option value="auto">{{ t("settings.toolDispatchers.auto") }}</option>
                <option value="native">{{ t("settings.toolDispatchers.native") }}</option>
                <option value="xml">{{ t("settings.toolDispatchers.xml") }}</option>
              </select>
              <span class="muted settings-field__hint">{{ toolDispatcherHint }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.compactContext") }}</span>
              <label class="settings-checkbox">
                <input v-model="form.agent.compact_context" type="checkbox" />
                <span>{{ t("settings.compactContextHint") }}</span>
              </label>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.parallelTools") }}</span>
              <label class="settings-checkbox">
                <input v-model="form.agent.parallel_tools" type="checkbox" />
                <span>{{ t("settings.parallelToolsHint") }}</span>
              </label>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.maxToolIterations") }}</span>
              <input v-model.number="form.agent.max_tool_iterations" class="field" type="number" min="1" max="50" step="1" />
              <span class="muted settings-field__hint">{{ t("settings.maxToolIterationsHint") }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.maxHistoryMessages") }}</span>
              <input v-model.number="form.agent.max_history_messages" class="field" type="number" min="1" max="200" step="1" />
              <span class="muted settings-field__hint">{{ t("settings.maxHistoryMessagesHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.maxContextTokens") }}</span>
              <input v-model.number="form.agent.max_context_tokens" class="field" type="number" min="1000" max="200000" step="1000" />
              <span class="muted settings-field__hint">{{ t("settings.maxContextTokensHint") }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.autonomyLevel") }}</span>
              <select v-model="form.autonomy.level" class="select">
                <option value="read_only">{{ t("settings.autonomyLevels.read_only") }}</option>
                <option value="supervised">{{ t("settings.autonomyLevels.supervised") }}</option>
                <option value="full">{{ t("settings.autonomyLevels.full") }}</option>
              </select>
              <span class="muted settings-field__hint">{{ autonomyLevelHint }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.workspaceOnly") }}</span>
              <label class="settings-checkbox">
                <input v-model="form.autonomy.workspace_only" type="checkbox" />
                <span>{{ t("settings.workspaceOnlyHint") }}</span>
              </label>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.approvalGate") }}</span>
              <label class="settings-checkbox">
                <input v-model="form.autonomy.require_approval_for_medium_risk" type="checkbox" />
                <span>{{ t("settings.approvalGateHint") }}</span>
              </label>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.blockHighRisk") }}</span>
              <label class="settings-checkbox">
                <input v-model="form.autonomy.block_high_risk_commands" type="checkbox" />
                <span>{{ t("settings.blockHighRiskHint") }}</span>
              </label>
            </label>
          </div>

          <div class="settings-inline-note">
            {{ t("settings.runtimePolicySummary", { workspace: settingsStore.status.workspace_dir || workspaceSummary, autonomy: t(`settings.autonomyLevels.${settingsStore.status.autonomy_level}`), dispatcher: t(`settings.toolDispatchers.${settingsStore.status.tool_dispatcher}`) }) }}
          </div>
        </div>
      </section>

      <section class="panel settings-panel">
        <button class="settings-collapsible" type="button" :aria-expanded="showAutonomyAdvanced" @click="showAutonomyAdvanced = !showAutonomyAdvanced">
          <div class="stack" style="gap: 6px; text-align: left;">
            <strong>{{ t("settings.autonomyAdvancedTitle") }}</strong>
            <span class="muted">{{ t("settings.autonomyAdvancedDescription") }}</span>
          </div>
          <span class="profile-inline-badge">{{ showAutonomyAdvanced ? t("settings.hide") : t("settings.show") }}</span>
        </button>

        <div v-if="showAutonomyAdvanced" class="stack settings-collapsible__body">
          <div class="settings-grid">
            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.allowedCommands") }}</span>
              <input v-model="autonomyAllowedCommandsText" class="field" :placeholder="t('settings.allowedCommandsPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.allowedCommandsHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.allowedRoots") }}</span>
              <textarea v-model="autonomyAllowedRootsText" class="textarea" rows="4" :placeholder="t('settings.allowedRootsPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.allowedRootsHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.shellEnvPassthrough") }}</span>
              <input v-model="autonomyEnvText" class="field" :placeholder="t('settings.shellEnvPassthroughPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.shellEnvPassthroughHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.autoApproveTools") }}</span>
              <input v-model="autonomyAutoApproveText" class="field" :placeholder="t('settings.autoApproveToolsPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.autoApproveToolsHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.alwaysAskTools") }}</span>
              <input v-model="autonomyAlwaysAskText" class="field" :placeholder="t('settings.alwaysAskToolsPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.alwaysAskToolsHint") }}</span>
            </label>
          </div>
        </div>
      </section>
    </template>

    <template v-else-if="activeSettingsTab === 'updates'">
      <section class="panel settings-panel">
        <div class="stack" style="gap: 12px;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("settings.updatesTitle") }}</strong>
            <span class="muted">{{ t("settings.updatesDescription") }}</span>
          </div>

          <div class="settings-grid">
            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.updateEnabled") }}</span>
              <label class="settings-checkbox">
                <input v-model="updateForm.enabled" type="checkbox" />
                <span>{{ t("settings.updateEnabledHint") }}</span>
              </label>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.updateAutoCheck") }}</span>
              <label class="settings-checkbox">
                <input v-model="updateForm.auto_check" type="checkbox" :disabled="!updateForm.enabled" />
                <span>{{ t("settings.updateAutoCheckHint") }}</span>
              </label>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.updateEndpoints") }}</span>
              <textarea v-model="updateEndpointsText" class="textarea" rows="4" :placeholder="t('settings.updateEndpointsPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.updateEndpointsHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.updatePubkey") }}</span>
              <textarea v-model="updateForm.pubkey" class="textarea" rows="5" :placeholder="t('settings.updatePubkeyPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.updatePubkeyHint") }}</span>
            </label>
          </div>

          <div class="row" style="justify-content: space-between; align-items: flex-start; margin-top: 4px; flex-wrap: wrap; gap: 16px;">
            <div class="stack" style="gap: 6px; max-width: 760px;">
              <span v-if="updateMessage" class="muted">{{ updateMessage }}</span>
              <span v-if="updateStore.error" class="settings-error">{{ updateStore.error }}</span>
              <span v-if="updateStore.lastCheck" class="muted">{{ t("settings.lastCheckedAt", { value: formatTimestamp(updateStore.lastCheck.checked_at, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
              <span v-if="updateStore.lastInstall" class="muted">{{ t("settings.lastUpdateInstallAt", { value: formatTimestamp(updateStore.lastInstall.checked_at, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
            </div>
            <div class="row settings-action-row">
              <Button variant="secondary" :disabled="updateStore.isSaving || updateStore.isChecking || updateStore.isInstalling" @click="resetUpdateForm">{{ t("settings.reset") }}</Button>
              <Button variant="secondary" :disabled="updateStore.isSaving || updateStore.isChecking || updateStore.isInstalling" @click="handleSaveUpdateSettings">
                {{ updateStore.isSaving ? t("settings.savingUpdates") : t("settings.saveUpdates") }}
              </Button>
              <Button variant="secondary" :disabled="updateStore.isSaving || updateStore.isChecking || updateStore.isInstalling || !canRunUpdater" @click="handleCheckForUpdates">
                {{ updateStore.isChecking ? t("settings.checkingUpdates") : t("settings.checkUpdates") }}
              </Button>
              <Button :disabled="updateStore.isSaving || updateStore.isChecking || updateStore.isInstalling || !canRunUpdater" @click="handleInstallUpdate">
                {{ updateStore.isInstalling ? t("settings.installingUpdate") : t("settings.installUpdate") }}
              </Button>
            </div>
          </div>

          <div v-if="updateStore.lastCheck" class="settings-test-card" :data-ok="updateStore.lastCheck.update_available ? 'true' : 'false'">
            <div class="stack" style="gap: 6px;">
              <strong>{{ updateStore.lastCheck.update_available ? t("settings.updateAvailable") : t("settings.updateUnavailable") }}</strong>
              <span class="muted">{{ updateStore.lastCheck.message }}</span>
              <span class="muted">{{ t("settings.currentVersionLabel", { value: updateStore.lastCheck.current_version }) }}</span>
              <span class="muted">{{ t("settings.latestVersionLabel", { value: updateStore.lastCheck.latest_version ?? t('settings.none') }) }}</span>
              <span v-if="updateStore.lastCheck.pub_date" class="muted">{{ t("settings.updatePublishedAt", { value: formatTimestamp(updateStore.lastCheck.pub_date, { year: 'numeric', month: '2-digit', day: '2-digit' }) }) }}</span>
              <span v-if="updateStore.lastCheck.download_url" class="muted">{{ t("settings.updateDownloadUrl", { value: updateStore.lastCheck.download_url }) }}</span>
            </div>
            <p v-if="updateStore.lastCheck.notes" class="settings-test-card__message">{{ updateStore.lastCheck.notes }}</p>
          </div>

          <div v-if="updateStore.lastInstall" class="settings-inline-note">
            {{ updateStore.lastInstall.message }}
          </div>
        </div>
      </section>
    </template>
  </div>
</template>
