<script setup lang="ts">
import { ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import Icon from "./Icon.vue";
import type { IconName } from "../icons";

const props = defineProps<{ icon?: IconName; label: string; value: string | null | undefined; href?: string; copyable?: boolean }>();
const copied = ref(false);

async function copy() {
  if (!props.value) return;
  try {
    await navigator.clipboard.writeText(props.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    /* clipboard unavailable: nothing to show */
  }
}
</script>

<template>
  <div class="ld-row">
    <span class="ld-row-label">
      <Icon v-if="icon" :name="icon" :size="14" />
      {{ label }}
    </span>
    <span class="ld-row-value" :title="value ?? undefined">
      <template v-if="value">
        <a v-if="href" class="ld-link" href="#" @click.prevent="openUrl(href)">{{ value }}</a>
        <template v-else>{{ value }}</template>
      </template>
      <span v-else class="ld-dash">—</span>
    </span>
    <button v-if="copyable && value" type="button" class="ld-copy" :title="`Copy ${label.toLowerCase()}`" :aria-label="`Copy ${label.toLowerCase()}`" @click="copy">
      <Icon :name="copied ? 'check' : 'copy'" :size="14" :stroke="copied ? 2 : 1.5" :style="copied ? 'color:#059669' : ''" />
    </button>
  </div>
</template>
