<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import Button from "@/components/ui/Button.vue";

const props = defineProps<{
  busy?: boolean;
}>();

const emit = defineEmits<{
  submit: [value: string];
}>();

const { t } = useI18n();
const draft = ref("");
const canSubmit = computed(() => draft.value.trim().length > 0 && !props.busy);

function handleSubmit() {
  if (!canSubmit.value) {
    return;
  }
  emit("submit", draft.value.trim());
  draft.value = "";
}

function handleKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
    event.preventDefault();
    handleSubmit();
  }
}
</script>

<template>
  <form class="composer" @submit.prevent="handleSubmit">
    <textarea
      v-model="draft"
      class="textarea"
      :placeholder="t('composer.placeholder')"
      @keydown="handleKeydown"
    />
    <div class="composer__actions">
      <span class="muted">{{ t("composer.hint") }}</span>
      <Button :disabled="!canSubmit" type="submit">{{ t("composer.send") }}</Button>
    </div>
  </form>
</template>
