<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { onBeforeRouteLeave } from "vue-router";
import SkillFileTree from "@/components/skills/SkillFileTree.vue";
import Button from "@/components/ui/Button.vue";
import { parseTags, useSkillStore, type SkillFileEntryItem } from "@/stores/skill";

const PROTECTED_CREATE_ROOT_PATHS = new Set(["SKILL.md", "SKILL.toml", "scripts", "references", "assets"]);
const DEFAULT_CREATE_VERSION = "0.1.0";
const DEFAULT_CREATE_PATHS = ["SKILL.md", "SKILL.toml", "assets", "references", "scripts"];

const props = defineProps<{
  mode: "create" | "edit";
  skillId?: string;
}>();

const emit = defineEmits<{
  cancel: [];
  created: [payload: { skillId: string; name: string }];
  dirtyChange: [value: boolean];
}>();

const skillStore = useSkillStore();
const { t } = useI18n();

const feedback = ref("");
const fileFeedback = ref("");
const ready = ref(false);
const fileLoading = ref(false);
const createEntryMode = ref<"file" | "directory" | null>(null);
const selectedPath = ref("SKILL.md");
const editorContent = ref("");
const fileDirty = ref(false);
const selectedEntry = ref<SkillFileEntryItem | null>(null);
const draftFileTree = ref<SkillFileEntryItem[]>([]);
const draftFileContents = ref<Record<string, string>>({});
const draftMarkdownTouched = ref(false);
const suppressUnsavedGuard = ref(false);
const createEntryForm = reactive({
  path: ""
});

const isCreateMode = computed(() => props.mode === "create");
const currentSkillId = computed(() => props.skillId ?? "");
const activeDetail = computed(() => {
  if (isCreateMode.value) {
    return null;
  }

  if (!currentSkillId.value || skillStore.detailSkillId !== currentSkillId.value) {
    return null;
  }

  return skillStore.activeSkillDetail;
});
const currentFileTree = computed(() => (isCreateMode.value ? draftFileTree.value : activeDetail.value?.fileTree ?? []));
const currentDirectoryPath = computed(() => (isCreateMode.value ? "" : activeDetail.value?.directoryPath ?? ""));
const primaryActionLabel = computed(() => (isCreateMode.value ? t("skills.createSkill") : t("skills.saveMetadata")));
const modeTitle = computed(() => (isCreateMode.value ? t("skills.createTitle") : t("skills.editTitle")));
const modeDescription = computed(() => (isCreateMode.value ? t("skills.createUnifiedDescription") : t("skills.editDescription")));
const canDeleteSelected = computed(() => {
  if (!selectedEntry.value) {
    return false;
  }

  if (isCreateMode.value) {
    return !PROTECTED_CREATE_ROOT_PATHS.has(selectedEntry.value.relativePath);
  }

  return selectedEntry.value.relativePath !== "SKILL.md" && selectedEntry.value.relativePath !== "SKILL.toml";
});
const metadataPreview = computed(() =>
  [
    "[skill]",
    `name = "${form.name.trim() || "Skill"}"`,
    `description = "${form.description.trim() || ""}"`,
    `version = "${form.version.trim() || "0.1.0"}"`,
    form.author.trim() ? `author = "${form.author.trim()}"` : "",
    `tags = [${splitTags(form.tags)
      .map((tag) => `"${tag}"`)
      .join(", ")}]`
  ]
    .filter(Boolean)
    .join("\n")
);
const hasMetadataChanges = computed(() => {
  if (isCreateMode.value) {
    const currentPaths = collectRelativePaths(draftFileTree.value);
    const hasDraftFilesChanged =
      currentPaths.length !== DEFAULT_CREATE_PATHS.length ||
      currentPaths.some((path, index) => path !== DEFAULT_CREATE_PATHS[index]);
    const hasDraftContentChanged = Object.entries(draftFileContents.value).some(([relativePath, content]) => {
      if (relativePath === "SKILL.md") {
        return normalizeMarkdownContent(content) !== buildDefaultMarkdown();
      }

      return content !== "";
    });

    return (
      form.name.trim() !== "" ||
      form.slug.trim() !== "" ||
      form.description.trim() !== "" ||
      form.version.trim() !== DEFAULT_CREATE_VERSION ||
      form.author.trim() !== "" ||
      form.tags.trim() !== "" ||
      form.enabled !== true ||
      hasDraftFilesChanged ||
      hasDraftContentChanged
    );
  }

  if (!activeDetail.value) {
    return false;
  }

  return (
    form.name.trim() !== activeDetail.value.skill.name ||
    form.slug.trim() !== activeDetail.value.skill.slug ||
    form.description.trim() !== activeDetail.value.skill.description ||
    form.version.trim() !== activeDetail.value.skill.version ||
    form.author.trim() !== activeDetail.value.skill.author ||
    JSON.stringify(splitTags(form.tags)) !== JSON.stringify(parseTags(activeDetail.value.skill.tagsJson)) ||
    form.enabled !== activeDetail.value.skill.enabled
  );
});
const hasUnsavedChanges = computed(() => fileDirty.value || hasMetadataChanges.value);
const selectedDirectory = computed(() => {
  if (!selectedEntry.value) {
    return "";
  }

  if (selectedEntry.value.kind === "directory") {
    return selectedEntry.value.relativePath;
  }

  const segments = selectedEntry.value.relativePath.split("/");
  segments.pop();
  return segments.join("/");
});
const selectedFileExtension = computed(() => {
  const path = selectedEntry.value?.relativePath ?? "";
  const lastDot = path.lastIndexOf(".");
  return lastDot >= 0 ? path.slice(lastDot + 1).toLowerCase() : "";
});
const selectedFileSizeLabel = computed(() => formatSize(selectedEntry.value?.sizeBytes));
const selectedFileStatusLabel = computed(() => {
  if (selectedEntry.value?.kind === "directory") {
    return t("skills.folderSelected");
  }
  if (selectedEntry.value?.editable) {
    return t("skills.fileEditable");
  }
  if (selectedEntry.value?.previewable) {
    return t("skills.fileReadOnly");
  }
  return t("skills.fileBinary");
});
const selectedAssetUrl = computed(() => {
  if (
    isCreateMode.value ||
    !currentDirectoryPath.value ||
    !selectedEntry.value ||
    selectedEntry.value.kind !== "file" ||
    selectedEntry.value.previewable ||
    !isImagePath(selectedEntry.value.relativePath)
  ) {
    return "";
  }

  return convertFileSrc(joinSkillPath(currentDirectoryPath.value, selectedEntry.value.relativePath));
});

