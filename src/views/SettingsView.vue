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
  type RuntimeProviderEntryRecord,
  type RuntimeProviderGroupRecord,
  type RuntimeProxySettingsRecord,
  type RuntimeSettingsRecord,
  type UpdateSettingsRecord
} from "@/api/tauri";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { useAppStore } from "@/stores/app";
import {
  defaultProxySettings,
  defaultRuntimeProviderEntry,
  defaultRuntimeProviderGroup,
  defaultRuntimeSettings,
  useSettingsStore
} from "@/stores/settings";
import { defaultUpdateSettings, useUpdateStore } from "@/stores/update";

interface RuntimePreset {
  id: string;
  labelKey: string;
  groupName: string;
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

type SettingsTabKey = "general" | "runtime" | "proxy" | "agent" | "updates";

interface SettingsTabDefinition {
  id: SettingsTabKey;
  labelKey: string;
  descriptionKey: string;
}

const runtimePresets: RuntimePreset[] = [
  {
    id: "openrouter",
    labelKey: "settings.presetLabels.openrouter",
    groupName: "General",
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
    groupName: "OpenAI",
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
    groupName: "Codex",
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
    groupName: "Gemini",
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
    groupName: "Claude",
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
    groupName: "Local",
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
    groupName: "Custom",
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
    id: "proxy",
    labelKey: "settings.tabs.proxy.label",
    descriptionKey: "settings.tabs.proxy.description"
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
const { activeProfile, profiles, proxySettings, proxySupport } = storeToRefs(settingsStore);
const activeSettingsTab = ref<SettingsTabKey>("general");
const showApiKey = ref(false);
const showAutonomyAdvanced = ref(false);
const saveMessage = ref("");
const testMessage = ref("");
const updateMessage = ref("");
const authProfiles = ref<AuthProfileRecord[]>([]);
const authProfilesLoading = ref(false);
const authLoginChallenge = ref<AuthLoginChallengeRecord | null>(null);
const authLoginStatus = ref<AuthLoginStatusRecord | null>(null);
const authPollingHandle = ref<number | null>(null);
const selectedRuntimeGroupId = ref("");
const selectedRuntimeEntryId = ref("");
const runtimeEditorMode = ref<"closed" | "create" | "edit">("closed");
const createGroupName = ref("");
const createEntryDraft = reactive(defaultRuntimeProviderEntry());
const drawerRuntimeGroupId = ref("");
const drawerRuntimeEntryId = ref("");

const form = reactive(defaultRuntimeSettings());
const proxyForm = reactive(defaultProxySettings());
const updateForm = reactive(defaultUpdateSettings());

const activeRuntimeGroup = computed(
  () =>
    form.groups.find((group) => group.id === form.active_group_id) ??
    form.groups[0] ??
    null
);
const selectedRuntimeGroup = computed(
  () =>
    form.groups.find((group) => group.id === selectedRuntimeGroupId.value) ??
    activeRuntimeGroup.value ??
    null
);
const activeRuntimeEntry = computed(
  () =>
    activeRuntimeGroup.value?.entries.find((entry) => entry.id === activeRuntimeGroup.value?.active_entry_id) ??
    activeRuntimeGroup.value?.entries[0] ??
    null
);
const selectedRuntimeEntry = computed(
  () =>
    selectedRuntimeGroup.value?.entries.find((entry) => entry.id === selectedRuntimeEntryId.value) ??
    selectedRuntimeGroup.value?.entries[0] ??
    (selectedRuntimeGroup.value?.id === activeRuntimeGroup.value?.id ? activeRuntimeEntry.value : null)
);
const isRuntimeDrawerOpen = computed(() => runtimeEditorMode.value !== "closed");
const isCreateDrawer = computed(() => runtimeEditorMode.value === "create");
const isEditDrawer = computed(() => runtimeEditorMode.value === "edit");
const drawerRuntimeGroup = computed(
  () =>
    form.groups.find((group) => group.id === drawerRuntimeGroupId.value) ??
    activeRuntimeGroup.value ??
    null
);
const currentEditableRuntimeEntry = computed(() => (isRuntimeDrawerOpen.value ? createEntryDraft : null));
const currentProviderKey = computed(() => resolveProviderKey(currentEditableRuntimeEntry.value?.provider ?? ""));
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

  return currentEditableRuntimeEntry.value?.credential_mode ?? "api_key";
});
const showAuthSettings = computed(() => currentProviderSupportsAuth.value && effectiveCredentialMode.value === "auth_profile");
const runtimeGroups = computed(() => form.groups);
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
  get: () => proxyForm.no_proxy.join(", "),
  set: (value: string) => {
    proxyForm.no_proxy = splitCommaSeparated(value);
  }
});
const proxyServicesText = computed({
  get: () => proxyForm.services.join(", "),
  set: (value: string) => {
    proxyForm.services = splitCommaSeparated(value);
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
  const query = currentEditableRuntimeEntry.value?.provider.trim().toLowerCase() ?? "";
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
  const query = currentEditableRuntimeEntry.value?.model.trim().toLowerCase() ?? "";
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
  switch (proxyForm.scope) {
    case "environment":
      return t("settings.proxyScopeHints.environment");
    case "services":
      return t("settings.proxyScopeHints.services");
    default:
      return t("settings.proxyScopeHints.zeroclaw");
  }
});
const proxySummary = computed(() => {
  if (!proxyForm.enabled) {
    return t("settings.proxyStatusDisabled");
  }

  const configuredTargets = [proxyForm.http_proxy, proxyForm.https_proxy, proxyForm.all_proxy].filter(
    (value) => value.trim().length > 0
  ).length;
  const scopeLabel = t(`settings.proxyScopes.${proxyForm.scope}`);

  if (proxyForm.scope === "services") {
    return t("settings.proxyStatusServices", {
      scope: scopeLabel,
      targets: configuredTargets,
      count: proxyForm.services.length
    });
  }

  return t("settings.proxyStatusEnabled", {
    scope: scopeLabel,
    targets: configuredTargets
  });
});
const supportedProxySelectors = computed(() => proxySupport.value.supported_selectors ?? []);
const supportedProxyServiceKeys = computed(() => proxySupport.value.supported_service_keys ?? []);
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

function resolveRuntimeEntryName(entry: Pick<RuntimeProviderEntryRecord, "name" | "provider" | "model"> | null | undefined) {
  if (!entry) {
    return t("settings.entryNames.current");
  }

  const trimmed = entry.name.trim();
  if (trimmed) {
    return trimmed;
  }

  if (entry.provider.trim() && entry.model.trim()) {
    return `${entry.provider} · ${entry.model}`;
  }

  if (entry.provider.trim()) {
    return entry.provider.trim();
  }

  return t("settings.entryNames.current");
}

function resolveRuntimeGroupName(group: Pick<RuntimeProviderGroupRecord, "name"> | null | undefined) {
  const value = group?.name.trim();
  return value || t("settings.groupNames.current");
}

function slugifyEntryName(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function createRuntimeGroupId(baseName: string) {
  const base = slugifyEntryName(baseName) || `group-${form.groups.length + 1}`;
  let candidate = base;
  let suffix = 2;

  while (form.groups.some((group) => group.id === candidate)) {
    candidate = `${base}-${suffix}`;
    suffix += 1;
  }

  return candidate;
}

function createRuntimeEntryId(baseName: string, group: RuntimeProviderGroupRecord | null | undefined = activeRuntimeGroup.value) {
  const existingEntries = group?.entries ?? [];
  const base = slugifyEntryName(baseName) || `entry-${existingEntries.length + 1}`;
  let candidate = base;
  let suffix = 2;

  while (existingEntries.some((entry) => entry.id === candidate)) {
    candidate = `${base}-${suffix}`;
    suffix += 1;
  }

  return candidate;
}

function createRuntimeEntryName(
  baseName: string,
  group: RuntimeProviderGroupRecord | null | undefined = activeRuntimeGroup.value
) {
  const existingEntries = group?.entries ?? [];
  const base = baseName.trim() || t("settings.entryNames.generated", { count: existingEntries.length + 1 });
  let candidate = base;
  let suffix = 2;

  while (existingEntries.some((entry) => entry.name === candidate)) {
    candidate = `${base} ${suffix}`;
    suffix += 1;
  }

  return candidate;
}

function createRuntimeGroup(name: string, overrides: Partial<RuntimeProviderGroupRecord> = {}) {
  const normalizedName = name.trim() || t("settings.groupNames.generated", { count: form.groups.length + 1 });
  return defaultRuntimeProviderGroup({
    ...overrides,
    id: createRuntimeGroupId(overrides.id ?? normalizedName),
    name: normalizedName,
    active_entry_id: overrides.active_entry_id ?? defaultRuntimeProviderEntry().id,
    entries: overrides.entries ?? [defaultRuntimeProviderEntry()]
  });
}

function createRuntimeEntry(
  overrides: Partial<RuntimeProviderEntryRecord> = {},
  group: RuntimeProviderGroupRecord | null | undefined = activeRuntimeGroup.value
) {
  const draft = defaultRuntimeProviderEntry({
    ...overrides
  });
  const name = createRuntimeEntryName(overrides.name ?? resolveRuntimeEntryName(draft), group);

  return defaultRuntimeProviderEntry({
    ...draft,
    ...overrides,
    name,
    id: createRuntimeEntryId(overrides.id ?? name, group)
  });
}

function ensureSelectedRuntimeEntry() {
  const currentGroup = selectedRuntimeGroup.value;
  const fallbackId = currentGroup?.active_entry_id || currentGroup?.entries[0]?.id || "";

  if (!selectedRuntimeEntryId.value || !currentGroup?.entries.some((entry) => entry.id === selectedRuntimeEntryId.value)) {
    selectedRuntimeEntryId.value = fallbackId;
  }
}

function resetCreateDraft() {
  const baseGroup = selectedRuntimeGroup.value;
  const baseEntry = selectedRuntimeEntry.value ?? activeRuntimeEntry.value;
  createGroupName.value = baseGroup ? resolveRuntimeGroupName(baseGroup) : "";
  drawerRuntimeGroupId.value = baseGroup?.id ?? "";
  drawerRuntimeEntryId.value = "";
  Object.assign(
    createEntryDraft,
    defaultRuntimeProviderEntry({
      name: "",
      provider: baseEntry?.provider ?? "",
      model: baseEntry?.model ?? "",
      provider_url: baseEntry?.provider_url ?? "",
      api_key: "",
      credential_mode: baseEntry?.credential_mode ?? "api_key",
      auth_profile: "",
      temperature: baseEntry?.temperature ?? defaultRuntimeProviderEntry().temperature
    })
  );
}

watch(
  () => settingsStore.runtime,
  (runtime) => {
    applyForm(runtime);
    runtimeEditorMode.value = "closed";
  },
  { deep: true, immediate: true }
);

watch(
  () => proxySettings.value,
  (settings) => {
    applyProxyForm(settings);
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
  () => [
    selectedRuntimeGroupId.value,
    form.active_group_id,
    ...form.groups.map((group) => `${group.id}:${group.active_entry_id}:${group.entries.map((entry) => entry.id).join(",")}`)
  ],
  () => {
    ensureSelectedRuntimeEntry();
  },
  { immediate: true }
);

watch(
  [currentProviderKey, effectiveCredentialMode],
  async ([providerKey, credentialMode]) => {
    const targetEntry = isRuntimeDrawerOpen.value ? createEntryDraft : null;
    if (!targetEntry) {
      return;
    }

    if (providerKey === "openai-codex") {
      targetEntry.credential_mode = "auth_profile";
    } else if (providerKey !== "gemini") {
      targetEntry.credential_mode = "api_key";
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
  if (!settingsStore.loaded || !settingsStore.proxyLoaded) {
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

async function handleSaveProxy() {
  saveMessage.value = "";

  try {
    await settingsStore.saveProxy(buildProxySettingsPayload());
    saveMessage.value = t("settings.feedback.proxySaved");
  } catch {
    saveMessage.value = t("settings.feedback.proxySaveFailed");
  }
}

async function handleSaveAgentSettings() {
  saveMessage.value = "";

  try {
    await settingsStore.save(buildRuntimeAgentPayload());
    saveMessage.value = t("settings.feedback.agentSaved");
  } catch {
    saveMessage.value = t("settings.feedback.agentSaveFailed");
  }
}

async function handleTest() {
  testMessage.value = "";

  try {
    const report = await settingsStore.test(
      buildRuntimeSettingsPayload(selectedRuntimeEntry.value?.id ?? activeRuntimeGroup.value?.active_entry_id ?? form.active_entry_id)
    );
    testMessage.value = report.message;
  } catch {
    testMessage.value = t("settings.feedback.testFailed");
  }
}

async function handleTestCreateRuntimeEntry() {
  if (!currentEditableRuntimeEntry.value || !isCreateDrawer.value) {
    return;
  }

  testMessage.value = "";

  try {
    const report = await settingsStore.test(buildRuntimeDrawerTestPayload(currentEditableRuntimeEntry.value));
    testMessage.value = report.message;
  } catch {
    testMessage.value = t("settings.feedback.testFailed");
  }
}

function resetProxyForm() {
  applyProxyForm(proxySettings.value);
  saveMessage.value = t("settings.feedback.proxyReset");
}

function resetAgentForm() {
  const normalized = cloneRuntimeSettings(settingsStore.runtime);
  Object.assign(form.agent, normalized.agent);
  Object.assign(form.autonomy, normalized.autonomy);
  saveMessage.value = t("settings.feedback.agentReset");
}

function appendProxyService(value: string) {
  const next = new Set(proxyForm.services);
  next.add(value);
  proxyForm.services = [...next];
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
  const entry = currentEditableRuntimeEntry.value;
  if (!showAuthSettings.value || !entry) {
    authProfiles.value = [];
    return;
  }

  authProfilesLoading.value = true;

  try {
    const state = await listAuthProfiles(entry.provider);
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
      if (currentEditableRuntimeEntry.value) {
        currentEditableRuntimeEntry.value.auth_profile = status.profile_name;
      }
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
  const entry = currentEditableRuntimeEntry.value;
  if (!showAuthSettings.value || !entry) {
    return;
  }

  saveMessage.value = "";
  authLoginStatus.value = null;

  try {
    const challenge = await startAuthLogin(entry.provider, entry.auth_profile.trim() || "default");
    authLoginChallenge.value = challenge;
    entry.auth_profile = challenge.profile_name;
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
  if (!currentEditableRuntimeEntry.value) {
    return;
  }

  currentEditableRuntimeEntry.value.auth_profile = profileName;
  saveMessage.value = t("settings.feedback.authProfileSelected", { name: profileName });
}

function applyPreset(preset: RuntimePreset) {
  Object.assign(
    createEntryDraft,
    defaultRuntimeProviderEntry({
      name: resolvePresetLabel(preset.labelKey),
      provider: preset.provider,
      model: preset.model,
      provider_url: preset.providerUrl,
      credential_mode: preset.credentialMode,
      temperature: preset.temperature
    })
  );
  createGroupName.value = preset.groupName;
  saveMessage.value = t("settings.feedback.entryPresetLoaded", {
    name: resolvePresetLabel(preset.labelKey)
  });
  testMessage.value = "";
  settingsStore.testReport = null;
}

function applyProviderSuggestion(suggestion: ProviderSuggestion) {
  if (!currentEditableRuntimeEntry.value) {
    return;
  }

  currentEditableRuntimeEntry.value.provider = suggestion.provider;
  currentEditableRuntimeEntry.value.provider_url = suggestion.providerUrl;
  currentEditableRuntimeEntry.value.credential_mode = suggestion.credentialMode;
  currentEditableRuntimeEntry.value.model = suggestion.defaultModel;
  saveMessage.value = t("settings.feedback.providerSuggestionLoaded", { name: resolvePresetLabel(suggestion.labelKey) });
  testMessage.value = "";
  settingsStore.testReport = null;
}

function applyModelSuggestion(model: string) {
  if (!currentEditableRuntimeEntry.value) {
    return;
  }

  currentEditableRuntimeEntry.value.model = model;
  saveMessage.value = t("settings.feedback.modelSuggestionLoaded", { name: model });
  testMessage.value = "";
  settingsStore.testReport = null;
}

function handleSelectRuntimeGroup(groupId: string) {
  selectedRuntimeGroupId.value = groupId;
  const group = form.groups.find((candidate) => candidate.id === groupId);
  selectedRuntimeEntryId.value = group?.active_entry_id ?? group?.entries[0]?.id ?? "";
  showApiKey.value = false;
}

function handleSelectRuntimeEntry(entryId: string) {
  selectedRuntimeEntryId.value = entryId;
  showApiKey.value = false;
}

function handleOpenCreateRuntimeEntry() {
  resetCreateDraft();
  runtimeEditorMode.value = "create";
  showApiKey.value = false;
  stopAuthLoginPolling();
  authLoginChallenge.value = null;
  authLoginStatus.value = null;
}

function handleOpenEditRuntimeEntry(groupId: string, entryId: string) {
  const group = form.groups.find((candidate) => candidate.id === groupId);
  const entry = group?.entries.find((candidate) => candidate.id === entryId);
  if (!group || !entry) {
    return;
  }

  drawerRuntimeGroupId.value = group.id;
  drawerRuntimeEntryId.value = entry.id;
  createGroupName.value = group.name;
  Object.assign(
    createEntryDraft,
    defaultRuntimeProviderEntry({
      ...entry
    })
  );
  runtimeEditorMode.value = "edit";
  showApiKey.value = false;
  stopAuthLoginPolling();
  authLoginChallenge.value = null;
  authLoginStatus.value = null;
}

function handleCloseRuntimeDrawer() {
  runtimeEditorMode.value = "closed";
  resetCreateDraft();
  stopAuthLoginPolling();
  authLoginChallenge.value = null;
  authLoginStatus.value = null;
}

function resolveTargetGroupByName(name: string) {
  const normalized = name.trim().toLowerCase();
  if (!normalized) {
    return selectedRuntimeGroup.value ?? activeRuntimeGroup.value ?? null;
  }

  return form.groups.find((group) => group.name.trim().toLowerCase() === normalized) ?? null;
}

function snapshotRuntimeDraftSections() {
  return {
    agent: {
      ...form.agent
    },
    autonomy: {
      ...form.autonomy,
      allowed_commands: [...form.autonomy.allowed_commands],
      allowed_roots: [...form.autonomy.allowed_roots],
      shell_env_passthrough: [...form.autonomy.shell_env_passthrough],
      auto_approve: [...form.autonomy.auto_approve],
      always_ask: [...form.autonomy.always_ask]
    }
  };
}

function restoreRuntimeDraftSections(snapshot: ReturnType<typeof snapshotRuntimeDraftSections>) {
  Object.assign(form.agent, snapshot.agent);
  Object.assign(form.autonomy, {
    ...snapshot.autonomy,
    allowed_commands: [...snapshot.autonomy.allowed_commands],
    allowed_roots: [...snapshot.autonomy.allowed_roots],
    shell_env_passthrough: [...snapshot.autonomy.shell_env_passthrough],
    auto_approve: [...snapshot.autonomy.auto_approve],
    always_ask: [...snapshot.autonomy.always_ask]
  });
}

function buildRuntimeEntryGroupsPayload(activeEntryId = activeRuntimeGroup.value?.active_entry_id ?? form.active_entry_id) {
  const persisted = cloneRuntimeSettings(settingsStore.runtime);
  const runtimePayload = buildRuntimeSettingsPayload(activeEntryId);

  return cloneRuntimeSettings({
    ...persisted,
    active_group_id: runtimePayload.active_group_id,
    groups: runtimePayload.groups,
    active_entry_id: runtimePayload.active_entry_id,
    entries: runtimePayload.entries,
    provider: runtimePayload.provider,
    model: runtimePayload.model,
    provider_url: runtimePayload.provider_url,
    api_key: runtimePayload.api_key,
    credential_mode: runtimePayload.credential_mode,
    auth_profile: runtimePayload.auth_profile,
    temperature: runtimePayload.temperature
  });
}

function buildRuntimeAgentPayload() {
  const persisted = cloneRuntimeSettings(settingsStore.runtime);

  return cloneRuntimeSettings({
    ...persisted,
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

async function persistRuntimeEntryGroups(successMessage: string, activeEntryId = activeRuntimeGroup.value?.active_entry_id ?? form.active_entry_id) {
  const draftSnapshot = snapshotRuntimeDraftSections();
  const selectedGroupId = selectedRuntimeGroup.value?.id ?? "";
  const selectedEntryId = selectedRuntimeEntryId.value;
  const activeGroupId = form.active_group_id;

  try {
    await settingsStore.save(buildRuntimeEntryGroupsPayload(activeEntryId));
    restoreRuntimeDraftSections(draftSnapshot);
    selectedRuntimeGroupId.value = form.groups.some((group) => group.id === selectedGroupId)
      ? selectedGroupId
      : form.active_group_id;
    form.active_group_id = form.groups.some((group) => group.id === activeGroupId) ? activeGroupId : form.active_group_id;
    selectedRuntimeEntryId.value = selectedEntryId;
    ensureSelectedRuntimeEntry();
    saveMessage.value = successMessage;
    testMessage.value = "";
    settingsStore.testReport = null;
    return true;
  } catch {
    saveMessage.value = t("settings.feedback.saveFailed");
    return false;
  }
}

function buildRuntimeDrawerTestPayload(entry: RuntimeProviderEntryRecord) {
  const base = buildRuntimeSettingsPayload();
  const groupName = createGroupName.value.trim() || resolveRuntimeGroupName(selectedRuntimeGroup.value ?? activeRuntimeGroup.value);
  const tempGroup = cloneRuntimeGroup(
    {
      id: drawerRuntimeGroupId.value || createRuntimeGroupId(groupName),
      name: groupName,
      active_entry_id: entry.id,
      entries: [
        cloneRuntimeEntry({
          ...entry,
          id: entry.id || createRuntimeEntryId(resolveRuntimeEntryName(entry), selectedRuntimeGroup.value ?? activeRuntimeGroup.value),
          name: entry.name.trim() || createRuntimeEntryName("", selectedRuntimeGroup.value ?? activeRuntimeGroup.value),
          credential_mode: resolveEffectiveCredentialMode(entry),
          temperature: Number(entry.temperature)
        })
      ]
    },
    0
  );
  const tempEntry = tempGroup.entries[0];

  return cloneRuntimeSettings({
    ...base,
    active_group_id: tempGroup.id,
    groups: [tempGroup],
    active_entry_id: tempEntry.id,
    entries: [tempEntry],
    provider: tempEntry.provider,
    model: tempEntry.model,
    provider_url: tempEntry.provider_url,
    api_key: tempEntry.api_key,
    credential_mode: tempEntry.credential_mode,
    auth_profile: tempEntry.auth_profile,
    temperature: Number(tempEntry.temperature)
  });
}

async function handleConfirmRuntimeDrawer() {
  if (isEditDrawer.value) {
    const group = drawerRuntimeGroup.value;
    const entryIndex = group?.entries.findIndex((entry) => entry.id === drawerRuntimeEntryId.value) ?? -1;
    if (!group || entryIndex < 0) {
      return;
    }

    const updatedEntry = createRuntimeEntry(
      {
        ...createEntryDraft,
        id: drawerRuntimeEntryId.value,
        name: createEntryDraft.name.trim() || resolveRuntimeEntryName(createEntryDraft)
      },
      group
    );
    group.entries[entryIndex] = updatedEntry;
    selectedRuntimeEntryId.value = updatedEntry.id;
    const persisted = await persistRuntimeEntryGroups(
      t("settings.feedback.updatedEntry", { name: resolveRuntimeEntryName(updatedEntry) }),
      updatedEntry.id
    );
    if (persisted) {
      handleCloseRuntimeDrawer();
    }
    return;
  }

  const resolvedGroupName = createGroupName.value.trim() || t("settings.groupNames.generated", { count: form.groups.length + 1 });
  let targetGroup = resolveTargetGroupByName(resolvedGroupName);

  if (!targetGroup) {
    targetGroup = createRuntimeGroup(resolvedGroupName, {
      entries: []
    });
    form.groups.push(targetGroup);
  }

  const entry = createRuntimeEntry(
    {
      ...createEntryDraft,
      name: createEntryDraft.name.trim() || createRuntimeEntryName("", targetGroup)
    },
    targetGroup
  );
  targetGroup.entries.push(entry);
  if (targetGroup.entries.length === 1) {
    targetGroup.active_entry_id = entry.id;
  }

  form.active_group_id = targetGroup.id;
  selectedRuntimeGroupId.value = targetGroup.id;
  selectedRuntimeEntryId.value = entry.id;
  const persisted = await persistRuntimeEntryGroups(
    t("settings.feedback.createdEntry", { name: resolveRuntimeEntryName(entry) }),
    entry.id
  );
  if (persisted) {
    handleCloseRuntimeDrawer();
  }
}

async function handleActivateRuntimeEntry(groupId: string, entryId: string) {
  const group = form.groups.find((candidate) => candidate.id === groupId);
  if (!group || (group.id === form.active_group_id && group.active_entry_id === entryId)) {
    return;
  }

  group.active_entry_id = entryId;
  form.active_group_id = group.id;
  selectedRuntimeGroupId.value = group.id;
  selectedRuntimeEntryId.value = entryId;
  await persistRuntimeEntryGroups(
    t("settings.feedback.activatedEntry", {
      name: resolveRuntimeEntryName(group.entries.find((entry) => entry.id === entryId))
    }),
    entryId
  );
}

async function handleDeleteRuntimeEntry() {
  const group = selectedRuntimeGroup.value;
  const entry = selectedRuntimeEntry.value;
  if (!group || !entry) {
    return;
  }

  const confirmed = window.confirm(
    t("settings.prompts.deleteEntry", { name: resolveRuntimeEntryName(entry) })
  );
  if (!confirmed) {
    return;
  }

  const deletedName = resolveRuntimeEntryName(entry);
  if (isEditDrawer.value && drawerRuntimeEntryId.value === entry.id && drawerRuntimeGroupId.value === group.id) {
    handleCloseRuntimeDrawer();
  }
  group.entries = group.entries.filter((candidate) => candidate.id !== entry.id);

  if (group.entries.length === 0) {
    form.groups = form.groups.filter((candidate) => candidate.id !== group.id);
    if (form.groups.length === 0) {
      const fallbackGroup = createRuntimeGroup(t("settings.groupNames.generated", { count: 1 }), {
        entries: [createRuntimeEntry({ name: t("settings.entryNames.generated", { count: 1 }) }, null)]
      });
      fallbackGroup.active_entry_id = fallbackGroup.entries[0].id;
      form.groups.push(fallbackGroup);
    }
    if (form.active_group_id === group.id) {
      form.active_group_id = form.groups[0].id;
    }
    selectedRuntimeGroupId.value = form.groups[0].id;
  } else {
    if (group.active_entry_id === entry.id) {
      group.active_entry_id = group.entries[0].id;
    }
    selectedRuntimeGroupId.value = group.id;
    if (form.active_group_id === group.id && entry.id === form.active_entry_id) {
      form.active_group_id = group.id;
    }
  }

  selectedRuntimeEntryId.value = selectedRuntimeGroup.value?.active_entry_id ?? selectedRuntimeGroup.value?.entries[0]?.id ?? "";
  await persistRuntimeEntryGroups(
    t("settings.feedback.deletedEntry", { name: deletedName }),
    activeRuntimeGroup.value?.active_entry_id ?? activeRuntimeGroup.value?.entries[0]?.id ?? form.active_entry_id
  );
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
  selectedRuntimeGroupId.value = normalized.active_group_id;
  resetCreateDraft();
  ensureSelectedRuntimeEntry();
}

function applyProxyForm(settings: RuntimeProxySettingsRecord) {
  const normalized = cloneProxySettings(settings);
  Object.assign(proxyForm, normalized);
}

function applyUpdateForm(settings: UpdateSettingsRecord) {
  const normalized = cloneUpdateSettings(settings);
  Object.assign(updateForm, normalized);
}

function cloneRuntimeSettings(runtime: RuntimeSettingsRecord): RuntimeSettingsRecord {
  const defaults = defaultRuntimeSettings();
  const legacyEntry = cloneRuntimeEntry(
    defaultRuntimeProviderEntry({
      provider: runtime.provider,
      model: runtime.model,
      provider_url: runtime.provider_url,
      api_key: runtime.api_key,
      credential_mode: runtime.credential_mode ?? defaults.credential_mode,
      auth_profile: runtime.auth_profile ?? defaults.auth_profile,
      temperature: runtime.temperature ?? defaults.entries[0].temperature
    })
  );
  const rawGroups = runtime.groups?.length
    ? runtime.groups
    : [
        defaultRuntimeProviderGroup({
          name: runtime.active_group_id || t("settings.groupNames.generated", { count: 1 }),
          active_entry_id: runtime.active_entry_id || legacyEntry.id,
          entries: runtime.entries?.length ? runtime.entries : [legacyEntry]
        })
      ];
  const groups = rawGroups.map((group, groupIndex) => cloneRuntimeGroup(group, groupIndex));
  const activeGroup =
    groups.find((group) => group.id === runtime.active_group_id) ??
    groups[0] ??
    cloneRuntimeGroup(defaults.groups[0], 0);
  const activeEntry =
    activeGroup.entries.find((entry) => entry.id === activeGroup.active_entry_id) ??
    activeGroup.entries[0] ??
    cloneRuntimeEntry(defaults.entries[0]);

  return {
    active_group_id: activeGroup.id,
    groups,
    active_entry_id: activeEntry.id,
    entries: [...activeGroup.entries],
    provider: activeEntry.provider,
    model: activeEntry.model,
    provider_url: activeEntry.provider_url,
    api_key: activeEntry.api_key,
    credential_mode: activeEntry.credential_mode,
    auth_profile: activeEntry.auth_profile,
    temperature: activeEntry.temperature,
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

function cloneRuntimeEntry(entry: RuntimeProviderEntryRecord): RuntimeProviderEntryRecord {
  const defaults = defaultRuntimeProviderEntry();

  return {
    id: entry.id ?? defaults.id,
    name: entry.name ?? defaults.name,
    provider: entry.provider ?? defaults.provider,
    model: entry.model ?? defaults.model,
    provider_url: entry.provider_url ?? defaults.provider_url,
    api_key: entry.api_key ?? defaults.api_key,
    credential_mode: entry.credential_mode ?? defaults.credential_mode,
    auth_profile: entry.auth_profile ?? defaults.auth_profile,
    temperature: entry.temperature ?? defaults.temperature
  };
}

function cloneRuntimeGroup(group: RuntimeProviderGroupRecord, index: number): RuntimeProviderGroupRecord {
  const defaults = defaultRuntimeProviderGroup();
  const entries = (group.entries?.length ? group.entries : defaults.entries).map((entry, entryIndex) =>
    cloneRuntimeEntry({
      ...entry,
      id: entry.id || `entry-${entryIndex + 1}`
    })
  );
  const activeEntry =
    entries.find((entry) => entry.id === group.active_entry_id) ??
    entries[0] ??
    cloneRuntimeEntry(defaults.entries[0]);

  return {
    id: group.id || `group-${index + 1}`,
    name: group.name || t("settings.groupNames.generated", { count: index + 1 }),
    active_entry_id: activeEntry.id,
    entries
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

function cloneProxySettings(settings: RuntimeProxySettingsRecord): RuntimeProxySettingsRecord {
  const defaults = defaultProxySettings();

  return {
    enabled: settings.enabled ?? defaults.enabled,
    scope: settings.scope ?? defaults.scope,
    http_proxy: settings.http_proxy ?? defaults.http_proxy,
    https_proxy: settings.https_proxy ?? defaults.https_proxy,
    all_proxy: settings.all_proxy ?? defaults.all_proxy,
    no_proxy: [...(settings.no_proxy ?? defaults.no_proxy)],
    services: [...(settings.services ?? defaults.services)]
  };
}

function buildRuntimeSettingsPayload(activeEntryId = activeRuntimeGroup.value?.active_entry_id ?? form.active_entry_id): RuntimeSettingsRecord {
  const groups = form.groups.map((group, groupIndex) =>
    cloneRuntimeGroup(
      {
        ...group,
        entries: group.entries.map((entry) =>
          cloneRuntimeEntry({
            ...entry,
            credential_mode: resolveEffectiveCredentialMode(entry),
            temperature: Number(entry.temperature)
          })
        )
      },
      groupIndex
    )
  );
  const activeGroup =
    groups.find((group) => group.id === form.active_group_id) ??
    groups[0] ??
    defaultRuntimeProviderGroup();
  const activeEntry =
    activeGroup.entries.find((entry) => entry.id === activeEntryId) ??
    activeGroup.entries.find((entry) => entry.id === activeGroup.active_entry_id) ??
    activeGroup.entries[0] ??
    defaultRuntimeProviderEntry();

  return cloneRuntimeSettings({
    active_group_id: activeGroup.id,
    groups,
    active_entry_id: activeEntry.id,
    entries: [...activeGroup.entries],
    provider: activeEntry.provider,
    model: activeEntry.model,
    provider_url: activeEntry.provider_url,
    api_key: activeEntry.api_key,
    credential_mode: activeEntry.credential_mode,
    auth_profile: activeEntry.auth_profile,
    temperature: Number(activeEntry.temperature),
    proxy: cloneProxySettings(proxySettings.value),
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

function resolveEffectiveCredentialMode(entry: RuntimeProviderEntryRecord): RuntimeCredentialModeRecord {
  const providerKey = resolveProviderKey(entry.provider);
  if (providerKey === "openai-codex") {
    return "auth_profile";
  }

  if (providerKey !== "gemini") {
    return "api_key";
  }

  return entry.credential_mode;
}

function buildUpdateSettingsPayload(): UpdateSettingsRecord {
  return cloneUpdateSettings({
    enabled: updateForm.enabled,
    auto_check: updateForm.auto_check,
    endpoints: [...updateForm.endpoints],
    pubkey: updateForm.pubkey
  });
}

function buildProxySettingsPayload(): RuntimeProxySettingsRecord {
  return cloneProxySettings({
    enabled: proxyForm.enabled,
    scope: proxyForm.scope,
    http_proxy: proxyForm.http_proxy,
    https_proxy: proxyForm.https_proxy,
    all_proxy: proxyForm.all_proxy,
    no_proxy: [...proxyForm.no_proxy],
    services: [...proxyForm.services]
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
        <div class="row" style="justify-content: space-between; align-items: flex-start; margin-bottom: 12px; flex-wrap: wrap;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ t("settings.groupsTitle") }}</strong>
            <span class="muted">{{ t("settings.groupsDescription") }}</span>
          </div>
          <div class="row settings-action-row">
            <Button variant="secondary" :disabled="settingsStore.isSaving || settingsStore.isLoading" @click="handleOpenCreateRuntimeEntry">
              {{ t("settings.newEntry") }}
            </Button>
          </div>
        </div>

        <div class="settings-group-tabs" role="tablist">
          <button
            v-for="group in runtimeGroups"
            :key="group.id"
            class="settings-group-tab"
            type="button"
            :data-active="group.id === selectedRuntimeGroup?.id"
            @click="handleSelectRuntimeGroup(group.id)"
          >
            <span>{{ resolveRuntimeGroupName(group) }}</span>
            <span class="settings-group-tab__meta">{{ group.entries.length }}</span>
          </button>
        </div>

        <div class="entry-grid" style="margin-top: 16px;">
          <button
            v-for="entry in selectedRuntimeGroup?.entries ?? []"
            :key="entry.id"
            class="entry-card"
            :data-active="selectedRuntimeGroup?.id === form.active_group_id && entry.id === selectedRuntimeGroup?.active_entry_id"
            :data-selected="entry.id === selectedRuntimeEntry?.id"
            type="button"
            @click="handleSelectRuntimeEntry(entry.id)"
          >
            <div class="stack" style="gap: 6px;">
              <strong>{{ resolveRuntimeEntryName(entry) }}</strong>
              <span class="muted">{{ entry.provider }}</span>
              <span class="muted">{{ entry.model }}</span>
            </div>
            <div class="row entry-card__badges">
              <span v-if="selectedRuntimeGroup?.id === form.active_group_id && entry.id === selectedRuntimeGroup?.active_entry_id" class="profile-card__badge">{{ t("settings.entryActive") }}</span>
              <span v-if="entry.id === selectedRuntimeEntry?.id" class="entry-card__badge entry-card__badge--selected">{{ t("settings.entrySelected") }}</span>
            </div>
            <div class="row entry-card__actions">
              <Button variant="ghost" @click.stop="handleOpenEditRuntimeEntry(selectedRuntimeGroup!.id, entry.id)">{{ t("settings.edit") }}</Button>
              <Button
                variant="secondary"
                :disabled="selectedRuntimeGroup?.id === form.active_group_id && entry.id === selectedRuntimeGroup?.active_entry_id"
                @click.stop="handleActivateRuntimeEntry(selectedRuntimeGroup!.id, entry.id)"
              >
                {{ t("settings.activateEntry") }}
              </Button>
              <Button variant="ghost" @click.stop="handleSelectRuntimeEntry(entry.id); handleDeleteRuntimeEntry()">{{ t("settings.delete") }}</Button>
            </div>
          </button>
        </div>
      </section>

      <datalist id="runtime-provider-options">
        <option v-for="suggestion in providerSuggestions" :key="suggestion.id" :value="suggestion.provider">{{ resolvePresetLabel(suggestion.labelKey) }}</option>
      </datalist>
      <datalist id="runtime-model-options">
        <option v-for="model in visibleModelSuggestions" :key="model" :value="model">{{ model }}</option>
      </datalist>
      <datalist id="runtime-auth-profile-options">
        <option v-for="profile in authProfiles" :key="profile.id" :value="profile.profile_name">{{ profile.profile_name }}</option>
      </datalist>

      <div v-if="isRuntimeDrawerOpen && currentEditableRuntimeEntry" class="settings-drawer-backdrop" @click="handleCloseRuntimeDrawer">
        <aside class="settings-drawer" @click.stop>
          <div class="settings-drawer__header">
            <div class="stack" style="gap: 4px;">
              <strong>{{ isCreateDrawer ? t("settings.createEntryTitle") : t("settings.editingEntry", { name: resolveRuntimeEntryName(currentEditableRuntimeEntry) }) }}</strong>
              <span class="muted">{{ isCreateDrawer ? t("settings.createEntryDescription") : t("settings.editingEntryDescription") }}</span>
            </div>
            <Button variant="ghost" @click="handleCloseRuntimeDrawer">{{ t("settings.closeDrawer") }}</Button>
          </div>

          <div class="settings-drawer__body stack" style="gap: 16px;">
            <div v-if="isCreateDrawer" class="stack" style="gap: 16px;">
              <div class="settings-inline-note">
                {{ t("settings.presetsDescription") }}
              </div>
              <div class="suggestion-row">
                <button
                  v-for="group in runtimeGroups"
                  :key="group.id"
                  class="suggestion-chip"
                  type="button"
                  @click="createGroupName = resolveRuntimeGroupName(group)"
                >
                  <span class="suggestion-chip__label">{{ resolveRuntimeGroupName(group) }}</span>
                  <span class="suggestion-chip__meta">{{ t("settings.useExistingGroup") }}</span>
                </button>
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
                    <span class="muted">{{ preset.groupName }}</span>
                    <span class="muted">{{ preset.model }}</span>
                  </div>
                  <p class="preset-card__note">{{ t(preset.noteKey) }}</p>
                </button>
              </div>
            </div>

            <div class="settings-grid">
              <label v-if="isCreateDrawer" class="settings-field settings-field--wide">
                <span class="settings-field__label">{{ t("settings.groupName") }}</span>
                <input v-model="createGroupName" class="field" :placeholder="t('settings.groupNamePlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.groupNameHint") }}</span>
              </label>

              <div class="settings-subsection settings-field--wide entry-editor-summary">
                <div class="stack" style="gap: 6px;">
                  <strong>{{ resolveRuntimeEntryName(currentEditableRuntimeEntry) }}</strong>
                  <span class="muted">
                    {{ isCreateDrawer ? t("settings.entryDraftDescription") : drawerRuntimeGroup?.id === form.active_group_id && currentEditableRuntimeEntry.id === drawerRuntimeGroup?.active_entry_id ? t("settings.entryActiveDescription") : t("settings.entryInactiveDescription") }}
                  </span>
                </div>
                <span class="entry-editor-summary__badge">
                  {{ isCreateDrawer ? t("settings.entryDraft") : drawerRuntimeGroup?.id === form.active_group_id && currentEditableRuntimeEntry.id === drawerRuntimeGroup?.active_entry_id ? t("settings.entryActive") : t("settings.entryInactive") }}
                </span>
              </div>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.entryName") }}</span>
                <input v-model="currentEditableRuntimeEntry.name" class="field" :placeholder="t('settings.entryNamePlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.entryNameHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.provider") }}</span>
                <input v-model="currentEditableRuntimeEntry.provider" list="runtime-provider-options" class="field" :placeholder="t('settings.providerPlaceholder')" />
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
                <input v-model="currentEditableRuntimeEntry.model" list="runtime-model-options" class="field" :placeholder="t('settings.modelPlaceholder')" />
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
                <input v-model="currentEditableRuntimeEntry.provider_url" class="field" :placeholder="t('settings.providerUrlPlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.providerUrlHint") }}</span>
                <span class="settings-context-note">{{ providerContextHint }}</span>
              </label>

              <label v-if="currentProviderSupportsAuth && !currentProviderRequiresAuthProfile" class="settings-field">
                <span class="settings-field__label">{{ t("settings.credentialMode") }}</span>
                <select v-model="currentEditableRuntimeEntry.credential_mode" class="select">
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
                    v-model="currentEditableRuntimeEntry.auth_profile"
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
                    v-model="currentEditableRuntimeEntry.api_key"
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
                <input v-model.number="currentEditableRuntimeEntry.temperature" class="field" type="number" min="0" max="2" step="0.1" />
                <span class="muted settings-field__hint">{{ t("settings.temperatureHint") }}</span>
              </label>
            </div>

            <div v-if="isCreateDrawer && (testMessage || settingsStore.testReport)" class="stack" style="gap: 10px;">
              <span v-if="testMessage" :class="settingsStore.testReport?.ok ? 'muted' : 'settings-error'">{{ testMessage }}</span>
              <div v-if="settingsStore.testReport" class="settings-test-card" :data-ok="settingsStore.testReport.ok">
                <div class="stack" style="gap: 4px;">
                  <strong>{{ settingsStore.testReport.ok ? t("settings.runtimeReachable") : t("settings.runtimeFailed") }}</strong>
                  <span class="muted">{{ t("settings.providerLabel", { value: settingsStore.testReport.provider }) }}</span>
                  <span class="muted">{{ t("settings.modelLabel", { value: settingsStore.testReport.model }) }}</span>
                </div>
                <p class="settings-test-card__message">{{ settingsStore.testReport.message }}</p>
                <div v-if="settingsStore.testReport.preview" class="code-block">{{ settingsStore.testReport.preview }}</div>
              </div>
            </div>
          </div>

          <div class="settings-drawer__footer">
            <Button variant="secondary" @click="handleCloseRuntimeDrawer">{{ t("settings.cancelCreateEntry") }}</Button>
            <Button v-if="isCreateDrawer" variant="secondary" :disabled="settingsStore.isTesting || settingsStore.isSaving" @click="handleTestCreateRuntimeEntry">
              {{ settingsStore.isTesting ? t("settings.testing") : t("settings.testConnection") }}
            </Button>
            <Button @click="handleConfirmRuntimeDrawer">
              {{ isCreateDrawer ? t("settings.confirmCreateEntry") : t("settings.saveEntryChanges") }}
            </Button>
          </div>
        </aside>
      </div>

      <section class="panel settings-panel settings-runtime-actions-panel">
        <div class="row settings-runtime-actions">
          <div class="stack settings-runtime-actions__meta">
            <strong>{{ t("settings.runtimeActionsTitle") }}</strong>
            <span class="muted">{{ isRuntimeDrawerOpen ? t("settings.finishEntryDrawerHint") : t("settings.runtimeActionsDescription") }}</span>
            <span v-if="saveMessage" class="muted">{{ saveMessage }}</span>
            <span v-if="settingsStore.error" class="settings-error">{{ settingsStore.error }}</span>
            <span v-if="settingsStore.lastSavedAt" class="muted">{{ t("settings.lastSavedAt", { value: formatTimestamp(settingsStore.lastSavedAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
          </div>
          <div class="row settings-action-row settings-runtime-actions__buttons">
            <Button variant="secondary" :disabled="isRuntimeDrawerOpen || settingsStore.isSaving || settingsStore.isTesting" @click="resetForm">{{ t("settings.reset") }}</Button>
            <Button variant="secondary" :disabled="isRuntimeDrawerOpen || settingsStore.isSaving || settingsStore.isLoading || settingsStore.isTesting" @click="handleTest">
              {{ settingsStore.isTesting ? t("settings.testing") : t("settings.testConnection") }}
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

    <template v-else-if="activeSettingsTab === 'proxy'">
      <section class="panel settings-panel">
        <div class="stack" style="gap: 12px;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("settings.proxyTitle") }}</strong>
            <span class="muted">{{ t("settings.proxyDescription") }}</span>
            <span class="settings-context-note">{{ proxySummary }}</span>
          </div>

          <div class="settings-grid">
            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.proxyEnabled") }}</span>
              <label class="settings-checkbox">
                <input v-model="proxyForm.enabled" type="checkbox" />
                <span>{{ t("settings.proxyEnabledHint") }}</span>
              </label>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.proxyScope") }}</span>
              <select v-model="proxyForm.scope" class="select">
                <option value="zeroclaw">{{ t("settings.proxyScopes.zeroclaw") }}</option>
                <option value="services">{{ t("settings.proxyScopes.services") }}</option>
                <option value="environment">{{ t("settings.proxyScopes.environment") }}</option>
              </select>
              <span class="muted settings-field__hint">{{ proxyScopeHint }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.allProxy") }}</span>
              <input v-model="proxyForm.all_proxy" class="field" :placeholder="t('settings.proxyUrlPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.allProxyHint") }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.httpProxy") }}</span>
              <input v-model="proxyForm.http_proxy" class="field" :placeholder="t('settings.proxyUrlPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.httpProxyHint") }}</span>
            </label>

            <label class="settings-field">
              <span class="settings-field__label">{{ t("settings.httpsProxy") }}</span>
              <input v-model="proxyForm.https_proxy" class="field" :placeholder="t('settings.proxyUrlPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.httpsProxyHint") }}</span>
            </label>

            <label class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.noProxy") }}</span>
              <input v-model="proxyNoProxyText" class="field" :placeholder="t('settings.noProxyPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.noProxyHint") }}</span>
            </label>

            <label v-if="proxyForm.scope === 'services'" class="settings-field settings-field--wide">
              <span class="settings-field__label">{{ t("settings.proxyServices") }}</span>
              <input v-model="proxyServicesText" class="field" :placeholder="t('settings.proxyServicesPlaceholder')" />
              <span class="muted settings-field__hint">{{ t("settings.proxyServicesHint") }}</span>
            </label>
          </div>

          <div class="settings-inline-note">
            {{ t("settings.proxyAppliesDescription") }}
          </div>
        </div>
      </section>

      <section v-if="supportedProxySelectors.length || supportedProxyServiceKeys.length" class="panel settings-panel">
        <div class="stack" style="gap: 12px;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("settings.proxySupportTitle") }}</strong>
            <span class="muted">{{ t("settings.proxySupportDescription") }}</span>
          </div>

          <div v-if="supportedProxySelectors.length" class="stack" style="gap: 8px;">
            <span class="settings-field__label">{{ t("settings.proxySupportedSelectors") }}</span>
            <div class="suggestion-row">
              <button
                v-for="selector in supportedProxySelectors"
                :key="selector"
                class="suggestion-chip"
                type="button"
                @click="appendProxyService(selector)"
              >
                <span class="suggestion-chip__label">{{ selector }}</span>
                <span class="suggestion-chip__meta">{{ t("settings.suggested") }}</span>
              </button>
            </div>
          </div>

          <div v-if="supportedProxyServiceKeys.length" class="stack" style="gap: 8px;">
            <span class="settings-field__label">{{ t("settings.proxySupportedKeys") }}</span>
            <div class="suggestion-row">
              <button
                v-for="serviceKey in supportedProxyServiceKeys"
                :key="serviceKey"
                class="suggestion-chip"
                type="button"
                @click="appendProxyService(serviceKey)"
              >
                <span class="suggestion-chip__label">{{ serviceKey }}</span>
                <span class="suggestion-chip__meta">{{ t("settings.suggested") }}</span>
              </button>
            </div>
          </div>
        </div>
      </section>

      <section class="panel settings-panel settings-runtime-actions-panel">
        <div class="row settings-runtime-actions">
          <div class="stack settings-runtime-actions__meta">
            <strong>{{ t("settings.proxySaveTitle") }}</strong>
            <span class="muted">{{ t("settings.proxySaveDescription") }}</span>
            <span v-if="saveMessage" class="muted">{{ saveMessage }}</span>
            <span v-if="settingsStore.error" class="settings-error">{{ settingsStore.error }}</span>
            <span v-if="settingsStore.lastSavedAt" class="muted">{{ t("settings.lastSavedAt", { value: formatTimestamp(settingsStore.lastSavedAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
          </div>
          <div class="row settings-action-row settings-runtime-actions__buttons">
            <Button variant="secondary" :disabled="settingsStore.isSaving" @click="resetProxyForm">{{ t("settings.reset") }}</Button>
            <Button :disabled="settingsStore.isSaving || settingsStore.isLoading" @click="handleSaveProxy">
              {{ t("settings.saveProxy") }}
            </Button>
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

      <section class="panel settings-panel settings-runtime-actions-panel">
        <div class="row settings-runtime-actions">
          <div class="stack settings-runtime-actions__meta">
            <strong>{{ t("settings.agentSaveTitle") }}</strong>
            <span class="muted">{{ t("settings.agentSaveDescription") }}</span>
            <span v-if="saveMessage" class="muted">{{ saveMessage }}</span>
            <span v-if="settingsStore.error" class="settings-error">{{ settingsStore.error }}</span>
            <span v-if="settingsStore.lastSavedAt" class="muted">{{ t("settings.lastSavedAt", { value: formatTimestamp(settingsStore.lastSavedAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
          </div>
          <div class="row settings-action-row settings-runtime-actions__buttons">
            <Button variant="secondary" :disabled="settingsStore.isSaving" @click="resetAgentForm">{{ t("settings.reset") }}</Button>
            <Button :disabled="settingsStore.isSaving || settingsStore.isLoading" @click="handleSaveAgentSettings">
              {{ t("settings.saveAgent") }}
            </Button>
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
