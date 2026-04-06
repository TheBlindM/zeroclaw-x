<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import SkillDetailPanel from "@/components/skills/SkillDetailPanel.vue";
import SkillEditorPanel from "@/components/skills/SkillEditorPanel.vue";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { parseTags, useSkillStore, type SkillItem, type SkillTemplateItem } from "@/stores/skill";

const skillStore = useSkillStore();
const { activeSkill, enabledCount, importedCount, skills, templates } = storeToRefs(skillStore);
const { t } = useI18n();

const search = ref("");
const feedback = ref("");
const panelMode = ref<"view" | "create" | "edit">("view");
const editorDirty = ref(false);

const filteredSkills = computed(() => {
  const query = search.value.trim().toLowerCase();

  return skills.value.filter((skill) => {
    if (!query) {
      return true;
    }

    const haystack = `${skill.name} ${skill.description} ${skill.slug} ${skill.sourceKind} ${skill.author}`.toLowerCase();
    return haystack.includes(query);
  });
});

const templateCards = computed(() => {
  const installed = new Set(skills.value.map((skill) => skill.slug));
  return templates.value.map((template) => ({
    ...template,
    installed: installed.has(template.slug),
    tags: parseTags(template.tagsJson)
  }));
});
const editorPanelKey = computed(() => `${panelMode.value}:${activeSkill.value?.id ?? "new"}`);

function resolveActionError(fallbackKey: string) {
  const message = skillStore.error || t(fallbackKey);
  skillStore.clearError();
  return message;
}

onMounted(async () => {
  if (!skillStore.loaded) {
    try {
      await skillStore.bootstrap();
    } catch {
      feedback.value = resolveActionError("skills.feedback.loadSkillsFailed");
    }
  }
});

function resolveSourceLabel(skill: SkillItem) {
  if (skill.sourceKind === "template") {
    return t("skills.sourceTemplate");
  }

  if (skill.sourceKind === "imported") {
    return t("skills.sourceImported");
  }

  return t("skills.sourceManual");
}

function resolveSkillTags(skill: SkillItem) {
  return parseTags(skill.tagsJson);
}

function confirmLeaveInlineEditor() {
  if (!editorDirty.value) {
    return true;
  }

  return window.confirm(t("skills.prompts.leaveEditorWithUnsavedChanges"));
}

function handleSelectSkill(skillId: string) {
  if (skillId === skillStore.activeSkillId) {
    return;
  }

  if ((panelMode.value === "create" || panelMode.value === "edit") && !confirmLeaveInlineEditor()) {
    return;
  }

  feedback.value = "";
  panelMode.value = "view";
  skillStore.setActiveSkill(skillId);
}

function enterCreateMode() {
  if ((panelMode.value === "create" || panelMode.value === "edit") && !confirmLeaveInlineEditor()) {
    return;
  }

  feedback.value = "";
  panelMode.value = "create";
}

function enterEditMode(skillId: string) {
  if ((panelMode.value === "create" || panelMode.value === "edit") && !confirmLeaveInlineEditor()) {
    return;
  }

  if (skillStore.activeSkillId !== skillId) {
    skillStore.setActiveSkill(skillId);
  }
  feedback.value = "";
  panelMode.value = "edit";
}

function handleEditorCancel() {
  editorDirty.value = false;
  panelMode.value = "view";
}

function handleEditorCreated(payload: { skillId: string; name: string }) {
  editorDirty.value = false;
  panelMode.value = "view";
  skillStore.setActiveSkill(payload.skillId);
  feedback.value = t("skills.feedback.created", { name: payload.name });
}

async function handleImportDirectory() {
  feedback.value = "";

  try {
    const skill = await skillStore.importDirectory();
    feedback.value = skill ? t("skills.feedback.imported", { name: skill.name }) : t("skills.feedback.importCancelled");
  } catch {
    feedback.value = resolveActionError("skills.feedback.importFailed");
  }
}

async function handleInstallTemplate(template: SkillTemplateItem & { installed: boolean }) {
  if (template.installed) {
    const confirmed = window.confirm(t("skills.prompts.reinstallTemplate", { name: template.name }));
    if (!confirmed) {
      return;
    }
  }

  feedback.value = "";

  try {
    const skill = await skillStore.installTemplate(template.templateId);
    feedback.value = t("skills.feedback.installed", { name: skill.name });
  } catch {
    feedback.value = resolveActionError("skills.feedback.installFailed");
  }
}

async function handleToggleSkill(skill: SkillItem) {
  feedback.value = "";

  try {
    const updated = await skillStore.toggleSkill(skill.id, !skill.enabled);
    feedback.value = updated.enabled ? t("skills.feedback.enabled", { name: skill.name }) : t("skills.feedback.disabled", { name: skill.name });
  } catch {
    feedback.value = resolveActionError("skills.feedback.toggleFailed");
  }
}

