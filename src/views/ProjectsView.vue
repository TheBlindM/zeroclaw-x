<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import Button from "@/components/ui/Button.vue";
import {
  createProjectKnowledgeNote,
  deleteKnowledgeDocument,
  importProjectKnowledgeFiles,
  listProjectKnowledge,
  listProjectSessions,
  type KnowledgeDocumentRecord,
  type SessionRecord
} from "@/api/tauri";
import { formatTimestamp } from "@/lib/datetime";
import { useProjectsStore } from "@/stores/projects";

const projectsStore = useProjectsStore();
const { t } = useI18n();
const { activeProject, activeCount, archivedCount, pinnedCount, projects } = storeToRefs(projectsStore);

const filter = ref<"all" | "pinned" | "active" | "archived">("all");
const search = ref("");
const feedback = ref("");
const linkedSessions = ref<SessionRecord[]>([]);
const knowledgeDocuments = ref<KnowledgeDocumentRecord[]>([]);
const selectedKnowledgeId = ref("");
const isLoadingLinkedSessions = ref(false);
const isLoadingKnowledge = ref(false);
const isImportingKnowledge = ref(false);
const isSavingKnowledge = ref(false);
const knowledgeFeedback = ref("");

const form = reactive({
  name: "",
  description: "",
  status: "active" as "active" | "archived",
  pinned: false
});

const noteForm = reactive({
  title: "",
  content: ""
});

const filteredProjects = computed(() => {
  const query = search.value.trim().toLowerCase();

  return projects.value.filter((project) => {
    const matchesFilter =
      filter.value === "all"
        ? true
        : filter.value === "pinned"
          ? project.pinned
          : project.status === filter.value;

    if (!matchesFilter) {
      return false;
    }

    if (!query) {
      return true;
    }

    const haystack = `${project.name} ${project.description}`.toLowerCase();
    return haystack.includes(query);
  });
});

const selectedKnowledge = computed(() => {
  if (!knowledgeDocuments.value.length) {
    return null;
  }

  return knowledgeDocuments.value.find((document) => document.id === selectedKnowledgeId.value) ?? knowledgeDocuments.value[0];
});

watch(
  activeProject,
  async (project) => {
    if (!project) {
      resetForm();
      resetKnowledgeDraft();
      linkedSessions.value = [];
      knowledgeDocuments.value = [];
      selectedKnowledgeId.value = "";
      return;
    }

    form.name = project.name;
    form.description = project.description;
    form.status = project.status;
    form.pinned = project.pinned;
    resetKnowledgeDraft();
    await loadProjectContext(project.id);
  },
  { immediate: true }
);

onMounted(async () => {
  if (!projectsStore.loaded) {
    try {
      await projectsStore.bootstrap();
    } catch {
      // store error is rendered below
    }
  }
});

function resolveSessionTitle(title: string) {
  if (title === "New session" || title === "新会话") {
    return t("chat.defaults.newSessionTitle");
  }

  return title;
}

function resolveSessionPreview(preview: string | null | undefined) {
  const value = preview ?? "Draft session";
  if (value === "Draft session" || value === "草稿会话") {
    return t("projects.draftSession");
  }

  return value;
}

function resolveStatusLabel(status: "active" | "archived") {
  return status === "archived" ? t("projects.statusArchived") : t("projects.statusActive");
}

async function loadProjectContext(projectId: string) {
  isLoadingLinkedSessions.value = true;
  isLoadingKnowledge.value = true;

  try {
    const [sessions, documents] = await Promise.all([
      listProjectSessions(projectId),
      listProjectKnowledge(projectId)
    ]);
    linkedSessions.value = sessions;
    knowledgeDocuments.value = documents;
    selectedKnowledgeId.value = documents[0]?.id ?? "";
  } catch {
    linkedSessions.value = [];
    knowledgeDocuments.value = [];
    selectedKnowledgeId.value = "";
  } finally {
    isLoadingLinkedSessions.value = false;
    isLoadingKnowledge.value = false;
  }
}

