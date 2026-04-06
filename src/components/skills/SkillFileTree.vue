<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { SkillFileEntryItem } from "@/stores/skill";

const props = defineProps<{
  entries: SkillFileEntryItem[];
  selectedPath: string;
}>();

const emit = defineEmits<{
  select: [path: string];
}>();
const { t } = useI18n();

interface FlattenedSkillFileEntry {
  entry: SkillFileEntryItem;
  depth: number;
}

const expandedPaths = ref<Set<string>>(new Set());

function collectExpandedAncestors(entries: SkillFileEntryItem[], targetPath: string, ancestors: string[] = []): string[] | null {
  for (const entry of entries) {
    const nextAncestors = entry.kind === "directory" ? [...ancestors, entry.relativePath] : ancestors;

    if (entry.relativePath === targetPath) {
      return ancestors;
    }

    const nested = collectExpandedAncestors(entry.children, targetPath, nextAncestors);
    if (nested) {
      return nested;
    }
  }

  return null;
}

function syncExpandedState() {
  const next = new Set<string>();
  for (const entry of props.entries) {
    if (entry.kind === "directory") {
      next.add(entry.relativePath);
    }
  }

  const ancestors = collectExpandedAncestors(props.entries, props.selectedPath) ?? [];
  for (const path of ancestors) {
    next.add(path);
  }

  for (const path of expandedPaths.value) {
    next.add(path);
  }

  expandedPaths.value = next;
}

function flattenSkillEntries(entries: SkillFileEntryItem[], depth = 0): FlattenedSkillFileEntry[] {
  return entries.flatMap((entry) => {
    const flattened: FlattenedSkillFileEntry[] = [{ entry, depth }];
    if (entry.kind === "directory" && expandedPaths.value.has(entry.relativePath)) {
      flattened.push(...flattenSkillEntries(entry.children, depth + 1));
    }
    return flattened;
  });
}

const flattenedEntries = computed(() => flattenSkillEntries(props.entries));
watch(() => props.entries, syncExpandedState, { immediate: true });
watch(() => props.selectedPath, syncExpandedState);

function handleSelect(path: string) {
  emit("select", path);
}

function handleDirectoryToggle(entry: SkillFileEntryItem) {
  if (expandedPaths.value.has(entry.relativePath)) {
    expandedPaths.value.delete(entry.relativePath);
  } else {
    expandedPaths.value.add(entry.relativePath);
  }
}
</script>

<template>
  <div class="skills-tree" v-if="flattenedEntries.length > 0">
    <div
      v-for="{ entry, depth } in flattenedEntries"
      :key="entry.relativePath"
      class="skills-tree__row"
      :data-root="depth === 0"
      :style="{ paddingLeft: `${12 + depth * 18}px` }"
    >
      <button
        v-if="entry.kind === 'directory'"
        type="button"
        class="skills-tree__toggle"
        :aria-label="expandedPaths.has(entry.relativePath) ? t('skills.collapseFolder') : t('skills.expandFolder')"
        @click="handleDirectoryToggle(entry)"
      >
        {{ expandedPaths.has(entry.relativePath) ? "▾" : "▸" }}
      </button>
      <span v-else class="skills-tree__toggle skills-tree__toggle--spacer" />
      <button
        type="button"
        class="skills-tree__item"
        :data-active="entry.relativePath === selectedPath"
        :data-root="depth === 0"
        @click="handleSelect(entry.relativePath)"
      >
        <span class="skills-tree__kind">{{ entry.kind === "directory" ? "DIR" : "FILE" }}</span>
        <span class="skills-tree__name">{{ entry.name }}</span>
        <span v-if="entry.kind === 'file' && !entry.editable && entry.previewable" class="skills-tree__meta">{{ t("skills.readOnly") }}</span>
        <span v-else-if="entry.kind === 'file' && !entry.previewable" class="skills-tree__meta">{{ t("skills.assetFile") }}</span>
      </button>
    </div>
  </div>
  <div v-else class="empty-state">
    <strong>{{ t("skills.noFilesTitle") }}</strong>
    <span class="muted">{{ t("skills.noFilesDescription") }}</span>
  </div>
</template>