async function handleDuplicateSkill(skill: SkillItem) {
  feedback.value = "";

  try {
    const duplicated = await skillStore.duplicateSkill(skill.id);
    feedback.value = t("skills.feedback.duplicated", { name: duplicated.name });
  } catch {
    feedback.value = resolveActionError("skills.feedback.duplicateFailed");
  }
}

async function handleRefreshSkill(skill: SkillItem) {
  const confirmed =
    skill.sourceKind === "template" || skill.sourceKind === "imported"
      ? window.confirm(t("skills.prompts.refreshSkill", { name: skill.name }))
      : true;

  if (!confirmed) {
    return;
  }

  feedback.value = "";

  try {
    const refreshed = await skillStore.refreshSkill(skill.id);
    feedback.value = t("skills.feedback.refreshed", { name: refreshed.name });
  } catch {
    feedback.value = resolveActionError("skills.feedback.refreshFailed");
  }
}

async function handleExportSkill(skill: SkillItem) {
  feedback.value = "";

  try {
    const exported = await skillStore.exportSkill(skill.id);
    feedback.value = exported
      ? t("skills.feedback.exported", { path: exported.path })
      : t("skills.feedback.exportCancelled");
  } catch {
    feedback.value = resolveActionError("skills.feedback.exportFailed");
  }
}

async function handleOpenDirectory(skill: SkillItem) {
  feedback.value = "";

  try {
    await skillStore.openSkillDirectory(skill.id);
    feedback.value = t("skills.feedback.openedDirectory");
  } catch {
    feedback.value = resolveActionError("skills.feedback.openDirectoryFailed");
  }
}

async function handleDeleteSkill(skill: SkillItem) {
  const confirmed = window.confirm(t("skills.prompts.deleteSkill", { name: skill.name }));
  if (!confirmed) {
    return;
  }

  feedback.value = "";
  const deletingActiveSkill = skill.id === skillStore.activeSkillId;

  try {
    await skillStore.removeSkill(skill.id);
    if (deletingActiveSkill) {
      editorDirty.value = false;
      panelMode.value = "view";
    }
    feedback.value = t("skills.feedback.deleted", { name: skill.name });
  } catch {
    feedback.value = resolveActionError("skills.feedback.deleteFailed");
  }
}
</script>

