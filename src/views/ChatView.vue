<script setup lang="ts">
import { storeToRefs } from "pinia";
import { Bot, Pencil, Search, Settings2, ShieldCheck, Trash2, X } from "lucide-vue-next";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { listProjectKnowledge, sendMessage, stopMessage, type KnowledgeDocumentRecord } from "@/api/tauri";
import ChatComposer from "@/components/chat/ChatComposer.vue";
import ChatMessageList from "@/components/chat/ChatMessageList.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Button from "@/components/ui/Button.vue";
import { useChatStream } from "@/composables/useChatStream";
import { formatTimestamp } from "@/lib/datetime";
import { useChatStore } from "@/stores/chat";
import { useProjectsStore } from "@/stores/projects";
import { useSettingsStore } from "@/stores/settings";

interface HighlightSegment {
  text: string;
  match: boolean;
}

const chatStore = useChatStore();
const projectsStore = useProjectsStore();
const settingsStore = useSettingsStore();
const router = useRouter();
const { t } = useI18n();
const { activeMessages, activeSessionId, isBootstrapping, isStreaming, sessions } = storeToRefs(chatStore);
const { projects } = storeToRefs(projectsStore);
const { status, statusLoaded } = storeToRefs(settingsStore);
const sessionSearch = ref("");
const searchInputRef = ref<HTMLInputElement | null>(null);
const projectKnowledgeDocuments = ref<KnowledgeDocumentRecord[]>([]);
const isLoadingKnowledgeScope = ref(false);
const knowledgeScopeFeedback = ref("");
const approvalFeedback = ref("");

const activeSession = computed(() =>
  sessions.value.find((session) => session.id === activeSessionId.value) ?? null
);
const activeContextPreview = computed(() =>
  activeSessionId.value ? chatStore.contextPreviews[activeSessionId.value] ?? null : null
);
const activeKnowledgeScope = computed(() => {
  if (!activeSessionId.value) {
    return null;
  }

  return (
    chatStore.knowledgeScopes[activeSessionId.value] ?? {
      session_id: activeSessionId.value,
      mode: "auto",
      document_ids: []
    }
  );
});
const activePendingApprovals = computed(() =>
  activeSessionId.value ? chatStore.pendingApprovals[activeSessionId.value] ?? [] : []
);
const activeAgentMode = computed(() =>
  activeSessionId.value ? chatStore.agentModes[activeSessionId.value] ?? false : false
);
const selectedScopedDocumentIds = computed(() => new Set(activeKnowledgeScope.value?.document_ids ?? []));
const projectNameById = computed(() =>
  Object.fromEntries(projects.value.map((project) => [project.id, project.name])) as Record<string, string>
);

