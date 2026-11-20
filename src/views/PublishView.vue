<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type PublishChannel, type Session, type SitePost } from "../api";
import { CONNECTIONS_PATH, channelLabel, friendlyDate, todayLocalDate } from "../publish";
import Icon from "../components/Icon.vue";
import PlatformIcon from "../components/PlatformIcon.vue";
import PostCard from "../components/PostCard.vue";
import PostDrawer from "../components/PostDrawer.vue";
import PublishCalendar from "../components/PublishCalendar.vue";
import RefreshControls from "../components/RefreshControls.vue";
import { createAutoRefresh } from "../autoRefresh";

type Tab = "queue" | "calendar" | "drafts" | "approvals" | "sent";

const props = defineProps<{ session: Session }>();

/** Changing a post happens in the web app; the app only links there. */
function openSite(path: string) {
  openUrl(`${props.session.baseUrl}${path}`);
}

const TABS: { id: Tab; label: string }[] = [
  { id: "queue", label: "Queue" },
  { id: "calendar", label: "Calendar" },
  { id: "drafts", label: "Drafts" },
  { id: "approvals", label: "Approvals" },
  { id: "sent", label: "Sent" },
];

const channels = ref<PublishChannel[]>([]);
const posts = ref<SitePost[] | null>(null);
const error = ref("");
const busy = ref(false);
const tab = ref<Tab>("queue");
const selectedChannelId = ref("all");
const drawerPostId = ref<string | null>(null);
const today = ref(todayLocalDate());

/** Keeps the current posts on screen while a refresh is in flight. */
async function load() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await api.sitePage("publish");
    channels.value = result.channels;
    posts.value = result.posts;
    today.value = todayLocalDate();
    if (selectedChannelId.value !== "all" && !result.channels.some((c) => c.id === selectedChannelId.value)) selectedChannelId.value = "all";
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = false;
    autoRefresh.restart();
  }
}

// Leaving the page stops the clock; coming back loads fresh posts and starts it over.
const autoRefresh = createAutoRefresh("publish.autoRefresh", () => load(), () => busy.value);
const { enabled: autoRefreshOn, countdown } = autoRefresh;

const allPosts = computed(() => posts.value ?? []);
const inChannel = computed(() => allPosts.value.filter((p) => selectedChannelId.value === "all" || p.channelId === selectedChannelId.value));

const drafts = computed(() => inChannel.value.filter((p) => p.status === "Draft"));
const approvals = computed(() => inChannel.value.filter((p) => p.status === "Needs approval"));
const sent = computed(() => inChannel.value.filter((p) => p.status === "Sent" || p.status === "Failed"));

const tabCounts = computed<Record<Tab, number>>(() => ({
  queue: inChannel.value.filter((p) => p.status === "Scheduled").length,
  calendar: 0,
  drafts: drafts.value.length,
  approvals: approvals.value.length,
  sent: sent.value.length,
}));

function queuedIn(channelId: string) {
  return allPosts.value.filter((p) => p.channelId === channelId && p.status === "Scheduled").length;
}
const totalQueued = computed(() => allPosts.value.filter((p) => p.status === "Scheduled").length);

/** Scheduled posts grouped by day, soonest first; days before today are marked overdue. */
const queueGroups = computed(() => {
  const scheduled = inChannel.value
    .filter((p) => p.status === "Scheduled" && p.scheduledDate)
    .sort((a, b) => `${a.scheduledDate} ${a.scheduledTime ?? "12:00"}`.localeCompare(`${b.scheduledDate} ${b.scheduledTime ?? "12:00"}`));
  const groups: { date: string; label: string; posts: SitePost[] }[] = [];
  for (const post of scheduled) {
    const date = post.scheduledDate as string;
    let group = groups[groups.length - 1];
    if (!group || group.date !== date) {
      const label = date < today.value ? `Overdue · ${friendlyDate(date, today.value)}` : friendlyDate(date, today.value);
      group = { date, label, posts: [] };
      groups.push(group);
    }
    group.posts.push(post);
  }
  return groups;
});

