<script setup lang="ts">
import { computed } from "vue";
import type { PublishChannel, SitePost } from "../api";
import { editPath, publishTabPath } from "../publish";
import Icon from "./Icon.vue";
import PlatformIcon from "./PlatformIcon.vue";
import PostAnalyticsBar from "./PostAnalyticsBar.vue";

const props = defineProps<{ post: SitePost; channels: PublishChannel[] }>();
const emit = defineEmits<{ open: [post: SitePost]; web: [path: string] }>();

const channel = computed(() => props.channels.find((c) => c.id === props.post.channelId));
const tabPath = computed(() => publishTabPath(props.post));
</script>

<template>
  <div
    class="pc"
    :class="{ 'pc-failed': post.status === 'Failed' }"
    role="button"
    tabindex="0"
    @click="emit('open', post)"
    @keydown.enter.self="emit('open', post)"
  >
    <div class="pc-head">
      <div class="pc-meta">
        <PlatformIcon :platform="channel?.platform" size="sm" />
        <span class="pc-channel">{{ channel?.name || post.channelId }}</span>
        <span v-if="post.scheduledTime" class="pc-time">· {{ post.scheduledTime }}</span>
        <span v-if="post.sentAt" class="pc-time">· {{ post.sentAt }}</span>
        <span v-if="post.siblingPostIds.length" class="pc-siblings" title="Part of a multi-channel batch composition">
          <Icon name="link" :size="12" /> Created together ({{ post.siblingPostIds.length + 1 }})
        </span>
      </div>
      <span class="status-badge" :class="`status-${post.status.replace(' ', '-').toLowerCase()}`">{{ post.status }}</span>
    </div>
    <p class="pc-text">{{ post.text }}</p>
    <div v-if="post.status === 'Failed' && post.errorMessage" class="pc-error">
      <Icon name="alertCircle" :size="14" /> <span>Failed: {{ post.errorMessage }}</span>
    </div>

    <PostAnalyticsBar v-if="post.status === 'Sent'" :post="post" />

    <div class="pc-actions" @click.stop>
      <div class="pc-actions-left">
        <template v-if="post.status === 'Needs approval'">
          <button class="pc-btn pc-btn-approve" @click="emit('web', tabPath)"><Icon name="check" :size="12" /> Approve</button>
          <button class="pc-btn" @click="emit('web', tabPath)">Request changes</button>
        </template>
        <button v-if="post.status === 'Draft'" class="pc-btn pc-btn-primary" @click="emit('web', editPath(post))">
          <Icon name="arrowUp" :size="12" /> Schedule
        </button>
        <button v-if="post.status === 'Scheduled'" class="pc-btn" title="Publish immediately" @click="emit('web', tabPath)">
          <span class="pc-play"><Icon name="play" :size="12" /></span> Share now
        </button>
        <button v-if="post.status === 'Failed'" class="pc-btn pc-btn-danger" @click="emit('web', tabPath)">
          <Icon name="refresh" :size="12" /> Retry
        </button>
      </div>
      <div class="pc-actions-right">
        <button v-if="post.status !== 'Sent' && post.status !== 'Failed'" class="pc-icon" title="Edit post" aria-label="Edit post" @click="emit('web', editPath(post))">
          <Icon name="pencil" :size="14" />
        </button>
        <button class="pc-icon pc-icon-danger" title="Delete post" aria-label="Delete post" @click="emit('web', tabPath)">
          <Icon name="trash" :size="14" />
        </button>
      </div>
    </div>
  </div>
</template>
