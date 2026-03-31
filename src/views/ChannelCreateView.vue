<script setup lang="ts">
import { reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import Button from "@/components/ui/Button.vue";
import { useChannelStore } from "@/stores/channel";

const channelStore = useChannelStore();
const router = useRouter();
const { t } = useI18n();
const feedback = ref("");

const form = reactive({
  name: "",
  kind: "telegram",
  enabled: true,
  telegramBotToken: "",
  telegramAllowedUsers: "",
  telegramMentionOnly: false,
  discordBotToken: "",
  discordGuildId: "",
  discordAllowedUsers: "",
  discordListenToBots: false,
  discordMentionOnly: true,
  slackBotToken: "",
  slackAppToken: "",
  slackChannelId: "",
  slackAllowedUsers: "",
  slackMentionOnly: true,
  webhookPort: 8787,
  webhookListenPath: "/webhook",
  webhookSendUrl: "",
  webhookSendMethod: "POST",
  webhookAuthHeader: "",
  webhookSecret: ""
});

function splitList(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function buildConfigJson(kind: string) {
  const config =
    kind === "telegram"
      ? {
          bot_token: form.telegramBotToken,
          allowed_users: splitList(form.telegramAllowedUsers),
          mention_only: form.telegramMentionOnly
        }
      : kind === "discord"
        ? {
            bot_token: form.discordBotToken,
            guild_id: form.discordGuildId.trim() || null,
            allowed_users: splitList(form.discordAllowedUsers),
            listen_to_bots: form.discordListenToBots,
            mention_only: form.discordMentionOnly
          }
        : kind === "slack"
          ? {
              bot_token: form.slackBotToken,
              app_token: form.slackAppToken.trim() || null,
              channel_id: form.slackChannelId.trim() || null,
              allowed_users: splitList(form.slackAllowedUsers),
              mention_only: form.slackMentionOnly
            }
          : {
              port: Number(form.webhookPort) || 8787,
              listen_path: form.webhookListenPath.trim() || "/webhook",
              send_url: form.webhookSendUrl.trim() || null,
              send_method: form.webhookSendMethod.trim() || "POST",
              auth_header: form.webhookAuthHeader.trim() || null,
              secret: form.webhookSecret.trim() || null
            };

  return JSON.stringify(config, null, 2);
}

async function handleSubmit() {
  feedback.value = "";

  try {
    await channelStore.createChannel({
      name: form.name.trim(),
      kind: form.kind,
      config_json: buildConfigJson(form.kind),
      enabled: form.enabled
    });
    router.push("/channels");
  } catch {
    feedback.value = t("channels.feedback.createFailed");
  }
}
</script>

<template>
  <div class="stack">
    <section class="panel" style="padding: 24px;">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
        <div class="stack" style="gap: 6px; max-width: 760px;">
          <strong>{{ t("channels.createStandaloneTitle") }}</strong>
          <span class="muted">{{ t("channels.createStandaloneDescription") }}</span>
        </div>
        <Button variant="secondary" @click="router.push('/channels')">{{ t("channels.cancelCreate") }}</Button>
      </div>
    </section>

    <section class="panel channels-editor">
      <label class="stack settings-field" style="gap: 8px;">
        <span class="settings-field__label">{{ t("channels.channelName") }}</span>
        <input v-model="form.name" class="field" :placeholder="t('channels.channelNamePlaceholder')" />
      </label>

      <label class="stack settings-field" style="gap: 8px;">
        <span class="settings-field__label">{{ t("channels.kind") }}</span>
        <select v-model="form.kind" class="select">
          <option value="telegram">{{ t("channels.kinds.telegram") }}</option>
          <option value="discord">{{ t("channels.kinds.discord") }}</option>
          <option value="slack">{{ t("channels.kinds.slack") }}</option>
          <option value="webhook">{{ t("channels.kinds.webhook") }}</option>
        </select>
        <span class="muted settings-field__hint">{{ t("channels.kindHint") }}</span>
      </label>

      <template v-if="form.kind === 'telegram'">
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.telegramBotToken") }}</span>
          <input v-model="form.telegramBotToken" class="field" :placeholder="t('channels.telegramBotTokenPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.telegramAllowedUsers") }}</span>
          <input v-model="form.telegramAllowedUsers" class="field" :placeholder="t('channels.telegramAllowedUsersPlaceholder')" />
        </label>
        <label class="row settings-toggle">
          <input v-model="form.telegramMentionOnly" type="checkbox" />
          <span>{{ t("channels.telegramMentionOnly") }}</span>
        </label>
      </template>

      <template v-else-if="form.kind === 'discord'">
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.discordBotToken") }}</span>
          <input v-model="form.discordBotToken" class="field" :placeholder="t('channels.discordBotTokenPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.discordGuildId") }}</span>
          <input v-model="form.discordGuildId" class="field" :placeholder="t('channels.discordGuildIdPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.discordAllowedUsers") }}</span>
          <input v-model="form.discordAllowedUsers" class="field" :placeholder="t('channels.discordAllowedUsersPlaceholder')" />
        </label>
        <label class="row settings-toggle">
          <input v-model="form.discordListenToBots" type="checkbox" />
          <span>{{ t("channels.discordListenToBots") }}</span>
        </label>
        <label class="row settings-toggle">
          <input v-model="form.discordMentionOnly" type="checkbox" />
          <span>{{ t("channels.discordMentionOnly") }}</span>
        </label>
      </template>

      <template v-else-if="form.kind === 'slack'">
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.slackBotToken") }}</span>
          <input v-model="form.slackBotToken" class="field" :placeholder="t('channels.slackBotTokenPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.slackAppToken") }}</span>
          <input v-model="form.slackAppToken" class="field" :placeholder="t('channels.slackAppTokenPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.slackChannelId") }}</span>
          <input v-model="form.slackChannelId" class="field" :placeholder="t('channels.slackChannelIdPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.slackAllowedUsers") }}</span>
          <input v-model="form.slackAllowedUsers" class="field" :placeholder="t('channels.slackAllowedUsersPlaceholder')" />
        </label>
        <label class="row settings-toggle">
          <input v-model="form.slackMentionOnly" type="checkbox" />
          <span>{{ t("channels.slackMentionOnly") }}</span>
        </label>
      </template>

      <template v-else>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.webhookPort") }}</span>
          <input v-model.number="form.webhookPort" type="number" class="field" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.webhookListenPath") }}</span>
          <input v-model="form.webhookListenPath" class="field" :placeholder="t('channels.webhookListenPathPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.webhookSendUrl") }}</span>
          <input v-model="form.webhookSendUrl" class="field" :placeholder="t('channels.webhookSendUrlPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.webhookSendMethod") }}</span>
          <select v-model="form.webhookSendMethod" class="select">
            <option value="POST">POST</option>
            <option value="PUT">PUT</option>
          </select>
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.webhookAuthHeader") }}</span>
          <input v-model="form.webhookAuthHeader" class="field" :placeholder="t('channels.webhookAuthHeaderPlaceholder')" />
        </label>
        <label class="stack settings-field" style="gap: 8px;">
          <span class="settings-field__label">{{ t("channels.webhookSecret") }}</span>
          <input v-model="form.webhookSecret" class="field" :placeholder="t('channels.webhookSecretPlaceholder')" />
        </label>
      </template>

      <label class="row settings-toggle">
        <input v-model="form.enabled" type="checkbox" />
        <span>{{ t("channels.enabledToggle") }}</span>
      </label>

      <div class="row" style="justify-content: flex-end; flex-wrap: wrap;">
        <span v-if="feedback || channelStore.error" class="settings-error">{{ feedback || channelStore.error }}</span>
        <Button variant="secondary" @click="router.push('/channels')">{{ t("channels.cancelCreate") }}</Button>
        <Button :disabled="channelStore.isSaving" @click="handleSubmit">
          {{ channelStore.isSaving ? t("channels.saving") : t("channels.createChannel") }}
        </Button>
      </div>
    </section>
  </div>
</template>
