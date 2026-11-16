<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  api,
  errorMessage,
  ideaCountFor,
  type CliStatus,
  type Idea,
  type Offer,
  type OfferIdeaCount,
  type PlanProgress,
} from "../api";
import { channelsFor, MAX_POSTS_PER_CHANNEL, nextMonday, slotsFor, weekLabel } from "../channels";

const props = defineProps<{ websiteUrl: string | null }>();
const emit = defineEmits<{ close: []; generated: [ideas: Idea[], requested: number]; openSettings: [] }>();

const STEPS = ["Posts per channel", "Creation mode", "Images"];

const channels = channelsFor(props.websiteUrl);
const monday = nextMonday();

const step = ref(1);
const loading = ref(true);
const error = ref("");
/** Offers loaded so far for the current search, newest first. */
const offers = ref<Offer[]>([]);
const offerPage = ref(1);
const offerLastPage = ref(1);
const offersLoading = ref(false);
/** Whether the site has any offers at all, whatever the search says. */
const hasOffers = ref(false);
const ideaCounts = ref<OfferIdeaCount[]>([]);
const clis = ref<CliStatus[]>([]);

const counts = reactive<Record<string, number>>(Object.fromEntries(channels.map((c) => [c.id, c.defaultCount])));
const offerSearch = ref("");
/** Kept as a whole offer so the pick survives searching and paging. */
const offer = ref<Offer | null>(null);
const offerId = computed(() => offer.value?.id ?? null);
const mode = ref<"ai" | "swipe">("ai");
const provider = ref("");
const withImages = ref(false);

/* Generation state */
const generating = ref(false);
const stopping = ref(false);
const activity = ref<PlanProgress[]>([]);
const liveText = ref("");
const liveBox = ref<HTMLElement | null>(null);
const elapsed = ref(0);
const neededTools = ref<string[]>([]);
let timer: number | undefined;
let unlisten: UnlistenFn | undefined;
let searchTimer: number | undefined;
let offerRequest = 0;

const total = computed(() => Object.values(counts).reduce((sum, n) => sum + n, 0));
const activeChannels = computed(() => channels.filter((c) => counts[c.id] > 0));
const installed = computed(() => clis.value.filter((c) => c.installed));
const providerName = computed(() => clis.value.find((c) => c.id === provider.value)?.name ?? "The AI agent");
const elapsedLabel = computed(() => {
  const minutes = Math.floor(elapsed.value / 60);
  return `${minutes}:${String(elapsed.value % 60).padStart(2, "0")}`;
});

const canContinue = computed(() => {
  if (step.value === 1) return total.value > 0 && offerId.value !== null;
  if (step.value === 2) return mode.value === "ai" && provider.value !== "";
  return true;
});