const normalizedSearch = computed(() => sessionSearch.value.trim().toLowerCase());
const filteredSessions = computed(() => {
  if (!normalizedSearch.value) {
    return sessions.value;
  }

  return sessions.value.filter((session) => {
    const haystack = [
      session.title,
      session.lastMessagePreview ?? "",
      session.projectId ? projectNameById.value[session.projectId] ?? "" : ""
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(normalizedSearch.value);
  });
});

const runtimeSummary = computed(() => {
  if (!statusLoaded.value) {
    return t("chat.runtimeLoading");
  }

  return `${status.value.provider} / ${status.value.model}`;
});

const runtimeMeta = computed(() => {
  if (!statusLoaded.value) {
    return t("chat.runtimeLoadingDetail");
  }

  const parts = [
    status.value.provider_url
      ? t("chat.runtimeEndpoint", { value: status.value.provider_url })
      : t("chat.runtimeDefaultEndpoint"),
    t("chat.runtimeTemperature", { value: status.value.temperature.toFixed(1) }),
    status.value.api_key_configured ? t("chat.runtimeCredentialReady") : t("chat.runtimeNoKey")
  ];

  return parts.join(" / ");
});

const runtimePolicy = computed(() => {
  if (!statusLoaded.value) {
    return t("chat.runtimeLoadingPolicy");
  }

  const parts = [
    t("chat.runtimeWorkspace", {
      value: status.value.workspace_dir || t("chat.runtimeWorkspaceDefault")
    }),
    t("chat.runtimeAutonomy", {
      value: t(`settings.autonomyLevels.${status.value.autonomy_level}`)
    }),
    t("chat.runtimeDispatcher", {
      value: t(`settings.toolDispatchers.${status.value.tool_dispatcher}`)
    }),
    status.value.parallel_tools ? t("chat.runtimeParallelToolsOn") : t("chat.runtimeParallelToolsOff")
  ];

  if (status.value.workspace_only) {
    parts.push(t("chat.runtimeWorkspaceOnly"));
  }

  return parts.join(" / ");
});

function highlightMatches(text: string | null | undefined) {
  const source = text ?? "";
  const query = normalizedSearch.value;
  if (!query) {
    return [{ text: source, match: false }] as HighlightSegment[];
  }

  const lower = source.toLowerCase();
  const segments: HighlightSegment[] = [];
  let index = 0;

  while (index < source.length) {
    const matchIndex = lower.indexOf(query, index);
    if (matchIndex === -1) {
      segments.push({ text: source.slice(index), match: false });
      break;
    }

    if (matchIndex > index) {
      segments.push({ text: source.slice(index, matchIndex), match: false });
    }

    segments.push({ text: source.slice(matchIndex, matchIndex + query.length), match: true });
    index = matchIndex + query.length;
  }

  return segments.length > 0 ? segments : [{ text: source, match: false }];
}

function resolveSessionTitle(title: string) {
  if (title === "New session" || title === "新会话") {
    return t("chat.defaults.newSessionTitle");
  }

  return title;
}

function resolveSessionPreview(preview: string | null | undefined) {
  const value = preview ?? "Draft session";
  if (value === "Draft session" || value === "草稿会话") {
    return t("chat.defaults.draftSession");
  }

  return value;
}

function resolveSessionProjectName(projectId: string | null | undefined) {
  if (!projectId) {
    return "";
  }

  return projectNameById.value[projectId] ?? t("chat.linkedProjectFallback");
}

function resolveContextDescription() {
  if (!activeContextPreview.value) {
    return "";
  }

  if (activeContextPreview.value.scope_mode === "manual") {
    return activeContextPreview.value.knowledge_titles.length > 0
      ? t("chat.contextManualWithHits")
      : t("chat.contextManualNoHits");
  }

  return activeContextPreview.value.knowledge_titles.length > 0
    ? t("chat.contextAutoWithHits")
    : t("chat.contextAutoNoHits");
}

function focusSearchInput() {
  searchInputRef.value?.focus();
  searchInputRef.value?.select();
}

function handleWindowKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    focusSearchInput();
    return;
  }

  if (event.key === "Escape" && document.activeElement === searchInputRef.value && sessionSearch.value) {
    event.preventDefault();
    sessionSearch.value = "";
  }
}

async function loadScopedKnowledge(projectId: string | null | undefined) {
  projectKnowledgeDocuments.value = [];

  if (!projectId) {
    return;
  }

  isLoadingKnowledgeScope.value = true;

  try {
    projectKnowledgeDocuments.value = await listProjectKnowledge(projectId);
  } catch (error) {
    console.error("Failed to load project knowledge", error);
    knowledgeScopeFeedback.value = t("chat.feedback.knowledgeLoadFailed");
  } finally {
    isLoadingKnowledgeScope.value = false;
  }
}

watch(
  () => activeSession.value?.projectId ?? "",
  async (projectId) => {
    knowledgeScopeFeedback.value = "";
    await loadScopedKnowledge(projectId || null);
  },
  { immediate: true }
);

useChatStream();

onMounted(async () => {
  window.addEventListener("keydown", handleWindowKeydown);

  try {
    await Promise.all([
      chatStore.bootstrap(),
      projectsStore.loaded ? Promise.resolve() : projectsStore.bootstrap(),
      settingsStore.statusLoaded ? Promise.resolve() : settingsStore.refreshStatus()
    ]);
  } catch (error) {
    console.error("Failed to bootstrap chat state", error);
    chatStore.ensureSession();
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleWindowKeydown);
});

async function handleSubmit(content: string) {
  const sessionId = chatStore.ensureSession();
  chatStore.setActiveSession(sessionId);
  chatStore.appendUserMessage(sessionId, content);
  chatStore.beginAssistantMessage(sessionId);

  try {
    const session = chatStore.findSession(sessionId);
    const scope = chatStore.knowledgeScopes[sessionId] ?? {
      session_id: sessionId,
      mode: "auto",
      document_ids: []
    };

    await sendMessage(sessionId, content, {
      sessionTitle: session?.title,
      projectId: session?.projectId ?? null,
      knowledgeMode: scope.mode,
      knowledgeDocumentIds: scope.document_ids,
      agentMode: chatStore.agentModes[sessionId] ?? false
    });
    chatStore.markSessionPersisted(sessionId);
    await chatStore.refreshKnowledgeScope(sessionId);
  } catch (error) {
    const message = error instanceof Error ? error.message : t("chat.feedback.sendFailed");
    chatStore.markAssistantError(sessionId, message);
  }
}

