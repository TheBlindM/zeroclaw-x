<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { storeToRefs } from "pinia";
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import SkillFileTree from "@/components/skills/SkillFileTree.vue";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { parseTags, useSkillStore, type SkillFileEntryItem, type SkillItem, type SkillTemplateItem } from "@/stores/skill";

const skillStore = useSkillStore();
const router = useRouter();
const { activeSkill, activeSkillDetail, enabledCount, importedCount, skills, templates } =
  storeToRefs(skillStore);
const { t } = useI18n();

const search = ref("");
const feedback = ref("");
const previewPath = ref("SKILL.md");
const previewContent = ref("");
const previewLoading = ref(false);
const previewEntry = ref<SkillFileEntryItem | null>(null);
const detailFileStats = computed(() => {
  const totals = {
    files: 0,
    folders: 0,
    assets: 0
  };

  function walk(entries: SkillFileEntryItem[]) {
    for (const entry of entries) {
      if (entry.kind === "directory") {
        totals.folders += 1;
        walk(entry.children);
      } else {
        totals.files += 1;
        if (!entry.previewable) {
          totals.assets += 1;
        }
      }
    }
  }

  walk(activeSkillDetail.value?.fileTree ?? []);
  return totals;
});
const previewFileExtension = computed(() => {
  const path = previewEntry.value?.relativePath ?? "";
  const lastDot = path.lastIndexOf(".");
  return lastDot >= 0 ? path.slice(lastDot + 1).toLowerCase() : "";
});
const previewFileSizeLabel = computed(() => {
  const size = previewEntry.value?.sizeBytes;
  if (typeof size !== "number") {
    return "";
  }
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
});
const previewAssetUrl = computed(() => {
  if (!activeSkillDetail.value || !previewEntry.value || previewEntry.value.kind !== "file" || previewEntry.value.previewable) {
    return "";
  }
  if (!isImagePath(previewEntry.value.relativePath)) {
    return "";
  }
  return convertFileSrc(joinSkillPath(activeSkillDetail.value.directoryPath, previewEntry.value.relativePath));
});

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

const activeTags = computed(() => parseTags(activeSkill.value?.tagsJson ?? "[]"));
const templateCards = computed(() => {
  const installed = new Set(skills.value.map((skill) => skill.slug));
  return templates.value.map((template) => ({
    ...template,
    installed: installed.has(template.slug),
    tags: parseTags(template.tagsJson)
  }));
});

watch(
  activeSkill,
  async (skill) => {
    if (!skill) {
      return;
    }

    if (skillStore.detailSkillId !== skill.id) {
      try {
        await skillStore.loadSkillDetail(skill.id, true);
      } catch {
        feedback.value = t("skills.feedback.loadDetailFailed");
      }
    }

    const firstPreviewable = findFirstPreviewable(skillStore.activeSkillDetail?.fileTree ?? []);
    if (firstPreviewable) {
      await handleSelectPreview(firstPreviewable.relativePath);
    }
  },
  { immediate: true }
);

