<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  pickRuntimeWorkspace,
  type RuntimeDelegateAgentRecord,
  type RuntimeProviderEntryRecord,
  type RuntimeProviderGroupRecord,
  type RuntimeSettingsRecord
} from "@/api/tauri";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import {
  defaultRuntimeDelegateAgent,
  defaultRuntimeSettings,
  useSettingsStore
} from "@/stores/settings";

type AgentFormState = Pick<RuntimeSettingsRecord, "delegate" | "agents" | "agent" | "autonomy">;
type AgentTemplateKey = "researcher" | "coder" | "reviewer";

const settingsStore = useSettingsStore();
const { t } = useI18n();
const saveMessage = ref("");
const showAutonomyAdvanced = ref(false);
const selectedSubAgentIndex = ref(0);
const form = reactive(createAgentFormState());

const runtimeGroups = computed(() => cloneRuntimeSettings(settingsStore.runtime).groups);
const workspaceSummary = computed(
  () => form.agent.workspace_dir.trim() || settingsStore.status.workspace_dir || t("settings.workspaceDirectoryDefault")
);
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
const currentSubAgent = computed(() => form.agents[selectedSubAgentIndex.value] ?? null);
const mainAgentRuntimeEntries = computed(() => runtimeEntriesFor(form.agent.runtime_group_id));
const currentSubAgentRuntimeEntries = computed(() =>
  currentSubAgent.value ? runtimeEntriesFor(currentSubAgent.value.runtime_group_id) : []
);
const mainAgentRuntimeSummary = computed(() => {
  const entry = resolveRuntimeEntry(form.agent.runtime_group_id, form.agent.runtime_entry_id);
  if (!entry) {
    return t("settings.runtimeSelectionMissing");
  }

  return `${entry.provider} / ${entry.model}`;
});
const subAgentSummary = computed(() => {
  if (!form.agents.length) {
    return t("settings.subAgentsSummaryEmpty");
  }

  const enabledCount = form.agents.filter((agent) => agent.enabled).length;
  return t("settings.subAgentsSummaryStructured", {
    total: form.agents.length,
    enabled: enabledCount
  });
});
const agentSummary = computed(() =>
  t("settings.agentWorkspaceSummary", {
    workspace: workspaceSummary.value,
    autonomy: t(`settings.autonomyLevels.${form.autonomy.level}`),
    dispatcher: t(`settings.toolDispatchers.${form.agent.tool_dispatcher}`)
  })
);
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
const currentAllowedToolsText = computed({
  get: () => currentSubAgent.value?.allowed_tools.join(", ") ?? "",
  set: (value: string) => {
    if (!currentSubAgent.value) {
      return;
    }

    currentSubAgent.value.allowed_tools = splitDelimitedList(value);
  }
});

