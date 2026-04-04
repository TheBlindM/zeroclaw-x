<script setup lang="ts">
import { ArrowUp } from "lucide-vue-next";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import Button from "@/components/ui/Button.vue";

const props = defineProps<{
  modelValue?: string;
  busy?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  submit: [value: string];
}>();

const { t } = useI18n();
const textareaRef = ref<HTMLTextAreaElement | null>(null);
const draft = computed({
  get: () => props.modelValue ?? "",
  set: (value: string) => emit("update:modelValue", value)
});
const canSubmit = computed(() => draft.value.trim().length > 0 && !props.busy);

function resizeComposer() {
  const element = textareaRef.value;
  if (!element) {
    return;
  }

  element.style.height = "auto";
  element.style.height = `${Math.min(Math.max(element.scrollHeight, 132), 280)}px`;
}

function handleSubmit() {
  if (!canSubmit.value) {
    return;
  }
  emit("submit", draft.value.trim());
}

function handleKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
    event.preventDefault();
    handleSubmit();
  }
}

watch(
  () => draft.value,
  async () => {
    await nextTick();
    resizeComposer();
  }
);

onMounted(() => {
  resizeComposer();
});
</script>

<template>
  <form class="composer" @submit.prevent="handleSubmit">
    <div class="composer__surface">
      <textarea
        ref="textareaRef"
        v-model="draft"
        class="textarea composer__input"
        rows="1"
        :placeholder="t('composer.placeholder')"
        @keydown="handleKeydown"
      />
      <div class="composer__actions">
        <span class="muted composer__hint">{{ t("composer.hint") }}</span>
        <Button class="composer__submit" :disabled="!canSubmit" type="submit">
          <ArrowUp :size="16" />
          {{ t("composer.send") }}
        </Button>
      </div>
    </div>
  </form>
</template>
