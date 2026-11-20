<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  api,
  errorMessage,
  ideaCountFor,
  type ChannelSkill,
  type CliStatus,
  type Idea,
  type ImageApiStatus,
  type Offer,
  type OfferIdeaCount,
  type PlanProgress,
  type Skill,
} from "../api";
import { settingsSection } from "../settingsNav";
import { channelsFor, MAX_POSTS_PER_CHANNEL, nextMonday, slotsFor, weekLabel } from "../channels";
import Icon from "../components/Icon.vue";
import PlatformIcon from "../components/PlatformIcon.vue";
import SectionCard from "../components/SectionCard.vue";

const props = defineProps<{ websiteUrl: string | null }>();
const emit = defineEmits<{ close: []; generated: [ideas: Idea[], requested: number]; openSettings: [] }>();

const STEPS = ["Posts per channel", "Creation mode", "Images"];
const TOPIC_TITLE_MAX = 120;
const TOPIC_DESCRIPTION_MAX = 600;

const channels = channelsFor(props.websiteUrl);
const monday = nextMonday();
const week = weekLabel(monday);

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
const topicTitle = ref("");
const topicDescription = ref("");
const offerSearch = ref("");
/** Kept as a whole offer so the pick survives searching and paging. */
const offer = ref<Offer | null>(null);
const offerId = computed(() => offer.value?.id ?? null);
const mode = ref<"ai" | "swipe">("ai");
const provider = ref("");
const withImages = ref(false);
/** With images: draw them with the image API, or only write each idea's image prompt. */
const imageMode = ref<"generate" | "prompt">("prompt");
const imageApi = ref<ImageApiStatus | null>(null);

/* Idea skill per channel, prefilled from Settings > Idea skills */
const skills = ref<Skill[]>([]);
const channelSkillIds = reactive<Record<string, string>>({});
const defaultSkill = computed(() => skills.value.find((skill) => skill.isDefault) ?? skills.value[0] ?? null);
function skillFor(channelId: string) {
  return skills.value.find((skill) => skill.id === channelSkillIds[channelId]) ?? defaultSkill.value;
}

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
const hasTopic = computed(() => topicTitle.value.trim() !== "" || topicDescription.value.trim() !== "");

const canContinue = computed(() => {
  if (step.value === 1) return total.value > 0 && offerId.value !== null;
  if (step.value === 2) return mode.value === "ai" && provider.value !== "";
  return !(withImages.value && imageMode.value === "generate" && !imageApi.value?.configured);
});
/** Why Next is disabled, so the footer can say it instead of leaving the user guessing. */
const blocker = computed(() => {
  if (loading.value || canContinue.value) return "";
  if (step.value === 1) return total.value === 0 ? "Add at least one post to a channel." : "Pick a focus offer.";
  if (step.value === 2) return "Pick an AI agent to write the ideas.";
  if (step.value === 3) return "Add an image API key in Settings, or pick image prompts only.";
  return "";
});

