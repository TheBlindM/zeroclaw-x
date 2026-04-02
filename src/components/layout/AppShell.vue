<script setup lang="ts">
import { onMounted } from "vue";
import { RouterView } from "vue-router";
import AppSidebar from "./AppSidebar.vue";
import { useAppStore } from "@/stores/app";
import { useUpdateStore } from "@/stores/update";

const appStore = useAppStore();
const updateStore = useUpdateStore();

onMounted(() => {
  appStore.applyTheme(appStore.theme);
  appStore.setLocale(appStore.locale);
  updateStore.bootstrap().catch(() => {
    // keep updater failures non-blocking for the main shell
  });
});
</script>

<template>
  <div class="app-shell">
    <AppSidebar />
    <main class="content">
      <section class="page-card">
        <RouterView />
      </section>
    </main>
  </div>
</template>
