<script setup lang="ts">
import { computed, onMounted } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Session } from "../api";
import type { IconName } from "../icons";
import Icon from "../components/Icon.vue";
import CompanyAvatar from "../components/CompanyAvatar.vue";
import RefreshControls from "../components/RefreshControls.vue";
import {
  loadOverview,
  overview,
  overviewAutoRefresh,
  overviewError as error,
  overviewLoaded as loaded,
  overviewRefreshing as refreshing,
} from "../overviewStore";

const { enabled: autoRefresh, countdown } = overviewAutoRefresh;

export type OverviewTarget = "offers" | "icps" | "proof" | "ideas" | "publish";

const props = defineProps<{ session: Session }>();
const emit = defineEmits<{ navigate: [page: OverviewTarget] }>();

const numberFormat = new Intl.NumberFormat();
const fmt = (n: number) => numberFormat.format(n);
const percent = (part: number, whole: number) => (whole > 0 ? Math.round((part / whole) * 100) : 0);

const today = new Date().toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
const firstName = props.session.userName.split(" ")[0] ?? "";

function greeting() {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
}

function timeAgo(iso: string) {
  const seconds = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function formatSchedule(iso: string | null) {
  if (!iso) return "Unscheduled";
  return new Date(iso).toLocaleString(undefined, { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
}

function dayLabel(date: string, index: number, length: number) {
  if (index % 2 !== 0 && index !== length - 1) return "";
  return new Date(`${date}T00:00:00`).toLocaleDateString(undefined, { day: "numeric", month: index === 0 ? "short" : undefined });
}

function openSite(path: string) {
  openUrl(`${props.session.baseUrl}${path}`);
}

const load = loadOverview;

// Coming back to the page keeps the data and the running countdown.
onMounted(() => {
  if (!loaded.value) load();
});

const trendMax = computed(() => Math.max(1, ...(overview.value?.trend ?? []).map((d) => Math.max(d.leads, d.posts))));
const trendTotals = computed(() =>
  (overview.value?.trend ?? []).reduce((acc, d) => ({ leads: acc.leads + d.leads, posts: acc.posts + d.posts }), { leads: 0, posts: 0 }),
);
const pipelineTotal = computed(() => (overview.value?.pipeline ?? []).reduce((sum, s) => sum + s.count, 0));
const icpMax = computed(() => Math.max(1, ...(overview.value?.icps ?? []).map((i) => i.count)));
const emailPercent = computed(() => (overview.value ? percent(overview.value.stats.emails.found, overview.value.stats.leads.total) : 0));

type Attention = { key: string; icon: IconName; tone: string; title: string; detail: string; count: number; go: () => void };

const attention = computed<Attention[]>(() => {
  const o = overview.value;
  if (!o) return [];
  const items: Attention[] = (
    [
      { key: "failed", icon: "alertTriangle", tone: "tone-rose", title: "Posts failed to publish", detail: "Check the error and send them again", count: o.stats.posts.failed, go: () => emit("navigate", "publish") },
      { key: "emails", icon: "mail", tone: "tone-amber", title: "Leads without an email", detail: "Run email lookup from the queue", count: o.stats.emails.missing, go: () => openSite("/leads/queue") },
      { key: "ideas", icon: "lightbulb", tone: "tone-violet", title: "Ideas not written yet", detail: "Turn them into posts", count: o.stats.ideas.unused, go: () => openSite("/ideas") },
      { key: "drafts", icon: "calendarClock", tone: "tone-blue", title: "Drafts to schedule", detail: "Give them a slot in the calendar", count: o.stats.posts.drafts, go: () => emit("navigate", "publish") },
    ] satisfies Attention[]
  ).filter((item) => item.count > 0);
  if (!o.channels.length) {
    items.unshift({ key: "channels", icon: "radio", tone: "tone-gray", title: "No publishing channel connected", detail: "Connect LinkedIn or Facebook", count: 0, go: () => openSite("/settings/connections_destinations") });
  }
  return items;
});

const STAGE_COLORS = ["gray", "blue", "cyan", "emerald", "amber", "rose", "violet"];
const stageClass = (color: string) => `stage-${STAGE_COLORS.includes(color) ? color : "gray"}`;
</script>

<template>
  <div class="ov">
    <div class="ov-top">
      <div>
        <p class="ov-xs ov-gray-500">{{ today }}</p>
        <h1 class="ov-title">{{ greeting() }}{{ firstName ? `, ${firstName}` : "" }}</h1>
        <p v-if="overview" class="ov-sm ov-gray-500" style="margin-top: 2px">Here's how {{ overview.profile.company_name }} is growing.</p>
      </div>
      <RefreshControls v-model="autoRefresh" :countdown="countdown" :refreshing="refreshing" @refresh="load()" />
    </div>

    <div v-if="error" class="alert alert-error" role="alert">
      {{ error }}
      <div style="margin-top: 8px"><button class="btn btn-sm" :disabled="refreshing" @click="load()">Try again</button></div>
    </div>

    <template v-if="!loaded && !error">
      <div class="ov-kpis" aria-busy="true">
        <div v-for="n in 4" :key="n" class="skeleton" style="height: 108px"></div>
      </div>
      <div class="skeleton" style="height: 260px"></div>
    </template>

    <div v-else-if="loaded && !overview" class="ov-card ov-empty">
      <span class="ov-empty-icon"><Icon name="layoutDashboard" /></span>
      <p class="ov-xs ov-gray-500">Finish onboarding in the Growth Engine web app to see your overview.</p>
    </div>

    <template v-else-if="overview">
      <!-- KPI row -->
      <div class="ov-kpis">
        <button class="ov-card ov-stat" @click="openSite('/leads/queue')">
          <div class="ov-between">
            <span class="ov-xs ov-medium ov-gray-500">People found</span>
            <span class="ov-stat-icon tone-blue"><Icon name="users" /></span>
          </div>
          <div class="ov-stat-value">{{ fmt(overview.stats.leads.total) }}</div>
          <div class="ov-stat-foot">
            <span v-if="overview.stats.leads.thisWeek === overview.stats.leads.lastWeek" class="ov-xs ov-gray-500">No change vs last week</span>
            <span v-else class="ov-delta" :class="overview.stats.leads.thisWeek > overview.stats.leads.lastWeek ? 'ov-up' : 'ov-down'">
              <Icon :name="overview.stats.leads.thisWeek > overview.stats.leads.lastWeek ? 'arrowUpRight' : 'arrowDownRight'" :size="14" :stroke="2" />
              {{ overview.stats.leads.thisWeek > overview.stats.leads.lastWeek ? "+" : "" }}{{ fmt(overview.stats.leads.thisWeek - overview.stats.leads.lastWeek) }}
              <span class="ov-delta-suffix">vs last week</span>
            </span>
          </div>
        </button>

        <button class="ov-card ov-stat" @click="openSite('/leads/queue')">
          <div class="ov-between">
            <span class="ov-xs ov-medium ov-gray-500">Companies</span>
            <span class="ov-stat-icon tone-violet"><Icon name="building" /></span>
          </div>
          <div class="ov-stat-value">{{ fmt(overview.stats.companies.total) }}</div>
          <div class="ov-stat-foot ov-xs ov-gray-500">
            <span class="ov-medium ov-gray-700">{{ overview.stats.companies.thisWeek }}</span> added this week
          </div>
        </button>

        <button class="ov-card ov-stat" @click="openSite('/leads/queue')">
          <div class="ov-between">
            <span class="ov-xs ov-medium ov-gray-500">Emails found</span>
            <span class="ov-stat-icon tone-amber"><Icon name="mail" /></span>
          </div>
          <div class="ov-stat-value">{{ emailPercent }}%</div>
          <div class="ov-stat-foot ov-row">
            <div class="ov-bar" style="flex: 1"><div class="ov-bar-fill" style="background: #f59e0b" :style="{ width: `${emailPercent}%` }"></div></div>
            <span class="ov-xs ov-gray-500 ov-num">{{ overview.stats.emails.found }}/{{ overview.stats.leads.total }}</span>
          </div>
        </button>

        <button class="ov-card ov-stat" @click="emit('navigate', 'publish')">
          <div class="ov-between">
            <span class="ov-xs ov-medium ov-gray-500">Posts sent · 30 days</span>
            <span class="ov-stat-icon tone-emerald"><Icon name="send" /></span>
          </div>
          <div class="ov-stat-value">{{ fmt(overview.stats.posts.sent30d) }}</div>
          <div class="ov-stat-foot">
            <span v-if="overview.stats.posts.sent30d === overview.stats.posts.sentPrev30d" class="ov-xs ov-gray-500">No change vs prior 30 days</span>
            <span v-else class="ov-delta" :class="overview.stats.posts.sent30d > overview.stats.posts.sentPrev30d ? 'ov-up' : 'ov-down'">
              <Icon :name="overview.stats.posts.sent30d > overview.stats.posts.sentPrev30d ? 'arrowUpRight' : 'arrowDownRight'" :size="14" :stroke="2" />
              {{ overview.stats.posts.sent30d > overview.stats.posts.sentPrev30d ? "+" : "" }}{{ fmt(overview.stats.posts.sent30d - overview.stats.posts.sentPrev30d) }}
              <span class="ov-delta-suffix">vs prior 30 days</span>
            </span>
          </div>
        </button>
      </div>

      <!-- Chart + attention -->
      <div class="ov-row3">
        <section class="ov-card ov-col ov-span2">
          <div class="ov-head" style="align-items: flex-start; flex-wrap: wrap">
            <div>
              <h2 class="ov-h2">Activity</h2>
              <p class="ov-xs ov-gray-500 ov-hint">Last {{ overview.trend.length }} days</p>
            </div>
            <div class="ov-legend">
              <span><span class="ov-swatch" style="background: #3b82f6"></span> Leads added <b>{{ trendTotals.leads }}</b></span>
              <span><span class="ov-swatch" style="background: #10b981"></span> Posts sent <b>{{ trendTotals.posts }}</b></span>
            </div>
          </div>
          <div class="ov-chart-wrap">
            <div class="ov-chart">
              <div class="ov-guides" aria-hidden="true"><div v-for="i in 4" :key="i"></div></div>
              <div v-for="day in overview.trend" :key="day.date" class="ov-day">
                <div class="ov-bar-v" style="background: #3b82f6" :style="{ height: `${(day.leads / trendMax) * 100}%`, minHeight: day.leads ? '3px' : '0' }"></div>
                <div class="ov-bar-v" style="background: #10b981" :style="{ height: `${(day.posts / trendMax) * 100}%`, minHeight: day.posts ? '3px' : '0' }"></div>
                <div class="ov-tip">
                  <div class="ov-medium">{{ new Date(`${day.date}T00:00:00`).toLocaleDateString(undefined, { month: "short", day: "numeric" }) }}</div>
                  <div style="color: #d1d5db">{{ day.leads }} leads · {{ day.posts }} posts</div>
                </div>
              </div>
            </div>
            <div class="ov-axis">
              <div v-for="(day, i) in overview.trend" :key="day.date">{{ dayLabel(day.date, i, overview.trend.length) }}</div>
            </div>
          </div>
        </section>

        <section class="ov-card ov-col">
          <div class="ov-head">
            <div>
              <h2 class="ov-h2">Needs your attention</h2>
              <p v-if="attention.length" class="ov-xs ov-gray-500 ov-hint">{{ attention.length }} thing{{ attention.length === 1 ? "" : "s" }} to look at</p>
            </div>
          </div>
          <div v-if="!attention.length" class="ov-caught-up">
            <span style="color: #059669"><Icon name="checkCircle" :size="20" /></span>
            <div>
              <p class="ov-sm ov-medium" style="color: #064e3b">All caught up</p>
              <p class="ov-xs" style="color: #047857">Nothing is waiting on you right now.</p>
            </div>
          </div>
          <ul v-else class="ov-list">
            <li v-for="item in attention" :key="item.key">
              <button class="ov-list-item ov-attention" @click="item.go()">
                <span class="ov-attn-icon" :class="item.tone"><Icon :name="item.icon" /></span>
                <div class="ov-grow">
                  <p class="ov-sm ov-medium ov-gray-900 ov-truncate">{{ item.title }}</p>
                  <p class="ov-xs ov-gray-500 ov-truncate">{{ item.detail }}</p>
                </div>
                <span v-if="item.count > 0" class="ov-count">{{ fmt(item.count) }}</span>
                <span class="ov-chevron"><Icon name="arrowRight" :size="14" /></span>
              </button>
            </li>
          </ul>
        </section>
      </div>

      <!-- Leads + pipeline + posts -->
      <div class="ov-row3">
        <section class="ov-card ov-col">
          <div class="ov-head">
            <div>
              <h2 class="ov-h2">Recently added</h2>
              <p class="ov-xs ov-gray-500 ov-hint">Newest people and companies</p>
            </div>
            <button class="ov-link" @click="openSite('/leads/queue')">View all <Icon name="arrowRight" :size="14" /></button>
          </div>
          <div v-if="!overview.recentLeads.length" class="ov-empty">
            <span class="ov-empty-icon"><Icon name="users" /></span>
            <p class="ov-xs ov-gray-500">No leads yet. Import them with the Chrome extension or run a website scan.</p>
          </div>
          <ul v-else class="ov-list">
            <li v-for="lead in overview.recentLeads" :key="lead.key">
              <button class="ov-list-item" @click="openSite('/leads/queue')">
                <CompanyAvatar :label="lead.name" :image-url="lead.avatarUrl" :domain="lead.type === 'prospect' ? lead.domain : null" :round="lead.type === 'lead'" />
                <div class="ov-grow">
                  <p class="ov-sm ov-medium ov-gray-900 ov-truncate">{{ lead.name }}</p>
                  <p class="ov-xs ov-gray-500 ov-truncate">{{ lead.subtitle ?? (lead.type === "lead" ? "Person" : "Company") }}</p>
                </div>
                <span v-if="lead.type === 'prospect'" class="ov-chip">Company</span>
                <span v-else-if="lead.hasEmail" class="ov-chip ov-chip-email"><Icon name="mail" :size="12" :stroke="2" /> Email</span>
                <span class="ov-xs ov-num ov-ago">{{ timeAgo(lead.createdAt) }}</span>
              </button>
            </li>
          </ul>
        </section>

        <section class="ov-card ov-col">
          <div class="ov-head">
            <div>
              <h2 class="ov-h2">Pipeline</h2>
              <p class="ov-xs ov-gray-500 ov-hint">{{ fmt(pipelineTotal) }} people and companies</p>
            </div>
            <button class="ov-link" @click="openSite('/leads/pipeline')">Open board <Icon name="arrowRight" :size="14" /></button>
          </div>
          <div class="ov-pad">
            <div v-if="pipelineTotal === 0" class="ov-empty">
              <span class="ov-empty-icon"><Icon name="users" /></span>
              <p class="ov-xs ov-gray-500">No one in the pipeline yet. Leads you import show up here.</p>
              <button class="ov-link" style="margin-top: 12px" @click="openSite('/leads/queue')">Go to queue <Icon name="arrowRight" :size="14" /></button>
            </div>
            <template v-else>
              <div class="ov-stack-bar">
                <div
                  v-for="stage in overview.pipeline.filter((s) => s.count > 0)"
                  :key="stage.id ?? stage.name"
                  :class="stageClass(stage.color)"
                  :style="{ width: `${(stage.count / pipelineTotal) * 100}%` }"
                  :title="`${stage.name}: ${stage.count}`"
                ></div>
              </div>
              <ul class="ov-stages">
                <li v-for="stage in overview.pipeline" :key="stage.id ?? stage.name">
                  <span class="ov-stage-dot" :class="stageClass(stage.color)"></span>
                  <span class="ov-grow ov-truncate ov-gray-700">{{ stage.name }}</span>
                  <span class="ov-num ov-medium ov-gray-900">{{ fmt(stage.count) }}</span>
                  <span class="ov-num ov-xs ov-pct">{{ percent(stage.count, pipelineTotal) }}%</span>
                </li>
              </ul>
            </template>
          </div>
        </section>

        <section class="ov-card ov-col">
          <div class="ov-head">
            <div>
              <h2 class="ov-h2">Up next</h2>
              <p class="ov-xs ov-gray-500 ov-hint">{{ overview.stats.posts.scheduled }} post{{ overview.stats.posts.scheduled === 1 ? "" : "s" }} in the queue</p>
            </div>
            <button class="ov-link" @click="emit('navigate', 'publish')">Open calendar <Icon name="arrowRight" :size="14" /></button>
          </div>
          <div v-if="!overview.upcomingPosts.length" class="ov-empty">
            <span class="ov-empty-icon"><Icon name="calendarClock" /></span>
            <p class="ov-xs ov-gray-500">Nothing scheduled. Pick an idea and write your next post.</p>
            <button class="ov-link" style="margin-top: 12px" @click="emit('navigate', 'ideas')">Browse ideas <Icon name="arrowRight" :size="14" /></button>
          </div>
          <ol v-else class="ov-timeline">
            <li v-for="(post, i) in overview.upcomingPosts" :key="post.id">
              <span v-if="i < overview.upcomingPosts.length - 1" class="ov-tl-line" aria-hidden="true"></span>
              <span class="ov-tl-dot" aria-hidden="true"></span>
              <div class="ov-row ov-xs" style="gap: 8px">
                <span class="ov-medium ov-gray-900">{{ formatSchedule(post.scheduledAt) }}</span>
                <span style="color: #d1d5db">·</span>
                <span class="ov-gray-500 ov-truncate">{{ post.platform ? `${post.platform} · ${post.channel}` : post.channel }}</span>
              </div>
              <p class="ov-sm ov-gray-600 ov-clamp2" style="margin-top: 4px">{{ post.text }}</p>
            </li>
          </ol>
        </section>
      </div>

      <!-- ICPs + workspace -->
      <div class="ov-row3">
        <section class="ov-card ov-col ov-span2">
          <div class="ov-head">
            <div>
              <h2 class="ov-h2">Leads by ICP</h2>
              <p class="ov-xs ov-gray-500 ov-hint">Who you're finding the most of</p>
            </div>
            <button class="ov-link" @click="emit('navigate', 'icps')">View all <Icon name="arrowRight" :size="14" /></button>
          </div>
          <div v-if="!overview.icps.length" class="ov-empty">
            <span class="ov-empty-icon"><Icon name="target" /></span>
            <p class="ov-xs ov-gray-500">Define your ideal client profiles to see where leads come from.</p>
          </div>
          <ul v-else class="ov-icps">
            <li v-for="icp in overview.icps.slice(0, 6)" :key="icp.id">
              <button class="ov-icp" @click="emit('navigate', 'icps')">
                <div class="ov-between ov-sm">
                  <span class="ov-truncate ov-gray-700 ov-icp-name">{{ icp.name }}</span>
                  <span class="ov-num ov-medium ov-gray-900">{{ fmt(icp.count) }}</span>
                </div>
                <div class="ov-bar" style="margin-top: 6px"><div class="ov-bar-fill ov-icp-fill" :style="{ width: `${(icp.count / icpMax) * 100}%` }"></div></div>
              </button>
            </li>
          </ul>
        </section>

        <section class="ov-card ov-col">
          <div class="ov-head">
            <div>
              <h2 class="ov-h2">Workspace</h2>
              <p class="ov-xs ov-gray-500 ov-hint">Your brain and publishing channels</p>
            </div>
          </div>
          <div class="ov-brain">
            <button
              v-for="item in ([
                { label: 'Offers', value: overview.brain.offers, icon: 'layers', page: 'offers' },
                { label: 'ICPs', value: overview.brain.icps, icon: 'target', page: 'icps' },
                { label: 'Proof', value: overview.brain.proof, icon: 'award', page: 'proof' },
              ] as const)"
              :key="item.label"
              class="ov-brain-item"
              @click="emit('navigate', item.page)"
            >
              <span style="color: #9ca3af"><Icon :name="item.icon" /></span>
              <div class="ov-brain-value">{{ item.value }}</div>
              <div class="ov-xs ov-gray-500">{{ item.label }}</div>
            </button>
          </div>
          <div class="ov-channels">
            <div class="ov-between" style="margin-bottom: 8px">
              <span class="ov-xs ov-medium ov-gray-500">Channels</span>
              <button class="ov-link" @click="openSite('/settings/connections_destinations')">Manage</button>
            </div>
            <button v-if="!overview.channels.length" class="ov-connect" @click="openSite('/settings/connections_destinations')">
              <Icon name="plus" :size="14" :stroke="2" /> Connect a channel
            </button>
            <ul v-else class="ov-channel-list">
              <li v-for="channel in overview.channels" :key="channel.id" class="ov-channel">
                <CompanyAvatar :label="channel.handle" :image-url="channel.avatarUrl" size="sm" round />
                <div class="ov-grow">
                  <p class="ov-xs ov-medium ov-gray-900 ov-truncate">{{ channel.handle }}</p>
                  <p class="ov-gray-500 ov-truncate" style="font-size: 11px">{{ channel.name }}</p>
                </div>
                <span class="ov-online" title="Connected"></span>
              </li>
            </ul>
          </div>
        </section>
      </div>
    </template>
  </div>
</template>
