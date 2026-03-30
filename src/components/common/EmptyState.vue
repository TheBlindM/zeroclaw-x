<script setup lang="ts">
defineProps<{
  title: string;
  description: string;
}>();
</script>

<template>
  <div class="empty-state">
    <strong>{{ title }}</strong>
    <p class="muted">{{ description }}</p>
    <slot />
  </div>
</template>
