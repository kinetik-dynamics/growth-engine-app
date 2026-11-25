<script setup lang="ts">
import { computed } from "vue";
import type { ComposerChannel } from "../api";

const props = withDefaults(defineProps<{ channel: ComposerChannel; size?: "sm" | "md" }>(), { size: "md" });

const marks: Record<string, { label: string; color: string }> = {
  Facebook: { label: "f", color: "#1877F2" },
  Website: { label: "W", color: "#047857" },
};
const mark = computed(() => marks[props.channel.platform] ?? { label: "in", color: "#0A66C2" });
</script>

<template>
  <span v-if="channel.avatarUrl && size === 'md'" class="ca">
    <img :src="channel.avatarUrl" alt="" class="ca-img" referrerpolicy="no-referrer" />
    <span class="ca-badge" :style="{ background: mark.color }">{{ mark.label }}</span>
  </span>
  <span v-else class="ca-mark" :class="`ca-${size}`" :style="{ background: mark.color }">{{ mark.label }}</span>
</template>
