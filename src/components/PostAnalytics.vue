<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type PostAnalytics } from "../api";
import Icon from "./Icon.vue";

const props = defineProps<{
  postId: string;
  channelId: string;
  channelPlatform?: string | null;
  initialPlatformPostId?: string | null;
  initialPostUrl?: string | null;
  autoFetch?: boolean;
}>();

const emit = defineEmits<{
  updated: [analytics: PostAnalytics];
}>();

const analytics = ref<PostAnalytics | null>(null);
const loading = ref(false);
const error = ref("");
const editingId = ref(false);
const customIdInput = ref("");
const savingId = ref(false);

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

const lastUpdatedFormatted = computed(() => {
  if (!analytics.value?.lastUpdated) return "Never updated";
  return `Updated ${timeAgo(analytics.value.lastUpdated)}`;
});

const lastUpdatedFull = computed(() => {
  if (!analytics.value?.lastUpdated) return "";
  return new Date(analytics.value.lastUpdated).toLocaleString();
});

const currentPlatformId = computed(() => {
  return analytics.value?.platformPostId || props.initialPlatformPostId || null;
});

const currentPostUrl = computed(() => {
  return analytics.value?.postUrl || props.initialPostUrl || null;
});

async function load() {
  try {
    const cached = await api.getPostAnalytics(props.postId);
    if (cached) {
      analytics.value = cached;
      emit("updated", cached);
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
    const res = await api.fetchPostAnalytics(
      props.postId,
      props.channelId,
      currentPlatformId.value,
      currentPostUrl.value,
    );
    analytics.value = res;
    emit("updated", res);
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
}

function startEditingId() {
  customIdInput.value = currentPlatformId.value || currentPostUrl.value || "";
  editingId.value = true;
}

async function saveCustomId() {
  const input = customIdInput.value.trim();
  if (!input) return;
  savingId.value = true;
  error.value = "";
  try {
    const isUrl = input.startsWith("http://") || input.startsWith("https://");
    const url = isUrl ? input : currentPostUrl.value;
    await api.setPostPlatformId(props.postId, input, url);
    editingId.value = false;
    await refresh();
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    savingId.value = false;
  }
}

function openExternalPost() {
  if (currentPostUrl.value) {
    openUrl(currentPostUrl.value);
  }
}

watch(
  () => props.postId,
  () => load(),
);

onMounted(() => {
  load();
});
</script>

<template>
  <div class="pa">
    <div class="pa-header">
      <div class="pa-header-left">
        <h3 class="pa-title">
          <Icon name="barChart2" :size="15" /> Post Analytics
        </h3>
        <span v-if="analytics?.lastUpdated" class="pa-updated" :title="lastUpdatedFull">
          {{ lastUpdatedFormatted }}
        </span>
      </div>

      <button
        class="pa-btn-refresh"
        :class="{ spinning: loading }"
        :disabled="loading"
        title="Refresh metrics from platform API"
        @click="refresh"
      >
        <Icon name="refresh" :size="13" />
        <span>{{ loading ? "Refreshing…" : "Refresh" }}</span>
      </button>
    </div>

    <!-- Error or Token Warning banner -->
    <div v-if="error || analytics?.error" class="pa-banner pa-banner-warning">
      <Icon name="alertCircle" :size="14" />
      <span>{{ error || analytics?.error }}</span>
    </div>

    <!-- Metrics Grid -->
    <div class="pa-grid">
      <!-- 1. Impressions -->
      <div
        class="pa-card"
        :class="{ unavailable: !analytics?.impressions?.available }"
        :title="analytics?.impressions?.available ? 'Total impressions across feed and shares' : (analytics?.impressions?.unavailableReason || 'Impressions metric unavailable')"
      >
        <div class="pa-card-head">
          <span class="pa-card-label">Impressions</span>
          <span class="pa-card-icon icon-impr"><Icon name="barChart2" :size="14" /></span>
        </div>
        <div class="pa-card-value">
          {{ analytics?.impressions?.formatted ?? (loading ? '…' : '—') }}
        </div>
        <div v-if="!analytics?.impressions?.available && analytics?.impressions?.unavailableReason" class="pa-card-note">
          {{ analytics.impressions.unavailableReason }}
        </div>
      </div>

      <!-- 2. Views -->
      <div
        class="pa-card"
        :class="{ unavailable: !analytics?.views?.available }"
        :title="analytics?.views?.available ? 'Unique viewers or reach' : (analytics?.views?.unavailableReason || 'Views metric unavailable')"
      >
        <div class="pa-card-head">
          <span class="pa-card-label">Views</span>
          <span class="pa-card-icon icon-views"><Icon name="eye" :size="14" /></span>
        </div>
        <div class="pa-card-value">
          {{ analytics?.views?.formatted ?? (loading ? '…' : '—') }}
        </div>
        <div v-if="!analytics?.views?.available && analytics?.views?.unavailableReason" class="pa-card-note">
          {{ analytics.views.unavailableReason }}
        </div>
      </div>

      <!-- 3. Likes -->
      <div
        class="pa-card"
        :class="{ unavailable: !analytics?.likes?.available }"
        :title="analytics?.likes?.available ? 'Likes and reactions' : (analytics?.likes?.unavailableReason || 'Likes metric unavailable')"
      >
        <div class="pa-card-head">
          <span class="pa-card-label">Likes</span>
          <span class="pa-card-icon icon-likes"><Icon name="heart" :size="14" /></span>
        </div>
        <div class="pa-card-value">
          {{ analytics?.likes?.formatted ?? (loading ? '…' : '—') }}
        </div>
        <div v-if="!analytics?.likes?.available && analytics?.likes?.unavailableReason" class="pa-card-note">
          {{ analytics.likes.unavailableReason }}
        </div>
      </div>

      <!-- 4. Comments -->
      <div
        class="pa-card"
        :class="{ unavailable: !analytics?.comments?.available }"
        :title="analytics?.comments?.available ? 'Comments on post' : (analytics?.comments?.unavailableReason || 'Comments metric unavailable')"
      >
        <div class="pa-card-head">
          <span class="pa-card-label">Comments</span>
          <span class="pa-card-icon icon-comments"><Icon name="messageSquare" :size="14" /></span>
        </div>
        <div class="pa-card-value">
          {{ analytics?.comments?.formatted ?? (loading ? '…' : '—') }}
        </div>
        <div v-if="!analytics?.comments?.available && analytics?.comments?.unavailableReason" class="pa-card-note">
          {{ analytics.comments.unavailableReason }}
        </div>
      </div>
    </div>

    <!-- Platform Post ID / Link Section -->
    <div class="pa-postid-box">
      <div class="pa-postid-info">
        <span class="pa-postid-label">Platform Post ID:</span>
        <code v-if="currentPlatformId" class="pa-postid-code" :title="currentPlatformId">{{ currentPlatformId }}</code>
        <span v-else class="pa-postid-missing">Not linked yet</span>
      </div>

      <div class="pa-postid-actions">
        <button
          v-if="currentPostUrl"
          class="pa-btn-link"
          title="Open live post on platform"
          @click="openExternalPost"
        >
          <Icon name="externalLink" :size="12" /> View post
        </button>
        <button
          class="pa-btn-edit"
          :title="currentPlatformId ? 'Change platform post ID or URL' : 'Link platform post ID or URL'"
          @click="startEditingId"
        >
          <Icon name="pencil" :size="12" /> {{ currentPlatformId ? 'Edit ID' : 'Link post ID' }}
        </button>
      </div>
    </div>

    <!-- Edit ID Prompt -->
    <div v-if="editingId" class="pa-edit-form">
      <label class="pa-edit-label">Enter Platform Post ID or URL:</label>
      <div class="pa-edit-row">
        <input
          v-model="customIdInput"
          type="text"
          class="input pa-edit-input"
          placeholder="e.g. urn:li:share:7123456789 or https://..."
          @keydown.enter="saveCustomId"
        />
        <button class="btn btn-sm btn-primary" :disabled="savingId || !customIdInput.trim()" @click="saveCustomId">
          {{ savingId ? 'Saving…' : 'Save & Refresh' }}
        </button>
        <button class="btn btn-sm btn-ghost" :disabled="savingId" @click="editingId = false">
          Cancel
        </button>
      </div>
      <p class="pa-edit-hint">
        Paste the full post URL from LinkedIn or Facebook, or the numeric ID / URN.
      </p>
    </div>

    <!-- Latest Comments Section (if available) -->
    <div v-if="analytics?.latestComments?.length" class="pa-comments-section">
      <h4 class="pa-comments-title">
        <Icon name="messageSquare" :size="13" /> Recent Comments ({{ analytics.latestComments.length }})
      </h4>
      <div class="pa-comments-list">
        <div v-for="c in analytics.latestComments" :key="c.id" class="pa-comment-card">
          <div class="pa-comment-head">
            <span class="pa-comment-author">{{ c.author }}</span>
            <span v-if="c.createdAt" class="pa-comment-date">{{ c.createdAt }}</span>
          </div>
          <div class="pa-comment-text">{{ c.text }}</div>
        </div>
      </div>
    </div>
  </div>
</template>
