<script setup lang="ts">
import { computed } from "vue";
import type { ComposerChannel } from "../api";

const props = withDefaults(defineProps<{ channel: ComposerChannel; size?: "sm" | "md" }>(), { size: "md" });

const mark = computed(() => (props.channel.platform === "Facebook" ? { label: "f", color: "#1877F2" } : { label: "in", color: "#0A66C2" }));
</script>

<template>
  <span v-if="channel.avatarUrl && size === 'md'" class="ca">
    <img :src="channel.avatarUrl" alt="" class="ca-img" referrerpolicy="no-referrer" />
    <span class="ca-badge" :style="{ background: mark.color }">{{ mark.label }}</span>
  </span>
  <span v-else class="ca-mark" :class="`ca-${size}`" :style="{ background: mark.color }">{{ mark.label }}</span>
</template>
