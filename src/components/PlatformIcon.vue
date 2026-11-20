<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(defineProps<{ platform?: string; size?: "sm" | "md" | "dot" }>(), { platform: undefined, size: "md" });

const look = computed(() => {
  switch (props.platform) {
    case "LinkedIn":
      return { color: "#0A66C2", text: "in" };
    case "Facebook":
      return { color: "#1877F2", text: "f" };
    case "X":
      return { color: "#000", text: "𝕏" };
    case "Website":
      return { color: "#047857", text: "KD" };
    default:
      return { color: "#4b5563", text: "#" };
  }
});
</script>

<template>
  <span v-if="size === 'dot'" class="pi-dot" :style="{ background: look.color }" aria-hidden="true"></span>
  <span v-else class="pi" :class="`pi-${size}`" :style="{ background: look.color }" aria-hidden="true">{{ look.text }}</span>
</template>