async function handleSubmit() {
  feedback.value = "";

  try {
    if (activeProject.value) {
      await projectsStore.updateProject(activeProject.value.id, { ...form });
      feedback.value = t("projects.feedback.updated", { name: form.name });
      return;
    }

    const project = await projectsStore.createProject({ ...form });
    feedback.value = t("projects.feedback.created", { name: project.name });
  } catch {
    feedback.value = activeProject.value ? t("projects.feedback.updateFailed") : t("projects.feedback.createFailed");
  }
}

function handleNewProject() {
  projectsStore.clearActiveProject();
  resetForm();
  resetKnowledgeDraft();
  feedback.value = t("projects.feedback.readyForNewProject");
}

async function handleDelete(projectId: string) {
  const project = projects.value.find((item) => item.id === projectId);
  if (!project) {
    return;
  }

  const confirmed = window.confirm(t("projects.prompts.deleteProject", { name: project.name }));
  if (!confirmed) {
    return;
  }

  try {
    await projectsStore.deleteProject(projectId);
    feedback.value = t("projects.feedback.deleted", { name: project.name });
  } catch {
    feedback.value = t("projects.feedback.deleteFailed");
  }
}

async function handleTogglePinned(projectId: string) {
  const project = projects.value.find((item) => item.id === projectId);
  if (!project) {
    return;
  }

  try {
    await projectsStore.togglePinned(projectId);
    feedback.value = project.pinned
      ? t("projects.feedback.unpinned", { name: project.name })
      : t("projects.feedback.pinned", { name: project.name });
  } catch {
    feedback.value = t("projects.feedback.pinFailed");
  }
}

async function handleToggleArchived(projectId: string) {
  const project = projects.value.find((item) => item.id === projectId);
  if (!project) {
    return;
  }

  try {
    await projectsStore.setProjectStatus(projectId, project.status === "archived" ? "active" : "archived");
    feedback.value =
      project.status === "archived"
        ? t("projects.feedback.restored", { name: project.name })
        : t("projects.feedback.archived", { name: project.name });
  } catch {
    feedback.value = t("projects.feedback.statusFailed");
  }
}

function handleSelectProject(projectId: string) {
  projectsStore.setActiveProject(projectId);
  feedback.value = "";
}

async function handleImportKnowledge() {
  if (!activeProject.value) {
    return;
  }

  knowledgeFeedback.value = "";
  isImportingKnowledge.value = true;

  try {
    const imported = await importProjectKnowledgeFiles(activeProject.value.id);
    if (imported.length === 0) {
      knowledgeFeedback.value = t("projects.feedback.importCancelled");
      return;
    }

    knowledgeDocuments.value = [...imported, ...knowledgeDocuments.value];
    selectedKnowledgeId.value = imported[0].id;
    knowledgeFeedback.value = t("projects.feedback.importedKnowledge", { count: imported.length });
  } catch {
    knowledgeFeedback.value = t("projects.feedback.importFailed");
  } finally {
    isImportingKnowledge.value = false;
  }
}

async function handleCreateKnowledgeNote() {
  if (!activeProject.value) {
    return;
  }

  knowledgeFeedback.value = "";
  isSavingKnowledge.value = true;

  try {
    const document = await createProjectKnowledgeNote(activeProject.value.id, noteForm.title, noteForm.content);
    knowledgeDocuments.value = [document, ...knowledgeDocuments.value];
    selectedKnowledgeId.value = document.id;
    resetKnowledgeDraft();
    knowledgeFeedback.value = t("projects.feedback.savedNote", { title: document.title });
  } catch {
    knowledgeFeedback.value = t("projects.feedback.saveNoteFailed");
  } finally {
    isSavingKnowledge.value = false;
  }
}

async function handleDeleteKnowledge(documentId: string) {
  const document = knowledgeDocuments.value.find((item) => item.id === documentId);
  if (!document) {
    return;
  }

  const confirmed = window.confirm(t("projects.prompts.deleteKnowledge", { title: document.title }));
  if (!confirmed) {
    return;
  }

  try {
    await deleteKnowledgeDocument(documentId);
    knowledgeDocuments.value = knowledgeDocuments.value.filter((item) => item.id !== documentId);
    if (selectedKnowledgeId.value === documentId) {
      selectedKnowledgeId.value = knowledgeDocuments.value[0]?.id ?? "";
    }
    knowledgeFeedback.value = t("projects.feedback.deletedKnowledge", { title: document.title });
  } catch {
    knowledgeFeedback.value = t("projects.feedback.deleteKnowledgeFailed");
  }
}