<template>
  <div class="stack skills-page">
    <section class="panel skills-hero">
      <div class="stack" style="gap: 8px; max-width: 780px;">
        <strong>{{ t("skills.workspaceTitle") }}</strong>
        <p class="muted skills-hero__copy">{{ t("skills.workspaceDescription") }}</p>
      </div>
      <div class="skills-summary-grid">
        <div class="summary-card summary-card--static">
          <strong>{{ skills.length }}</strong>
          <span class="muted">{{ t("skills.summaryInstalled") }}</span>
        </div>
        <div class="summary-card summary-card--static">
          <strong>{{ enabledCount }}</strong>
          <span class="muted">{{ t("skills.summaryEnabled") }}</span>
        </div>
        <div class="summary-card summary-card--static">
          <strong>{{ importedCount }}</strong>
          <span class="muted">{{ t("skills.summaryImported") }}</span>
        </div>
        <div class="summary-card summary-card--static">
          <strong>{{ templates.length }}</strong>
          <span class="muted">{{ t("skills.summaryTemplates") }}</span>
        </div>
      </div>
    </section>

    <section class="skills-layout">
      <section class="panel skills-library-panel">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("skills.libraryTitle") }}</strong>
            <span class="muted">{{ t("skills.libraryDescription") }}</span>
          </div>
          <div class="row" style="flex-wrap: wrap;">
            <Button variant="secondary" :disabled="skillStore.isSaving || skillStore.isImporting" @click="enterCreateMode">
              {{ t("skills.createSkill") }}
            </Button>
            <input v-model="search" class="field skills-search" :placeholder="t('skills.searchPlaceholder')" />
            <Button variant="secondary" :disabled="skillStore.isImporting || skillStore.isSaving" @click="handleImportDirectory">
              {{ skillStore.isImporting ? t("skills.importing") : t("skills.importDirectory") }}
            </Button>
          </div>
        </div>

        <div v-if="skillStore.isLoading && !skillStore.loaded" class="empty-state">
          <strong>{{ t("skills.loadingTitle") }}</strong>
          <span class="muted">{{ t("skills.loadingDescription") }}</span>
        </div>

        <div v-else-if="filteredSkills.length === 0" class="empty-state">
          <strong>{{ t("skills.emptyTitle") }}</strong>
          <span class="muted">{{ t("skills.emptyDescription") }}</span>
        </div>

        <div v-else class="skills-list">
          <article
            v-for="skill in filteredSkills"
            :key="skill.id"
            class="skills-card"
            :data-active="skill.id === skillStore.activeSkillId"
            @click="handleSelectSkill(skill.id)"
          >
            <div class="stack" style="gap: 10px;">
              <div class="row" style="justify-content: space-between; align-items: flex-start;">
                <div class="stack" style="gap: 6px;">
                  <div class="row" style="flex-wrap: wrap; gap: 8px;">
                    <strong>{{ skill.name }}</strong>
                    <span class="project-badge">{{ resolveSourceLabel(skill) }}</span>
                    <span class="project-badge" :data-archived="!skill.enabled">{{ skill.enabled ? t("skills.enabled") : t("skills.disabled") }}</span>
                  </div>
                  <p class="mcp-card__note">{{ skill.description }}</p>
                </div>
              </div>

              <div class="row" style="flex-wrap: wrap; gap: 8px;">
                <span class="muted">{{ t("skills.versionLabel", { value: skill.version }) }}</span>
                <span class="muted">{{ t("skills.authorLabel", { value: skill.author || t('skills.none') }) }}</span>
              </div>

              <div class="skills-tags" v-if="resolveSkillTags(skill).length > 0">
                <span v-for="tag in resolveSkillTags(skill)" :key="`${skill.id}-${tag}`" class="skills-tag">{{ tag }}</span>
              </div>

                <div class="row" style="justify-content: space-between; flex-wrap: wrap; gap: 10px;">
                  <span class="muted">{{ t("skills.updatedAt", { value: formatTimestamp(skill.updatedAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) }) }}</span>
                  <div class="row" style="flex-wrap: wrap;">
                    <Button variant="ghost" :disabled="skillStore.isSaving || skillStore.isImporting" @click.stop="enterEditMode(skill.id)">
                      {{ t("skills.editSkill") }}
                    </Button>
                  <Button variant="secondary" :disabled="skillStore.isSaving || skillStore.isImporting" @click.stop="handleToggleSkill(skill)">
                    {{ skill.enabled ? t("skills.disable") : t("skills.enable") }}
                  </Button>
                  <Button variant="ghost" :disabled="skillStore.isSaving || skillStore.isImporting" @click.stop="handleDeleteSkill(skill)">
                    {{ t("skills.delete") }}
                  </Button>
                </div>
              </div>
            </div>
          </article>
        </div>
      </section>

      <section class="panel skills-detail-panel">
        <SkillEditorPanel
          v-if="panelMode === 'create' || panelMode === 'edit'"
          :key="editorPanelKey"
          :mode="panelMode"
          :skill-id="panelMode === 'edit' ? activeSkill?.id : undefined"
          @cancel="handleEditorCancel"
          @created="handleEditorCreated"
          @dirty-change="editorDirty = $event"
        />
        <SkillDetailPanel
          v-else
          :skill="activeSkill"
          @edit="activeSkill && enterEditMode(activeSkill.id)"
          @duplicate="activeSkill && handleDuplicateSkill(activeSkill)"
          @refresh="activeSkill && handleRefreshSkill(activeSkill)"
          @export="activeSkill && handleExportSkill(activeSkill)"
          @open-directory="activeSkill && handleOpenDirectory(activeSkill)"
        />
      </section>
    </section>

    <section class="panel skills-templates-panel">
      <div class="stack" style="gap: 6px;">
        <strong>{{ t("skills.templatesTitle") }}</strong>
        <span class="muted">{{ t("skills.templatesDescription") }}</span>
      </div>

      <div class="skills-template-grid">
        <article v-for="template in templateCards" :key="template.templateId" class="skills-template-card">
          <div class="stack" style="gap: 10px;">
            <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 10px;">
              <div class="stack" style="gap: 6px;">
                <div class="row" style="flex-wrap: wrap; gap: 8px;">
                  <strong>{{ template.name }}</strong>
                  <span v-if="template.installed" class="project-badge">{{ t("skills.installed") }}</span>
                </div>
                <span class="muted">{{ template.description }}</span>
              </div>
            </div>

            <div class="skills-tags" v-if="template.tags.length > 0">
              <span v-for="tag in template.tags" :key="`${template.templateId}-${tag}`" class="skills-tag">{{ tag }}</span>
            </div>

            <div class="row" style="justify-content: space-between; flex-wrap: wrap; gap: 10px;">
              <span class="muted">{{ t("skills.authorLabel", { value: template.author || t('skills.none') }) }}</span>
              <Button :disabled="skillStore.isSaving || skillStore.isImporting" @click="handleInstallTemplate(template)">
                {{ template.installed ? t("skills.reinstall") : t("skills.install") }}
              </Button>
            </div>
          </div>
        </article>
      </div>
    </section>

    <p v-if="feedback" class="muted">{{ feedback }}</p>
  </div>
</template>