watch(
  () => settingsStore.runtime,
  (runtime) => {
    applyAgentForm(runtime);
  },
  {
    deep: true,
    immediate: true
  }
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

onMounted(async () => {
  try {
    if (!settingsStore.loaded || !settingsStore.proxyLoaded) {
      await settingsStore.bootstrap();
      return;
    }

    if (!settingsStore.statusLoaded) {
      await settingsStore.refreshStatus();
    }
  } catch (error) {
    console.error("Failed to bootstrap agent settings", error);
  }
});

function createAgentFormState(): AgentFormState {
  const defaults = defaultRuntimeSettings();
  return {
    delegate: {
      ...defaults.delegate
    },
    agents: defaults.agents.map(cloneDelegateAgent),
    agent: {
      ...defaults.agent
    },
    autonomy: cloneAutonomy(defaults.autonomy)
  };
}

function cloneAutonomy(autonomy: RuntimeSettingsRecord["autonomy"]): RuntimeSettingsRecord["autonomy"] {
  return {
    ...autonomy,
    allowed_commands: [...autonomy.allowed_commands],
    allowed_roots: [...autonomy.allowed_roots],
    shell_env_passthrough: [...autonomy.shell_env_passthrough],
    auto_approve: [...autonomy.auto_approve],
    always_ask: [...autonomy.always_ask]
  };
}

function cloneDelegateAgent(agent: RuntimeDelegateAgentRecord): RuntimeDelegateAgentRecord {
  const defaults = defaultRuntimeDelegateAgent();
  return {
    ...defaults,
    ...agent,
    enabled: agent.enabled ?? defaults.enabled,
    runtime_group_id: agent.runtime_group_id ?? defaults.runtime_group_id,
    runtime_entry_id: agent.runtime_entry_id ?? defaults.runtime_entry_id,
    allowed_tools: [...(agent.allowed_tools ?? defaults.allowed_tools)]
  };
}

function cloneRuntimeSettings(runtime: RuntimeSettingsRecord): RuntimeSettingsRecord {
  const defaults = defaultRuntimeSettings();
  return {
    ...runtime,
    groups: runtime.groups.map((group) => ({
      ...group,
      entries: group.entries.map((entry) => ({ ...entry }))
    })),
    entries: runtime.entries.map((entry) => ({ ...entry })),
    proxy: {
      ...runtime.proxy,
      no_proxy: [...runtime.proxy.no_proxy],
      services: [...runtime.proxy.services]
    },
    delegate: {
      ...defaults.delegate,
      ...(runtime.delegate ?? defaults.delegate)
    },
    agents: (runtime.agents ?? defaults.agents).map(cloneDelegateAgent),
    agent: {
      ...defaults.agent,
      ...(runtime.agent ?? defaults.agent)
    },
    autonomy: cloneAutonomy(runtime.autonomy)
  };
}

function applyAgentForm(runtime: RuntimeSettingsRecord) {
  const normalized = cloneRuntimeSettings(runtime);
  Object.assign(form.delegate, normalized.delegate);
  form.agents = normalized.agents.map(cloneDelegateAgent);
  Object.assign(form.agent, normalized.agent);
  Object.assign(form.autonomy, cloneAutonomy(normalized.autonomy));
  syncMainAgentBinding();
  form.agents.forEach((agent) => syncSubAgentBinding(agent));
  ensureSubAgentSelection();
}

function buildRuntimeAgentPayload() {
  const persisted = cloneRuntimeSettings(settingsStore.runtime);
  return cloneRuntimeSettings({
    ...persisted,
    delegate: {
      timeout_secs: Number(form.delegate.timeout_secs),
      agentic_timeout_secs: Number(form.delegate.agentic_timeout_secs)
    },
    agents: form.agents.map((agent) => cloneDelegateAgent(agent)),
    agent: {
      enabled: form.agent.enabled,
      runtime_group_id: form.agent.runtime_group_id,
      runtime_entry_id: form.agent.runtime_entry_id,
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

function resetAgentForm() {
  applyAgentForm(settingsStore.runtime);
  saveMessage.value = t("settings.feedback.agentReset");
}

async function handleSaveAgentSettings() {
  saveMessage.value = "";

  try {
    validateSubAgents();
    await settingsStore.save(buildRuntimeAgentPayload());
    saveMessage.value = t("settings.feedback.agentSaved");
  } catch (error) {
    saveMessage.value = error instanceof Error ? error.message : t("settings.feedback.agentSaveFailed");
  }
}

function createAgentTemplate(kind: AgentTemplateKey): RuntimeDelegateAgentRecord {
  switch (kind) {
    case "coder":
      return defaultRuntimeDelegateAgent({
        name: "coder",
        system_prompt: "You write small, correct patches and explain the outcome briefly.",
        allowed_tools: ["file_read", "file_write", "file_edit", "shell"],
        max_iterations: 10,
        memory_namespace: "coding"
      });
    case "reviewer":
      return defaultRuntimeDelegateAgent({
        name: "reviewer",
        system_prompt: "You review changes for bugs, regressions, and missing tests.",
        allowed_tools: ["file_read", "glob_search", "content_search"],
        memory_namespace: "review"
      });
    default:
      return defaultRuntimeDelegateAgent({
        name: "researcher",
        system_prompt: "You gather evidence, inspect context, and summarize findings clearly.",
        allowed_tools: ["file_read", "glob_search", "content_search"],
        memory_namespace: "research"
      });
  }
}

function handleAddSubAgent(kind?: AgentTemplateKey) {
  const template = kind ? createAgentTemplate(kind) : defaultRuntimeDelegateAgent();
  const next = cloneDelegateAgent({
    ...template,
    name: template.name ? makeUniqueSubAgentName(template.name) : makeUniqueSubAgentName("agent")
  });
  syncSubAgentBinding(next);
  form.agents = [...form.agents, next];
  selectedSubAgentIndex.value = form.agents.length - 1;
  saveMessage.value = kind
    ? t("settings.feedback.subAgentTemplateAdded", { name: next.name })
    : t("settings.feedback.subAgentCreated", { name: next.name });
}

function handleRemoveSubAgent(index: number) {
  const target = form.agents[index];
  if (!target) {
    return;
  }

  form.agents = form.agents.filter((_, currentIndex) => currentIndex !== index);
  ensureSubAgentSelection();
  saveMessage.value = t("settings.feedback.subAgentRemoved", { name: target.name || t("settings.subAgentFallbackName") });
}

function handleSelectSubAgent(index: number) {
  selectedSubAgentIndex.value = index;
}

function handleMainAgentRuntimeGroupChange(event: Event) {
  const groupId = (event.target as HTMLSelectElement).value;
  form.agent.runtime_group_id = groupId;
  const entry = runtimeEntriesFor(groupId)[0];
  form.agent.runtime_entry_id = entry?.id ?? "";
}

function handleSubAgentRuntimeGroupChange(event: Event) {
  if (!currentSubAgent.value) {
    return;
  }

  const groupId = (event.target as HTMLSelectElement).value;
  currentSubAgent.value.runtime_group_id = groupId;
  const entry = runtimeEntriesFor(groupId)[0];
  currentSubAgent.value.runtime_entry_id = entry?.id ?? "";
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

function ensureSubAgentSelection() {
  if (!form.agents.length) {
    selectedSubAgentIndex.value = 0;
    return;
  }

  if (selectedSubAgentIndex.value < 0 || selectedSubAgentIndex.value >= form.agents.length) {
    selectedSubAgentIndex.value = 0;
  }
}

function runtimeEntriesFor(groupId: string) {
  return runtimeGroups.value.find((group) => group.id === groupId)?.entries ?? runtimeGroups.value[0]?.entries ?? [];
}

function resolveRuntimeEntry(groupId: string, entryId: string) {
  const group = runtimeGroups.value.find((item) => item.id === groupId) ?? runtimeGroups.value[0];
  if (!group) {
    return null;
  }

  return (
    group.entries.find((entry) => entry.id === entryId) ??
    group.entries.find((entry) => entry.id === group.active_entry_id) ??
    group.entries[0] ??
    null
  );
}

function resolveRuntimeEntryLabel(entry: RuntimeProviderEntryRecord | null | undefined) {
  if (!entry) {
    return t("settings.runtimeSelectionMissing");
  }

  return entry.name?.trim() || `${entry.provider} / ${entry.model}`;
}

function resolveRuntimeGroupLabel(group: RuntimeProviderGroupRecord) {
  return group.name?.trim() || group.id;
}

function resolveSubAgentRuntimeSummary(agent: RuntimeDelegateAgentRecord) {
  const entry = resolveRuntimeEntry(agent.runtime_group_id, agent.runtime_entry_id);
  return resolveRuntimeEntryLabel(entry);
}

function syncMainAgentBinding() {
  const group = runtimeGroups.value.find((item) => item.id === form.agent.runtime_group_id) ?? runtimeGroups.value[0];
  if (!group) {
    form.agent.runtime_group_id = "";
    form.agent.runtime_entry_id = "";
    return;
  }

  form.agent.runtime_group_id = group.id;
  form.agent.runtime_entry_id =
    group.entries.find((entry) => entry.id === form.agent.runtime_entry_id)?.id ??
    group.entries.find((entry) => entry.id === group.active_entry_id)?.id ??
    group.entries[0]?.id ??
    "";
}

function syncSubAgentBinding(agent: RuntimeDelegateAgentRecord) {
  const group = runtimeGroups.value.find((item) => item.id === agent.runtime_group_id) ?? runtimeGroups.value[0];
  if (!group) {
    agent.runtime_group_id = "";
    agent.runtime_entry_id = "";
    return;
  }

  agent.runtime_group_id = group.id;
  agent.runtime_entry_id =
    group.entries.find((entry) => entry.id === agent.runtime_entry_id)?.id ??
    group.entries.find((entry) => entry.id === group.active_entry_id)?.id ??
    group.entries[0]?.id ??
    "";
}

function makeUniqueSubAgentName(baseName: string) {
  const normalizedBase = baseName.trim() || "agent";
  let candidate = normalizedBase;
  let suffix = 2;
  const existing = new Set(form.agents.map((agent) => agent.name));

  while (existing.has(candidate)) {
    candidate = `${normalizedBase}-${suffix}`;
    suffix += 1;
  }

  return candidate;
}

function validateSubAgents() {
  const names = new Set<string>();

  for (const agent of form.agents) {
    const name = agent.name.trim();
    if (!name) {
      throw new Error(t("settings.subAgentsNameRequired", { index: form.agents.indexOf(agent) + 1 }));
    }

    if (names.has(name)) {
      throw new Error(t("settings.subAgentsDuplicateName", { name }));
    }

    names.add(name);
  }
}

function splitDelimitedList(value: string) {
  return value
    .split(/\r?\n|,/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}
</script>

<template>
  <section class="panel settings-panel">
    <div class="stack" style="gap: 14px;">
      <div class="stack" style="gap: 6px;">
        <strong>{{ t("settings.mainAgentTitle") }}</strong>
        <span class="muted">{{ t("settings.mainAgentDescription") }}</span>
        <span class="settings-context-note">{{ agentSummary }}</span>
      </div>

      <div class="settings-grid">
        <label class="settings-field">
          <span class="settings-field__label">{{ t("settings.agentEnabled") }}</span>
          <label class="settings-checkbox">
            <input v-model="form.agent.enabled" type="checkbox" />
            <span>{{ t("settings.mainAgentEnabledHint") }}</span>
          </label>
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("settings.runtimeGroup") }}</span>
          <select v-model="form.agent.runtime_group_id" class="select" @change="handleMainAgentRuntimeGroupChange">
            <option v-for="group in runtimeGroups" :key="group.id" :value="group.id">{{ resolveRuntimeGroupLabel(group) }}</option>
          </select>
          <span class="muted settings-field__hint">{{ t("settings.runtimeGroupHint") }}</span>
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("settings.runtimeEntry") }}</span>
          <select v-model="form.agent.runtime_entry_id" class="select">
            <option v-for="entry in mainAgentRuntimeEntries" :key="entry.id" :value="entry.id">{{ resolveRuntimeEntryLabel(entry) }}</option>
          </select>
          <span class="muted settings-field__hint">{{ mainAgentRuntimeSummary }}</span>
        </label>

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
        {{
          t("settings.runtimePolicySummary", {
            workspace: settingsStore.status.workspace_dir || workspaceSummary,
            autonomy: t(`settings.autonomyLevels.${settingsStore.status.autonomy_level}`),
            dispatcher: t(`settings.toolDispatchers.${settingsStore.status.tool_dispatcher}`)
          })
        }}
      </div>
    </div>
  </section>

  <section class="panel settings-panel">
    <div class="stack" style="gap: 14px;">
      <div class="stack" style="gap: 6px;">
        <strong>{{ t("settings.subAgentsTitle") }}</strong>
        <span class="muted">{{ t("settings.subAgentsDescriptionStructured") }}</span>
        <span class="settings-context-note">{{ subAgentSummary }}</span>
      </div>

      <div class="settings-grid">
        <label class="settings-field">
          <span class="settings-field__label">{{ t("settings.delegateTimeout") }}</span>
          <input v-model.number="form.delegate.timeout_secs" class="field" type="number" min="1" max="3600" step="1" />
          <span class="muted settings-field__hint">{{ t("settings.delegateTimeoutHint") }}</span>
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("settings.delegateAgenticTimeout") }}</span>
          <input v-model.number="form.delegate.agentic_timeout_secs" class="field" type="number" min="1" max="7200" step="1" />
          <span class="muted settings-field__hint">{{ t("settings.delegateAgenticTimeoutHint") }}</span>
        </label>

        <div class="settings-field settings-field--wide stack" style="gap: 10px;">
          <span class="settings-field__label">{{ t("settings.subAgentTemplates") }}</span>
          <div class="suggestion-row">
            <button class="suggestion-chip" type="button" @click="handleAddSubAgent('researcher')">
              <span class="suggestion-chip__label">{{ t("settings.subAgentTemplateLabels.researcher") }}</span>
              <span class="suggestion-chip__meta">{{ t("settings.suggested") }}</span>
            </button>
            <button class="suggestion-chip" type="button" @click="handleAddSubAgent('coder')">
              <span class="suggestion-chip__label">{{ t("settings.subAgentTemplateLabels.coder") }}</span>
              <span class="suggestion-chip__meta">{{ t("settings.suggested") }}</span>
            </button>
            <button class="suggestion-chip" type="button" @click="handleAddSubAgent('reviewer')">
              <span class="suggestion-chip__label">{{ t("settings.subAgentTemplateLabels.reviewer") }}</span>
              <span class="suggestion-chip__meta">{{ t("settings.suggested") }}</span>
            </button>
            <button class="suggestion-chip" type="button" @click="handleAddSubAgent()">
              <span class="suggestion-chip__label">{{ t("settings.createSubAgent") }}</span>
              <span class="suggestion-chip__meta">{{ t("settings.emptyTemplate") }}</span>
            </button>
          </div>
          <span class="muted settings-field__hint">{{ t("settings.subAgentTemplatesStructuredHint") }}</span>
        </div>
      </div>

      <div class="agent-layout">
        <div class="agent-list">
          <button
            v-for="(agent, index) in form.agents"
            :key="`${agent.name}-${index}`"
            class="agent-card"
            :data-active="index === selectedSubAgentIndex"
            type="button"
            @click="handleSelectSubAgent(index)"
          >
            <div class="stack" style="gap: 6px;">
              <div class="row" style="justify-content: space-between; gap: 12px; align-items: center;">
                <strong>{{ agent.name || t("settings.subAgentFallbackName") }}</strong>
                <span class="profile-inline-badge">{{ agent.enabled ? t("settings.enabled") : t("settings.disabled") }}</span>
              </div>
              <span class="muted">{{ resolveSubAgentRuntimeSummary(agent) }}</span>
              <span class="muted">
                {{ agent.agentic ? t("settings.subAgentAgenticEnabled") : t("settings.subAgentAgenticDisabled") }}
                · {{ t("settings.subAgentAllowedToolsCount", { count: agent.allowed_tools.length }) }}
              </span>
            </div>
          </button>
        </div>

        <div class="agent-editor panel" v-if="currentSubAgent">
          <div class="stack" style="gap: 14px;">
            <div class="row" style="justify-content: space-between; gap: 16px; align-items: flex-start; flex-wrap: wrap;">
              <div class="stack" style="gap: 4px;">
                <strong>{{ currentSubAgent.name || t("settings.subAgentFallbackName") }}</strong>
                <span class="muted">{{ resolveSubAgentRuntimeSummary(currentSubAgent) }}</span>
              </div>
              <Button variant="ghost" @click="handleRemoveSubAgent(selectedSubAgentIndex)">{{ t("settings.delete") }}</Button>
            </div>

            <div class="settings-grid">
              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.agentEnabled") }}</span>
                <label class="settings-checkbox">
                  <input v-model="currentSubAgent.enabled" type="checkbox" />
                  <span>{{ t("settings.subAgentEnabledHint") }}</span>
                </label>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.subAgentName") }}</span>
                <input v-model="currentSubAgent.name" class="field" :placeholder="t('settings.subAgentNamePlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.subAgentNameHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.runtimeGroup") }}</span>
                <select v-model="currentSubAgent.runtime_group_id" class="select" @change="handleSubAgentRuntimeGroupChange">
                  <option v-for="group in runtimeGroups" :key="group.id" :value="group.id">{{ resolveRuntimeGroupLabel(group) }}</option>
                </select>
                <span class="muted settings-field__hint">{{ t("settings.runtimeGroupHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.runtimeEntry") }}</span>
                <select v-model="currentSubAgent.runtime_entry_id" class="select">
                  <option v-for="entry in currentSubAgentRuntimeEntries" :key="entry.id" :value="entry.id">{{ resolveRuntimeEntryLabel(entry) }}</option>
                </select>
                <span class="muted settings-field__hint">{{ resolveSubAgentRuntimeSummary(currentSubAgent) }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.subAgentAgentic") }}</span>
                <label class="settings-checkbox">
                  <input v-model="currentSubAgent.agentic" type="checkbox" />
                  <span>{{ t("settings.subAgentAgenticHint") }}</span>
                </label>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.maxDepth") }}</span>
                <input v-model.number="currentSubAgent.max_depth" class="field" type="number" min="1" max="8" step="1" />
                <span class="muted settings-field__hint">{{ t("settings.maxDepthHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.maxIterations") }}</span>
                <input v-model.number="currentSubAgent.max_iterations" class="field" type="number" min="1" max="50" step="1" />
                <span class="muted settings-field__hint">{{ t("settings.maxIterationsHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.subAgentTimeout") }}</span>
                <input v-model.number="currentSubAgent.timeout_secs" class="field" type="number" min="1" max="3600" step="1" />
                <span class="muted settings-field__hint">{{ t("settings.subAgentTimeoutHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.subAgentAgenticTimeout") }}</span>
                <input v-model.number="currentSubAgent.agentic_timeout_secs" class="field" type="number" min="1" max="7200" step="1" />
                <span class="muted settings-field__hint">{{ t("settings.subAgentAgenticTimeoutHint") }}</span>
              </label>

              <label class="settings-field settings-field--wide">
                <span class="settings-field__label">{{ t("settings.allowedTools") }}</span>
                <input v-model="currentAllowedToolsText" class="field" :placeholder="t('settings.allowedToolsPlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.allowedToolsHint") }}</span>
              </label>

              <label class="settings-field settings-field--wide">
                <span class="settings-field__label">{{ t("settings.subAgentSystemPrompt") }}</span>
                <textarea v-model="currentSubAgent.system_prompt" class="textarea" rows="4" :placeholder="t('settings.subAgentSystemPromptPlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.subAgentSystemPromptHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.memoryNamespace") }}</span>
                <input v-model="currentSubAgent.memory_namespace" class="field" :placeholder="t('settings.memoryNamespacePlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.memoryNamespaceHint") }}</span>
              </label>

              <label class="settings-field">
                <span class="settings-field__label">{{ t("settings.skillsDirectory") }}</span>
                <input v-model="currentSubAgent.skills_directory" class="field" :placeholder="t('settings.skillsDirectoryPlaceholder')" />
                <span class="muted settings-field__hint">{{ t("settings.skillsDirectoryHint") }}</span>
              </label>
            </div>
          </div>
        </div>

        <div v-else class="panel agent-editor agent-editor--empty">
          <div class="stack" style="gap: 8px;">
            <strong>{{ t("settings.subAgentsEmptyStateTitle") }}</strong>
            <span class="muted">{{ t("settings.subAgentsEmptyStateDescription") }}</span>
          </div>
        </div>
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
        <span v-if="settingsStore.lastSavedAt" class="muted">
          {{
            t("settings.lastSavedAt", {
              value: formatTimestamp(settingsStore.lastSavedAt, {
                year: "numeric",
                month: "2-digit",
                day: "2-digit",
                hour: "2-digit",
                minute: "2-digit"
              })
            })
          }}
        </span>
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