function handleSelectKnowledge(documentId: string) {
  selectedKnowledgeId.value = documentId;
}

function resetForm() {
  form.name = "";
  form.description = "";
  form.status = "active";
  form.pinned = false;
}

function resetKnowledgeDraft() {
  noteForm.title = "";
  noteForm.content = "";
}
</script>

<template>
  <div class="stack projects-page">
    <section class="panel projects-hero">
      <div class="stack" style="gap: 8px; max-width: 760px;">
        <strong>{{ t("projects.workspaceTitle") }}</strong>
        <p class="muted projects-hero__copy">
          {{ t("projects.workspaceDescription") }}
        </p>
      </div>
      <div class="projects-summary-grid">
        <button class="summary-card" type="button" :data-active="filter === 'all'" @click="filter = 'all'">
          <strong>{{ projects.length }}</strong>
          <span class="muted">{{ t("projects.summaryAll") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'pinned'" @click="filter = 'pinned'">
          <strong>{{ pinnedCount }}</strong>
          <span class="muted">{{ t("projects.summaryPinned") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'active'" @click="filter = 'active'">
          <strong>{{ activeCount }}</strong>
          <span class="muted">{{ t("projects.summaryActive") }}</span>
        </button>
        <button class="summary-card" type="button" :data-active="filter === 'archived'" @click="filter = 'archived'">
          <strong>{{ archivedCount }}</strong>
          <span class="muted">{{ t("projects.summaryArchived") }}</span>
        </button>
      </div>
    </section>

    <section class="projects-layout">
      <section class="panel projects-editor">
        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ activeProject ? t("projects.editing", { name: activeProject.name }) : t("projects.newProject") }}</strong>
            <span class="muted">{{ t("projects.editorDescription") }}</span>
          </div>
          <Button variant="secondary" :disabled="projectsStore.isSaving" @click="handleNewProject">{{ t("projects.newDraft") }}</Button>
        </div>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("projects.projectName") }}</span>
          <input v-model="form.name" class="field" :placeholder="t('projects.projectNamePlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("projects.brief") }}</span>
          <textarea v-model="form.description" class="field projects-textarea" :placeholder="t('projects.briefPlaceholder')" />
        </label>

        <div class="projects-editor__grid">
          <label class="settings-field">
            <span class="settings-field__label">{{ t("projects.status") }}</span>
            <select v-model="form.status" class="field">
              <option value="active">{{ t("projects.statusActive") }}</option>
              <option value="archived">{{ t("projects.statusArchived") }}</option>
            </select>
          </label>

          <label class="projects-checkbox">
            <input v-model="form.pinned" type="checkbox" />
            <span>{{ t("projects.pinToTop") }}</span>
          </label>
        </div>

        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; margin-top: 8px;">
          <div class="stack" style="gap: 6px; max-width: 560px;">
            <span v-if="feedback" class="muted">{{ feedback }}</span>
            <span v-if="projectsStore.error" class="settings-error">{{ projectsStore.error }}</span>
          </div>
          <div class="row settings-action-row">
            <Button variant="secondary" :disabled="projectsStore.isSaving" @click="resetForm">{{ t("projects.resetForm") }}</Button>
            <Button :disabled="projectsStore.isSaving" @click="handleSubmit">
              {{ projectsStore.isSaving ? t("projects.saving") : activeProject ? t("projects.saveProject") : t("projects.createProject") }}
            </Button>
          </div>
        </div>
      </section>

      <section class="panel projects-board">
        <div class="row" style="justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 12px;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ t("projects.projectBoard") }}</strong>
            <span class="muted">{{ t("projects.projectBoardDescription") }}</span>
          </div>
          <input v-model="search" class="field projects-search" :placeholder="t('projects.searchPlaceholder')" />
        </div>

        <div v-if="projectsStore.isBootstrapping" class="empty-state">
          <strong>{{ t("projects.loadingTitle") }}</strong>
          <span class="muted">{{ t("projects.loadingDescription") }}</span>
        </div>

        <div v-else-if="filteredProjects.length === 0" class="empty-state">
          <strong>{{ t("projects.emptyTitle") }}</strong>
          <span class="muted">{{ t("projects.emptyDescription") }}</span>
        </div>

        <div v-else class="projects-list">
          <article
            v-for="project in filteredProjects"
            :key="project.id"
            class="project-card"
            :data-active="project.id === projectsStore.activeProjectId"
            @click="handleSelectProject(project.id)"
          >
            <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px;">
              <div class="stack" style="gap: 6px; min-width: 0;">
                <div class="row" style="align-items: center; gap: 8px; flex-wrap: wrap;">
                  <strong>{{ project.name }}</strong>
                  <span v-if="project.pinned" class="project-badge">{{ t("projects.summaryPinned") }}</span>
                  <span class="project-badge" :data-archived="project.status === 'archived'">
                    {{ resolveStatusLabel(project.status) }}
                  </span>
                </div>
                <p class="project-card__description">{{ project.description || t("projects.noBrief") }}</p>
              </div>
            </div>

            <div class="row project-card__meta">
              <span class="muted">{{ t("projects.updatedAt", { value: formatTimestamp(project.updatedAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) }) }}</span>
              <span class="muted">{{ t("projects.createdAt", { value: formatTimestamp(project.createdAt, { month: 'short', day: 'numeric' }) }) }}</span>
            </div>

            <div class="row settings-action-row" style="justify-content: flex-end;">
              <Button variant="ghost" :disabled="projectsStore.isSaving" @click.stop="handleTogglePinned(project.id)">
                {{ project.pinned ? t("projects.unpin") : t("projects.pin") }}
              </Button>
              <Button variant="ghost" :disabled="projectsStore.isSaving" @click.stop="handleToggleArchived(project.id)">
                {{ project.status === 'archived' ? t("projects.restore") : t("projects.archive") }}
              </Button>
              <Button variant="ghost" :disabled="projectsStore.isSaving" @click.stop="handleDelete(project.id)">{{ t("projects.delete") }}</Button>
            </div>
          </article>
        </div>
      </section>
    </section>

    <section class="panel projects-linked-panel">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
        <div class="stack" style="gap: 4px;">
          <strong>{{ activeProject ? t("projects.linkedSessionsTitle", { name: activeProject.name }) : t("projects.linkedSessionsFallback") }}</strong>
          <span class="muted">{{ t("projects.linkedSessionsDescription") }}</span>
        </div>
        <span v-if="activeProject" class="project-badge">{{ t("projects.linkedCount", { count: linkedSessions.length }) }}</span>
      </div>

      <div v-if="!activeProject" class="empty-state">
        <strong>{{ t("projects.selectProjectTitle") }}</strong>
        <span class="muted">{{ t("projects.selectProjectSessionsDescription") }}</span>
      </div>

      <div v-else-if="isLoadingLinkedSessions" class="empty-state">
        <strong>{{ t("projects.loadingLinkedTitle") }}</strong>
        <span class="muted">{{ t("projects.loadingLinkedDescription") }}</span>
      </div>

      <div v-else-if="linkedSessions.length === 0" class="empty-state">
        <strong>{{ t("projects.emptyLinkedTitle") }}</strong>
        <span class="muted">{{ t("projects.emptyLinkedDescription") }}</span>
      </div>

      <div v-else class="projects-linked-list">
        <article v-for="session in linkedSessions" :key="session.id" class="linked-session-card">
          <div class="stack" style="gap: 6px;">
            <strong>{{ resolveSessionTitle(session.title) }}</strong>
            <span class="muted">{{ resolveSessionPreview(session.last_message_preview) }}</span>
          </div>
          <div class="row linked-session-card__meta">
            <span class="muted">{{ t("chat.messagesCount", { count: session.message_count }) }}</span>
            <span class="muted">{{ t("projects.updatedAt", { value: formatTimestamp(session.updated_at, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) }) }}</span>
          </div>
        </article>
      </div>
    </section>

    <section class="projects-knowledge-layout">
      <section class="panel projects-knowledge-editor">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ activeProject ? t("projects.knowledgeTitle", { name: activeProject.name }) : t("projects.knowledgeFallback") }}</strong>
            <span class="muted">{{ t("projects.knowledgeDescription") }}</span>
          </div>
          <Button
            variant="secondary"
            :disabled="!activeProject || isImportingKnowledge"
            @click="handleImportKnowledge"
          >
            {{ isImportingKnowledge ? t("projects.importing") : t("projects.importFiles") }}
          </Button>
        </div>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("projects.noteTitle") }}</span>
          <input v-model="noteForm.title" class="field" :placeholder="t('projects.noteTitlePlaceholder')" :disabled="!activeProject || isSavingKnowledge" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("projects.noteContent") }}</span>
          <textarea
            v-model="noteForm.content"
            class="field projects-textarea"
            :placeholder="t('projects.noteContentPlaceholder')"
            :disabled="!activeProject || isSavingKnowledge"
          />
        </label>

        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
          <div class="stack" style="gap: 6px; max-width: 560px;">
            <span v-if="knowledgeFeedback" class="muted">{{ knowledgeFeedback }}</span>
          </div>
          <Button :disabled="!activeProject || isSavingKnowledge" @click="handleCreateKnowledgeNote">
            {{ isSavingKnowledge ? t("projects.saving") : t("projects.saveNote") }}
          </Button>
        </div>
      </section>

      <section class="panel projects-knowledge-board">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
          <div class="stack" style="gap: 4px;">
            <strong>{{ t("projects.knowledgeLibrary") }}</strong>
            <span class="muted">{{ t("projects.knowledgeLibraryDescription") }}</span>
          </div>
          <span v-if="activeProject" class="project-badge">{{ t("projects.docsCount", { count: knowledgeDocuments.length }) }}</span>
        </div>

        <div v-if="!activeProject" class="empty-state">
          <strong>{{ t("projects.selectProjectTitle") }}</strong>
          <span class="muted">{{ t("projects.selectProjectKnowledgeDescription") }}</span>
        </div>

        <div v-else-if="isLoadingKnowledge" class="empty-state">
          <strong>{{ t("projects.loadingKnowledgeTitle") }}</strong>
          <span class="muted">{{ t("projects.loadingKnowledgeDescription") }}</span>
        </div>

        <div v-else-if="knowledgeDocuments.length === 0" class="empty-state">
          <strong>{{ t("projects.emptyKnowledgeTitle") }}</strong>
          <span class="muted">{{ t("projects.emptyKnowledgeDescription") }}</span>
        </div>

        <div v-else class="projects-knowledge-grid">
          <div class="projects-knowledge-list">
            <button
              v-for="document in knowledgeDocuments"
              :key="document.id"
              class="knowledge-card"
              :data-active="document.id === selectedKnowledge?.id"
              type="button"
              @click="handleSelectKnowledge(document.id)"
            >
              <div class="stack" style="gap: 6px; min-width: 0;">
                <strong>{{ document.title }}</strong>
                <span class="muted knowledge-card__source">{{ document.source_path }}</span>
                <p class="knowledge-card__preview">{{ document.content_preview }}</p>
              </div>
              <div class="row linked-session-card__meta">
                <span class="muted">{{ formatTimestamp(document.updated_at, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) }}</span>
              </div>
            </button>
          </div>

          <div v-if="selectedKnowledge" class="knowledge-detail">
            <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap;">
              <div class="stack" style="gap: 4px; max-width: 720px;">
                <strong>{{ selectedKnowledge.title }}</strong>
                <span class="muted">{{ selectedKnowledge.source_path }}</span>
              </div>
              <Button variant="ghost" @click="handleDeleteKnowledge(selectedKnowledge.id)">{{ t("projects.delete") }}</Button>
            </div>
            <div class="code-block knowledge-detail__content">{{ selectedKnowledge.content }}</div>
          </div>
        </div>
      </section>
    </section>
  </div>
</template>

