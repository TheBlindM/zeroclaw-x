<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/lib/utils";

const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost";
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
  }>(),
  {
    variant: "primary",
    type: "button",
    disabled: false
  }
);

const classes = computed(() =>
  cn(
    "btn",
    props.variant === "primary" && "btn--primary",
    props.variant === "secondary" && "btn--secondary",
    props.variant === "ghost" && "btn--ghost"
  )
);
</script>

<template>
  <button :type="type" :disabled="disabled" :class="classes">
    <slot />
  </button>
</template>
