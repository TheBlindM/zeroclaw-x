<script setup lang="ts">
import { Bot, Clock3, FolderKanban, PlugZap, Radio, Settings2, Sparkles } from "lucide-vue-next";
import Button from "@/components/ui/Button.vue";
import { useAppStore } from "@/stores/app";
import { useI18n } from "vue-i18n";
import { RouterLink, useRoute } from "vue-router";

const appStore = useAppStore();
const { t } = useI18n();
const route = useRoute();

function isActive(path: string) {
  return route.path === path || route.path.startsWith(`${path}/`);
}

const navItems = [
  {
    to: "/chat",
    icon: Bot,
    labelKey: "nav.chat"
  },
  {
    to: "/projects",
    icon: FolderKanban,
    labelKey: "nav.projects"
  },
  {
    to: "/channels",
    icon: Radio,
    labelKey: "nav.channels"
  },
  {
    to: "/mcp",
    icon: PlugZap,
    labelKey: "nav.mcp"
  },
  {
    to: "/skills",
    icon: Sparkles,
    labelKey: "nav.skills"
  },
  {
    to: "/cron",
    icon: Clock3,
    labelKey: "nav.cron"
  },
  {
    to: "/settings",
    icon: Settings2,
    labelKey: "nav.settings"
  }
];
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar__brand">
      <div class="stack" style="gap: 6px;">
        <span class="eyebrow">ZeroClawX</span>
        <strong>{{ t("sidebar.desktopRuntime") }}</strong>
      </div>
      <Sparkles :size="20" />
    </div>

    <div class="panel" style="padding: 14px 16px;">
      <div class="stack" style="gap: 6px;">
        <strong>{{ t("sidebar.phase") }}</strong>
        <span class="muted">{{ t("sidebar.phaseSummary") }}</span>
      </div>
    </div>

    <nav class="sidebar__nav">
      <RouterLink
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        custom
        v-slot="{ href, navigate }"
      >
        <a
          :href="href"
          class="sidebar-link"
          :data-active="isActive(item.to)"
          @click="navigate"
        >
          <component :is="item.icon" :size="18" />
          <span>{{ t(item.labelKey) }}</span>
        </a>
      </RouterLink>
    </nav>

    <div class="sidebar__footer" style="margin-top: auto; padding: 14px 16px;">
      <div class="stack sidebar__footer-stack">
        <div class="stack sidebar__footer-copy">
          <strong>{{ t("sidebar.runtimeStatus") }}</strong>
          <span class="muted">{{ t("sidebar.runtimeSummary") }}</span>
        </div>
        <div class="sidebar__preferences">
          <Button variant="secondary" class="sidebar__preferences-button" @click="appStore.setLocale(appStore.locale === 'zh' ? 'en' : 'zh')">
            {{ appStore.locale === "zh" ? "EN" : "中文" }}
          </Button>
          <Button variant="secondary" class="sidebar__preferences-button" @click="appStore.toggleTheme()">
            {{ appStore.theme === "dark" ? t("topbar.light") : t("topbar.dark") }}
          </Button>
        </div>
      </div>
    </div>
  </aside>
</template>
