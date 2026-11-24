<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, errorMessage, type PostAnalytics, type SitePost } from "../api";
import Icon from "./Icon.vue";

const props = defineProps<{
  post: SitePost;
  autoFetch?: boolean;
}>();

const analytics = ref<PostAnalytics | null>(null);
const loading = ref(false);
const error = ref("");

function timeAgo(ms: number): string {
  if (!ms) return "";
  const sec = Math.max(0, Math.floor((Date.now() - ms) / 1000));
  if (sec < 60) return "just now";
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const days = Math.floor(hr / 24);
  return `${days}d ago`;
}

const lastUpdatedText = computed(() => {
  if (!analytics.value?.lastUpdated) return "";
  return timeAgo(analytics.value.lastUpdated);
});

async function load() {
  try {
    const cached = await api.getPostAnalytics(props.post.id);
    if (cached) {
      analytics.value = cached;
    } else if (props.autoFetch !== false) {
      await refresh();
    }
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

async function refresh() {
  if (loading.value) return;
  loading.value = true;
  error.value = "";
  try {
    analytics.value = await api.fetchPostAnalytics(
      props.post.id,
      props.post.channelId,
      props.post.platformPostId,
      props.post.postUrl,
    );
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  load();
});
</script>

<template>
  <div class="pab" @click.stop>
    <div class="pab-metrics">
      <!-- Impressions -->
      <div
        class="pab-item"
        :class="{ unavailable: !analytics?.impressions?.available }"
        :title="analytics?.impressions?.available ? 'Impressions' : (analytics?.impressions?.unavailableReason || 'Impressions unavailable for this channel/scope')"
      >
        <span class="pab-icon"><Icon name="barChart2" :size="12" /></span>
        <span class="pab-label">Impr:</span>
        <span class="pab-value">{{ analytics?.impressions?.formatted ?? '—' }}</span>
      </div>

      <!-- Views -->
      <div
        class="pab-item"
        :class="{ unavailable: !analytics?.views?.available }"
        :title="analytics?.views?.available ? 'Views / Reach' : (analytics?.views?.unavailableReason || 'Views unavailable for this channel/scope')"
      >
        <span class="pab-icon"><Icon name="eye" :size="12" /></span>
        <span class="pab-label">Views:</span>
        <span class="pab-value">{{ analytics?.views?.formatted ?? '—' }}</span>
      </div>

      <!-- Likes -->
      <div
        class="pab-item"
        :class="{ unavailable: !analytics?.likes?.available }"
        :title="analytics?.likes?.available ? 'Likes / Reactions' : (analytics?.likes?.unavailableReason || 'Likes unavailable for this channel/scope')"
      >
        <span class="pab-icon"><Icon name="heart" :size="12" /></span>
        <span class="pab-label">Likes:</span>
        <span class="pab-value">{{ analytics?.likes?.formatted ?? '—' }}</span>
      </div>

      <!-- Comments -->
      <div
        class="pab-item"
        :class="{ unavailable: !analytics?.comments?.available }"
        :title="analytics?.comments?.available ? 'Comments' : (analytics?.comments?.unavailableReason || 'Comments unavailable for this channel/scope')"
      >
        <span class="pab-icon"><Icon name="messageSquare" :size="12" /></span>
        <span class="pab-label">Comments:</span>
        <span class="pab-value">{{ analytics?.comments?.formatted ?? '—' }}</span>
      </div>
    </div>

    <div class="pab-meta">
      <span v-if="lastUpdatedText && !loading" class="pab-time" :title="new Date(analytics?.lastUpdated || 0).toLocaleString()">
        {{ lastUpdatedText }}
      </span>
      <span v-else-if="loading" class="pab-time">Updating…</span>
      <button
        class="pab-refresh"
        :class="{ spinning: loading }"
        title="Refresh post analytics"
        aria-label="Refresh post analytics"
        :disabled="loading"
        @click="refresh"
      >
        <Icon name="refresh" :size="11" />
      </button>
    </div>
  </div>
</template>
