<script setup lang="ts">
import { reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import Button from "@/components/ui/Button.vue";
import { useSkillStore } from "@/stores/skill";

const skillStore = useSkillStore();
const router = useRouter();
const { t } = useI18n();
const feedback = ref("");

const form = reactive({
  name: "",
  slug: "",
  description: "",
  version: "0.1.0",
  author: "",
  tags: "",
  instructions: "",
  enabled: true
});

function splitTags(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function buildMarkdown() {
  const sections = [`# ${form.name.trim()}`, form.description.trim()];
  const instructions = form.instructions.trim();

  if (instructions) {
    sections.push("", instructions);
  }

  return sections.filter((section) => section.length > 0).join("\n");
}

async function handleSubmit() {
  feedback.value = "";

  try {
    await skillStore.createSkill({
      slug: form.slug.trim(),
      name: form.name.trim(),
      description: form.description.trim(),
      version: form.version.trim(),
      author: form.author.trim(),
      tags_json: JSON.stringify(splitTags(form.tags)),
      markdown_content: buildMarkdown(),
      enabled: form.enabled
    });
    router.push("/skills");
  } catch {
    feedback.value = t("skills.feedback.createFailed");
  }
}
</script>

<template>
  <div class="stack">
    <section class="panel" style="padding: 24px;">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
        <div class="stack" style="gap: 6px; max-width: 760px;">
          <strong>{{ t("skills.createTitle") }}</strong>
          <span class="muted">{{ t("skills.createDescription") }}</span>
        </div>
        <Button variant="secondary" @click="router.push('/skills')">{{ t("skills.cancelCreate") }}</Button>
      </div>
    </section>

    <section class="panel" style="padding: 20px;">
      <div class="stack" style="gap: 16px;">
        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.name") }}</span>
          <input v-model="form.name" class="field" :placeholder="t('skills.namePlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("skills.slugInput") }}</span>
          <input v-model="form.slug" class="field" :placeholder="t('skills.slugPlaceholder')" />
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
          <span class="settings-field__label">{{ t("skills.instructionsInput") }}</span>
          <textarea v-model="form.instructions" class="textarea" :placeholder="t('skills.instructionsPlaceholder')" />
        </label>

        <label class="projects-checkbox">
          <input v-model="form.enabled" type="checkbox" />
          <span>{{ t("skills.enabledToggleCreate") }}</span>
        </label>

        <div class="row" style="justify-content: flex-end; flex-wrap: wrap;">
          <span v-if="feedback || skillStore.error" class="settings-error">{{ feedback || skillStore.error }}</span>
          <Button variant="secondary" @click="router.push('/skills')">{{ t("skills.cancelCreate") }}</Button>
          <Button :disabled="skillStore.isSaving" @click="handleSubmit">
            {{ skillStore.isSaving ? t("skills.creating") : t("skills.createSkill") }}
          </Button>
        </div>
      </div>
    </section>
  </div>
</template>
