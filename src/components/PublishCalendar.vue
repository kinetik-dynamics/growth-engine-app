<script setup lang="ts">
import { computed, ref } from "vue";
import type { PublishChannel, SitePost } from "../api";
import { addDays, dayOfWeek, parseDateStr } from "../publish";
import Icon from "./Icon.vue";
import PlatformIcon from "./PlatformIcon.vue";

const props = defineProps<{ channels: PublishChannel[]; posts: SitePost[]; selectedChannelId: string; today: string }>();
const emit = defineEmits<{ open: [post: SitePost] }>();

const view = ref<"week" | "month">("week");
const weekOffset = ref(0);
const draftsOpen = ref(false);
const moreFor = ref<{ date: string; posts: SitePost[] } | null>(null);

const startDay = computed(() => addDays(addDays(props.today, -parseDateStr(props.today).getUTCDay()), weekOffset.value * 7));

const weekDays = computed(() =>
  Array.from({ length: 7 }, (_, i) => {
    const date = addDays(startDay.value, i);
    return { date, dayOfWeek: dayOfWeek(date), dayNum: parseDateStr(date).getUTCDate(), isToday: date === props.today };
  }),
);

const monthDays = computed(() =>
  Array.from({ length: 28 }, (_, i) => {
    const date = addDays(startDay.value, i);
    return { date, dayNum: parseDateStr(date).getUTCDate(), isToday: date === props.today };
  }),
);

const inChannel = computed(() => props.posts.filter((p) => props.selectedChannelId === "all" || p.channelId === props.selectedChannelId));
const drafts = computed(() => inChannel.value.filter((p) => p.status === "Draft"));

function postsOn(date: string) {
  return inChannel.value.filter((p) => p.scheduledDate === date);
}

function platformOf(post: SitePost) {
  return props.channels.find((c) => c.id === post.channelId)?.platform;
}

function channelOf(post: SitePost) {
  return props.channels.find((c) => c.id === post.channelId);
}

function openFromMore(post: SitePost) {
  moreFor.value = null;
  emit("open", post);
}
</script>

<template>
  <div class="cal">
    <div class="cal-main">
      <div class="cal-toolbar">
        <div class="cal-toolbar-left">
          <div class="cal-segment" role="group" aria-label="Calendar view">
            <button :class="{ active: view === 'week' }" @click="view = 'week'">Week</button>
            <button :class="{ active: view === 'month' }" @click="view = 'month'">Month</button>
          </div>
          <div class="cal-nav">
            <button class="cal-arrow" title="Previous week" aria-label="Previous week" @click="weekOffset--"><Icon name="chevronLeft" :size="14" /></button>
            <button class="cal-today" @click="weekOffset = 0">Today</button>
            <button class="cal-arrow" title="Next week" aria-label="Next week" @click="weekOffset++"><Icon name="chevronRight" :size="14" /></button>
          </div>
          <span class="cal-range">{{ weekDays[0].date }} — {{ (view === "week" ? weekDays[6] : monthDays[27]).date }}</span>
        </div>
        <button class="cal-drafts-btn" :class="{ active: draftsOpen }" @click="draftsOpen = !draftsOpen">
          <Icon name="fileText" :size="14" /> Drafts ({{ drafts.length }})
        </button>
      </div>

      <div v-if="view === 'week'" class="cal-grid cal-week">
        <div v-for="day in weekDays" :key="day.date" class="cal-cell" :class="{ 'cal-cell-today': day.isToday }">
          <div class="cal-day-head">
            <span class="cal-dow">{{ day.dayOfWeek }}</span>
            <span class="cal-num" :class="{ today: day.isToday }">{{ day.dayNum }}</span>
          </div>
          <div class="cal-chips">
            <button
              v-for="post in postsOn(day.date).slice(0, 3)"
              :key="post.id"
              class="cal-chip"
              :title="post.text"
              @click="emit('open', post)"
            >
              <PlatformIcon :platform="platformOf(post)" size="dot" />
              <span class="cal-chip-time">{{ post.scheduledTime || "—" }}</span>
              <span class="cal-chip-text">{{ post.text.substring(0, 28) }}</span>
              <span class="cal-status-dot" :class="`dot-${post.status.replace(' ', '-').toLowerCase()}`" :title="post.status"></span>
            </button>
            <button v-if="postsOn(day.date).length > 3" class="cal-more" @click="moreFor = { date: day.date, posts: postsOn(day.date) }">
              +{{ postsOn(day.date).length - 3 }} more
            </button>
          </div>
        </div>
      </div>

      <div v-else class="cal-grid cal-month">
        <div v-for="day in monthDays" :key="day.date" class="cal-cell cal-cell-month" :class="{ 'cal-cell-today': day.isToday }">
          <span class="cal-num cal-num-sm" :class="{ today: day.isToday }">{{ day.dayNum }}</span>
          <div class="cal-chips" style="gap: 4px; margin-top: 4px">
            <button
              v-for="post in postsOn(day.date).slice(0, 2)"
              :key="post.id"
              class="cal-chip cal-chip-sm"
              :title="post.text"
              @click="emit('open', post)"
            >
              <PlatformIcon :platform="platformOf(post)" size="dot" />
              <span class="cal-chip-text">{{ post.text }}</span>
            </button>
            <button v-if="postsOn(day.date).length > 2" class="cal-more cal-more-sm" @click="moreFor = { date: day.date, posts: postsOn(day.date) }">
              +{{ postsOn(day.date).length - 2 }} more
            </button>
          </div>
        </div>
      </div>
    </div>

    <aside v-if="draftsOpen" class="cal-tray" aria-label="Drafts">
      <div class="cal-tray-head">
        <span><Icon name="fileText" /> Drafts ({{ drafts.length }})</span>
        <button class="pd-close" aria-label="Close drafts" @click="draftsOpen = false"><Icon name="x" /></button>
      </div>
      <div class="cal-tray-body">
        <p v-if="!drafts.length" class="cal-tray-empty">No drafts available.</p>
        <button v-for="draft in drafts" :key="draft.id" class="cal-draft" @click="emit('open', draft)">
          <span class="cal-draft-head">
            <span class="cal-draft-channel">
              <PlatformIcon :platform="platformOf(draft)" size="dot" />
              {{ channelOf(draft)?.name.split(" · ")[0] }}
            </span>
            <span class="cal-draft-badge">Draft</span>
          </span>
          <span class="cal-draft-text">{{ draft.text }}</span>
          <span class="cal-draft-foot">{{ draft.pattern || "General" }}</span>
        </button>
      </div>
    </aside>

    <div v-if="moreFor" class="pd" role="dialog" aria-modal="true" :aria-label="`Posts on ${moreFor.date}`">
      <div class="pd-backdrop" @click="moreFor = null"></div>
      <div class="cal-popover">
        <div class="cal-popover-head">
          <span>Posts on {{ moreFor.date }} ({{ moreFor.posts.length }})</span>
          <button class="pd-close" aria-label="Close" @click="moreFor = null"><Icon name="x" /></button>
        </div>
        <div class="cal-popover-list">
          <button v-for="post in moreFor.posts" :key="post.id" class="cal-popover-item" @click="openFromMore(post)">
            <span class="cal-popover-row">
              <b>{{ channelOf(post)?.name }}</b>
              <span>{{ post.scheduledTime }}</span>
            </span>
            <span class="cal-draft-text">{{ post.text }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
