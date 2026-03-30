<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { useMcpStore, type McpServerItem } from "@/stores/mcp";

const mcpStore = useMcpStore();
const { activeServer, activeToolCount, enabledCount, remoteCount, servers, stdioCount } = storeToRefs(mcpStore);
const { t } = useI18n();

const filter = ref<"all" | "enabled" | "stdio" | "remote">("all");
const search = ref("");
const feedback = ref("");

const form = reactive({
  name: "",
  transport: "stdio",
  command: "",
  argumentsJson: "[]",
  url: "",
  headersJson: "{}",
  environmentJson: "{}",
  enabled: true
});

const filteredServers = computed(() => {
  const query = search.value.trim().toLowerCase();

  return servers.value.filter((server) => {
    const matchesFilter =
      filter.value === "all"
        ? true
        : filter.value === "enabled"
          ? server.enabled
          : filter.value === "stdio"
            ? server.transport === "stdio"
            : server.transport !== "stdio";

    if (!matchesFilter) {
      return false;
    }

    if (!query) {
      return true;
    }

    const haystack = `${server.name} ${server.transport} ${server.command} ${server.url}`.toLowerCase();
    return haystack.includes(query);
  });
});

const activeTestReport = computed(() => mcpStore.lastTestReport);
const activeToolDiscovery = computed(() => {
  const discovery = mcpStore.lastToolDiscovery;
  if (!discovery || !activeServer.value || discovery.server.id !== activeServer.value.id) {
    return null;
  }

  return discovery;
});

watch(
  activeServer,
  (server) => {
    if (!server) {
      resetForm();
      return;
    }

    form.name = server.name;
    form.transport = server.transport;
    form.command = server.command;
    form.argumentsJson = server.argumentsJson;
    form.url = server.url;
    form.headersJson = server.headersJson;
    form.environmentJson = server.environmentJson;
    form.enabled = server.enabled;
  },
  { immediate: true }
);

onMounted(async () => {
  if (!mcpStore.loaded) {
    try {
      await mcpStore.bootstrap();
    } catch {
      // store error is rendered below
    }
  }
});

function resetForm() {
  form.name = "";
  form.transport = "stdio";
  form.command = "";
  form.argumentsJson = "[]";
  form.url = "";
  form.headersJson = "{}";
  form.environmentJson = "{}";
  form.enabled = true;
}

function handleNewServer() {
  mcpStore.clearActiveServer();
  resetForm();
  feedback.value = t("mcp.feedback.readyForNewServer");
}

function resolveTransportLabel(server: McpServerItem) {
  return server.transport === "stdio" ? t("mcp.transportStdio") : server.transport === "sse" ? t("mcp.transportSse") : t("mcp.transportStreamableHttp");
}

function resolveTarget(server: McpServerItem) {
  return server.transport === "stdio" ? server.command || t("mcp.none") : server.url || t("mcp.none");
}

function resolveStatusLabel(server: McpServerItem) {
  return server.enabled ? t("mcp.enabled") : t("mcp.disabled");
}

async function handleSubmit() {
  feedback.value = "";

  const payload = {
    name: form.name,
    transport: form.transport,
    command: form.command,
    arguments_json: form.argumentsJson,
    url: form.url,
    headers_json: form.headersJson,
    environment_json: form.environmentJson,
    enabled: form.enabled
  };

  try {
    if (activeServer.value) {
      await mcpStore.updateServer(activeServer.value.id, payload);
      feedback.value = t("mcp.feedback.updated", { name: form.name });
      return;
    }

    const server = await mcpStore.createServer(payload);
    feedback.value = t("mcp.feedback.created", { name: server.name });
  } catch {
    feedback.value = activeServer.value ? t("mcp.feedback.updateFailed") : t("mcp.feedback.createFailed");
  }
}

function handleSelectServer(serverId: string) {
  mcpStore.setActiveServer(serverId);
  feedback.value = "";
}

async function handleDeleteServer(serverId: string) {
  const server = servers.value.find((item) => item.id === serverId);
  if (!server) {
    return;
  }

  const confirmed = window.confirm(t("mcp.prompts.deleteServer", { name: server.name }));
  if (!confirmed) {
    return;
  }

  try {
    await mcpStore.deleteServer(serverId);
    feedback.value = t("mcp.feedback.deleted", { name: server.name });
  } catch {
    feedback.value = t("mcp.feedback.deleteFailed");
  }
}

