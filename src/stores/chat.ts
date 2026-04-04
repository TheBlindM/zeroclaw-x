import { defineStore } from "pinia";
import {
  assignSessionProject as assignSessionProjectFromApi,
  deleteSession as deleteSessionFromApi,
  getSessionKnowledgeScope,
  listMessages,
  listSessions,
  renameSession as renameSessionFromApi,
  respondToToolApproval,
  saveSessionKnowledgeScope,
  setSessionAgentMode as setSessionAgentModeFromApi,
  type ChatApprovalDecision,
  type ChatApprovalRequestPayload,
  type ChatContextPayload,
  type MessageRecord,
  type SessionKnowledgeScopeRecord,
  type SessionRecord
} from "@/api/tauri";

export interface ChatMessage {
  id: string;
  role: "system" | "user" | "assistant";
  content: string;
  createdAt: string;
  status?: "streaming" | "done" | "error";
}

export interface ChatSession {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  lastMessagePreview: string | null;
  projectId: string | null;
  source?: "local" | "database";
}

function makeId(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
}

function normalizeTime(value: string) {
  if (/^\d+$/.test(value)) {
    return Number(value);
  }

  return Number(new Date(value)) || 0;
}

function truncatePreview(content: string | null | undefined) {
  if (!content) {
    return null;
  }

  const normalized = content.replace(/\s+/g, " ").trim();
  if (!normalized) {
    return null;
  }

  return normalized.slice(0, 88);
}

function getPersistedMessages(list: ChatMessage[]) {
  return list.filter((message) => message.role !== "system");
}

function createDefaultKnowledgeScope(sessionId: string): SessionKnowledgeScopeRecord {
  return {
    session_id: sessionId,
    mode: "auto",
    document_ids: []
  };
}

function createSessionRecord(): ChatSession {
  const createdAt = new Date().toISOString();
  return {
    id: makeId("session"),
    title: "New session",
    createdAt,
    updatedAt: createdAt,
    messageCount: 0,
    lastMessagePreview: "Draft session",
    projectId: null,
    source: "local"
  };
}

function mapSession(record: SessionRecord): ChatSession {
  return {
    id: record.id,
    title: record.title,
    createdAt: record.created_at,
    updatedAt: record.updated_at,
    messageCount: record.message_count,
    lastMessagePreview: truncatePreview(record.last_message_preview),
    projectId: record.project_id,
    source: "database"
  };
}

function mapMessage(record: MessageRecord): ChatMessage {
  return {
    id: record.id,
    role: record.role === "user" || record.role === "assistant" ? record.role : "system",
    content: record.content,
    createdAt: record.created_at,
    status: "done"
  };
}

function buildWelcomeMessage(createdAt: string): ChatMessage {
  return {
    id: makeId("system"),
    role: "system",
    content: "ZeroClawX skeleton is ready. Ask for a project plan, code review, or build step.",
    createdAt,
    status: "done"
  };
}

function getLastMessage(list: ChatMessage[]) {
  return list[list.length - 1];
}

let bootstrapPromise: Promise<void> | null = null;

