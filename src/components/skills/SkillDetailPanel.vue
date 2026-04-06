<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SkillFileTree from "@/components/skills/SkillFileTree.vue";
import Button from "@/components/ui/Button.vue";
import { parseTags, useSkillStore, type SkillFileEntryItem, type SkillItem } from "@/stores/skill";

const props = defineProps<{
  skill: SkillItem | null;
}>();

const emit = defineEmits<{
  edit: [];
  duplicate: [];
  refresh: [];
  export: [];
  openDirectory: [];
}>();

const skillStore = useSkillStore();
const { t } = useI18n();

const feedback = ref("");
const previewPath = ref("SKILL.md");
const previewContent = ref("");
const previewLoading = ref(false);
const detailLoading = ref(false);
const previewEntry = ref<SkillFileEntryItem | null>(null);

const activeDetail = computed(() => {
  if (!props.skill || skillStore.detailSkillId !== props.skill.id) {
    return null;
  }

  return skillStore.activeSkillDetail;
});
const activeTags = computed(() => parseTags(activeDetail.value?.skill.tagsJson ?? "[]"));
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

  walk(activeDetail.value?.fileTree ?? []);
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
  if (!activeDetail.value || !previewEntry.value || previewEntry.value.kind !== "file" || previewEntry.value.previewable) {
    return "";
  }
  if (!isImagePath(previewEntry.value.relativePath)) {
    return "";
  }
  return convertFileSrc(joinSkillPath(activeDetail.value.directoryPath, previewEntry.value.relativePath));
});

function resolveActionError(fallbackKey: string) {
  const message = skillStore.error || t(fallbackKey);
  skillStore.clearError();
  return message;
}

function resolveSourceLabel(skill: SkillItem) {
  if (skill.sourceKind === "template") {
    return t("skills.sourceTemplate");
  }

  if (skill.sourceKind === "imported") {
    return t("skills.sourceImported");
  }

  return t("skills.sourceManual");
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

async function loadDetail(skillId: string) {
  detailLoading.value = true;
  feedback.value = "";
  previewPath.value = "";
  previewEntry.value = null;
  previewContent.value = "";
  previewLoading.value = false;

  try {
    const detail = await skillStore.loadSkillDetail(skillId, true);
    const firstPreviewable = findFirstPreviewable(detail.fileTree);
    if (firstPreviewable) {
      await handleSelectPreview(firstPreviewable.relativePath);
    }
  } catch {
    feedback.value = resolveActionError("skills.feedback.loadDetailFailed");
  } finally {
    detailLoading.value = false;
  }
}

async function handleSelectPreview(relativePath: string) {
  if (!props.skill || !activeDetail.value) {
    return;
  }

  const entry = findFileEntry(activeDetail.value.fileTree, relativePath);
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
    const file = await skillStore.loadSkillFileContent(props.skill.id, relativePath);
    previewContent.value = file.content;
  } catch {
    feedback.value = resolveActionError("skills.feedback.loadFileFailed");
  } finally {
    previewLoading.value = false;
  }
}

watch(
  () => props.skill?.id ?? "",
  async (skillId) => {
    if (!skillId) {
      feedback.value = "";
      previewPath.value = "";
      previewEntry.value = null;
      previewContent.value = "";
      previewLoading.value = false;
      detailLoading.value = false;
      return;
    }

    await loadDetail(skillId);
  },
  { immediate: true }
);

watch(
  () => activeDetail.value?.fileTree,
  async (fileTree) => {
    if (!fileTree || !previewPath.value) {
      return;
    }

    const nextEntry = findFileEntry(fileTree, previewPath.value);
    if (nextEntry) {
      previewEntry.value = nextEntry;
      return;
    }

    const firstPreviewable = findFirstPreviewable(fileTree);
    if (firstPreviewable) {
      await handleSelectPreview(firstPreviewable.relativePath);
      return;
    }

    previewPath.value = "";
    previewEntry.value = null;
    previewContent.value = "";
  }
);
</script>