const selectedChannel = computed(() => channels.value.find((c) => c.id === selectedChannelId.value));
const drawerPost = computed(() => allPosts.value.find((p) => p.id === drawerPostId.value) ?? null);

function openPost(post: SitePost) {
  drawerPostId.value = post.id;
}

onMounted(() => {
  load();
  autoRefresh.start();
});
onUnmounted(() => autoRefresh.stop());
</script>

<template>
  <div class="pub">
    <div class="pub-header">
      <div class="pub-header-left">
        <h1 class="pub-title">Publish</h1>
        <nav class="pub-tabs" aria-label="Publish tabs">
          <button
            v-for="item in TABS"
            :key="item.id"
            class="pub-tab"
            :class="{ active: tab === item.id }"
            :aria-current="tab === item.id ? 'page' : undefined"
            @click="tab = item.id"
          >
            <span>{{ item.label }}</span>
            <span v-if="tabCounts[item.id] > 0" class="pub-tab-count" :class="{ amber: item.id === 'approvals' }">{{ tabCounts[item.id] }}</span>
          </button>
        </nav>
      </div>
      <RefreshControls v-model="autoRefreshOn" :countdown="countdown" :refreshing="busy" size="sm" @refresh="load()" />
    </div>

    <div class="pub-layout">
      <aside class="pub-channels" aria-label="Channels">
        <div class="pub-channels-head">
          <span>Channels</span>
          <button class="pub-connect-link" title="Connect a new channel" @click="openSite(CONNECTIONS_PATH)">
            <Icon name="plus" :size="14" /> Connect
          </button>
        </div>
        <div class="pub-channels-list">
          <button class="pub-all" :class="{ active: selectedChannelId === 'all' }" @click="selectedChannelId = 'all'">
            <span class="pub-all-left"><span class="pub-all-icon">All</span><span>All channels</span></span>
            <span class="pub-all-count">{{ totalQueued }}</span>
          </button>
          <div class="pub-divider"></div>
          <p v-if="posts && !channels.length" class="pub-no-channels">No channels connected yet. Connect LinkedIn in the web app's Settings to start publishing.</p>
          <button
            v-for="channel in channels"
            :key="channel.id"
            class="pub-channel"
            :class="{ active: selectedChannelId === channel.id, disconnected: channel.status === 'not_connected' }"
            :title="`${channel.name} (${channel.handleOrSub})`"
            @click="selectedChannelId = channel.id"
          >
            <PlatformIcon :platform="channel.platform" />
            <span class="pub-channel-text">
              <span class="pub-channel-name">
                <span>{{ channelLabel(channel) }}</span>
                <span
                  v-if="channel.status === 'connected'"
                  class="pub-health"
                  :class="channel.isPaused ? 'paused' : 'ok'"
                  :title="channel.isPaused ? 'Queue paused' : 'Connected & healthy'"
                ></span>
                <span v-else-if="channel.status === 'pending_review'" class="pub-health review" title="Pending LinkedIn review"></span>
                <span v-else-if="channel.status === 'needs_attention'" class="pub-health bad" title="Needs attention"></span>
              </span>
              <span class="pub-channel-sub">
                {{ channel.status === "pending_review" ? "Review pending" : channel.isPaused ? "Queue paused" : channel.handleOrSub }}
              </span>
            </span>
            <span v-if="channel.status !== 'pending_review' && channel.status !== 'not_connected'" class="pub-channel-count">{{ queuedIn(channel.id) }}</span>
          </button>
        </div>
        <div class="pub-channels-foot">
          <button class="pub-connect-btn" @click="openSite(CONNECTIONS_PATH)"><Icon name="plus" :size="14" /> Connect channel</button>
        </div>
      </aside>

      <main class="pub-main" :class="{ 'bp-busy': busy && posts }">
        <div v-if="error" class="alert alert-error" role="alert" style="margin-bottom: 16px">
          {{ error }}
          <div style="margin-top: 8px"><button class="btn btn-sm" :disabled="busy" @click="load()">Try again</button></div>
        </div>

        <div v-if="!posts && !error" class="pub-section" aria-busy="true">
          <div v-for="n in 3" :key="n" class="skeleton" style="height: 96px"></div>
        </div>

        <template v-else-if="posts">
          <!-- Queue -->
          <div v-if="tab === 'queue'" class="pub-section">
            <div v-if="selectedChannel?.isPaused" class="pub-banner-paused">
              <b>Queue is paused for {{ selectedChannel.name }}.</b>
              <span>Posts will stay queued and will not publish until resumed.</span>
            </div>
            <div v-if="!queueGroups.length" class="pub-empty">
              <Icon name="clock" :size="32" />
              <p class="pub-empty-title">Nothing in the queue</p>
              <p>Write a post from an idea and press Send to schedule it here.</p>
            </div>
            <div class="pub-days">
              <div v-for="group in queueGroups" :key="group.date" class="pub-day">
                <div class="pub-day-head">
                  <h3>{{ group.label }}</h3>
                  <span></span>
                </div>
                <div class="pub-day-posts">
                  <PostCard v-for="post in group.posts" :key="post.id" :post="post" :channels="channels" @open="openPost" @web="openSite" />
                </div>
              </div>
            </div>
          </div>

          <!-- Calendar -->
          <PublishCalendar
            v-else-if="tab === 'calendar'"
            :channels="channels"
            :posts="allPosts"
            :selected-channel-id="selectedChannelId"
            :today="today"
            @open="openPost"
          />

          <!-- Drafts -->
          <div v-else-if="tab === 'drafts'" class="pub-section">
            <div class="pub-section-head">
              <h2>Drafts ({{ drafts.length }})</h2>
              <p>Unscheduled posts saved by the team or converted from ideas.</p>
            </div>
            <div v-if="!drafts.length" class="pub-empty">
              <Icon name="fileText" :size="32" />
              <p class="pub-empty-title">No drafts currently saved</p>
              <p>Open an idea in the web app, write the post, and press Save as draft.</p>
            </div>
            <div v-else class="pub-list">
              <PostCard v-for="post in drafts" :key="post.id" :post="post" :channels="channels" @open="openPost" @web="openSite" />
            </div>
          </div>

          <!-- Approvals -->
          <div v-else-if="tab === 'approvals'" class="pub-section">
            <div class="pub-section-head">
              <h2>Waiting for Approval ({{ approvals.length }})</h2>
              <p>Posts drafted by agents requiring human sign-off before joining channel queues.</p>
            </div>
            <div v-if="!approvals.length" class="pub-empty">
              <span style="color: #059669"><Icon name="check" :size="32" /></span>
              <p class="pub-empty-title">All caught up!</p>
              <p>No posts currently waiting for human approval.</p>
            </div>
            <div v-else class="pub-list">
              <PostCard v-for="post in approvals" :key="post.id" :post="post" :channels="channels" @open="openPost" @web="openSite" />
            </div>
          </div>

          <!-- Sent -->
          <div v-else class="pub-section">
            <div class="pub-section-head">
              <h2>Sent Posts &amp; Delivery Results ({{ sent.length }})</h2>
              <p>Click any post to open its details.</p>
            </div>
            <div v-if="!sent.length" class="pub-empty">
              <p class="pub-empty-title">No sent posts found</p>
              <p>When posts in the queue reach their scheduled slot, they appear here.</p>
            </div>
            <div v-else class="pub-list">
              <PostCard v-for="post in sent" :key="post.id" :post="post" :channels="channels" @open="openPost" @web="openSite" />
            </div>
          </div>
        </template>
      </main>
    </div>

    <PostDrawer
      v-if="drawerPost"
      :post="drawerPost"
      :channels="channels"
      :posts="allPosts"
      @close="drawerPostId = null"
      @select="openPost"
      @web="openSite"
    />
  </div>
</template>