onMounted(async () => {
  if (!skillStore.loaded) {
    try {
      await skillStore.bootstrap();
    } catch {
      // render store error inline
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

function joinSkillPath(root: string, relativePath: string) {
  const separator = root.includes("\\") ? "\\" : "/";
  return `${root.replace(/[\\/]+$/, "")}${separator}${relativePath.split("/").join(separator)}`;
}

function isImagePath(relativePath: string) {
  return [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg"].some((extension) =>
    relativePath.toLowerCase().endsWith(extension)
  );
}

function handleSelectSkill(skillId: string) {
  feedback.value = "";
  skillStore.setActiveSkill(skillId);
}

function findFileEntry(entries: SkillFileEntryItem[], relativePath: string): SkillFileEntryItem | null {
  for (const entry of entries) {
    if (entry.relativePath === relativePath) {
      return entry;
    }

    const nested = findFileEntry(entry.children, relativePath);
    if (nested) {
      return nested;
    }
  }

  return null;
}

function findFirstPreviewable(entries: SkillFileEntryItem[]): SkillFileEntryItem | null {
  for (const entry of entries) {
    if (entry.kind === "file" && entry.previewable) {
      return entry;
    }

    const nested = findFirstPreviewable(entry.children);
    if (nested) {
      return nested;
    }
  }

  return null;
}

async function handleSelectPreview(relativePath: string) {
  if (!activeSkill.value || !activeSkillDetail.value) {
    return;
  }

  const entry = findFileEntry(activeSkillDetail.value.fileTree, relativePath);
  if (!entry) {
    return;
  }

  previewPath.value = relativePath;
  previewEntry.value = entry;
  previewContent.value = "";

  if (entry.kind === "directory" || !entry.previewable) {
    return;
  }

  previewLoading.value = true;

  try {
    const file = await skillStore.loadSkillFileContent(activeSkill.value.id, relativePath);
    previewContent.value = file.content;
  } catch {
    feedback.value = t("skills.feedback.loadFileFailed");
  } finally {
    previewLoading.value = false;
  }
}

function handleEditSkill(skillId: string) {
  router.push(`/skills/${skillId}/edit`);
}

async function handleImportDirectory() {
  feedback.value = "";

  try {
    const skill = await skillStore.importDirectory();
    feedback.value = skill ? t("skills.feedback.imported", { name: skill.name }) : t("skills.feedback.importCancelled");
  } catch {
    feedback.value = t("skills.feedback.importFailed");
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
    feedback.value = t("skills.feedback.installFailed");
  }
}

async function handleToggleSkill(skill: SkillItem) {
  feedback.value = "";

  try {
    const updated = await skillStore.toggleSkill(skill.id, !skill.enabled);
    feedback.value = updated.enabled ? t("skills.feedback.enabled", { name: skill.name }) : t("skills.feedback.disabled", { name: skill.name });
  } catch {
    feedback.value = t("skills.feedback.toggleFailed");
  }
}

async function handleDuplicateSkill(skill: SkillItem) {
  feedback.value = "";

  try {
    const duplicated = await skillStore.duplicateSkill(skill.id);
    feedback.value = t("skills.feedback.duplicated", { name: duplicated.name });
  } catch {
    feedback.value = t("skills.feedback.duplicateFailed");
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
    feedback.value = t("skills.feedback.refreshFailed");
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
    feedback.value = t("skills.feedback.exportFailed");
  }
}

async function handleOpenDirectory(skill: SkillItem) {
  feedback.value = "";

  try {
    await skillStore.openSkillDirectory(skill.id);
    feedback.value = t("skills.feedback.openedDirectory");
  } catch {
    feedback.value = t("skills.feedback.openDirectoryFailed");
  }
}

async function handleDeleteSkill(skill: SkillItem) {
  const confirmed = window.confirm(t("skills.prompts.deleteSkill", { name: skill.name }));
  if (!confirmed) {
    return;
  }

  feedback.value = "";

  try {
    await skillStore.removeSkill(skill.id);
    feedback.value = t("skills.feedback.deleted", { name: skill.name });
  } catch {
    feedback.value = t("skills.feedback.deleteFailed");
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
            <Button variant="secondary" :disabled="skillStore.isSaving || skillStore.isImporting" @click="router.push('/skills/new')">
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
                  <Button variant="ghost" :disabled="skillStore.isSaving || skillStore.isImporting" @click.stop="handleEditSkill(skill.id)">
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
        <div class="stack" style="gap: 6px;">
          <strong>{{ activeSkill ? t("skills.detailTitle", { name: activeSkill.name }) : t("skills.detailFallback") }}</strong>
          <span class="muted">{{ t("skills.detailDescription") }}</span>
        </div>

        <div v-if="!activeSkill" class="empty-state">
          <strong>{{ t("skills.selectSkillTitle") }}</strong>
          <span class="muted">{{ t("skills.selectSkillDescription") }}</span>
        </div>

        <div v-else-if="!activeSkillDetail" class="empty-state">
          <strong>{{ t("skills.loadingDetailTitle") }}</strong>
          <span class="muted">{{ t("skills.loadingDetailDescription") }}</span>
        </div>

        <div v-else class="stack" style="gap: 16px; min-height: 0;">
          <div class="skills-detail-meta">
            <div class="stack" style="gap: 6px;">
              <strong>{{ activeSkillDetail.skill.name }}</strong>
              <span class="muted">{{ activeSkillDetail.skill.description }}</span>
            </div>
            <div class="skills-tags" v-if="activeTags.length > 0">
              <span v-for="tag in activeTags" :key="tag" class="skills-tag">{{ tag }}</span>
            </div>
          </div>

          <div class="skills-detail-grid">
            <div class="summary-card summary-card--static">
              <span class="muted">{{ t("skills.version") }}</span>
              <strong>{{ activeSkillDetail.skill.version }}</strong>
            </div>
            <div class="summary-card summary-card--static">
              <span class="muted">{{ t("skills.source") }}</span>
              <strong>{{ resolveSourceLabel(activeSkillDetail.skill) }}</strong>
            </div>
            <div class="summary-card summary-card--static">
              <span class="muted">{{ t("skills.slug") }}</span>
              <strong>{{ activeSkillDetail.skill.slug }}</strong>
            </div>
            <div class="summary-card summary-card--static">
              <span class="muted">{{ t("skills.fileCount") }}</span>
              <strong>{{ detailFileStats.files }}</strong>
            </div>
            <div class="summary-card summary-card--static">
              <span class="muted">{{ t("skills.folderCount") }}</span>
              <strong>{{ detailFileStats.folders }}</strong>
            </div>
            <div class="summary-card summary-card--static">
              <span class="muted">{{ t("skills.assetCount") }}</span>
              <strong>{{ detailFileStats.assets }}</strong>
            </div>
          </div>

          <div class="row" style="flex-wrap: wrap;">
            <Button variant="secondary" :disabled="skillStore.isSaving" @click="handleEditSkill(activeSkillDetail.skill.id)">
              {{ t("skills.editSkill") }}
            </Button>
            <Button variant="ghost" :disabled="skillStore.isSaving" @click="handleDuplicateSkill(activeSkillDetail.skill)">
              {{ t("skills.duplicate") }}
            </Button>
            <Button variant="ghost" :disabled="skillStore.isSaving" @click="handleRefreshSkill(activeSkillDetail.skill)">
              {{ t("skills.refresh") }}
            </Button>
            <Button variant="ghost" :disabled="skillStore.isExporting" @click="handleExportSkill(activeSkillDetail.skill)">
              {{ t("skills.export") }}
            </Button>
            <Button variant="ghost" @click="handleOpenDirectory(activeSkillDetail.skill)">
              {{ t("skills.openFolder") }}
            </Button>
          </div>

          <div class="stack skills-paths" style="gap: 10px;">
            <div class="stack" style="gap: 4px;">
              <span class="muted">{{ t("skills.directoryPath") }}</span>
              <code class="skills-path-block">{{ activeSkillDetail.directoryPath }}</code>
            </div>
            <div class="stack" style="gap: 4px;">
              <span class="muted">{{ t("skills.manifestPath") }}</span>
              <code class="skills-path-block">{{ activeSkillDetail.manifestPath }}</code>
            </div>
            <div v-if="activeSkillDetail.sourcePath" class="stack" style="gap: 4px;">
              <span class="muted">{{ t("skills.sourcePath") }}</span>
              <code class="skills-path-block">{{ activeSkillDetail.sourcePath }}</code>
            </div>
          </div>

          <div class="skills-editor-layout">
            <div class="skills-editor-layout__sidebar">
              <SkillFileTree
                :entries="activeSkillDetail.fileTree"
                :selected-path="previewPath"
                @select="handleSelectPreview"
              />
            </div>
            <div class="skills-editor-layout__main">
              <div class="stack" style="gap: 10px;">
                <div class="stack" style="gap: 4px;">
                  <strong>{{ previewEntry?.relativePath || t("skills.noFileSelected") }}</strong>
                  <div class="row" style="flex-wrap: wrap; gap: 8px;">
                    <span class="muted">
                      {{
                        previewEntry?.kind === "directory"
                          ? t("skills.folderSelected")
                          : previewEntry?.previewable
                            ? t("skills.previewMode")
                            : t("skills.fileBinary")
                      }}
                    </span>
                    <span v-if="previewFileExtension" class="skills-inline-badge">{{ previewFileExtension }}</span>
                    <span v-if="previewFileSizeLabel" class="skills-inline-badge">{{ previewFileSizeLabel }}</span>
                  </div>
                </div>

                <div v-if="previewLoading" class="empty-state">
                  <strong>{{ t("skills.loadingFileTitle") }}</strong>
                  <span class="muted">{{ t("skills.loadingFileDescription") }}</span>
                </div>
                <div v-else-if="previewEntry?.kind === 'directory'" class="empty-state">
                  <strong>{{ t("skills.folderSelectedTitle") }}</strong>
                  <span class="muted">{{ t("skills.folderSelectedDescription") }}</span>
                </div>
                <div v-else-if="previewEntry?.kind === 'file' && !previewEntry.previewable" class="empty-state">
                  <template v-if="previewAssetUrl">
                    <div class="stack skills-asset-preview" style="gap: 12px;">
                      <img :src="previewAssetUrl" :alt="previewEntry?.name || 'asset'" class="skills-asset-preview__image" />
                      <div class="row" style="flex-wrap: wrap; gap: 8px;">
                        <span class="skills-inline-badge">{{ t("skills.assetImage") }}</span>
                        <span v-if="previewFileSizeLabel" class="skills-inline-badge">{{ previewFileSizeLabel }}</span>
                      </div>
                    </div>
                  </template>
                  <template v-else>
                    <strong>{{ t("skills.binaryFileTitle") }}</strong>
                    <span class="muted">{{ t("skills.binaryFileDescription") }}</span>
                  </template>
                </div>
                <pre v-else class="code-block skills-editor-preview">{{ previewContent || activeSkillDetail.markdownContent }}</pre>
              </div>
            </div>
          </div>
        </div>
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

    <p v-if="feedback || skillStore.error" class="muted">{{ feedback || skillStore.error }}</p>
  </div>
</template>
