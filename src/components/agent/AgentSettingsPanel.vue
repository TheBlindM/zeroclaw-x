<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { pickRuntimeWorkspace, type RuntimeSettingsRecord } from "@/api/tauri";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { defaultRuntimeSettings, useSettingsStore } from "@/stores/settings";

type AgentFormState = Pick<RuntimeSettingsRecord, "agent" | "autonomy">;

const settingsStore = useSettingsStore();
const { t } = useI18n();
const saveMessage = ref("");
const showAutonomyAdvanced = ref(false);
const form = reactive(createAgentFormState());

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

function cloneRuntimeSettings(runtime: RuntimeSettingsRecord): RuntimeSettingsRecord {
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
    agent: {
      ...runtime.agent
    },
    autonomy: cloneAutonomy(runtime.autonomy)
  };
}

function applyAgentForm(runtime: RuntimeSettingsRecord) {
  const normalized = cloneRuntimeSettings(runtime);
  Object.assign(form.agent, normalized.agent);
  Object.assign(form.autonomy, cloneAutonomy(normalized.autonomy));
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

function resetAgentForm() {
  applyAgentForm(settingsStore.runtime);
  saveMessage.value = t("settings.feedback.agentReset");
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

function splitDelimitedList(value: string) {
  return value
    .split(/\r?\n|,/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}
</script>

<template>
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