async function handleStop() {
  if (!activeSessionId.value) {
    return;
  }

  try {
    await stopMessage(activeSessionId.value);
  } finally {
    chatStore.finishAssistantMessage(activeSessionId.value);
  }
}

async function handleSelectSession(sessionId: string) {
  try {
    await chatStore.selectSession(sessionId);
  } catch (error) {
    console.error("Failed to load session messages", error);
    chatStore.setActiveSession(sessionId);
  }
}

async function handleRenameSession() {
  if (!activeSession.value) {
    return;
  }

  const nextTitle = window.prompt(
    t("chat.prompts.renameSession"),
    resolveSessionTitle(activeSession.value.title)
  )?.trim();
  if (!nextTitle) {
    return;
  }

  try {
    await chatStore.renameSession(activeSession.value.id, nextTitle);
  } catch (error) {
    console.error("Failed to rename session", error);
  }
}

async function handleAssignProject(event: Event) {
  if (!activeSession.value) {
    return;
  }

  const projectId = (event.target as HTMLSelectElement).value || null;

  try {
    await chatStore.assignSessionProject(activeSession.value.id, projectId);
  } catch (error) {
    console.error("Failed to assign session project", error);
  }
}

async function handleDeleteSession() {
  if (!activeSession.value) {
    return;
  }

  const confirmed = window.confirm(
    t("chat.prompts.deleteSession", { title: resolveSessionTitle(activeSession.value.title) })
  );
  if (!confirmed) {
    return;
  }

  try {
    await chatStore.deleteSession(activeSession.value.id);
  } catch (error) {
    console.error("Failed to delete session", error);
  }
}

async function handleSetKnowledgeMode(mode: "auto" | "manual") {
  if (!activeSession.value) {
    return;
  }

  knowledgeScopeFeedback.value = "";

  try {
    const documentIds = mode === "manual" ? activeKnowledgeScope.value?.document_ids ?? [] : [];
    await chatStore.saveKnowledgeScope(activeSession.value.id, mode, documentIds);
    knowledgeScopeFeedback.value =
      mode === "manual" ? t("chat.feedback.manualEnabled") : t("chat.feedback.autoEnabled");
  } catch (error) {
    console.error("Failed to update knowledge scope", error);
    knowledgeScopeFeedback.value = t("chat.feedback.knowledgeScopeFailed");
  }
}

async function handleToggleKnowledgeDocument(documentId: string) {
  if (!activeSession.value || activeKnowledgeScope.value?.mode !== "manual") {
    return;
  }

  knowledgeScopeFeedback.value = "";
  const currentIds = activeKnowledgeScope.value.document_ids;
  const nextIds = currentIds.includes(documentId)
    ? currentIds.filter((value) => value !== documentId)
    : [...currentIds, documentId];

  try {
    await chatStore.saveKnowledgeScope(activeSession.value.id, "manual", nextIds);
    knowledgeScopeFeedback.value = t("chat.feedback.scopedKnowledge", { count: nextIds.length });
  } catch (error) {
    console.error("Failed to update scoped knowledge documents", error);
    knowledgeScopeFeedback.value = t("chat.feedback.knowledgeScopeFailed");
  }
}

async function handleSelectAllKnowledge() {
  if (!activeSession.value) {
    return;
  }

  try {
    await chatStore.saveKnowledgeScope(
      activeSession.value.id,
      "manual",
      projectKnowledgeDocuments.value.map((document) => document.id)
    );
    knowledgeScopeFeedback.value = t("chat.feedback.selectAllKnowledge");
  } catch (error) {
    console.error("Failed to select all knowledge", error);
    knowledgeScopeFeedback.value = t("chat.feedback.knowledgeScopeFailed");
  }
}

async function handleClearKnowledgeSelection() {
  if (!activeSession.value) {
    return;
  }

  try {
    await chatStore.saveKnowledgeScope(activeSession.value.id, "manual", []);
    knowledgeScopeFeedback.value = t("chat.feedback.clearKnowledge");
  } catch (error) {
    console.error("Failed to clear manual knowledge scope", error);
    knowledgeScopeFeedback.value = t("chat.feedback.knowledgeScopeFailed");
  }
}

