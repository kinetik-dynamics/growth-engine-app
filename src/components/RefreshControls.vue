<script setup lang="ts">
import Icon from "./Icon.vue";

withDefaults(defineProps<{ countdown: string; refreshing: boolean; size?: "sm" | "md" }>(), { size: "md" });
const enabled = defineModel<boolean>({ required: true });
const emit = defineEmits<{ refresh: [] }>();
</script>

<template>
  <div class="rc" :class="`rc-${size}`">
    <label class="rc-auto" :title="enabled ? 'Turn off auto refresh' : 'Refresh every 30 minutes'">
      <input v-model="enabled" type="checkbox" role="switch" class="rc-switch-input" />
      <span class="rc-switch" aria-hidden="true"></span>
      <span>Auto refresh</span>
      <span v-if="enabled" class="rc-countdown" aria-live="off">{{ refreshing ? "refreshing…" : `in ${countdown}` }}</span>
    </label>
    <button class="rc-btn" :disabled="refreshing" @click="emit('refresh')">
      <span v-if="refreshing" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
      <Icon v-else name="refresh" :size="size === 'sm' ? 14 : 16" />
      Refresh
    </button>
  </div>
</template>