const form = reactive({
  name: "",
  slug: "",
  description: "",
  version: "0.1.0",
  author: "",
  tags: "",
  enabled: true
});

function splitTags(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function collectRelativePaths(entries: SkillFileEntryItem[]) {
  const paths: string[] = [];

  const visit = (nodeEntries: SkillFileEntryItem[]) => {
    for (const entry of nodeEntries) {
      paths.push(entry.relativePath);
      visit(entry.children);
    }
  };

  visit(entries);
  return paths.sort((left, right) => left.localeCompare(right));
}

function normalizeMarkdownContent(markdown: string) {
  const trimmed = markdown.trim();
  return trimmed ? `${trimmed}\n` : "";
}

function buildDefaultMarkdown() {
  const title = `# ${form.name.trim() || "New Skill"}`;
  const description = form.description.trim();
  return normalizeMarkdownContent([title, description].filter(Boolean).join("\n\n"));
}

function formatSize(size?: number | null) {
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

function isPreviewableTextPath(relativePath: string) {
  const normalized = relativePath.toLowerCase();
  return (
    normalized === "skill.md" ||
    normalized === "skill.toml" ||
    [
      ".md",
      ".markdown",
      ".txt",
      ".json",
      ".toml",
      ".yaml",
      ".yml",
      ".rs",
      ".ts",
      ".tsx",
      ".js",
      ".jsx",
      ".vue",
      ".py",
      ".sh",
      ".bash",
      ".zsh",
      ".css",
      ".html"
    ].some((extension) => normalized.endsWith(extension))
  );
}

function isEditableTextPath(relativePath: string) {
  return relativePath !== "SKILL.toml" && isPreviewableTextPath(relativePath);
}

function skillEntryOrder(relativePath: string, kind: SkillFileEntryItem["kind"]) {
  const rootName = relativePath.split("/")[0] ?? relativePath;
  const bucket = rootName === "SKILL.md"
    ? 0
    : rootName === "SKILL.toml"
      ? 1
      : rootName === "scripts"
        ? 2
        : rootName === "references"
          ? 3
          : rootName === "assets"
            ? 4
            : rootName === "agents"
              ? 5
              : 6;
  return [bucket, kind === "directory" ? 0 : 1] as const;
}

function cloneSkillFileEntry(entry: SkillFileEntryItem): SkillFileEntryItem {
  return {
    ...entry,
    children: entry.children.map(cloneSkillFileEntry)
  };
}

function sortSkillFileEntries(entries: SkillFileEntryItem[]) {
  entries.sort((left, right) => {
    const [leftBucket, leftKind] = skillEntryOrder(left.relativePath, left.kind);
    const [rightBucket, rightKind] = skillEntryOrder(right.relativePath, right.kind);
    return leftBucket - rightBucket || leftKind - rightKind || left.name.localeCompare(right.name);
  });

  for (const entry of entries) {
    if (entry.kind === "directory") {
      sortSkillFileEntries(entry.children);
    }
  }
}

function makeDraftDirectory(relativePath: string): SkillFileEntryItem {
  const segments = relativePath.split("/");
  return {
    name: segments[segments.length - 1] ?? relativePath,
    relativePath,
    kind: "directory",
    editable: false,
    previewable: false,
    sizeBytes: null,
    children: []
  };
}

function makeDraftFile(relativePath: string, content = ""): SkillFileEntryItem {
  const segments = relativePath.split("/");
  const previewable = isPreviewableTextPath(relativePath);
  const editable = isEditableTextPath(relativePath);
  return {
    name: segments[segments.length - 1] ?? relativePath,
    relativePath,
    kind: "file",
    editable,
    previewable,
    sizeBytes: previewable ? content.length : null,
    children: []
  };
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

function createDefaultDraftTree() {
  return [
    makeDraftFile("SKILL.md", buildDefaultMarkdown()),
    makeDraftFile("SKILL.toml", metadataPreview.value),
    makeDraftDirectory("scripts"),
    makeDraftDirectory("references"),
    makeDraftDirectory("assets")
  ];
}

function updateDraftFileSize(relativePath: string, sizeBytes: number | null) {
  const nextTree = draftFileTree.value.map(cloneSkillFileEntry);
  const entry = findFileEntry(nextTree, relativePath);
  if (entry && entry.kind === "file") {
    entry.sizeBytes = sizeBytes;
    draftFileTree.value = nextTree;
  }
}

function setDraftFileContent(relativePath: string, content: string) {
  draftFileContents.value = {
    ...draftFileContents.value,
    [relativePath]: content
  };
  updateDraftFileSize(relativePath, content.length);
}

function resolveActionError(fallbackKey: string) {
  const message = skillStore.error || t(fallbackKey);
  skillStore.clearError();
  return message;
}

function validateMetadataForm() {
  if (!form.name.trim()) {
    feedback.value = t("skills.validation.nameRequired");
    return false;
  }

  if (!form.description.trim()) {
    feedback.value = t("skills.validation.descriptionRequired");
    return false;
  }

  return true;
}

function hasUnsavedNonMarkdownFileChanges() {
  return (
    fileDirty.value &&
    selectedEntry.value?.kind === "file" &&
    selectedEntry.value.relativePath !== "SKILL.md"
  );
}

function shouldWarnAboutUnsavedChanges() {
  return !suppressUnsavedGuard.value && hasUnsavedChanges.value;
}

function confirmLeaveEditor() {
  if (!shouldWarnAboutUnsavedChanges()) {
    return true;
  }

  return window.confirm(t("skills.prompts.leaveEditorWithUnsavedChanges"));
}

function syncFormFromDetail() {
  if (!activeDetail.value) {
    return;
  }

  form.name = activeDetail.value.skill.name;
  form.slug = activeDetail.value.skill.slug;
  form.description = activeDetail.value.skill.description;
  form.version = activeDetail.value.skill.version;
  form.author = activeDetail.value.skill.author;
  form.tags = parseTags(activeDetail.value.skill.tagsJson).join(", ");
  form.enabled = activeDetail.value.skill.enabled;
}

function initializeCreateMode() {
  ready.value = false;
  feedback.value = "";
  fileFeedback.value = "";
  skillStore.clearError();
  fileDirty.value = false;
  fileLoading.value = false;
  draftMarkdownTouched.value = false;
  createEntryMode.value = null;
  createEntryForm.path = "";

  form.name = "";
  form.slug = "";
  form.description = "";
  form.version = "0.1.0";
  form.author = "";
  form.tags = "";
  form.enabled = true;

  const markdown = buildDefaultMarkdown();
  draftFileContents.value = {
    "SKILL.md": markdown
  };
  draftFileTree.value = createDefaultDraftTree();
  selectedPath.value = "SKILL.md";
  selectedEntry.value = findFileEntry(draftFileTree.value, "SKILL.md");
  editorContent.value = markdown;
  ready.value = true;
}

async function loadExistingSkill() {
  if (!currentSkillId.value) {
    ready.value = false;
    return;
  }

  ready.value = false;
  feedback.value = "";
  fileFeedback.value = "";
  skillStore.clearError();
  fileDirty.value = false;
  fileLoading.value = false;
  createEntryMode.value = null;
  createEntryForm.path = "";

  try {
    if (!skillStore.loaded) {
      await skillStore.bootstrap();
    }

    await skillStore.loadSkillDetail(currentSkillId.value, true);
    syncFormFromDetail();
    const firstPreviewable = findFirstPreviewable(activeDetail.value?.fileTree ?? []);
    await openFile(firstPreviewable?.relativePath ?? "SKILL.md", { force: true });
    ready.value = true;
  } catch {
    feedback.value = resolveActionError("skills.feedback.loadDetailFailed");
  }
}

async function openFile(relativePath: string, options?: { force?: boolean }) {
  const force = options?.force ?? false;
  const nextEntry = findFileEntry(currentFileTree.value, relativePath);
  if (!nextEntry) {
    return;
  }

  if (!force && fileDirty.value && relativePath !== selectedPath.value) {
    const confirmed = window.confirm(t("skills.prompts.discardFileChanges"));
    if (!confirmed) {
      return;
    }
  }

  selectedPath.value = relativePath;
  selectedEntry.value = nextEntry;
  fileFeedback.value = "";
  fileDirty.value = false;

  if (nextEntry.kind === "directory") {
    editorContent.value = "";
    return;
  }

  if (isCreateMode.value) {
    editorContent.value =
      relativePath === "SKILL.toml" ? metadataPreview.value : (draftFileContents.value[relativePath] ?? "");
    return;
  }

  if (!nextEntry.previewable) {
    editorContent.value = "";
    return;
  }

  fileLoading.value = true;

  try {
    const file = await skillStore.loadSkillFileContent(currentSkillId.value, relativePath);
    editorContent.value = file.content;
  } catch {
    fileFeedback.value = resolveActionError("skills.feedback.loadFileFailed");
  } finally {
    fileLoading.value = false;
  }
}

function parsePromptPath(value: string) {
  const trimmed = value.trim().replace(/^\/+|\/+$/g, "");
  if (!trimmed) {
    return null;
  }

  const parts = trimmed.split("/").filter(Boolean);
  const name = parts.pop() ?? "";
  return {
    parentPath: parts.join("/"),
    name
  };
}

function addDraftEntry(parentPath: string, name: string, entryKind: "file" | "directory") {
  const relativePath = parentPath ? `${parentPath}/${name}` : name;
  const nextTree = draftFileTree.value.map(cloneSkillFileEntry);
  const parent = parentPath ? findFileEntry(nextTree, parentPath) : null;

  if (parentPath && (!parent || parent.kind !== "directory")) {
    throw new Error("parent missing");
  }

  const collection = parentPath && parent?.kind === "directory" ? parent.children : nextTree;
  if (collection.some((entry) => entry.relativePath === relativePath)) {
    throw new Error("duplicate entry");
  }

  if (entryKind === "directory") {
    collection.push(makeDraftDirectory(relativePath));
  } else {
    const initialContent = "";
    collection.push(makeDraftFile(relativePath, initialContent));
    if (isPreviewableTextPath(relativePath) && relativePath !== "SKILL.toml") {
      setDraftFileContent(relativePath, initialContent);
    }
  }

  sortSkillFileEntries(nextTree);
  draftFileTree.value = nextTree;
}

function removeDraftEntry(relativePath: string) {
  const removeRecursively = (entries: SkillFileEntryItem[]): SkillFileEntryItem[] =>
    entries
      .filter((entry) => entry.relativePath !== relativePath)
      .map((entry) => ({
        ...entry,
        children: entry.kind === "directory" ? removeRecursively(entry.children) : entry.children
      }));

  const nextTree = removeRecursively(draftFileTree.value);
  const nextContents = { ...draftFileContents.value };

  for (const path of Object.keys(nextContents)) {
    if (path === relativePath || path.startsWith(`${relativePath}/`)) {
      delete nextContents[path];
    }
  }

  draftFileTree.value = nextTree;
  draftFileContents.value = nextContents;
}

function collectDraftEntries(entries: SkillFileEntryItem[]) {
  const directories: Array<{ parentPath: string; name: string }> = [];
  const files: Array<{ parentPath: string; name: string; relativePath: string; content: string }> = [];

  const visit = (nodeEntries: SkillFileEntryItem[]) => {
    for (const entry of nodeEntries) {
      if (entry.kind === "directory") {
        if (!PROTECTED_CREATE_ROOT_PATHS.has(entry.relativePath)) {
          const segments = entry.relativePath.split("/");
          const name = segments.pop() ?? entry.relativePath;
          directories.push({
            parentPath: segments.join("/"),
            name
          });
        }
        visit(entry.children);
      } else if (entry.relativePath !== "SKILL.md" && entry.relativePath !== "SKILL.toml") {
        const segments = entry.relativePath.split("/");
        const name = segments.pop() ?? entry.relativePath;
        files.push({
          parentPath: segments.join("/"),
          name,
          relativePath: entry.relativePath,
          content: draftFileContents.value[entry.relativePath] ?? ""
        });
      }
    }
  };

  visit(entries);

  directories.sort((left, right) => left.parentPath.split("/").filter(Boolean).length - right.parentPath.split("/").filter(Boolean).length);
  files.sort((left, right) => left.relativePath.localeCompare(right.relativePath));

  return { directories, files };
}

async function persistDraftSelectedFile() {
  if (!selectedEntry.value || selectedEntry.value.kind !== "file" || !selectedEntry.value.editable) {
    return;
  }

  const content = selectedEntry.value.relativePath === "SKILL.md"
    ? normalizeMarkdownContent(editorContent.value)
    : editorContent.value;

  setDraftFileContent(selectedEntry.value.relativePath, content);
  if (selectedEntry.value.relativePath === "SKILL.md") {
    draftMarkdownTouched.value = true;
  }
  editorContent.value = content;
  fileDirty.value = false;
}

async function handleCreateSkillPackage() {
  feedback.value = "";
  fileFeedback.value = "";

  if (!validateMetadataForm()) {
    return;
  }

  try {
    if (fileDirty.value) {
      await persistDraftSelectedFile();
    }

    const created = await skillStore.createSkill({
      slug: form.slug.trim(),
      name: form.name.trim(),
      description: form.description.trim(),
      version: form.version.trim(),
      author: form.author.trim(),
      tags_json: JSON.stringify(splitTags(form.tags)),
      markdown_content: draftFileContents.value["SKILL.md"] ?? buildDefaultMarkdown(),
      enabled: form.enabled
    });

    const draftEntries = collectDraftEntries(draftFileTree.value);

    for (const directory of draftEntries.directories) {
      await skillStore.createSkillEntry(created.id, {
        parent_path: directory.parentPath,
        name: directory.name,
        entry_kind: "directory"
      });
    }

    for (const file of draftEntries.files) {
      await skillStore.createSkillEntry(created.id, {
        parent_path: file.parentPath,
        name: file.name,
        entry_kind: "file"
      });

      if (isEditableTextPath(file.relativePath)) {
        await skillStore.saveSkillFileContent(created.id, file.relativePath, file.content);
      }
    }

    emit("created", { skillId: created.id, name: created.name });
  } catch {
    feedback.value = resolveActionError("skills.feedback.createFailed");
  }
}

async function handleSaveMetadata() {
  feedback.value = "";
  fileFeedback.value = "";

  if (!validateMetadataForm()) {
    return;
  }

  if (hasUnsavedNonMarkdownFileChanges()) {
    feedback.value = t("skills.feedback.saveFileBeforeMetadata", {
      path: selectedEntry.value?.relativePath ?? selectedPath.value ?? "SKILL.md"
    });
    return;
  }

  try {
    await skillStore.updateSkill(currentSkillId.value, {
      slug: form.slug.trim(),
      name: form.name.trim(),
      description: form.description.trim(),
      version: form.version.trim(),
      author: form.author.trim(),
      tags_json: JSON.stringify(splitTags(form.tags)),
      markdown_content:
        selectedPath.value === "SKILL.md" && selectedEntry.value?.kind === "file" ? editorContent.value : activeDetail.value?.markdownContent ?? "",
      enabled: form.enabled
    });
    await skillStore.loadSkillDetail(currentSkillId.value, true);
    syncFormFromDetail();
    if (selectedPath.value === "SKILL.md") {
      selectedEntry.value = findFileEntry(skillStore.activeSkillDetail?.fileTree ?? [], "SKILL.md");
      editorContent.value = activeDetail.value?.markdownContent ?? "";
      fileDirty.value = false;
    }
    feedback.value = t("skills.feedback.updated", { name: form.name.trim() });
  } catch {
    feedback.value = resolveActionError("skills.feedback.updateFailed");
  }
}

async function handlePrimaryAction() {
  if (isCreateMode.value) {
    await handleCreateSkillPackage();
  } else {
    await handleSaveMetadata();
  }
}

async function handleSaveFile() {
  if (!selectedEntry.value || selectedEntry.value.kind !== "file" || !selectedEntry.value.editable) {
    return;
  }

  fileFeedback.value = "";

  try {
    if (isCreateMode.value) {
      await persistDraftSelectedFile();
      fileFeedback.value = t("skills.feedback.fileSaved", { path: selectedEntry.value.relativePath });
      return;
    }

    const saved = await skillStore.saveSkillFileContent(currentSkillId.value, selectedEntry.value.relativePath, editorContent.value);
    editorContent.value = saved.content;
    fileDirty.value = false;
    await skillStore.loadSkillDetail(currentSkillId.value, true);
    selectedEntry.value = findFileEntry(skillStore.activeSkillDetail?.fileTree ?? [], saved.relativePath);
    fileFeedback.value = t("skills.feedback.fileSaved", { path: saved.relativePath });
  } catch {
    fileFeedback.value = resolveActionError("skills.feedback.fileSaveFailed");
  }
}

function startCreateEntry(entryKind: "file" | "directory") {
  createEntryMode.value = entryKind;
  createEntryForm.path = selectedDirectory.value ? `${selectedDirectory.value}/` : "";
  fileFeedback.value = "";
}

function cancelCreateEntry() {
  createEntryMode.value = null;
  createEntryForm.path = "";
}

async function submitCreateEntry() {
  if (!createEntryMode.value) {
    return;
  }

  const parsed = parsePromptPath(createEntryForm.path);
  if (!parsed?.name) {
    fileFeedback.value = t("skills.feedback.createEntryFailed");
    return;
  }

  fileFeedback.value = "";

  try {
    if (isCreateMode.value) {
      addDraftEntry(parsed.parentPath, parsed.name, createEntryMode.value);
      if (createEntryMode.value === "file") {
        const relativePath = parsed.parentPath ? `${parsed.parentPath}/${parsed.name}` : parsed.name;
        await openFile(relativePath, { force: true });
      } else {
        selectedEntry.value = findFileEntry(draftFileTree.value, parsed.parentPath ? `${parsed.parentPath}/${parsed.name}` : parsed.name);
      }
      fileFeedback.value = t("skills.feedback.entryCreated", { name: parsed.name });
      cancelCreateEntry();
      return;
    }

    await skillStore.createSkillEntry(currentSkillId.value, {
      parent_path: parsed.parentPath,
      name: parsed.name,
      entry_kind: createEntryMode.value
    });
    if (createEntryMode.value === "file") {
      const relativePath = parsed.parentPath ? `${parsed.parentPath}/${parsed.name}` : parsed.name;
      await openFile(relativePath, { force: true });
    } else {
      selectedEntry.value = findFileEntry(skillStore.activeSkillDetail?.fileTree ?? [], parsed.parentPath ? `${parsed.parentPath}/${parsed.name}` : parsed.name);
    }
    fileFeedback.value = t("skills.feedback.entryCreated", { name: parsed.name });
    cancelCreateEntry();
  } catch {
    fileFeedback.value = resolveActionError("skills.feedback.createEntryFailed");
  }
}

async function handleDeleteSelected() {
  if (!selectedEntry.value) {
    return;
  }

  const confirmed = window.confirm(t("skills.prompts.deleteEntry", { path: selectedEntry.value.relativePath }));
  if (!confirmed) {
    return;
  }

  const deletedPath = selectedEntry.value.relativePath;
  fileFeedback.value = "";

  try {
    if (isCreateMode.value) {
      removeDraftEntry(deletedPath);
      const fallback = findFirstPreviewable(draftFileTree.value);
      if (fallback) {
        await openFile(fallback.relativePath, { force: true });
      } else {
        selectedPath.value = "";
        selectedEntry.value = null;
        editorContent.value = "";
      }
      fileFeedback.value = t("skills.feedback.entryDeleted", { path: deletedPath });
      return;
    }

    const detail = await skillStore.deleteSkillEntry(currentSkillId.value, deletedPath);
    const fallback = findFirstPreviewable(detail.fileTree);
    if (fallback) {
      await openFile(fallback.relativePath, { force: true });
    } else {
      selectedPath.value = "";
      selectedEntry.value = null;
      editorContent.value = "";
    }
    fileFeedback.value = t("skills.feedback.entryDeleted", { path: deletedPath });
  } catch {
    fileFeedback.value = resolveActionError("skills.feedback.deleteEntryFailed");
  }
}

async function handleImportAssets() {
  if (isCreateMode.value) {
    fileFeedback.value = t("skills.feedback.importAssetsAfterCreate");
    return;
  }

  fileFeedback.value = "";

  try {
    const report = await skillStore.importSkillAssets(currentSkillId.value);
    if (!report) {
      return;
    }

    await skillStore.loadSkillDetail(currentSkillId.value, true);
    selectedEntry.value = findFileEntry(skillStore.activeSkillDetail?.fileTree ?? [], report.imported_paths[0] ?? "assets");
    if (report.imported_paths[0]) {
      await openFile(report.imported_paths[0], { force: true });
    }
    fileFeedback.value = t("skills.feedback.assetsImported", { count: report.imported_paths.length });
  } catch {
    fileFeedback.value = resolveActionError("skills.feedback.importAssetsFailed");
  }
}

function handleCancel() {
  if (!confirmLeaveEditor()) {
    return;
  }

  emit("cancel");
}

function handleBeforeUnload(event: BeforeUnloadEvent) {
  if (!shouldWarnAboutUnsavedChanges()) {
    return;
  }

  event.preventDefault();
  event.returnValue = "";
}

onMounted(() => {
  window.addEventListener("beforeunload", handleBeforeUnload);
});

onBeforeUnmount(() => {
  window.removeEventListener("beforeunload", handleBeforeUnload);
});

onBeforeRouteLeave(() => {
  if (!confirmLeaveEditor()) {
    return false;
  }

  return true;
});

watch(
  () => [props.mode, currentSkillId.value],
  async () => {
    if (isCreateMode.value) {
      initializeCreateMode();
    } else {
      await loadExistingSkill();
    }
  },
  { immediate: true }
);

watch(
  () => currentFileTree.value,
  () => {
    if (selectedPath.value) {
      selectedEntry.value = findFileEntry(currentFileTree.value, selectedPath.value);
    }
  }
);

watch(
  () => [form.name, form.description],
  () => {
    if (!isCreateMode.value || draftMarkdownTouched.value) {
      return;
    }

    const markdown = buildDefaultMarkdown();
    setDraftFileContent("SKILL.md", markdown);
    if (selectedPath.value === "SKILL.md" && !fileDirty.value) {
      editorContent.value = markdown;
    }
  }
);

watch(metadataPreview, (value) => {
  if (isCreateMode.value && selectedPath.value === "SKILL.toml") {
    editorContent.value = value;
  }
});

watch(
  hasUnsavedChanges,
  (value) => {
    emit("dirtyChange", value);
  },
  { immediate: true }
);
</script>

<template>
  <div class="stack" style="gap: 20px; min-height: 0;">
    <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
      <div class="stack" style="gap: 6px; max-width: 760px;">
        <strong>{{ modeTitle }}</strong>
        <span class="muted">{{ modeDescription }}</span>
      </div>
      <div class="row" style="flex-wrap: wrap;">
        <Button variant="secondary" @click="handleCancel">{{ t("skills.cancelCreate") }}</Button>
        <Button :disabled="skillStore.isSaving" @click="handlePrimaryAction">
          {{ skillStore.isSaving ? (isCreateMode ? t("skills.creating") : t("skills.saving")) : primaryActionLabel }}
        </Button>
      </div>
    </div>

    <div v-if="!ready" class="empty-state">
      <strong>{{ t("skills.loadingDetailTitle") }}</strong>
      <span class="muted">{{ feedback || t("skills.loadingDetailDescription") }}</span>
    </div>

    <template v-else>
      <div class="stack" style="gap: 16px;">
        <div class="stack" style="gap: 6px;">
          <strong>{{ form.name || t("skills.createSkill") }}</strong>
          <span class="muted">{{ t("skills.metadataDescription") }}</span>
        </div>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.name") }}</span>
          <input v-model="form.name" class="field" :placeholder="t('skills.namePlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.slugInput") }}</span>
          <input v-model="form.slug" class="field" :placeholder="isCreateMode ? t('skills.slugPlaceholder') : undefined" :disabled="!isCreateMode" />
          <span v-if="!isCreateMode" class="muted settings-field__hint">{{ t("skills.slugLockedHint") }}</span>
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.descriptionInput") }}</span>
          <input v-model="form.description" class="field" :placeholder="t('skills.descriptionPlaceholder')" />
        </label>

        <div class="row" style="align-items: flex-start; flex-wrap: wrap;">
          <label class="settings-field" style="flex: 1 1 220px;">
            <span class="settings-field__label">{{ t("skills.versionInput") }}</span>
            <input v-model="form.version" class="field" :placeholder="t('skills.versionPlaceholder')" />
          </label>

          <label class="settings-field" style="flex: 1 1 220px;">
            <span class="settings-field__label">{{ t("skills.authorInput") }}</span>
            <input v-model="form.author" class="field" :placeholder="t('skills.authorPlaceholder')" />
          </label>
        </div>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.tagsInput") }}</span>
          <input v-model="form.tags" class="field" :placeholder="t('skills.tagsPlaceholder')" />
        </label>

        <label class="projects-checkbox">
          <input v-model="form.enabled" type="checkbox" />
          <span>{{ t("skills.enabledToggleCreate") }}</span>
        </label>

        <span v-if="feedback" class="settings-error">{{ feedback }}</span>

        <div class="stack" style="gap: 8px;">
          <span class="settings-field__label">{{ t("skills.manifestPreview") }}</span>
          <span class="muted">{{ t("skills.manifestPreviewHint") }}</span>
          <pre class="code-block skills-editor-preview">{{ metadataPreview }}</pre>
        </div>
      </div>

      <div class="stack" style="gap: 16px; min-height: 0;">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("skills.filesTitle") }}</strong>
            <span class="muted">{{ isCreateMode ? t("skills.filesDescriptionCreate") : t("skills.filesDescription") }}</span>
          </div>
          <div class="row" style="flex-wrap: wrap;">
            <Button variant="ghost" :disabled="skillStore.isSaving" @click="startCreateEntry('file')">{{ t("skills.newFile") }}</Button>
            <Button variant="ghost" :disabled="skillStore.isSaving" @click="startCreateEntry('directory')">{{ t("skills.newFolder") }}</Button>
            <Button variant="ghost" :disabled="skillStore.isImporting" @click="handleImportAssets">{{ t("skills.importAssets") }}</Button>
            <Button variant="ghost" :disabled="skillStore.isSaving || !canDeleteSelected" @click="handleDeleteSelected">
              {{ t("skills.deleteSelected") }}
            </Button>
          </div>
        </div>

        <div v-if="createEntryMode" class="skills-inline-create">
          <div class="stack" style="gap: 8px; flex: 1 1 320px;">
            <span class="settings-field__label">
              {{ createEntryMode === "file" ? t("skills.inlineCreateFile") : t("skills.inlineCreateFolder") }}
            </span>
            <input
              v-model="createEntryForm.path"
              class="field"
              :placeholder="createEntryMode === 'file' ? t('skills.inlineCreateFilePlaceholder') : t('skills.inlineCreateFolderPlaceholder')"
              @keydown.enter.prevent="submitCreateEntry"
              @keydown.esc.prevent="cancelCreateEntry"
            />
            <span class="muted">{{ t("skills.inlineCreateHint") }}</span>
          </div>
          <div class="row" style="flex-wrap: wrap; align-items: flex-end;">
            <Button variant="secondary" @click="cancelCreateEntry">{{ t("skills.cancelCreateEntry") }}</Button>
            <Button :disabled="skillStore.isSaving" @click="submitCreateEntry">{{ t("skills.confirmCreateEntry") }}</Button>
          </div>
        </div>

        <div class="skills-editor-layout">
          <div class="skills-editor-layout__sidebar">
            <SkillFileTree :entries="currentFileTree" :selected-path="selectedPath" @select="openFile" />
          </div>

          <div class="skills-editor-layout__main">
            <div class="stack" style="gap: 10px;">
              <div class="row" style="justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap;">
                <div class="stack" style="gap: 4px;">
                  <strong>{{ selectedEntry?.relativePath || t("skills.noFileSelected") }}</strong>
                  <div class="row" style="flex-wrap: wrap; gap: 8px;">
                    <span class="muted">{{ selectedFileStatusLabel }}</span>
                    <span v-if="selectedFileExtension" class="skills-inline-badge">{{ selectedFileExtension }}</span>
                    <span v-if="selectedFileSizeLabel" class="skills-inline-badge">{{ selectedFileSizeLabel }}</span>
                    <span v-if="fileDirty" class="skills-inline-badge skills-inline-badge--warn">{{ t("skills.unsavedChanges") }}</span>
                  </div>
                </div>
                <Button
                  v-if="!isCreateMode && selectedEntry?.kind === 'file' && selectedEntry.editable"
                  :disabled="skillStore.isSaving || !fileDirty"
                  @click="handleSaveFile"
                >
                  {{ skillStore.isSaving ? t("skills.saving") : t("skills.saveFile") }}
                </Button>
              </div>

              <div v-if="fileLoading" class="empty-state">
                <strong>{{ t("skills.loadingFileTitle") }}</strong>
                <span class="muted">{{ t("skills.loadingFileDescription") }}</span>
              </div>

              <div v-else-if="selectedEntry?.kind === 'directory'" class="empty-state">
                <strong>{{ t("skills.folderSelectedTitle") }}</strong>
                <span class="muted">{{ t("skills.folderSelectedDescription") }}</span>
              </div>

              <div v-else-if="selectedEntry?.kind === 'file' && !selectedEntry.previewable" class="empty-state">
                <template v-if="selectedAssetUrl">
                  <div class="stack skills-asset-preview" style="gap: 12px;">
                    <img :src="selectedAssetUrl" :alt="selectedEntry?.name || 'asset'" class="skills-asset-preview__image" />
                    <div class="row" style="flex-wrap: wrap; gap: 8px;">
                      <span class="skills-inline-badge">{{ t("skills.assetImage") }}</span>
                      <span v-if="selectedFileSizeLabel" class="skills-inline-badge">{{ selectedFileSizeLabel }}</span>
                    </div>
                  </div>
                </template>
                <template v-else>
                  <strong>{{ t("skills.binaryFileTitle") }}</strong>
                  <span class="muted">{{ isCreateMode ? t("skills.binaryFileDescriptionCreate") : t("skills.binaryFileDescription") }}</span>
                </template>
              </div>

              <textarea
                v-else-if="selectedEntry?.kind === 'file' && selectedEntry.editable"
                v-model="editorContent"
                class="textarea skills-editor"
                @input="fileDirty = true"
              />

              <pre v-else class="code-block skills-editor-preview">{{ editorContent }}</pre>

              <span v-if="fileFeedback" class="settings-error">{{ fileFeedback }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
