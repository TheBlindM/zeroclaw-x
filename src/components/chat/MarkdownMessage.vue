<script setup lang="ts">
import MarkdownIt from "markdown-it";
import { computed } from "vue";

const props = defineProps<{
  content: string;
}>();

function escapeHtml(input: string) {
  return input
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

const markdown = new MarkdownIt({
  breaks: true,
  linkify: true,
  highlight(code) {
    return `<pre class="code-block"><code>${escapeHtml(code)}</code></pre>`;
  }
});

const html = computed(() => markdown.render(props.content));
</script>

<template>
  <div v-html="html"></div>
</template>
