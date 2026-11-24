<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import type { PublishChannel, SitePost } from "../api";
import { editPath, publishTabPath } from "../publish";
import Icon from "./Icon.vue";
import PostAnalytics from "./PostAnalytics.vue";

const props = defineProps<{ post: SitePost; channels: PublishChannel[]; posts: SitePost[] }>();
const emit = defineEmits<{ close: []; select: [post: SitePost]; web: [path: string] }>();

const tabPath = computed(() => publishTabPath(props.post));

const channel = computed(() => props.channels.find((c) => c.id === props.post.channelId));
const siblings = computed(() =>
  props.post.siblingPostIds.map((id) => props.posts.find((p) => p.id === id)).filter((p): p is SitePost => Boolean(p)),
);
const platformColor = computed(
  () => ({ LinkedIn: "#0A66C2", Facebook: "#1877F2", X: "#000", Website: "#047857" })[channel.value?.platform ?? ""] ?? "#4b5563",
);

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="pd" role="dialog" aria-modal="true" :aria-label="`Post on ${channel?.name || post.channelId}`">
    <div class="pd-backdrop" @click="emit('close')"></div>
    <div class="pd-panel">
      <div class="pd-head">
        <div class="pd-title">
          <span class="pd-platform" :style="{ background: platformColor }">{{ channel?.platform || "Channel" }}</span>
          <h2>{{ channel?.name || post.channelId }}</h2>
          <span class="status-badge" :class="`status-${post.status.replace(' ', '-').toLowerCase()}`">{{ post.status }}</span>
        </div>
        <div class="pd-head-actions">
          <button v-if="post.status !== 'Sent'" class="pd-edit" title="Edit in the web app" @click="emit('web', editPath(post))">
            <Icon name="pencil" :size="14" /> Edit
          </button>
          <button class="pd-close" aria-label="Close" @click="emit('close')"><Icon name="x" /></button>
        </div>
      </div>

      <div class="pd-body">
        <div v-if="siblings.length" class="pd-siblings">
          <p class="pd-siblings-title"><Icon name="link" :size="14" /> Created together in one composition ({{ siblings.length + 1 }} channels)</p>
          <p class="pd-siblings-hint">This post has sibling versions tailored for other channels:</p>
          <div class="pd-sibling-list">
            <button v-for="sibling in siblings" :key="sibling.id" class="pd-sibling" @click="emit('select', sibling)">
              {{ channels.find((c) => c.id === sibling.channelId)?.name || sibling.channelId }}
              <span>({{ sibling.status }})</span>
            </button>
          </div>
        </div>

        <div class="pd-facts">
          <div class="pd-fact">
            <span>Posting Time:</span>
            <b>{{ post.scheduledDate ? `${post.scheduledDate} at ${post.scheduledTime}` : post.sentAt || "Unscheduled" }}</b>
          </div>
          <div v-if="post.pattern" class="pd-fact pd-fact-sm">
            <span>Writing Pattern:</span>
            <b>{{ post.pattern }}</b>
          </div>
          <div v-if="post.tag" class="pd-fact pd-fact-sm">
            <span>Tag:</span>
            <b>{{ post.tag }}</b>
          </div>
        </div>

        <PostAnalytics
          v-if="post.status === 'Sent'"
          :post-id="post.id"
          :channel-id="post.channelId"
          :channel-platform="channel?.platform"
          :initial-platform-post-id="post.platformPostId"
          :initial-post-url="post.postUrl"
        />

        <div v-if="post.status === 'Failed' && post.errorMessage" class="pc-error">
          <Icon name="alertCircle" :size="14" /> <span>Failed: {{ post.errorMessage }}</span>
        </div>

        <div>
          <h3 class="pd-label">Message Content</h3>
          <div class="pd-content">{{ post.text }}</div>
        </div>
      </div>

      <div class="pd-foot">
        <button class="pd-delete" @click="emit('web', tabPath)"><Icon name="trash" :size="14" /> Delete</button>
        <div class="pd-foot-right">
          <template v-if="post.status === 'Needs approval'">
            <button class="pd-btn" @click="emit('web', tabPath)">Request changes</button>
            <button class="pd-btn pd-btn-approve" @click="emit('web', tabPath)"><Icon name="check" :size="14" /> Approve &amp; Schedule</button>
          </template>
          <button v-if="post.status === 'Scheduled'" class="pd-btn pd-btn-primary" @click="emit('web', tabPath)">
            <Icon name="play" :size="14" /> Share now
          </button>
          <button v-if="post.status === 'Failed'" class="pd-btn pd-btn-danger" @click="emit('web', tabPath)">
            <Icon name="refresh" :size="14" /> Retry
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