onMounted(async () => {
  unlisten = await listen<PlanProgress>("plan-progress", ({ payload }) => onProgress(payload));
  try {
    [, clis.value, ideaCounts.value] = await Promise.all([loadOffers(1), api.detectClis(), api.offerIdeaCounts()]);
    hasOffers.value = offers.value.length > 0;
    offer.value = offers.value[0] ?? null;
    provider.value = installed.value[0]?.id ?? "";
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
});

onUnmounted(() => {
  unlisten?.();
  window.clearInterval(timer);
  window.clearTimeout(searchTimer);
});

/**
 * Loads a page of offers matching the search box. Page 1 replaces the list;
 * later pages are added below it. The server searches every offer, so one
 * that is not loaded yet can still be found.
 */
async function loadOffers(page: number) {
  const request = ++offerRequest;
  offersLoading.value = true;
  try {
    const result = await api.listOffers(page, offerSearch.value);
    if (request !== offerRequest) return; // a newer search replaced this one
    offers.value = page === 1 ? result.offers : [...offers.value, ...result.offers];
    offerPage.value = result.meta.current_page;
    offerLastPage.value = result.meta.last_page;
  } finally {
    if (request === offerRequest) offersLoading.value = false;
  }
}

async function searchOffers(page: number) {
  try {
    await loadOffers(page);
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

watch(offerSearch, () => {
  window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => searchOffers(1), 300);
});

function onProgress(progress: PlanProgress) {
  if (progress.kind === "text") {
    liveText.value += progress.message;
    nextTick(() => liveBox.value?.scrollTo({ top: liveBox.value.scrollHeight }));
  } else {
    activity.value.push(progress);
  }
}

function next() {
  error.value = "";
  step.value++;
}

async function generate() {
  if (offerId.value === null) return;
  error.value = "";
  neededTools.value = [];
  activity.value = [];
  liveText.value = "";
  elapsed.value = 0;
  stopping.value = false;
  generating.value = true;
  const slots = slotsFor(channels, counts);
  timer = window.setInterval(() => elapsed.value++, 1000);
  try {
    const outcome = await api.generatePlan({
      provider: provider.value,
      offerId: offerId.value,
      slots,
      withImages: withImages.value,
    });
    if (outcome.kind === "created") {
      emit("generated", outcome.ideas, slots.length);
    } else {
      neededTools.value = outcome.tools;
    }
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    window.clearInterval(timer);
    generating.value = false;
  }
}

async function stop() {
  stopping.value = true;
  await api.cancelPlan();
}

async function allowAndRetry() {
  try {
    await api.grantPermissions(provider.value, neededTools.value);
  } catch (caught) {
    error.value = errorMessage(caught);
    return;
  }
  await generate();
}
</script>

<template>
  <header class="page-head">
    <div>
      <div class="row" style="gap: 8px">
        <span class="badge badge-primary">EDITORIAL AGENT</span>
        <h1>Plan next week's ideas</h1>
      </div>
      <p class="muted small">Your AI agent drafts one idea per post, based on the offer you pick.</p>
    </div>
    <button class="btn" :disabled="generating" @click="emit('close')">Back to ideas</button>
  </header>

  <div class="steps">
    <template v-for="(label, index) in STEPS" :key="label">
      <div class="step" :class="{ current: step === index + 1, done: step > index + 1 }">
        <span class="step-num">{{ step > index + 1 ? "✓" : index + 1 }}</span>
        {{ label }}
      </div>
      <span v-if="index < STEPS.length - 1" class="step-line"></span>
    </template>
  </div>

  <div class="page-body plan-body">
    <div v-if="loading" class="stack" aria-busy="true">
      <div class="skeleton" style="height: 48px"></div>
      <div class="skeleton" style="height: 48px"></div>
      <div class="skeleton" style="height: 160px"></div>
    </div>

    <!-- 1. Posts per channel -->
    <template v-else-if="step === 1">
      <h2>Target posts per channel for next week ({{ weekLabel(monday) }})</h2>
      <div class="channel-grid">
        <div v-for="channel in channels" :key="channel.id" class="channel-row">
          <div class="row" style="gap: 8px">
            <span class="dot" :style="{ background: channel.color }"></span>
            <span style="font-weight: 600">{{ channel.name }}</span>
          </div>
          <div class="stepper">
            <button type="button" :disabled="counts[channel.id] === 0" :aria-label="`Fewer ${channel.name} posts`" @click="counts[channel.id]--">−</button>
            <output>{{ counts[channel.id] }}</output>
            <button type="button" :disabled="counts[channel.id] >= MAX_POSTS_PER_CHANNEL" :aria-label="`More ${channel.name} posts`" @click="counts[channel.id]++">+</button>
          </div>
        </div>
      </div>

      <div class="field">
        <label for="offer-search">Focus offer (primary CTA and case angle)</label>
        <template v-if="hasOffers">
          <input id="offer-search" v-model="offerSearch" class="input" type="search" placeholder="Search offers by name or promise" />
          <div class="offer-list" role="radiogroup" aria-label="Focus offer" :aria-busy="offersLoading">
            <button
              v-for="item in offers"
              :key="item.id"
              type="button"
              class="offer-option"
              :class="{ selected: offerId === item.id }"
              role="radio"
              :aria-checked="offerId === item.id"
              @click="offer = item"
            >
              <span class="offer-option-text">
                <b>{{ item.name }}</b>
                <span class="muted small">{{ item.promise }}</span>
              </span>
              <span class="badge" :class="{ 'badge-primary': ideaCountFor(item, ideaCounts) > 0 }">
                {{ ideaCountFor(item, ideaCounts) }} {{ ideaCountFor(item, ideaCounts) === 1 ? "idea" : "ideas" }}
              </span>
            </button>
            <button
              v-if="offerPage < offerLastPage"
              type="button"
              class="offer-more"
              :disabled="offersLoading"
              @click="searchOffers(offerPage + 1)"
            >
              {{ offersLoading ? "Loading…" : "Load more offers" }}
            </button>
            <div v-if="!offers.length && offersLoading" class="offer-empty">
              <span class="muted">Searching offers…</span>
            </div>
            <div v-else-if="!offers.length" class="offer-empty">
              <span class="muted">No offers match "{{ offerSearch }}".</span>
              <button type="button" class="btn btn-sm" @click="offerSearch = ''">Clear search</button>
            </div>
          </div>
          <span v-if="offer" class="muted small">Selected: {{ offer.name }}</span>
        </template>
        <div v-else-if="!error" class="alert alert-error">
          There are no offers to plan from. Add an offer in the Growth Engine web app (Brain &gt; Offers) first.
        </div>
      </div>

    </template>

    <!-- 2. Creation mode -->
    <template v-else-if="step === 2">
      <h2>How should the editorial agent write these ideas?</h2>
      <div class="choice-grid">
        <button type="button" class="choice" :class="{ selected: mode === 'ai' }" :aria-pressed="mode === 'ai'" @click="mode = 'ai'">
          <b>AI Mode</b>
          <span class="muted small">Written by an AI agent CLI on this computer, from your offer and idea skill.</span>
        </button>
        <button type="button" class="choice" disabled>
          <b>Swipe File Mode <span class="badge">Coming soon</span></b>
          <span class="muted small">Adapt structures from saved swipe file items.</span>
        </button>
      </div>

      <div v-if="installed.length" class="field">
        <label for="plan-provider">AI agent</label>
        <select id="plan-provider" v-model="provider" class="input">
          <option v-for="cli in installed" :key="cli.id" :value="cli.id">
            {{ cli.name }}{{ cli.version ? ` (${cli.version})` : "" }}
          </option>
        </select>
        <span class="muted small">Agents come from the CLIs detected in Settings &gt; AI agents.</span>
      </div>
      <div v-else class="alert alert-error">
        No AI agent CLI was found on this computer. Install Claude Code or Antigravity, then check Settings.
        <div style="margin-top: 8px"><button class="btn btn-sm" @click="emit('openSettings')">Open Settings</button></div>
      </div>
    </template>

    <!-- 3. Images + generation -->
    <template v-else>
      <h2>Plan cover images for these posts?</h2>
      <div class="choice-grid">
        <button type="button" class="choice" :class="{ selected: withImages }" :aria-pressed="withImages" :disabled="generating" @click="withImages = true">
          <b>Yes, with images</b>
          <span class="muted small">The agent adds a cover image brief to each idea.</span>
        </button>
        <button type="button" class="choice" :class="{ selected: !withImages }" :aria-pressed="!withImages" :disabled="generating" @click="withImages = false">
          <b>No, text only</b>
          <span class="muted small">Ideas are drafted as text only.</span>
        </button>
      </div>

      <div class="panel">
        <h3>Plan summary</h3>
        <ul class="small" style="margin: 6px 0 0; padding-left: 18px">
          <li>{{ total }} {{ total === 1 ? "idea" : "ideas" }} across {{ activeChannels.length }} {{ activeChannels.length === 1 ? "channel" : "channels" }}</li>
          <li>Focus offer: {{ offer?.name }}</li>
          <li>Written by: {{ providerName }}</li>
          <li>Cover image briefs: {{ withImages ? "Yes" : "No" }}</li>
        </ul>
      </div>

      <!-- Live activity: what the agent is doing right now -->
      <section v-if="generating || activity.length" class="card stack" aria-label="Generation activity">
        <div class="row-between">
          <div class="row" style="gap: 8px">
            <span v-if="generating" class="spinner" style="color: var(--primary)"></span>
            <h3>{{ generating ? `${providerName} is working` : "Last run" }}</h3>
            <span class="muted small" aria-label="Elapsed time">{{ elapsedLabel }}</span>
          </div>
          <button v-if="generating" class="btn btn-sm" :disabled="stopping" @click="stop">
            {{ stopping ? "Stopping…" : "Stop generating" }}
          </button>
        </div>

        <ol class="activity" aria-live="polite">
          <li v-for="(entry, index) in activity" :key="index" :class="`activity-${entry.kind}`">
            <span class="activity-mark" aria-hidden="true">
              {{ entry.kind === "denied" ? "!" : entry.kind === "stage" && (!generating || index < activity.length - 1) ? "✓" : "•" }}
            </span>
            <span>{{ entry.message }}</span>
          </li>
        </ol>

        <div v-if="liveText" class="field">
          <span class="label">Live output from {{ providerName }}</span>
          <pre ref="liveBox" class="live-output" tabindex="0">{{ liveText }}</pre>
        </div>
        <p v-else-if="generating" class="muted small">
          Nothing written yet. Agents usually think for a while before the first words appear.
        </p>
      </section>

      <!-- Permission request the headless agent could not ask for itself -->
      <section v-if="neededTools.length && !generating" class="card stack permission" role="alert">
        <h3>{{ providerName }} needs your permission</h3>
        <p>
          {{ providerName }} asked to use
          <template v-for="(tool, index) in neededTools" :key="tool"><code>{{ tool }}</code>{{ index < neededTools.length - 1 ? ", " : "" }}</template>.
          It runs in the background and can't ask you directly, so the request was denied and no ideas came back.
        </p>
        <p class="muted small">
          Allowing lets {{ providerName }} use {{ neededTools.length === 1 ? "this tool" : "these tools" }} on every run without asking.
          It works in an empty app folder, and you can revoke this in Settings &gt; AI agents.
        </p>
        <div class="row">
          <button class="btn btn-primary" @click="allowAndRetry">Allow and run again</button>
          <button class="btn" @click="neededTools = []">Don't allow</button>
        </div>
      </section>
    </template>

    <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>
  </div>

  <footer class="plan-foot">
    <button v-if="step === 1" class="btn" @click="emit('close')">Cancel</button>
    <button v-else class="btn" :disabled="generating" @click="step--; error = ''">Back</button>

    <button v-if="step < 3" class="btn btn-primary" :disabled="loading || !canContinue" @click="next">Next</button>
    <button v-else class="btn btn-primary" :disabled="generating" @click="generate">
      <span v-if="generating" class="spinner"></span>
      {{ generating ? "Generating…" : error || neededTools.length ? "Try again" : `Generate weekly plan (${total} ideas)` }}
    </button>
  </footer>
</template>