function handleToggleAgentMode() {
  if (!activeSessionId.value) {
    return;
  }

  approvalFeedback.value = "";
  chatStore.setAgentMode(activeSessionId.value, !activeAgentMode.value);
}

async function handleRespondToApproval(requestId: string, decision: "yes" | "no" | "always") {
  approvalFeedback.value = "";

  try {
    await chatStore.respondToApproval(requestId, decision);
    approvalFeedback.value =
      decision === "always"
        ? t("chat.feedback.approvalAlways")
        : decision === "yes"
          ? t("chat.feedback.approvalYes")
          : t("chat.feedback.approvalNo");
  } catch (error) {
    console.error("Failed to resolve tool approval", error);
    approvalFeedback.value = t("chat.feedback.approvalFailed");
  }
}
</script>

<template>
  <div class="chat-layout">
    <aside class="chat-sessions panel">
      <div class="row" style="justify-content: space-between;">
        <strong>{{ t("chat.sessions") }}</strong>
        <Button variant="secondary" @click="chatStore.createSession()">{{ t("chat.new") }}</Button>
      </div>

      <label class="search-box">
        <Search :size="16" class="muted" />
        <input
          ref="searchInputRef"
          v-model="sessionSearch"
          class="search-box__input"
          :placeholder="t('chat.searchPlaceholder')"
        />
        <span class="muted search-box__hint">Ctrl/Cmd+K</span>
        <button v-if="sessionSearch" class="search-box__clear" type="button" @click="sessionSearch = ''">
          <X :size="14" />
        </button>
      </label>

      <div class="row" style="justify-content: space-between;">
        <span class="muted">{{ t("chat.shown", { count: filteredSessions.length }) }}</span>
        <span v-if="normalizedSearch" class="muted">{{ t("chat.filtered") }}</span>
      </div>

      <div v-if="activeSession" class="panel" style="padding: 12px 14px;">
        <div class="stack" style="gap: 10px;">
          <strong>{{ resolveSessionTitle(activeSession.title) }}</strong>
          <span class="muted">{{ resolveSessionPreview(activeSession.lastMessagePreview) }}</span>
          <label class="settings-field" style="gap: 6px;">
            <span class="muted">{{ t("chat.linkedProject") }}</span>
            <select class="field" :value="activeSession.projectId ?? ''" @change="handleAssignProject">
              <option value="">{{ t("chat.noProject") }}</option>
              <option v-for="project in projects" :key="project.id" :value="project.id">{{ project.name }}</option>
            </select>
            <span class="muted">
              {{ activeSession.projectId ? t("chat.linkedProjectHintAssigned") : t("chat.linkedProjectHintEmpty") }}
            </span>
          </label>
          <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
            <div class="stack" style="gap: 4px;">
              <span class="muted">{{ t("chat.messagesCount", { count: activeSession.messageCount }) }}</span>
              <span v-if="activeSession.projectId" class="session-project-chip">{{ resolveSessionProjectName(activeSession.projectId) }}</span>
            </div>
            <div class="row">
              <Button variant="ghost" :disabled="isStreaming || isBootstrapping" @click="handleRenameSession()">
                <Pencil :size="16" />
                {{ t("chat.rename") }}
              </Button>
              <Button variant="ghost" :disabled="isStreaming || isBootstrapping" @click="handleDeleteSession()">
                <Trash2 :size="16" />
                {{ t("chat.delete") }}
              </Button>
            </div>
          </div>
        </div>
      </div>

      <div v-if="filteredSessions.length > 0" class="stack chat-sessions__list">
        <button
          v-for="session in filteredSessions"
          :key="session.id"
          class="session-item"
          :data-active="session.id === activeSessionId"
          @click="handleSelectSession(session.id)"
        >
          <strong>
            <template v-for="(segment, index) in highlightMatches(resolveSessionTitle(session.title))" :key="`${session.id}-title-${index}`">
              <mark v-if="segment.match" class="highlight">{{ segment.text }}</mark>
              <template v-else>{{ segment.text }}</template>
            </template>
          </strong>
          <span class="muted">
            <template
              v-for="(segment, index) in highlightMatches(resolveSessionPreview(session.lastMessagePreview))"
              :key="`${session.id}-preview-${index}`"
            >
              <mark v-if="segment.match" class="highlight">{{ segment.text }}</mark>
              <template v-else>{{ segment.text }}</template>
            </template>
          </span>
          <div v-if="session.projectId" class="row" style="justify-content: flex-start;">
            <span class="session-project-chip">{{ resolveSessionProjectName(session.projectId) }}</span>
          </div>
          <div class="row" style="justify-content: space-between; align-items: center;">
            <span class="muted">{{ t("chat.messagesCountShort", { count: session.messageCount }) }}</span>
            <span class="muted">
              {{ formatTimestamp(session.updatedAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) }}
            </span>
          </div>
        </button>
      </div>
      <EmptyState
        v-else
        :title="t('chat.noMatchingSessionsTitle')"
        :description="t('chat.noMatchingSessionsDescription')"
      />
    </aside>

    <section class="chat-room panel">
      <div class="panel runtime-panel">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px;">
          <div class="stack" style="gap: 6px;">
            <span class="eyebrow">{{ t("chat.runtimeEyebrow") }}</span>
            <div class="row" style="align-items: center; gap: 10px; flex-wrap: wrap;">
              <strong>{{ runtimeSummary }}</strong>
              <span class="session-project-chip">{{ activeAgentMode ? t("chat.agentMode") : t("chat.chatMode") }}</span>
            </div>
            <span class="muted">{{ runtimeMeta }}</span>
            <span class="muted">{{ runtimePolicy }}</span>
          </div>
          <Button variant="secondary" @click="router.push('/settings')">
            <Settings2 :size="16" />
            {{ t("chat.runtimeSettings") }}
          </Button>
        </div>
      </div>

      <div class="panel agent-panel">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
          <div class="stack" style="gap: 4px;">
            <span class="eyebrow">{{ t("chat.agentEyebrow") }}</span>
            <strong>{{ t("chat.agentTitle") }}</strong>
            <span class="muted">
              {{ activeAgentMode ? t("chat.agentEnabledDescription") : t("chat.agentDisabledDescription") }}
            </span>
          </div>
          <Button :variant="activeAgentMode ? 'primary' : 'secondary'" :disabled="isStreaming || !activeSessionId" @click="handleToggleAgentMode()">
            <Bot :size="16" />
            {{ activeAgentMode ? t("chat.agentEnabled") : t("chat.enableAgent") }}
          </Button>
        </div>
      </div>

      <div v-if="activePendingApprovals.length > 0" class="panel approval-panel">
        <div class="stack" style="gap: 12px;">
          <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
            <div class="stack" style="gap: 4px;">
              <span class="eyebrow">{{ t("chat.approvalEyebrow") }}</span>
              <strong>{{ t("chat.pendingCalls", { count: activePendingApprovals.length }) }}</strong>
              <span class="muted">{{ t("chat.approvalDescription") }}</span>
            </div>
            <span class="session-project-chip">{{ t("chat.waitingForYou") }}</span>
          </div>
          <span v-if="approvalFeedback" class="muted">{{ approvalFeedback }}</span>
          <div class="approval-list">
            <article v-for="approval in activePendingApprovals" :key="approval.request_id" class="approval-card">
              <div class="stack" style="gap: 6px;">
                <div class="row" style="justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap;">
                  <strong>{{ approval.tool_name }}</strong>
                  <span class="approval-chip">
                    <ShieldCheck :size="14" />
                    {{ t("chat.approvalRequired") }}
                  </span>
                </div>
                <p class="approval-card__summary">{{ approval.arguments_summary }}</p>
              </div>
              <div class="row approval-actions">
                <Button variant="ghost" :disabled="isBootstrapping" @click="handleRespondToApproval(approval.request_id, 'no')">{{ t("chat.deny") }}</Button>
                <Button variant="secondary" :disabled="isBootstrapping" @click="handleRespondToApproval(approval.request_id, 'yes')">{{ t("chat.approveOnce") }}</Button>
                <Button :disabled="isBootstrapping" @click="handleRespondToApproval(approval.request_id, 'always')">{{ t("chat.alwaysAllow") }}</Button>
              </div>
            </article>
          </div>
        </div>
      </div>

      <div v-if="activeSession?.projectId" class="panel knowledge-scope-panel">
        <div class="stack" style="gap: 12px;">
          <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
            <div class="stack" style="gap: 4px;">
              <span class="eyebrow">{{ t("chat.knowledgeEyebrow") }}</span>
              <strong>{{ resolveSessionProjectName(activeSession.projectId) }}</strong>
              <span class="muted">{{ t("chat.knowledgeDescription") }}</span>
            </div>
            <div class="row knowledge-scope-actions">
              <Button
                :variant="activeKnowledgeScope?.mode === 'auto' ? 'primary' : 'secondary'"
                :disabled="isStreaming"
                @click="handleSetKnowledgeMode('auto')"
              >
                {{ t("chat.auto") }}
              </Button>
              <Button
                :variant="activeKnowledgeScope?.mode === 'manual' ? 'primary' : 'secondary'"
                :disabled="isStreaming"
                @click="handleSetKnowledgeMode('manual')"
              >
                {{ t("chat.manual") }}
              </Button>
            </div>
          </div>

          <span v-if="knowledgeScopeFeedback" class="muted">{{ knowledgeScopeFeedback }}</span>

          <div v-if="isLoadingKnowledgeScope" class="settings-inline-note">
            {{ t("chat.knowledgeLoading") }}
          </div>

          <div v-else-if="projectKnowledgeDocuments.length === 0" class="settings-inline-note">
            {{ t("chat.knowledgeEmpty") }}
          </div>

          <template v-else-if="activeKnowledgeScope?.mode === 'manual'">
            <div class="row knowledge-scope-actions" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap;">
              <span class="muted">{{ t("chat.knowledgeManualHint") }}</span>
              <div class="row">
                <Button variant="ghost" :disabled="isStreaming" @click="handleSelectAllKnowledge">{{ t("chat.selectAll") }}</Button>
                <Button variant="ghost" :disabled="isStreaming" @click="handleClearKnowledgeSelection">{{ t("chat.clear") }}</Button>
              </div>
            </div>

            <div class="knowledge-scope-list">
              <label
                v-for="document in projectKnowledgeDocuments"
                :key="document.id"
                class="scope-document-card"
                :data-active="selectedScopedDocumentIds.has(document.id)"
              >
                <input
                  class="scope-document-card__checkbox"
                  type="checkbox"
                  :checked="selectedScopedDocumentIds.has(document.id)"
                  :disabled="isStreaming"
                  @change="handleToggleKnowledgeDocument(document.id)"
                />
                <div class="stack" style="gap: 6px; min-width: 0;">
                  <strong>{{ document.title }}</strong>
                  <span class="muted knowledge-card__source">{{ document.source_path }}</span>
                  <p class="knowledge-card__preview">{{ document.content_preview }}</p>
                </div>
              </label>
            </div>
          </template>

          <div v-else class="settings-inline-note">
            {{ t("chat.knowledgeAutoHint") }}
          </div>
        </div>
      </div>

      <div v-if="activeContextPreview" class="panel context-preview-panel">
        <div class="stack" style="gap: 8px;">
          <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
            <div class="stack" style="gap: 4px;">
              <span class="eyebrow">{{ t("chat.contextEyebrow") }}</span>
              <strong>{{ activeContextPreview.project_name }}</strong>
              <span class="muted">{{ resolveContextDescription() }}</span>
            </div>
            <span class="session-project-chip">{{ activeContextPreview.scope_mode === 'manual' ? t("chat.contextManualBadge") : t("chat.contextAutoBadge") }}</span>
          </div>
          <div v-if="activeContextPreview.knowledge_titles.length > 0" class="context-preview-chips">
            <span
              v-for="title in activeContextPreview.knowledge_titles"
              :key="title"
              class="context-preview-chip"
            >
              {{ title }}
            </span>
          </div>
        </div>
      </div>

      <EmptyState
        v-if="isBootstrapping"
        :title="t('chat.loadingHistoryTitle')"
        :description="t('chat.loadingHistoryDescription')"
      />
      <ChatMessageList v-else-if="activeMessages.length > 0" :messages="activeMessages" />
      <EmptyState
        v-else
        :title="t('chat.noMessagesTitle')"
        :description="t('chat.noMessagesDescription')"
      />

      <div class="row" style="justify-content: flex-end;">
        <Button v-if="isStreaming" variant="ghost" @click="handleStop()">{{ t("chat.stopStream") }}</Button>
      </div>
      <ChatComposer :busy="isStreaming || isBootstrapping" @submit="handleSubmit" />
    </section>
  </div>
</template>