onMounted(async () => {
  unlisten = await listen<PlanProgress>("plan-progress", ({ payload }) => onProgress(payload));
  try {
    let channelSkills: ChannelSkill[];
    [, clis.value, ideaCounts.value, skills.value, channelSkills, imageApi.value] = await Promise.all([
      loadOffers(1),
      api.detectClis(),
      api.offerIdeaCounts(),
      api.listSkills(),
      api.listChannelSkills(),
      api.getImageApi(),
    ]);
    for (const row of channelSkills) channelSkillIds[row.channelId] = row.skillId;
    if (imageApi.value.configured) imageMode.value = "generate";
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
    const result = await api.listOffers(page, { search: offerSearch.value });
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

function back() {
  error.value = "";
  step.value--;
}

/** Steps already passed can be revisited from the step bar. */
function goToStep(target: number) {
  if (generating.value || target >= step.value) return;
  error.value = "";
  step.value = target;
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
      generateImages: withImages.value && imageMode.value === "generate",
      channelSkills: Object.fromEntries(
        activeChannels.value.flatMap((channel) => {
          const skill = skillFor(channel.id);
          return skill ? [[channel.id, skill.id]] : [];
        }),
      ),
      topicTitle: topicTitle.value.trim(),
      topicDescription: topicDescription.value.trim(),
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

function openImageSettings() {
  settingsSection.value = "images";
  emit("openSettings");
}

const coverImagesLabel = computed(() => {
  if (step.value <= 2) return "Picked later";
  if (!withImages.value) return "No";
  return imageMode.value === "generate" ? "Yes · drawn by API" : "Yes · prompts only";
});

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
  <div class="wp pw">
    <div class="wp-header">
      <div class="wp-header-left">
        <button class="wp-back" title="Back to Ideas" aria-label="Back to Ideas" :disabled="generating" @click="emit('close')">
          <Icon name="arrowLeft" />
        </button>
        <div style="min-width: 0">
          <div class="wp-title-row">
            <h1>Plan next week</h1>
            <span class="pw-agent-badge"><Icon name="sparkles" :size="11" /> Editorial agent</span>
          </div>
          <p class="wp-subtitle">{{ week }} · Your AI agent drafts one idea per post, based on the offer you pick.</p>
        </div>
      </div>
    </div>

    <nav class="pw-steps" aria-label="Plan steps">
      <template v-for="(label, index) in STEPS" :key="label">
        <button
          type="button"
          class="pw-step"
          :class="{ current: step === index + 1, done: step > index + 1 }"
          :aria-current="step === index + 1 ? 'step' : undefined"
          :disabled="generating || step <= index + 1"
          @click="goToStep(index + 1)"
        >
          <span class="pw-step-num">
            <Icon v-if="step > index + 1" name="check" :size="12" :stroke="3" />
            <template v-else>{{ index + 1 }}</template>
          </span>
          {{ label }}
        </button>
        <span v-if="index < STEPS.length - 1" class="pw-step-line" :class="{ done: step > index + 1 }"></span>
      </template>
    </nav>

    <div class="wp-layout">
      <main class="wp-main">
        <div class="wp-column">
          <div v-if="error" class="wp-alert wp-alert-error" role="alert">
            <Icon name="alertCircle" :size="16" />
            <span style="white-space: pre-wrap; word-break: break-word">{{ error }}</span>
          </div>

          <div v-if="loading" class="pw-skeletons" aria-busy="true">
            <div class="skeleton" style="height: 150px"></div>
            <div class="skeleton" style="height: 170px"></div>
            <div class="skeleton" style="height: 260px"></div>
          </div>

          <!-- 1. Topic, posts per channel and focus offer -->
          <template v-else-if="step === 1">
            <SectionCard icon="hash" title="Topic" hint="Optional. What should next week's ideas be about?">
              <div class="wp-details">
                <label class="wp-field">
                  <span class="pw-label-row">
                    Topic title
                    <span class="pw-count">{{ topicTitle.length }}/{{ TOPIC_TITLE_MAX }}</span>
                  </span>
                  <input
                    v-model="topicTitle"
                    class="wp-input"
                    :maxlength="TOPIC_TITLE_MAX"
                    placeholder="e.g. Getting e-commerce stores ready for Black Friday traffic"
                  />
                </label>
                <label class="wp-field">
                  <span class="pw-label-row">
                    Topic description
                    <span class="pw-count">{{ topicDescription.length }}/{{ TOPIC_DESCRIPTION_MAX }}</span>
                  </span>
                  <textarea
                    v-model="topicDescription"
                    rows="3"
                    class="wp-input"
                    :maxlength="TOPIC_DESCRIPTION_MAX"
                    placeholder="The angle, audience or points the agent should cover. Leave blank to let it pick angles from the offer."
                  ></textarea>
                </label>
              </div>
            </SectionCard>

            <SectionCard icon="share" title="Posts per channel" hint="How many ideas to draft for each channel next week.">
              <template #action>
                <span class="pw-total">{{ total }} {{ total === 1 ? "post" : "posts" }}</span>
              </template>
              <div class="pw-channels">
                <div
                  v-for="channel in channels"
                  :key="channel.id"
                  class="pw-channel"
                  :class="{ active: counts[channel.id] > 0 }"
                  :style="{ '--ch': channel.color }"
                >
                  <PlatformIcon :platform="channel.platform" />
                  <span class="pw-channel-text">
                    <span class="pw-channel-name">{{ channel.platform }}</span>
                    <span class="pw-channel-sub">{{ channel.name.split(" · ")[1] ?? channel.name }}</span>
                  </span>
                  <div class="pw-stepper">
                    <button
                      type="button"
                      :disabled="counts[channel.id] === 0"
                      :aria-label="`Fewer ${channel.name} posts`"
                      @click="counts[channel.id]--"
                    >
                      −
                    </button>
                    <output :aria-label="`${channel.name} posts`">{{ counts[channel.id] }}</output>
                    <button
                      type="button"
                      :disabled="counts[channel.id] >= MAX_POSTS_PER_CHANNEL"
                      :aria-label="`More ${channel.name} posts`"
                      @click="counts[channel.id]++"
                    >
                      +
                    </button>
                  </div>
                </div>
              </div>
            </SectionCard>

            <SectionCard icon="target" title="Focus offer" hint="The primary CTA and case angle every idea sells.">
              <div v-if="hasOffers" class="wp-details">
                <div class="pw-search">
                  <Icon name="search" :size="14" />
                  <input v-model="offerSearch" type="search" placeholder="Search offers by name or promise" aria-label="Search offers" />
                </div>
                <div class="pw-offers" role="radiogroup" aria-label="Focus offer" :aria-busy="offersLoading">
                  <button
                    v-for="item in offers"
                    :key="item.id"
                    type="button"
                    class="pw-offer"
                    :class="{ selected: offerId === item.id }"
                    role="radio"
                    :aria-checked="offerId === item.id"
                    @click="offer = item"
                  >
                    <span class="pw-radio" aria-hidden="true"></span>
                    <span class="pw-offer-text">
                      <span class="pw-offer-name">{{ item.name }}</span>
                      <span class="pw-offer-promise">{{ item.promise }}</span>
                    </span>
                    <span class="pw-offer-count" :class="{ has: ideaCountFor(item, ideaCounts) > 0 }">
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
                    <span class="wp-muted">Searching offers…</span>
                  </div>
                  <div v-else-if="!offers.length" class="offer-empty">
                    <span class="wp-muted">No offers match "{{ offerSearch }}".</span>
                    <button type="button" class="wp-btn" @click="offerSearch = ''">Clear search</button>
                  </div>
                </div>
              </div>
              <div v-else-if="!error" class="cp-empty">
                <p class="cp-empty-title">No offers to plan from</p>
                <p>Add an offer in the Growth Engine web app (Brain &gt; Offers) first.</p>
              </div>
            </SectionCard>
          </template>

          <!-- 2. Creation mode -->
          <template v-else-if="step === 2">
            <SectionCard icon="bot" title="Creation mode" hint="How the editorial agent writes these ideas.">
              <div class="pw-choices">
                <button type="button" class="pw-choice" :class="{ selected: mode === 'ai' }" :aria-pressed="mode === 'ai'" @click="mode = 'ai'">
                  <span class="pw-choice-icon"><Icon name="sparkles" :size="16" /></span>
                  <b>AI Mode</b>
                  <span>Written by an AI agent CLI on this computer, from your offer and idea skill.</span>
                </button>
                <button type="button" class="pw-choice" disabled>
                  <span class="pw-choice-icon"><Icon name="layers" :size="16" /></span>
                  <b>Swipe File Mode <span class="pw-soon">Coming soon</span></b>
                  <span>Adapt structures from saved swipe file items.</span>
                </button>
              </div>
            </SectionCard>

            <SectionCard icon="settings" title="AI agent" hint="Agents come from the CLIs detected in Settings > AI agents.">
              <label v-if="installed.length" class="wp-field">
                <span>Agent</span>
                <select v-model="provider" class="wp-input">
                  <option v-for="cli in installed" :key="cli.id" :value="cli.id">
                    {{ cli.name }}{{ cli.version ? ` (${cli.version})` : "" }}
                  </option>
                </select>
              </label>
              <div v-else class="cp-empty">
                <p class="cp-empty-title">No AI agent CLI found</p>
                <p>Install Claude Code or Antigravity, then check Settings.</p>
                <button type="button" class="cp-connect" @click="emit('openSettings')">Open Settings</button>
              </div>
            </SectionCard>
          </template>

          <!-- 3. Images + generation -->
          <template v-else>
            <SectionCard icon="eye" title="Cover images" hint="Should the agent plan a cover image for each post?">
              <div class="pw-choices">
                <button
                  type="button"
                  class="pw-choice"
                  :class="{ selected: withImages }"
                  :aria-pressed="withImages"
                  :disabled="generating"
                  @click="withImages = true"
                >
                  <span class="pw-choice-icon"><Icon name="eye" :size="16" /></span>
                  <b>Yes, with images</b>
                  <span>The agent adds a cover image brief to each idea.</span>
                </button>
                <button
                  type="button"
                  class="pw-choice"
                  :class="{ selected: !withImages }"
                  :aria-pressed="!withImages"
                  :disabled="generating"
                  @click="withImages = false"
                >
                  <span class="pw-choice-icon"><Icon name="fileText" :size="16" /></span>
                  <b>No, text only</b>
                  <span>Ideas are drafted as text only.</span>
                </button>
              </div>
            </SectionCard>

            <SectionCard v-if="withImages" icon="sparkles" title="How to make the images" hint="Draw them now, or get a prompt to use in any image tool.">
              <div class="pw-choices">
                <button
                  type="button"
                  class="pw-choice"
                  :class="{ selected: imageMode === 'generate' }"
                  :aria-pressed="imageMode === 'generate'"
                  :disabled="generating"
                  @click="imageMode = 'generate'"
                >
                  <span class="pw-choice-icon"><Icon name="sparkles" :size="16" /></span>
                  <b>Generate with the image API</b>
                  <span v-if="imageApi?.configured">Each idea gets a cover image drawn by {{ imageApi.model }} after planning.</span>
                  <span v-else>Needs an API key in Settings &gt; Image API.</span>
                </button>
                <button
                  type="button"
                  class="pw-choice"
                  :class="{ selected: imageMode === 'prompt' }"
                  :aria-pressed="imageMode === 'prompt'"
                  :disabled="generating"
                  @click="imageMode = 'prompt'"
                >
                  <span class="pw-choice-icon"><Icon name="penLine" :size="16" /></span>
                  <b>Write image prompts only</b>
                  <span>Copy each idea's prompt on its Write post page, make the image anywhere, then upload it.</span>
                </button>
              </div>
              <div v-if="imageMode === 'generate' && !imageApi?.configured" class="wp-alert wp-alert-info pw-image-setup" role="status">
                <Icon name="alertCircle" :size="16" />
                <span>
                  No image API key yet.
                  <button type="button" class="wp-link" @click="openImageSettings">Add one in Settings</button>
                </span>
              </div>
            </SectionCard>

            <!-- Live activity: what the agent is doing right now -->
            <SectionCard
              v-if="generating || activity.length"
              icon="radio"
              :title="generating ? `${providerName} is working` : 'Last run'"
              :hint="`Elapsed ${elapsedLabel}`"
            >
              <template v-if="generating" #action>
                <button class="wp-btn" :disabled="stopping" @click="stop">{{ stopping ? "Stopping…" : "Stop generating" }}</button>
              </template>
              <div class="wp-details">
                <ol class="activity" aria-live="polite">
                  <li v-for="(entry, index) in activity" :key="index" :class="`activity-${entry.kind}`">
                    <span class="activity-mark" aria-hidden="true">
                      {{ entry.kind === "denied" ? "!" : entry.kind === "stage" && (!generating || index < activity.length - 1) ? "✓" : "•" }}
                    </span>
                    <span>{{ entry.message }}</span>
                  </li>
                </ol>
                <div v-if="liveText" class="wp-field">
                  <span>Live output from {{ providerName }}</span>
                  <pre ref="liveBox" class="live-output" tabindex="0">{{ liveText }}</pre>
                </div>
                <p v-else-if="generating" class="wp-muted">
                  Nothing written yet. Agents usually think for a while before the first words appear.
                </p>
              </div>
            </SectionCard>

            <!-- Permission request the headless agent could not ask for itself -->
            <div v-if="neededTools.length && !generating" class="wp-alert wp-alert-info pw-permission" role="alert">
              <Icon name="alertTriangle" :size="16" />
              <div class="wp-details">
                <b>{{ providerName }} needs your permission</b>
                <p>
                  {{ providerName }} asked to use
                  <template v-for="(tool, index) in neededTools" :key="tool"><code class="wp-code">{{ tool }}</code>{{ index < neededTools.length - 1 ? ", " : "" }}</template>.
                  It runs in the background and can't ask you directly, so the request was denied and no ideas came back.
                </p>
                <p class="wp-muted">
                  Allowing lets {{ providerName }} use {{ neededTools.length === 1 ? "this tool" : "these tools" }} on every run without asking.
                  It works in an empty app folder, and you can revoke this in Settings &gt; AI agents.
                </p>
                <div class="wp-actions">
                  <button class="wp-btn wp-btn-primary" @click="allowAndRetry">Allow and run again</button>
                  <button class="wp-btn" @click="neededTools = []">Don't allow</button>
                </div>
              </div>
            </div>
          </template>
        </div>
      </main>

      <!-- Live summary of the plan, like the composer's live preview -->
      <aside class="wp-preview pw-summary" aria-label="Plan summary">
        <div class="wp-preview-head">
          <span class="wp-preview-label"><Icon name="fileText" :size="14" /> Idea skill per channel</span>
        </div>
        <div class="pw-skills">
          <p v-if="!activeChannels.length" class="pw-summary-empty">Add posts to a channel to pick its skill.</p>
          <label v-for="channel in activeChannels" :key="channel.id" class="pw-skill-row">
            <span class="pw-skill-channel">
              <PlatformIcon :platform="channel.platform" size="sm" />
              <span>{{ channel.name }}</span>
            </span>
            <select
              class="sk-select"
              :value="skillFor(channel.id)?.id ?? ''"
              :disabled="generating || !skills.length"
              :aria-label="`Idea skill for ${channel.name}`"
              @change="channelSkillIds[channel.id] = ($event.target as HTMLSelectElement).value"
            >
              <option v-for="skill in skills" :key="skill.id" :value="skill.id">
                {{ skill.name }}{{ skill.isDefault ? " (default)" : "" }}
              </option>
            </select>
          </label>
          <p class="pw-skills-note">Defaults come from Settings &gt; Idea skills.</p>
        </div>

        <div class="wp-preview-head">
          <span class="wp-preview-label"><Icon name="calendarClock" :size="14" /> Plan summary</span>
        </div>
        <div class="pw-summary-card">
          <div class="pw-summary-total">
            <span class="pw-summary-num">{{ total }}</span>
            <span>{{ total === 1 ? "idea" : "ideas" }} for {{ week }}</span>
          </div>

          <div class="pw-summary-block">
            <span class="pw-summary-label">Topic</span>
            <template v-if="hasTopic">
              <span class="pw-summary-value">{{ topicTitle.trim() || "Untitled topic" }}</span>
              <span v-if="topicDescription.trim()" class="pw-summary-desc">{{ topicDescription.trim() }}</span>
            </template>
            <span v-else class="pw-summary-empty">None. The agent picks angles from the offer.</span>
          </div>

          <div class="pw-summary-block">
            <span class="pw-summary-label">Channels</span>
            <ul class="pw-summary-channels">
              <li v-for="channel in channels" :key="channel.id" :class="{ off: !counts[channel.id] }">
                <PlatformIcon :platform="channel.platform" size="sm" />
                <span class="pw-summary-channel">{{ channel.name }}</span>
                <span class="pw-bar" aria-hidden="true">
                  <span :style="{ width: `${(counts[channel.id] / MAX_POSTS_PER_CHANNEL) * 100}%`, background: channel.color }"></span>
                </span>
                <b>{{ counts[channel.id] }}</b>
              </li>
            </ul>
          </div>

          <dl class="pw-summary-facts">
            <div>
              <dt>Focus offer</dt>
              <dd>{{ offer?.name ?? "Not picked" }}</dd>
            </div>
            <div>
              <dt>Written by</dt>
              <dd>{{ step > 1 ? providerName : "Picked next" }}</dd>
            </div>
            <div>
              <dt>Cover images</dt>
              <dd>{{ coverImagesLabel }}</dd>
            </div>
          </dl>
        </div>
        <p class="wp-muted pw-summary-note">
          {{ activeChannels.length }} {{ activeChannels.length === 1 ? "channel" : "channels" }}. Each idea is tagged with its channel on the Ideas page.
        </p>
      </aside>
    </div>

    <footer class="pw-foot">
      <button v-if="step === 1" class="wp-btn" @click="emit('close')">Cancel</button>
      <button v-else class="wp-btn" :disabled="generating" @click="back"><Icon name="arrowLeft" :size="14" /> Back</button>

      <div class="pw-foot-right">
        <span v-if="blocker" class="wp-muted">{{ blocker }}</span>
        <button v-if="step < 3" class="wp-btn wp-btn-primary" :disabled="loading || !canContinue" @click="next">
          Next <Icon name="arrowRight" :size="14" />
        </button>
        <button v-else class="wp-btn wp-btn-primary" :disabled="generating" @click="generate">
          <span v-if="generating" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
          <Icon v-else name="sparkles" :size="14" />
          {{ generating ? "Generating…" : error || neededTools.length ? "Try again" : `Generate weekly plan (${total} ${total === 1 ? "idea" : "ideas"})` }}
        </button>
      </div>
    </footer>
  </div>
</template>