async function handleTestServer(serverId: string) {
  try {
    const result = await mcpStore.testServer(serverId);
    feedback.value = result.report.ok ? t("mcp.feedback.testPassed") : t("mcp.feedback.testFailed");
  } catch {
    feedback.value = t("mcp.feedback.testFailed");
  }
}

async function handleDiscoverTools(serverId: string) {
  try {
    const result = await mcpStore.discoverServerTools(serverId);
    feedback.value = t("mcp.feedback.discoveredTools", { count: result.tools.length, name: result.server.name });
  } catch {
    feedback.value = t("mcp.feedback.discoverFailed");
  }
}
</script>

<template>
  <div class="stack mcp-page">
    <section class="panel mcp-hero">
      <div class="stack" style="gap: 8px; max-width: 760px;">
        <strong>{{ t("mcp.workspaceTitle") }}</strong>
        <p class="muted mcp-hero__copy">{{ t("mcp.workspaceDescription") }}</p>
      </div>
      <div class="mcp-summary-grid">
        <button class="summary-card" type="button" :data-active="filter === 'all'" @click="filter = 'all'">
          <strong>{{ servers.length }}</strong>
          <span class="muted">{{ t("mcp.summaryAll") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'enabled'" @click="filter = 'enabled'">
          <strong>{{ enabledCount }}</strong>
          <span class="muted">{{ t("mcp.summaryEnabled") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'stdio'" @click="filter = 'stdio'">
          <strong>{{ stdioCount }}</strong>
          <span class="muted">{{ t("mcp.summaryStdio") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'remote'" @click="filter = 'remote'">
          <strong>{{ remoteCount }}</strong>
          <span class="muted">{{ t("mcp.summaryRemote") }}</span>
        </button>
      </div>
    </section>

    <section class="mcp-layout">
      <section class="panel mcp-editor">
        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 12px;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ activeServer ? t("mcp.editing", { name: activeServer.name }) : t("mcp.newServer") }}</strong>
            <span class="muted">{{ t("mcp.editorDescription") }}</span>
          </div>
          <Button variant="secondary" :disabled="mcpStore.isSaving || mcpStore.isTesting || mcpStore.isDiscovering" @click="handleNewServer">{{ t("mcp.newDraft") }}</Button>
        </div>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("mcp.serverName") }}</span>
          <input v-model="form.name" class="field" :placeholder="t('mcp.serverNamePlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("mcp.transport") }}</span>
          <select v-model="form.transport" class="field">
            <option value="stdio">{{ t("mcp.transportStdio") }}</option>
            <option value="sse">{{ t("mcp.transportSse") }}</option>
            <option value="streamable_http">{{ t("mcp.transportStreamableHttp") }}</option>
          </select>
          <span class="muted settings-field__hint">{{ t("mcp.transportHint") }}</span>
        </label>

        <template v-if="form.transport === 'stdio'">
          <label class="settings-field">
            <span class="settings-field__label">{{ t("mcp.command") }}</span>
            <input v-model="form.command" class="field" :placeholder="t('mcp.commandPlaceholder')" />
          </label>

          <label class="settings-field">
            <span class="settings-field__label">{{ t("mcp.argumentsJson") }}</span>
            <textarea v-model="form.argumentsJson" class="field mcp-jsonarea" :placeholder="t('mcp.argumentsJsonPlaceholder')" />
            <span class="muted settings-field__hint">{{ t("mcp.argumentsJsonHint") }}</span>
          </label>

          <label class="settings-field">
            <span class="settings-field__label">{{ t("mcp.environmentJson") }}</span>
            <textarea v-model="form.environmentJson" class="field mcp-jsonarea" :placeholder="t('mcp.environmentJsonPlaceholder')" />
            <span class="muted settings-field__hint">{{ t("mcp.environmentJsonHint") }}</span>
          </label>
        </template>

        <template v-else>
          <label class="settings-field">
            <span class="settings-field__label">{{ t("mcp.url") }}</span>
            <input v-model="form.url" class="field" :placeholder="t('mcp.urlPlaceholder')" />
          </label>

          <label class="settings-field">
            <span class="settings-field__label">{{ t("mcp.headersJson") }}</span>
            <textarea v-model="form.headersJson" class="field mcp-jsonarea" :placeholder="t('mcp.headersJsonPlaceholder')" />
            <span class="muted settings-field__hint">{{ t("mcp.headersJsonHint") }}</span>
          </label>
        </template>

        <label class="projects-checkbox">
          <input v-model="form.enabled" type="checkbox" />
          <span>{{ t("mcp.enabledToggle") }}</span>
        </label>

        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 12px; margin-top: 8px;">
          <div class="stack" style="gap: 6px; max-width: 560px;">
            <span v-if="feedback" class="muted">{{ feedback }}</span>
            <span v-if="mcpStore.error" class="settings-error">{{ mcpStore.error }}</span>
          </div>
          <div class="row settings-action-row">
            <Button variant="secondary" :disabled="mcpStore.isSaving || mcpStore.isTesting || mcpStore.isDiscovering" @click="resetForm">{{ t("mcp.resetForm") }}</Button>
            <Button :disabled="mcpStore.isSaving || mcpStore.isTesting || mcpStore.isDiscovering" @click="handleSubmit">
              {{ mcpStore.isSaving ? t("mcp.saving") : activeServer ? t("mcp.saveServer") : t("mcp.createServer") }}
            </Button>
          </div>
        </div>
      </section>

      <section class="panel mcp-board">
        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 12px;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ t("mcp.boardTitle") }}</strong>
            <span class="muted">{{ t("mcp.boardDescription") }}</span>
          </div>
          <input v-model="search" class="field mcp-search" :placeholder="t('mcp.searchPlaceholder')" />
        </div>

        <div v-if="mcpStore.isLoading" class="empty-state">
          <strong>{{ t("mcp.loadingTitle") }}</strong>
          <span class="muted">{{ t("mcp.loadingDescription") }}</span>
        </div>

        <div v-else-if="filteredServers.length === 0" class="empty-state">
          <strong>{{ t("mcp.emptyTitle") }}</strong>
          <span class="muted">{{ t("mcp.emptyDescription") }}</span>
        </div>

        <div v-else class="mcp-list">
          <article
            v-for="server in filteredServers"
            :key="server.id"
            class="mcp-card"
            :data-active="server.id === mcpStore.activeServerId"
            @click="handleSelectServer(server.id)"
          >
            <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px;">
              <div class="stack" style="gap: 6px; min-width: 0;">
                <div class="row" style="align-items: center; gap: 8px; flex-wrap: wrap;">
                  <strong>{{ server.name }}</strong>
                  <span class="project-badge">{{ resolveTransportLabel(server) }}</span>
                  <span class="project-badge" :data-archived="!server.enabled">{{ resolveStatusLabel(server) }}</span>
                </div>
                <p class="mcp-card__target">{{ resolveTarget(server) }}</p>
                <p v-if="server.lastTestMessage" class="mcp-card__note">{{ server.lastTestMessage }}</p>
              </div>
            </div>

            <div class="row mcp-card__meta">
              <span class="muted">{{ t("mcp.updatedAt", { value: formatTimestamp(server.updatedAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) }) }}</span>
              <span class="muted">{{ t("mcp.lastTestedAt", { value: server.lastTestedAt ? formatTimestamp(server.lastTestedAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) : t('mcp.none') }) }}</span>
            </div>

            <div class="row settings-action-row" style="justify-content: flex-end;">
              <Button variant="secondary" :disabled="mcpStore.isSaving || mcpStore.isTesting || mcpStore.isDiscovering" @click.stop="handleTestServer(server.id)">
                {{ mcpStore.isTesting && mcpStore.activeServerId === server.id ? t("mcp.testing") : t("mcp.testConnection") }}
              </Button>
              <Button variant="ghost" :disabled="mcpStore.isSaving || mcpStore.isTesting || mcpStore.isDiscovering" @click.stop="handleDeleteServer(server.id)">
                {{ t("mcp.delete") }}
              </Button>
            </div>
          </article>
        </div>
      </section>
    </section>

    <section class="panel mcp-test-panel">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
        <div class="stack" style="gap: 4px;">
          <strong>{{ activeServer ? t("mcp.testTitle", { name: activeServer.name }) : t("mcp.testFallback") }}</strong>
          <span class="muted">{{ t("mcp.testDescription") }}</span>
        </div>
        <Button
          variant="secondary"
          :disabled="!activeServer || mcpStore.isSaving || mcpStore.isTesting || mcpStore.isDiscovering"
          @click="activeServer && handleTestServer(activeServer.id)"
        >
          {{ mcpStore.isTesting ? t("mcp.testing") : t("mcp.testConnection") }}
        </Button>
      </div>

      <div v-if="!activeServer" class="empty-state">
        <strong>{{ t("mcp.selectServerTitle") }}</strong>
        <span class="muted">{{ t("mcp.selectServerDescription") }}</span>
      </div>

      <div v-else-if="!activeTestReport && !activeServer.lastTestedAt" class="empty-state">
        <strong>{{ t("mcp.noTestTitle") }}</strong>
        <span class="muted">{{ t("mcp.noTestDescription") }}</span>
      </div>

      <div v-else class="settings-test-card" :data-ok="activeTestReport?.ok ?? activeServer?.lastTestStatus === 'success'">
        <strong>{{ (activeTestReport?.ok ?? activeServer?.lastTestStatus === 'success') ? t("mcp.testOk") : t("mcp.testError") }}</strong>
        <p class="settings-test-card__message">{{ activeTestReport?.message ?? activeServer?.lastTestMessage }}</p>
        <span class="muted">{{ t("mcp.lastTestedAt", { value: formatTimestamp(activeTestReport?.checkedAt ?? activeServer?.lastTestedAt ?? '', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
        <div v-if="activeTestReport?.details" class="code-block">{{ activeTestReport.details }}</div>
      </div>
    </section>

    <section class="panel mcp-tools-panel">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
        <div class="stack" style="gap: 4px;">
          <strong>{{ activeServer ? t("mcp.toolsTitle", { name: activeServer.name }) : t("mcp.toolsFallback") }}</strong>
          <span class="muted">{{ t("mcp.toolsDescription") }}</span>
        </div>
        <Button
          variant="secondary"
          :disabled="!activeServer || mcpStore.isSaving || mcpStore.isTesting || mcpStore.isDiscovering"
          @click="activeServer && handleDiscoverTools(activeServer.id)"
        >
          {{ mcpStore.isDiscovering ? t("mcp.discoveringTools") : t("mcp.discoverTools") }}
        </Button>
      </div>

      <div v-if="!activeServer" class="empty-state">
        <strong>{{ t("mcp.selectServerTitle") }}</strong>
        <span class="muted">{{ t("mcp.selectServerDescription") }}</span>
      </div>

      <div v-else-if="!activeToolDiscovery" class="empty-state">
        <strong>{{ t("mcp.noToolsTitle") }}</strong>
        <span class="muted">{{ t("mcp.noToolsDescription") }}</span>
      </div>

      <div v-else class="stack" style="gap: 14px;">
        <div class="row" style="justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap;">
          <strong>{{ t("mcp.discoveredToolsCount", { count: activeToolCount }) }}</strong>
          <span class="muted">{{ t("mcp.discoveredAt", { value: formatTimestamp(activeToolDiscovery.discoveredAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }) }}</span>
        </div>

        <div v-if="activeToolDiscovery.tools.length === 0" class="empty-state mcp-tools-empty">
          <strong>{{ t("mcp.emptyToolsTitle") }}</strong>
          <span class="muted">{{ t("mcp.emptyToolsDescription") }}</span>
        </div>

        <div v-else class="mcp-tool-list">
          <article v-for="tool in activeToolDiscovery.tools" :key="tool.fullName" class="mcp-tool-card">
            <div class="stack" style="gap: 6px;">
              <div class="row" style="align-items: center; gap: 8px; flex-wrap: wrap;">
                <strong>{{ tool.toolName }}</strong>
                <span class="project-badge">{{ tool.serverName }}</span>
              </div>
              <span class="muted">{{ tool.fullName }}</span>
              <p class="mcp-card__note">{{ tool.description }}</p>
            </div>
            <div class="stack" style="gap: 6px;">
              <span class="muted">{{ t("mcp.inputSchema") }}</span>
              <pre class="code-block mcp-schema-block">{{ tool.inputSchemaJson }}</pre>
            </div>
          </article>
        </div>
      </div>
    </section>
  </div>
</template>
