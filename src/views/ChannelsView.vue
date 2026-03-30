<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { onChannelRuntimeStatus, type ChannelDraft } from "@/api/tauri";
import Button from "@/components/ui/Button.vue";
import { formatTimestamp } from "@/lib/datetime";
import { useChannelStore, type ChannelItem } from "@/stores/channel";

const channelStore = useChannelStore();
const { activeChannel, channels, enabledCount, healthyCount, runtimeStatus } = storeToRefs(channelStore);
const { t } = useI18n();

const search = ref("");
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

let disposeRuntimeStatus: (() => void) | null = null;

const filteredChannels = computed(() => {
  const query = search.value.trim().toLowerCase();

  return channels.value.filter((channel) => {
    if (!query) {
      return true;
    }

    const haystack = `${channel.name} ${channel.kind} ${channel.lastHealthStatus ?? ""} ${channel.lastHealthMessage ?? ""}`.toLowerCase();
    return haystack.includes(query);
  });
});

watch(
  activeChannel,
  (channel) => {
    if (!channel) {
      resetForm(true);
      return;
    }

    applyChannelToForm(channel);
  },
  { immediate: true }
);

onMounted(async () => {
  disposeRuntimeStatus = await onChannelRuntimeStatus((status) => {
    channelStore.applyRuntimeStatus(status);
  });

  if (!channelStore.loaded) {
    try {
      await channelStore.bootstrap();
    } catch {
      // render store error inline
    }
  }
});

onBeforeUnmount(() => {
  disposeRuntimeStatus?.();
  disposeRuntimeStatus = null;
});

