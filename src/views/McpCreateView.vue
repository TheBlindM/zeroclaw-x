<script setup lang="ts">
import { reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import Button from "@/components/ui/Button.vue";
import { useMcpStore } from "@/stores/mcp";

const mcpStore = useMcpStore();
const router = useRouter();
const { t } = useI18n();
const feedback = ref("");

const form = reactive({
  name: "",
  transport: "stdio",
  command: "",
  argumentsJson: "[]",
  url: "",
  headersJson: "{}",
  environmentJson: "{}",
  enabled: true
});

async function handleSubmit() {
  feedback.value = "";

  try {
    await mcpStore.createServer({
      name: form.name.trim(),
      transport: form.transport,
      command: form.command.trim(),
      arguments_json: form.argumentsJson,
      url: form.url.trim(),
      headers_json: form.headersJson,
      environment_json: form.environmentJson,
      enabled: form.enabled
    });
    router.push("/mcp");
  } catch {
    if (!mcpStore.error) {
      feedback.value = t("mcp.feedback.createFailed");
    }
  }
}
</script>

<template>
  <div class="stack">
    <section class="panel" style="padding: 24px;">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
        <div class="stack" style="gap: 6px; max-width: 760px;">
          <strong>{{ t("mcp.createStandaloneTitle") }}</strong>
          <span class="muted">{{ t("mcp.createStandaloneDescription") }}</span>
        </div>
        <Button variant="secondary" @click="router.push('/mcp')">{{ t("mcp.cancelCreate") }}</Button>
      </div>
    </section>

    <section class="panel mcp-editor">
      <label class="settings-field">
        <span class="settings-field__label">{{ t("mcp.serverName") }}</span>
        <input v-model="form.name" class="field" :placeholder="t('mcp.serverNamePlaceholder')" />
      </label>

      <label class="settings-field">
        <span class="settings-field__label">{{ t("mcp.transport") }}</span>
        <select v-model="form.transport" class="field">
          <option value="stdio">{{ t("mcp.transportStdio") }}</option>
          <option value="sse">{{ t("mcp.transportSse") }}</option>
          <option value="streamable_http">{{ t("mcp.transportStreamableHttp") }}</option>
        </select>
        <span class="muted settings-field__hint">{{ t("mcp.transportHint") }}</span>
      </label>

      <template v-if="form.transport === 'stdio'">
        <label class="settings-field">
          <span class="settings-field__label">{{ t("mcp.command") }}</span>
          <input v-model="form.command" class="field" :placeholder="t('mcp.commandPlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("mcp.argumentsJson") }}</span>
          <textarea v-model="form.argumentsJson" class="field mcp-jsonarea" :placeholder="t('mcp.argumentsJsonPlaceholder')" />
          <span class="muted settings-field__hint">{{ t("mcp.argumentsJsonHint") }}</span>
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("mcp.environmentJson") }}</span>
          <textarea v-model="form.environmentJson" class="field mcp-jsonarea" :placeholder="t('mcp.environmentJsonPlaceholder')" />
          <span class="muted settings-field__hint">{{ t("mcp.environmentJsonHint") }}</span>
        </label>
      </template>

      <template v-else>
        <label class="settings-field">
          <span class="settings-field__label">{{ t("mcp.url") }}</span>
          <input v-model="form.url" class="field" :placeholder="t('mcp.urlPlaceholder')" />
        </label>

        <label class="settings-field">
          <span class="settings-field__label">{{ t("mcp.headersJson") }}</span>
          <textarea v-model="form.headersJson" class="field mcp-jsonarea" :placeholder="t('mcp.headersJsonPlaceholder')" />
          <span class="muted settings-field__hint">{{ t("mcp.headersJsonHint") }}</span>
        </label>
      </template>

      <label class="projects-checkbox">
        <input v-model="form.enabled" type="checkbox" />
        <span>{{ t("mcp.enabledToggle") }}</span>
      </label>

      <div class="row" style="justify-content: flex-end; flex-wrap: wrap;">
        <span v-if="feedback || mcpStore.error" class="settings-error">{{ feedback || mcpStore.error }}</span>
        <Button variant="secondary" @click="router.push('/mcp')">{{ t("mcp.cancelCreate") }}</Button>
        <Button :disabled="mcpStore.isSaving" @click="handleSubmit">
          {{ mcpStore.isSaving ? t("mcp.saving") : t("mcp.createServer") }}
        </Button>
      </div>
    </section>
  </div>
</template>
