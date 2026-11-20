<script setup lang="ts">
import type { ComposerChannel } from "../api";
import ChannelAvatar from "./ChannelAvatar.vue";
import Icon from "./Icon.vue";

/** `plannedPlatform` marks the channels matching the platform the idea was written for. */
defineProps<{ channels: ComposerChannel[]; selectedIds: string[]; plannedPlatform?: string }>();
const emit = defineEmits<{ toggle: [id: string]; connect: [] }>();
</script>

<template>
  <div v-if="!channels.length" class="cp-empty">
    <p class="cp-empty-title">No channels connected yet</p>
    <p>Connect LinkedIn in the web app's Settings, then come back to publish.</p>
    <button type="button" class="cp-connect" @click="emit('connect')">Open connections</button>
  </div>
  <div v-else class="cp-grid">
    <button
      v-for="channel in channels"
      :key="channel.id"
      type="button"
      class="cp-item"
      :class="{ selected: selectedIds.includes(channel.id) }"
      :aria-pressed="selectedIds.includes(channel.id)"
      @click="emit('toggle', channel.id)"
    >
      <ChannelAvatar :channel="channel" />
      <span class="cp-text">
        <span class="cp-name">
          {{ channel.name }}<span v-if="plannedPlatform && channel.platform === plannedPlatform" class="cp-planned">Planned</span>
        </span>
        <span class="cp-handle">{{ channel.handleOrSub }}</span>
      </span>
      <span class="cp-check"><Icon name="check" :size="12" :stroke="3" /></span>
    </button>
  </div>
</template>
