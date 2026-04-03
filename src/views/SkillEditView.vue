<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import Button from "@/components/ui/Button.vue";
import { parseTags, useSkillStore } from "@/stores/skill";

const skillStore = useSkillStore();
const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const feedback = ref("");
const ready = ref(false);

const skillId = computed(() => String(route.params.skillId ?? ""));
const form = reactive({
  name: "",
  slug: "",
  description: "",
  version: "0.1.0",
  author: "",
  tags: "",
  markdownContent: "",
  enabled: true
});

function splitTags(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

async function loadSkill() {
  ready.value = false;
  feedback.value = "";

  try {
    if (!skillStore.loaded) {
      await skillStore.bootstrap();
    }

    const detail = await skillStore.loadSkillDetail(skillId.value);
    form.name = detail.skill.name;
    form.slug = detail.skill.slug;
    form.description = detail.skill.description;
    form.version = detail.skill.version;
    form.author = detail.skill.author;
    form.tags = parseTags(detail.skill.tagsJson).join(", ");
    form.markdownContent = detail.markdownContent;
    form.enabled = detail.skill.enabled;
    ready.value = true;
  } catch {
    feedback.value = t("skills.feedback.loadDetailFailed");
  }
}

async function handleSubmit() {
  feedback.value = "";

  try {
    await skillStore.updateSkill(skillId.value, {
      slug: form.slug,
      name: form.name.trim(),
      description: form.description.trim(),
      version: form.version.trim(),
      author: form.author.trim(),
      tags_json: JSON.stringify(splitTags(form.tags)),
      markdown_content: form.markdownContent,
      enabled: form.enabled
    });
    router.push("/skills");
  } catch {
    feedback.value = t("skills.feedback.updateFailed");
  }
}

watch(skillId, async () => {
  if (skillId.value) {
    await loadSkill();
  }
});

onMounted(async () => {
  if (skillId.value) {
    await loadSkill();
  }
});
</script>

<template>
  <div class="stack">
    <section class="panel" style="padding: 24px;">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
        <div class="stack" style="gap: 6px; max-width: 760px;">
          <strong>{{ t("skills.editTitle") }}</strong>
          <span class="muted">{{ t("skills.editDescription") }}</span>
        </div>
        <Button variant="secondary" @click="router.push('/skills')">{{ t("skills.cancelCreate") }}</Button>
      </div>
    </section>

    <section v-if="!ready" class="panel" style="padding: 20px;">
      <div class="empty-state">
        <strong>{{ t("skills.loadingDetailTitle") }}</strong>
        <span class="muted">{{ feedback || skillStore.error || t("skills.loadingDetailDescription") }}</span>
      </div>
    </section>

    <section v-else class="panel" style="padding: 20px;">
      <div class="stack" style="gap: 16px;">
        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.name") }}</span>
          <input v-model="form.name" class="field" :placeholder="t('skills.namePlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.slugInput") }}</span>
          <input v-model="form.slug" class="field" disabled />
          <span class="muted settings-field__hint">{{ t("skills.slugLockedHint") }}</span>
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

        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.markdownInput") }}</span>
          <textarea v-model="form.markdownContent" class="textarea" :placeholder="t('skills.markdownPlaceholder')" />
        </label>

        <label class="projects-checkbox">
          <input v-model="form.enabled" type="checkbox" />
          <span>{{ t("skills.enabledToggleCreate") }}</span>
        </label>

        <div class="row" style="justify-content: flex-end; flex-wrap: wrap;">
          <span v-if="feedback || skillStore.error" class="settings-error">{{ feedback || skillStore.error }}</span>
          <Button variant="secondary" @click="router.push('/skills')">{{ t("skills.cancelCreate") }}</Button>
          <Button :disabled="skillStore.isSaving" @click="handleSubmit">
            {{ skillStore.isSaving ? t("skills.saving") : t("skills.saveSkill") }}
          </Button>
        </div>
      </div>
    </section>
  </div>
</template>
