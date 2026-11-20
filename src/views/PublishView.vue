<script setup lang="ts">
import { ref } from "vue";
import SitePage from "../components/SitePage.vue";
import type { PublishChannel, SitePost } from "../api";

const TABS = [
  { id: "queue", label: "Queue", statuses: ["Scheduled", "Publishing"] },
  { id: "drafts", label: "Drafts", statuses: ["Draft"] },
  { id: "sent", label: "Sent", statuses: ["Sent"] },
  { id: "failed", label: "Failed", statuses: ["Failed"] },
] as const;

type TabId = (typeof TABS)[number]["id"];

const tab = ref<TabId>("queue");

function inTab(posts: SitePost[], id: TabId) {
  const statuses: readonly string[] = TABS.find((t) => t.id === id)!.statuses;
  return posts.filter((p) => statuses.includes(p.status));
}

function channelName(channels: PublishChannel[], post: SitePost) {
  return channels.find((c) => c.id === post.channelId)?.handleOrSub ?? post.channelId;
}
</script>

<template>
  <SitePage page="publish" title="Publish">
    <template #default="{ data: { channels, posts } }">
      <div class="tabs" role="tablist">
        <button
          v-for="item in TABS"
          :key="item.id"
          class="tab"
          role="tab"
          :class="{ active: tab === item.id }"
          :aria-selected="tab === item.id"
          @click="tab = item.id"
        >
          {{ item.label }}<span class="tab-count">{{ inTab(posts, item.id).length }}</span>
        </button>
      </div>

      <div v-if="!inTab(posts, tab).length" class="card empty">
        <h2>No posts here</h2>
        <p class="muted">Posts are written from ideas on the Growth Engine web app.</p>
      </div>

      <div v-else class="stack">
        <article v-for="post in inTab(posts, tab)" :key="post.id" class="card stack" style="gap: 8px">
          <div class="row-between">
            <span class="small" style="font-weight: 600">{{ channelName(channels, post) }}</span>
            <span class="muted small">
              {{ post.status === "Sent" ? post.sentAt : [post.scheduledDate, post.scheduledTime].filter(Boolean).join(" ") }}
            </span>
          </div>
          <p class="small" style="white-space: pre-wrap">{{ post.text }}</p>
          <p v-if="post.errorMessage" class="field-error">{{ post.errorMessage }}</p>
          <div v-if="post.tag"><span class="badge">{{ post.tag }}</span></div>
        </article>
      </div>
    </template>
  </SitePage>
</template>
