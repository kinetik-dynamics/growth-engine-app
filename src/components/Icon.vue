<script setup lang="ts">
import { ICONS, type IconName } from "../icons";

withDefaults(defineProps<{ name: IconName; size?: number; stroke?: number }>(), { size: 16, stroke: 1.75 });
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="stroke"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    style="flex: none"
    v-html="ICONS[name]"
  ></svg>
</template>
