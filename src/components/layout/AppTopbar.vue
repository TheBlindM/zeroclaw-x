<script setup lang="ts">
import Button from "@/components/ui/Button.vue";
import { useAppStore } from "@/stores/app";
import { useI18n } from "vue-i18n";
import { computed } from "vue";
import { useRoute } from "vue-router";

const appStore = useAppStore();
const route = useRoute();
const { t } = useI18n();

const title = computed(() => {
  const titleKey = route.meta.titleKey;
  return typeof titleKey === "string" ? t(titleKey) : "ZeroClawX";
});

const description = computed(() => {
  const descriptionKey = route.meta.descriptionKey;
  return typeof descriptionKey === "string" ? t(descriptionKey) : "";
});
</script>

<template>
  <header class="topbar panel">
    <div class="topbar__meta">
      <span class="eyebrow">{{ t("topbar.eyebrow") }}</span>
      <h1 class="title">{{ title }}</h1>
      <p class="muted">{{ description }}</p>
    </div>

    <div class="row">
      <Button variant="secondary" @click="appStore.setLocale(appStore.locale === 'zh' ? 'en' : 'zh')">
        {{ appStore.locale === "zh" ? "EN" : "中文" }}
      </Button>
      <Button variant="secondary" @click="appStore.toggleTheme()">
        {{ appStore.theme === "dark" ? t("topbar.light") : t("topbar.dark") }}
      </Button>
    </div>
  </header>
</template>
