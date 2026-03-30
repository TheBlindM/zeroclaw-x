<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ChatMessage } from "@/stores/chat";
import { formatTimestamp } from "@/lib/datetime";
import MarkdownMessage from "./MarkdownMessage.vue";

const props = defineProps<{
  messages: ChatMessage[];
}>();

const { t } = useI18n();
const containerRef = ref<HTMLElement | null>(null);

function scrollToBottom() {
  const element = containerRef.value;
  if (!element) {
    return;
  }

  element.scrollTo({
    top: element.scrollHeight,
    behavior: "smooth"
  });
}

function resolveRole(role: ChatMessage["role"]) {
  return t(`chat.roles.${role}`);
}

function resolveContent(content: string) {
  return content === "ZeroClawX skeleton is ready. Ask for a project plan, code review, or build step."
    ? t("chat.defaults.welcome")
    : content;
}

watch(
  () => props.messages.map((message) => `${message.id}:${message.content.length}:${message.status ?? "done"}`).join("|"),
  async () => {
    await nextTick();
    scrollToBottom();
  },
  { immediate: true }
);

onMounted(() => {
  scrollToBottom();
});
</script>

<template>
  <div ref="containerRef" class="chat-messages">
    <article
      v-for="message in messages"
      :key="message.id"
      class="message"
      :data-role="message.role"
    >
      <div class="message__meta">
        <strong>{{ resolveRole(message.role) }}</strong>
        <span>
          {{ formatTimestamp(message.createdAt, { hour: "2-digit", minute: "2-digit" }) }}
        </span>
      </div>
      <MarkdownMessage :content="resolveContent(message.content)" />
    </article>
  </div>
</template>