function splitList(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function parseConfigJson(channel: ChannelItem) {
  try {
    return JSON.parse(channel.configJson);
  } catch {
    return {} as Record<string, unknown>;
  }
}

function applyChannelToForm(channel: ChannelItem) {
  const config = parseConfigJson(channel);

  form.name = channel.name;
  form.kind = channel.kind;
  form.enabled = channel.enabled;
  form.telegramBotToken = String(config.bot_token ?? "");
  form.telegramAllowedUsers = Array.isArray(config.allowed_users) ? config.allowed_users.join(", ") : "";
  form.telegramMentionOnly = Boolean(config.mention_only ?? false);
  form.discordBotToken = String(config.bot_token ?? "");
  form.discordGuildId = String(config.guild_id ?? "");
  form.discordAllowedUsers = Array.isArray(config.allowed_users) ? config.allowed_users.join(", ") : "";
  form.discordListenToBots = Boolean(config.listen_to_bots ?? false);
  form.discordMentionOnly = Boolean(config.mention_only ?? true);
  form.slackBotToken = String(config.bot_token ?? "");
  form.slackAppToken = String(config.app_token ?? "");
  form.slackChannelId = String(config.channel_id ?? "");
  form.slackAllowedUsers = Array.isArray(config.allowed_users) ? config.allowed_users.join(", ") : "";
  form.slackMentionOnly = Boolean(config.mention_only ?? true);
  form.webhookPort = Number(config.port ?? 8787);
  form.webhookListenPath = String(config.listen_path ?? "/webhook");
  form.webhookSendUrl = String(config.send_url ?? "");
  form.webhookSendMethod = String(config.send_method ?? "POST");
  form.webhookAuthHeader = String(config.auth_header ?? "");
  form.webhookSecret = String(config.secret ?? "");
}

function resetForm(clearSelection = false) {
  if (clearSelection) {
    channelStore.clearActiveChannel();
  }

  form.name = "";
  form.kind = "telegram";
  form.enabled = true;
  form.telegramBotToken = "";
  form.telegramAllowedUsers = "";
  form.telegramMentionOnly = false;
  form.discordBotToken = "";
  form.discordGuildId = "";
  form.discordAllowedUsers = "";
  form.discordListenToBots = false;
  form.discordMentionOnly = true;
  form.slackBotToken = "";
  form.slackAppToken = "";
  form.slackChannelId = "";
  form.slackAllowedUsers = "";
  form.slackMentionOnly = true;
  form.webhookPort = 8787;
  form.webhookListenPath = "/webhook";
  form.webhookSendUrl = "";
  form.webhookSendMethod = "POST";
  form.webhookAuthHeader = "";
  form.webhookSecret = "";
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

function buildDraft(): ChannelDraft {
  return {
    name: form.name.trim(),
    kind: form.kind,
    config_json: buildConfigJson(form.kind),
    enabled: form.enabled
  };
}

function resolveKindLabel(kind: string) {
  return t(`channels.kinds.${kind}`);
}

function resolveHealthLabel(channel: ChannelItem) {
  return channel.lastHealthStatus === "healthy"
    ? t("channels.healthy")
    : channel.lastHealthStatus === "error"
      ? t("channels.unhealthy")
      : t("channels.untested");
}

function handleNewChannel() {
  resetForm(true);
  feedback.value = t("channels.feedback.readyForNewChannel");
}

function handleSelectChannel(channelId: string) {
  feedback.value = "";
  channelStore.setActiveChannel(channelId);
}

async function handleSubmit() {
  feedback.value = "";

  try {
    const payload = buildDraft();
    if (activeChannel.value) {
      await channelStore.updateChannel(activeChannel.value.id, payload);
      feedback.value = t("channels.feedback.updated", { name: payload.name });
      return;
    }

    const channel = await channelStore.createChannel(payload);
    feedback.value = t("channels.feedback.created", { name: channel.name });
  } catch {
    feedback.value = activeChannel.value ? t("channels.feedback.updateFailed") : t("channels.feedback.createFailed");
  }
}

async function handleDeleteChannel(channel: ChannelItem) {
  const confirmed = window.confirm(t("channels.prompts.deleteChannel", { name: channel.name }));
  if (!confirmed) {
    return;
  }

  try {
    await channelStore.deleteChannel(channel.id);
    feedback.value = t("channels.feedback.deleted", { name: channel.name });
  } catch {
    feedback.value = t("channels.feedback.deleteFailed");
  }
}

async function handleTestChannel(channelId: string) {
  try {
    const result = await channelStore.testChannel(channelId);
    feedback.value = result.report.ok ? t("channels.feedback.testPassed") : t("channels.feedback.testFailed");
  } catch {
    feedback.value = t("channels.feedback.testFailed");
  }
}

async function handleStartRuntime() {
  try {
    await channelStore.startRuntime();
    feedback.value = t("channels.feedback.runtimeStarted");
  } catch {
    feedback.value = t("channels.feedback.runtimeStartFailed");
  }
}

async function handleStopRuntime() {
  try {
    await channelStore.stopRuntime();
    feedback.value = t("channels.feedback.runtimeStopped");
  } catch {
    feedback.value = t("channels.feedback.runtimeStopFailed");
  }
}
</script>

<template>
  <div class="stack channels-page">
    <section class="panel channels-hero">
      <div class="stack" style="gap: 8px; max-width: 760px;">
        <strong>{{ t("channels.workspaceTitle") }}</strong>
        <p class="muted channels-hero__copy">{{ t("channels.workspaceDescription") }}</p>
      </div>
      <div class="channels-summary-grid">
        <div class="summary-card summary-card--static">
          <strong>{{ channels.length }}</strong>
          <span class="muted">{{ t("channels.summaryAll") }}</span>
        </div>
        <div class="summary-card summary-card--static">
          <strong>{{ enabledCount }}</strong>
          <span class="muted">{{ t("channels.summaryEnabled") }}</span>
        </div>
        <div class="summary-card summary-card--static">
          <strong>{{ healthyCount }}</strong>
          <span class="muted">{{ t("channels.summaryHealthy") }}</span>
        </div>
        <div class="summary-card summary-card--static" :data-active="runtimeStatus.running">
          <strong>{{ runtimeStatus.running ? t("channels.runtimeRunning") : t("channels.runtimeStopped") }}</strong>
          <span class="muted">{{ t("channels.summarySupervisor") }}</span>
        </div>
      </div>
    </section>

    <section class="channels-layout">
      <section class="panel channels-editor">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ activeChannel ? t("channels.editing", { name: activeChannel.name }) : t("channels.newChannel") }}</strong>
            <span class="muted">{{ t("channels.editorDescription") }}</span>
          </div>
          <Button variant="secondary" :disabled="channelStore.isSaving || channelStore.isTesting" @click="handleNewChannel">{{ t("channels.newDraft") }}</Button>
        </div>

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
          <span v-if="channelStore.error" class="settings-error">{{ channelStore.error }}</span>
          <Button variant="secondary" :disabled="channelStore.isSaving || channelStore.isTesting" @click="resetForm(true)">{{ t("channels.resetForm") }}</Button>
          <Button :disabled="channelStore.isSaving || channelStore.isTesting" @click="handleSubmit">
            {{ channelStore.isSaving ? t("channels.saving") : activeChannel ? t("channels.saveChannel") : t("channels.createChannel") }}
          </Button>
        </div>
      </section>

      <section class="panel channels-board">
        <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
          <div class="stack" style="gap: 6px;">
            <strong>{{ t("channels.boardTitle") }}</strong>
            <span class="muted">{{ t("channels.boardDescription") }}</span>
          </div>
          <input v-model="search" class="field channels-search" :placeholder="t('channels.searchPlaceholder')" />
        </div>

        <div v-if="channelStore.isLoading && !channelStore.loaded" class="empty-state">
          <strong>{{ t("channels.loadingTitle") }}</strong>
          <span class="muted">{{ t("channels.loadingDescription") }}</span>
        </div>

        <div v-else-if="filteredChannels.length === 0" class="empty-state">
          <strong>{{ t("channels.emptyTitle") }}</strong>
          <span class="muted">{{ t("channels.emptyDescription") }}</span>
        </div>

        <div v-else class="channels-list">
          <article
            v-for="channel in filteredChannels"
            :key="channel.id"
            class="channels-card"
            :data-active="channel.id === channelStore.activeChannelId"
            @click="handleSelectChannel(channel.id)"
          >
            <div class="stack" style="gap: 10px;">
              <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 12px;">
                <div class="stack" style="gap: 6px;">
                  <div class="row" style="flex-wrap: wrap; gap: 8px;">
                    <strong>{{ channel.name }}</strong>
                    <span class="project-badge">{{ resolveKindLabel(channel.kind) }}</span>
                    <span class="project-badge" :data-archived="!channel.enabled">{{ channel.enabled ? t("channels.enabled") : t("channels.disabled") }}</span>
                  </div>
                  <p class="mcp-card__note">{{ channel.lastHealthMessage || t("channels.noHealthNote") }}</p>
                </div>
              </div>

              <div class="row" style="justify-content: space-between; flex-wrap: wrap; gap: 10px;">
                <span class="muted">{{ t("channels.healthLabel", { value: resolveHealthLabel(channel) }) }}</span>
                <span class="muted">{{ t("channels.updatedAt", { value: formatTimestamp(channel.updatedAt, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) }) }}</span>
              </div>

              <div class="row" style="justify-content: flex-end; flex-wrap: wrap;">
                <Button variant="secondary" :disabled="channelStore.isSaving || channelStore.isTesting" @click.stop="handleTestChannel(channel.id)">
                  {{ channelStore.isTesting && channelStore.activeChannelId === channel.id ? t("channels.testing") : t("channels.testConnection") }}
                </Button>
                <Button variant="ghost" :disabled="channelStore.isSaving || channelStore.isTesting" @click.stop="handleDeleteChannel(channel)">
                  {{ t("channels.delete") }}
                </Button>
              </div>
            </div>
          </article>
        </div>
      </section>
    </section>

    <section class="panel channels-runtime-panel">
      <div class="row" style="justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;">
        <div class="stack" style="gap: 6px; max-width: 720px;">
          <strong>{{ t("channels.runtimeTitle") }}</strong>
          <span class="muted">{{ t("channels.runtimeDescription") }}</span>
        </div>
        <div class="row" style="flex-wrap: wrap;">
          <Button variant="secondary" :disabled="channelStore.isStartingRuntime || runtimeStatus.running" @click="handleStartRuntime">
            {{ channelStore.isStartingRuntime ? t("channels.starting") : t("channels.startRuntime") }}
          </Button>
          <Button variant="ghost" :disabled="channelStore.isStoppingRuntime || !runtimeStatus.running" @click="handleStopRuntime">
            {{ channelStore.isStoppingRuntime ? t("channels.stopping") : t("channels.stopRuntime") }}
          </Button>
        </div>
      </div>

      <div class="settings-test-card" :data-ok="runtimeStatus.running">
        <strong>{{ runtimeStatus.running ? t("channels.runtimeRunning") : t("channels.runtimeStopped") }}</strong>
        <p class="settings-test-card__message">{{ runtimeStatus.message }}</p>
        <span class="muted">{{ t("channels.runtimeUpdatedAt", { value: runtimeStatus.updatedAt ? formatTimestamp(runtimeStatus.updatedAt, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) : t('channels.none') }) }}</span>
      </div>
    </section>

    <p v-if="feedback || channelStore.error" class="muted">{{ feedback || channelStore.error }}</p>
  </div>
</template>