<template>
  <div class="stack" style="gap: 16px; min-height: 0;">
    <div class="stack" style="gap: 6px;">
      <strong>{{ skill ? t("skills.detailTitle", { name: skill.name }) : t("skills.detailFallback") }}</strong>
      <span class="muted">{{ t("skills.detailDescription") }}</span>
    </div>

    <div v-if="!skill" class="empty-state">
      <strong>{{ t("skills.selectSkillTitle") }}</strong>
      <span class="muted">{{ t("skills.selectSkillDescription") }}</span>
    </div>

    <div v-else-if="detailLoading || !activeDetail" class="empty-state">
      <strong>{{ t("skills.loadingDetailTitle") }}</strong>
      <span class="muted">{{ feedback || t("skills.loadingDetailDescription") }}</span>
    </div>

    <template v-else>
      <div class="skills-detail-meta">
        <div class="stack" style="gap: 6px;">
          <strong>{{ activeDetail.skill.name }}</strong>
          <span class="muted">{{ activeDetail.skill.description }}</span>
        </div>
        <div class="skills-tags" v-if="activeTags.length > 0">
          <span v-for="tag in activeTags" :key="tag" class="skills-tag">{{ tag }}</span>
        </div>
      </div>

      <div class="skills-detail-grid">
        <div class="summary-card summary-card--static">
          <span class="muted">{{ t("skills.version") }}</span>
          <strong>{{ activeDetail.skill.version }}</strong>
        </div>
        <div class="summary-card summary-card--static">
          <span class="muted">{{ t("skills.source") }}</span>
          <strong>{{ resolveSourceLabel(activeDetail.skill) }}</strong>
        </div>
        <div class="summary-card summary-card--static">
          <span class="muted">{{ t("skills.slug") }}</span>
          <strong>{{ activeDetail.skill.slug }}</strong>
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
        <Button variant="secondary" :disabled="skillStore.isSaving" @click="emit('edit')">
          {{ t("skills.editSkill") }}
        </Button>
        <Button variant="ghost" :disabled="skillStore.isSaving" @click="emit('duplicate')">
          {{ t("skills.duplicate") }}
        </Button>
        <Button variant="ghost" :disabled="skillStore.isSaving" @click="emit('refresh')">
          {{ t("skills.refresh") }}
        </Button>
        <Button variant="ghost" :disabled="skillStore.isExporting" @click="emit('export')">
          {{ t("skills.export") }}
        </Button>
        <Button variant="ghost" @click="emit('openDirectory')">
          {{ t("skills.openFolder") }}
        </Button>
      </div>

      <div class="stack skills-paths" style="gap: 10px;">
        <div class="stack" style="gap: 4px;">
          <span class="muted">{{ t("skills.directoryPath") }}</span>
          <code class="skills-path-block">{{ activeDetail.directoryPath }}</code>
        </div>
        <div class="stack" style="gap: 4px;">
          <span class="muted">{{ t("skills.manifestPath") }}</span>
          <code class="skills-path-block">{{ activeDetail.manifestPath }}</code>
        </div>
        <div v-if="activeDetail.sourcePath" class="stack" style="gap: 4px;">
          <span class="muted">{{ t("skills.sourcePath") }}</span>
          <code class="skills-path-block">{{ activeDetail.sourcePath }}</code>
        </div>
      </div>

      <div class="skills-editor-layout">
        <div class="skills-editor-layout__sidebar">
          <SkillFileTree
            :entries="activeDetail.fileTree"
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
            <div v-else-if="!previewEntry" class="empty-state">
              <strong>{{ t("skills.noFileSelected") }}</strong>
              <span class="muted">{{ t("skills.noPreviewAvailableDescription") }}</span>
            </div>
            <pre v-else class="code-block skills-editor-preview">{{ previewContent || activeDetail.markdownContent }}</pre>
          </div>
        </div>
      </div>

      <p v-if="feedback" class="muted">{{ feedback }}</p>
    </template>
  </div>
</template>