export const useChatStore = defineStore("chat", {
  state: () => ({
    sessions: [] as ChatSession[],
    activeSessionId: "" as string,
    messages: {} as Record<string, ChatMessage[]>,
    drafts: {} as Record<string, string>,
    contextPreviews: {} as Record<string, ChatContextPayload | null>,
    knowledgeScopes: {} as Record<string, SessionKnowledgeScopeRecord>,
    pendingApprovals: {} as Record<string, ChatApprovalRequestPayload[]>,
    agentModes: {} as Record<string, boolean>,
    loadedSessionIds: [] as string[],
    hasBootstrapped: false,
    isStreaming: false,
    isBootstrapping: false
  }),
  getters: {
    activeMessages(state) {
      return state.messages[state.activeSessionId] ?? [];
    }
  },
  actions: {
    sortSessions() {
      this.sessions.sort((left, right) => normalizeTime(right.updatedAt) - normalizeTime(left.updatedAt));
    },
    findSession(sessionId: string) {
      return this.sessions.find((item) => item.id === sessionId);
    },
    setContextPreview(payload: ChatContextPayload) {
      this.contextPreviews[payload.session_id] = payload;
    },
    clearContextPreview(sessionId: string) {
      this.contextPreviews[sessionId] = null;
    },
    addApprovalRequest(payload: ChatApprovalRequestPayload) {
      const list = this.pendingApprovals[payload.session_id] ?? [];
      if (list.some((item) => item.request_id === payload.request_id)) {
        return;
      }
      this.pendingApprovals[payload.session_id] = [...list, payload];
    },
    clearSessionApprovals(sessionId: string) {
      this.pendingApprovals[sessionId] = [];
    },
    async respondToApproval(requestId: string, decision: ChatApprovalDecision) {
      const sessionId = Object.keys(this.pendingApprovals).find((key) =>
        (this.pendingApprovals[key] ?? []).some((approval) => approval.request_id === requestId)
      );

      await respondToToolApproval(requestId, decision);

      if (sessionId) {
        this.pendingApprovals[sessionId] = (this.pendingApprovals[sessionId] ?? []).filter(
          (approval) => approval.request_id !== requestId
        );
      }
    },
    setAgentMode(sessionId: string, enabled: boolean) {
      this.agentModes[sessionId] = enabled;
    },
    async saveAgentMode(sessionId: string, enabled: boolean) {
      const previousValue = this.agentModes[sessionId] ?? false;
      const session = this.findSession(sessionId);

      this.agentModes[sessionId] = enabled;

      try {
        if (session?.source === "database") {
          await setSessionAgentModeFromApi(sessionId, enabled);
        }
      } catch (error) {
        this.agentModes[sessionId] = previousValue;
        throw error;
      }

      return this.agentModes[sessionId];
    },
    setKnowledgeScope(scope: SessionKnowledgeScopeRecord) {
      this.knowledgeScopes[scope.session_id] = scope;
    },
    async refreshKnowledgeScope(sessionId: string) {
      const session = this.findSession(sessionId);
      if (!session || session.source !== "database") {
        this.knowledgeScopes[sessionId] ??= createDefaultKnowledgeScope(sessionId);
        return this.knowledgeScopes[sessionId];
      }

      const scope = await getSessionKnowledgeScope(sessionId);
      this.knowledgeScopes[sessionId] = scope;
      return scope;
    },
    async saveKnowledgeScope(sessionId: string, mode: string, documentIds: string[]) {
      const session = this.findSession(sessionId);
      const previousScope = this.knowledgeScopes[sessionId] ?? createDefaultKnowledgeScope(sessionId);
      const nextScope: SessionKnowledgeScopeRecord = {
        session_id: sessionId,
        mode,
        document_ids: Array.from(new Set(documentIds))
      };

      this.knowledgeScopes[sessionId] = nextScope;
      this.clearContextPreview(sessionId);

      try {
        if (session?.source === "database") {
          this.knowledgeScopes[sessionId] = await saveSessionKnowledgeScope(sessionId, mode, nextScope.document_ids);
        }
      } catch (error) {
        this.knowledgeScopes[sessionId] = previousScope;
        throw error;
      }

      return this.knowledgeScopes[sessionId];
    },
    syncSessionSummary(sessionId: string) {
      const session = this.findSession(sessionId);
      if (!session) {
        return;
      }

      const relevantMessages = getPersistedMessages(this.messages[sessionId] ?? []);
      const lastMessage = relevantMessages[relevantMessages.length - 1];
      session.messageCount = relevantMessages.length;
      session.lastMessagePreview = truncatePreview(lastMessage?.content) ?? "Draft session";
      session.updatedAt = lastMessage?.createdAt ?? session.createdAt;
      this.sortSessions();
    },
    async bootstrap() {
      if (this.hasBootstrapped) {
        return;
      }

      if (this.isBootstrapping) {
        await bootstrapPromise;
        return;
      }

      this.isBootstrapping = true;
      bootstrapPromise = (async () => {
        const records = await listSessions();
        this.sessions = records.map(mapSession);
        this.messages = {};
        this.drafts = {};
        this.contextPreviews = {};
        this.knowledgeScopes = {};
        this.pendingApprovals = {};
        this.agentModes = Object.fromEntries(records.map((record) => [record.id, record.agent_mode]));
        this.loadedSessionIds = [];
        this.sortSessions();

        if (this.sessions.length > 0) {
          this.activeSessionId = this.sessions[0].id;
          await this.loadSessionMessages(this.activeSessionId);
        } else {
          this.createSession();
        }

        this.hasBootstrapped = true;
      })();

      try {
        await bootstrapPromise;
      } finally {
        this.isBootstrapping = false;
        bootstrapPromise = null;
      }
    },
    ensureSession() {
      if (this.activeSessionId) {
        return this.activeSessionId;
      }

      const existing = this.sessions[0];
      if (existing) {
        this.activeSessionId = existing.id;
        return existing.id;
      }

      return this.createSession();
    },
    createSession() {
      const session = createSessionRecord();
      this.sessions.unshift(session);
      this.messages[session.id] = [buildWelcomeMessage(session.createdAt)];
      this.drafts[session.id] = "";
      this.contextPreviews[session.id] = null;
      this.knowledgeScopes[session.id] = createDefaultKnowledgeScope(session.id);
      this.pendingApprovals[session.id] = [];
      this.agentModes[session.id] = false;
      if (!this.loadedSessionIds.includes(session.id)) {
        this.loadedSessionIds.push(session.id);
      }
      this.activeSessionId = session.id;
      this.sortSessions();
      return session.id;
    },
    setActiveSession(sessionId: string) {
      this.activeSessionId = sessionId;
    },
    setSessionDraft(sessionId: string, content: string) {
      this.drafts[sessionId] = content;
    },
    clearSessionDraft(sessionId: string) {
      this.drafts[sessionId] = "";
    },
    markSessionPersisted(sessionId: string) {
      const session = this.findSession(sessionId);
      if (session) {
        session.source = "database";
      }
    },
    async assignSessionProject(sessionId: string, projectId: string | null) {
      const session = this.findSession(sessionId);
      if (!session) {
        return;
      }

      const previousProjectId = session.projectId;
      const previousScope = this.knowledgeScopes[sessionId] ?? createDefaultKnowledgeScope(sessionId);
      session.projectId = projectId;
      this.knowledgeScopes[sessionId] = createDefaultKnowledgeScope(sessionId);
      this.clearContextPreview(sessionId);

      try {
        if (session.source === "database") {
          await assignSessionProjectFromApi(sessionId, projectId);
        }
      } catch (error) {
        session.projectId = previousProjectId;
        this.knowledgeScopes[sessionId] = previousScope;
        throw error;
      }
    },
    async renameSession(sessionId: string, title: string) {
      const trimmed = title.trim();
      if (!trimmed) {
        return;
      }

      const session = this.findSession(sessionId);
      if (!session) {
        return;
      }

      const previousTitle = session.title;
      session.title = trimmed;

      try {
        if (session.source === "database") {
          await renameSessionFromApi(sessionId, trimmed);
        }
      } catch (error) {
        session.title = previousTitle;
        throw error;
      }
    },
    async deleteSession(sessionId: string) {
      const session = this.findSession(sessionId);
      if (!session) {
        return;
      }

      if (session.source === "database") {
        await deleteSessionFromApi(sessionId);
      }

      this.sessions = this.sessions.filter((item) => item.id !== sessionId);
      delete this.messages[sessionId];
      delete this.drafts[sessionId];
      delete this.contextPreviews[sessionId];
      delete this.knowledgeScopes[sessionId];
      delete this.pendingApprovals[sessionId];
      delete this.agentModes[sessionId];
      this.loadedSessionIds = this.loadedSessionIds.filter((item) => item !== sessionId);

      if (this.activeSessionId === sessionId) {
        const nextSession = this.sessions[0];
        if (nextSession) {
          this.activeSessionId = nextSession.id;
          await this.loadSessionMessages(nextSession.id);
          return;
        }

        this.activeSessionId = "";
        this.createSession();
      }
    },
    async selectSession(sessionId: string) {
      this.activeSessionId = sessionId;
      await this.loadSessionMessages(sessionId);
    },
    async loadSessionMessages(sessionId: string) {
      this.drafts[sessionId] ??= "";
      this.pendingApprovals[sessionId] ??= [];
      this.agentModes[sessionId] ??= false;

      if (this.loadedSessionIds.includes(sessionId)) {
        await this.refreshKnowledgeScope(sessionId);
        return this.messages[sessionId] ?? [];
      }

      const records = await listMessages(sessionId);
      this.messages[sessionId] = records.map(mapMessage);
      this.contextPreviews[sessionId] ??= null;
      this.loadedSessionIds.push(sessionId);
      await this.refreshKnowledgeScope(sessionId);
      this.syncSessionSummary(sessionId);
      return this.messages[sessionId];
    },
    appendUserMessage(sessionId: string, content: string) {
      const entry: ChatMessage = {
        id: makeId("user"),
        role: "user",
        content,
        createdAt: new Date().toISOString(),
        status: "done"
      };
      this.messages[sessionId] ??= [];
      this.messages[sessionId].push(entry);
      if (!this.loadedSessionIds.includes(sessionId)) {
        this.loadedSessionIds.push(sessionId);
      }
      this.knowledgeScopes[sessionId] ??= createDefaultKnowledgeScope(sessionId);
      this.pendingApprovals[sessionId] ??= [];
      this.agentModes[sessionId] ??= false;
      const session = this.findSession(sessionId);
      if (session && session.title === "New session") {
        session.title = content.slice(0, 28) || "New session";
      }
      this.syncSessionSummary(sessionId);
    },
    beginAssistantMessage(sessionId: string) {
      const entry: ChatMessage = {
        id: makeId("assistant"),
        role: "assistant",
        content: "",
        createdAt: new Date().toISOString(),
        status: "streaming"
      };
      this.messages[sessionId] ??= [];
      this.messages[sessionId].push(entry);
      this.clearContextPreview(sessionId);
      this.clearSessionApprovals(sessionId);
      this.knowledgeScopes[sessionId] ??= createDefaultKnowledgeScope(sessionId);
      this.pendingApprovals[sessionId] ??= [];
      this.agentModes[sessionId] ??= false;
      if (!this.loadedSessionIds.includes(sessionId)) {
        this.loadedSessionIds.push(sessionId);
      }
      this.syncSessionSummary(sessionId);
      this.isStreaming = true;
    },
    appendAssistantDelta(sessionId: string, token: string) {
      const list = this.messages[sessionId];
      if (!list || list.length === 0) {
        this.beginAssistantMessage(sessionId);
      }
      const current = getLastMessage(this.messages[sessionId] ?? []);
      if (!current || current.role !== "assistant") {
        this.beginAssistantMessage(sessionId);
      }
      const active = getLastMessage(this.messages[sessionId] ?? []);
      if (active) {
        active.content += token;
      }
      this.syncSessionSummary(sessionId);
    },
    finishAssistantMessage(sessionId: string) {
      const active = getLastMessage(this.messages[sessionId] ?? []);
      if (active?.role === "assistant") {
        active.status = "done";
      }
      this.clearSessionApprovals(sessionId);
      this.syncSessionSummary(sessionId);
      this.isStreaming = false;
    },
    markAssistantError(sessionId: string, error: string) {
      const active = getLastMessage(this.messages[sessionId] ?? []);
      if (active?.role === "assistant") {
        active.status = "error";
        active.content ||= error;
      } else {
        this.messages[sessionId] ??= [];
        this.messages[sessionId].push({
          id: makeId("assistant"),
          role: "assistant",
          content: error,
          createdAt: new Date().toISOString(),
          status: "error"
        });
      }
      if (!this.loadedSessionIds.includes(sessionId)) {
        this.loadedSessionIds.push(sessionId);
      }
      this.clearSessionApprovals(sessionId);
      this.syncSessionSummary(sessionId);
      this.isStreaming = false;
    }
  }
});
