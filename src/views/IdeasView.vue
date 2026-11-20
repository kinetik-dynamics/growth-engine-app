<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, errorMessage, type Idea, type IdeaTab, type Session } from "../api";
import Icon from "../components/Icon.vue";
import Pagination from "../components/Pagination.vue";
import PlatformIcon from "../components/PlatformIcon.vue";
import { plannedChannel } from "../channels";
import PlanWeekView from "./PlanWeekView.vue";
import IdeaDetailView from "./IdeaDetailView.vue";
import BulkPublishView from "./BulkPublishView.vue";

defineProps<{ session: Session }>();
const emit = defineEmits<{ openSettings: [] }>();

/** The ideas on the current page of the current tab, latest first. */
const ideas = ref<Idea[] | null>(null);
const page = ref(1);
const lastPage = ref(1);
const total = ref(0);
const draftCount = ref(0);
const sentCount = ref(0);
const loading = ref(false);
const error = ref("");
const notice = ref("");
const newTitle = ref("");
const planning = ref(false);
const openIdeaId = ref<string | null>(null);
const tab = ref<IdeaTab>("drafts");
const confirmDeleteId = ref<string | null>(null);

/** Ideas ticked for publishing together, kept across pages and tabs. */
const picked = ref<Map<string, Idea>>(new Map());
const bulkIdeas = ref<Idea[] | null>(null);

const openIdea = computed(() => ideas.value?.find((idea) => idea.id === openIdeaId.value) ?? null);
const allOnPagePicked = computed(() => !!ideas.value?.length && ideas.value.every((idea) => picked.value.has(idea.id)));

async function load(target = page.value) {
  loading.value = true;
  try {
    const requested = tab.value;
    const result = await api.listIdeas(requested, target);
    if (requested !== tab.value) return; // the tab changed while this page loaded
    ideas.value = result.ideas;
    page.value = result.page;
    lastPage.value = result.lastPage;
    total.value = result.total;
    draftCount.value = result.draftCount;
    sentCount.value = result.sentCount;
  } catch (caught) {
    error.value = errorMessage(caught);
    ideas.value = [];
  } finally {
    loading.value = false;
  }
}

onMounted(() => load());

async function goToPage(target: number) {
  await load(target);
  document.querySelector(".main")?.scrollTo({ top: 0 });
}

function showTab(next: IdeaTab) {
  if (tab.value === next) return;
  tab.value = next;
  ideas.value = null;
  return load(1);
}

/** New ideas land at the top of the first page of drafts. */
function showNewestDrafts() {
  tab.value = "drafts";
  return load(1);
}

async function addIdea() {
  if (!newTitle.value.trim()) return;
  error.value = "";
  try {
    await api.addIdea(newTitle.value);
    newTitle.value = "";
    await showNewestDrafts();
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

async function onGenerated(created: Idea[], requested: number) {
  planning.value = false;
  await showNewestDrafts();
  notice.value =
    created.length === requested
      ? `Added ${created.length} ideas to your drafts.`
      : `The AI returned ${created.length} of the ${requested} ideas you asked for. Run the planner again for the rest.`;
}

function onSaved(saved: Idea) {
  ideas.value = (ideas.value ?? []).map((idea) => (idea.id === saved.id ? saved : idea));
  if (picked.value.has(saved.id)) picked.value.set(saved.id, saved);
}

/** Sending moves an idea to the other tab, so the page is re-read on the way back. */
function closeIdea() {
  openIdeaId.value = null;
  return load();
}

async function onDeleted() {
  if (openIdeaId.value) picked.value.delete(openIdeaId.value);
  await closeIdea();
  notice.value = "Idea deleted.";
}

async function deleteIdea(idea: Idea) {
  error.value = "";
  try {
    await api.deleteIdea(idea.id);
    picked.value.delete(idea.id);
    confirmDeleteId.value = null;
    await load();
    notice.value = "Idea deleted.";
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

function togglePicked(idea: Idea) {
  const next = new Map(picked.value);
  if (next.has(idea.id)) next.delete(idea.id);
  else next.set(idea.id, idea);
  picked.value = next;
}

function togglePage() {
  const next = new Map(picked.value);
  const pick = !allOnPagePicked.value;
  for (const idea of ideas.value ?? []) {
    if (pick) next.set(idea.id, idea);
    else next.delete(idea.id);
  }
  picked.value = next;
}

function closeBulk(published: boolean) {
  bulkIdeas.value = null;
  if (published) {
    picked.value = new Map();
    notice.value = "The selected ideas were sent to the publish queue.";
  }
  return load();
}

/** Where the idea stands, in the web app's words: a draft post saved, or sent to Publish. */
function publishState(idea: Idea): "Sent" | "Draft" | null {
  if (idea.publishedAt) return "Sent";
  if (idea.publishChannels.length) return "Draft";
  return null;
}

/** Colours the card with the platform the idea was planned for. */
function channelStyle(idea: Idea) {
  const channel = plannedChannel(idea);
  return channel ? { "--ch": channel.color, "--ch-soft": channel.soft } : undefined;
}

function actionLabel(idea: Idea) {
  const state = publishState(idea);
  return state === "Sent" ? "Edit post" : state === "Draft" ? "Continue draft" : "Write post";
}
</script>

<template>
  <PlanWeekView
    v-if="planning"
    :website-url="session.websiteUrl"
    @close="planning = false"
    @generated="onGenerated"
    @open-settings="planning = false; emit('openSettings')"
  />

  <IdeaDetailView
    v-else-if="openIdea"
    :key="openIdea.id"
    :idea="openIdea"
    :session="session"
    @close="closeIdea"
    @saved="onSaved"
    @deleted="onDeleted"
  />

  <BulkPublishView v-else-if="bulkIdeas" :ideas="bulkIdeas" :session="session" @close="closeBulk" @saved="onSaved" />

  <div v-else class="iv">
    <div class="iv-header">
      <div class="iv-header-left">
        <h1>Ideas</h1>
        <nav class="iv-tabs" aria-label="Ideas tabs">
          <button class="iv-tab" :class="{ active: tab === 'drafts' }" :aria-current="tab === 'drafts' ? 'page' : undefined" @click="showTab('drafts')">
            Drafts ({{ draftCount }})
          </button>
          <button class="iv-tab" :class="{ active: tab === 'sent' }" :aria-current="tab === 'sent' ? 'page' : undefined" @click="showTab('sent')">
            Sent ({{ sentCount }})
          </button>
        </nav>
      </div>
      <button class="iv-plan" @click="planning = true"><Icon name="sparkles" :size="14" /> Plan next week</button>
    </div>

    <div class="iv-body">
      <input
        v-if="tab === 'drafts'"
        v-model="newTitle"
        class="iv-add"
        placeholder="Add an idea… (Press Enter to save)"
        aria-label="Add an idea"
        @keydown.enter="addIdea"
      />
      <p v-else class="iv-hint">These ideas are on the website's Ideas page. Open one to edit it, send the update or publish it.</p>

      <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>
      <div v-if="notice" class="alert alert-success row-between" role="status">
        {{ notice }}
        <button class="btn btn-sm btn-ghost" @click="notice = ''">Dismiss</button>
      </div>

      <div v-if="ideas?.length || picked.size" class="iv-select" :class="{ active: picked.size > 0 }">
        <button class="iv-select-toggle" :disabled="!ideas?.length" @click="togglePage">
          <span :class="allOnPagePicked ? 'iv-check-on' : 'iv-check-off'"><Icon :name="allOnPagePicked ? 'squareCheck' : 'square'" /></span>
          <span>{{ picked.size ? `${picked.size} selected` : "Select ideas to publish several at once" }}</span>
        </button>
        <div v-if="picked.size" class="iv-select-actions">
          <button class="iv-clear" @click="picked = new Map()">Clear</button>
          <button class="iv-publish" @click="bulkIdeas = [...picked.values()]">
            <Icon name="send" :size="14" /> Publish selected ({{ picked.size }})
          </button>
        </div>
      </div>

      <div v-if="!ideas" class="iv-grid" aria-busy="true">
        <div v-for="n in 3" :key="n" class="skeleton" style="height: 240px"></div>
      </div>

      <div v-else-if="!ideas.length && tab === 'drafts'" class="iv-empty">
        <h3>{{ sentCount ? "No drafts left" : "No ideas yet" }}</h3>
        <p>Let your AI agent draft next week's ideas from one of your offers, or type one above and press Enter.</p>
        <button class="iv-plan" @click="planning = true"><Icon name="sparkles" :size="14" /> Plan next week</button>
      </div>

      <div v-else-if="!ideas.length" class="iv-empty">
        <h3>Nothing sent yet</h3>
        <p>Open a draft, polish it, then send it to the website. It will show up here.</p>
        <button class="iv-secondary" @click="showTab('drafts')">View drafts</button>
      </div>

      <div v-else class="iv-grid" :aria-busy="loading">
        <article
          v-for="idea in ideas"
          :key="idea.id"
          class="iv-card"
          :class="{ picked: picked.has(idea.id), 'has-channel': plannedChannel(idea) }"
          :style="channelStyle(idea)"
        >
          <div class="iv-card-body">
            <div class="iv-card-top">
              <label class="iv-source">
                <input type="checkbox" :checked="picked.has(idea.id)" :aria-label="`Select the idea “${idea.title}”`" @change="togglePicked(idea)" />
                {{ idea.source }}
              </label>
              <div class="iv-badges">
                <span v-if="publishState(idea) === 'Draft'" class="iv-badge iv-badge-draft" title="A draft post is saved for this idea">
                  <Icon name="fileText" :size="12" /> Draft
                </span>
                <span v-if="publishState(idea) === 'Sent'" class="iv-badge iv-badge-sent" title="This idea was sent to Publish">
                  <Icon name="send" :size="12" /> Sent
                </span>
                <span v-if="idea.sentAt && !idea.publishedAt" class="iv-badge iv-badge-web" title="This idea is on the website's Ideas page">On website</span>
                <span v-if="idea.imageFile" class="iv-tag iv-tag-image" title="Has a cover image"><Icon name="image" :size="11" /></span>
                <span v-if="idea.tag" class="iv-tag">{{ idea.tag }}</span>
              </div>
            </div>
            <span v-if="plannedChannel(idea)" class="ch-chip" :title="`Planned for ${plannedChannel(idea)?.name}`">
              <PlatformIcon :platform="plannedChannel(idea)?.platform" size="sm" />
              <b>{{ plannedChannel(idea)?.platform }}</b>
              <span v-if="plannedChannel(idea)?.account" class="ch-account">{{ plannedChannel(idea)?.account }}</span>
            </span>
            <h3 class="iv-title">{{ idea.title }}</h3>
            <p v-if="idea.shortNotes" class="iv-notes">{{ idea.shortNotes }}</p>
            <div class="iv-meta">
              <div v-if="idea.painAddressed" class="iv-truncate"><b>Pain:</b> {{ idea.painAddressed }}</div>
              <div v-if="idea.pattern"><b>Pattern:</b> {{ idea.pattern }}</div>
              <div v-if="idea.targetKeyword" class="iv-truncate"><b>Keyword:</b> {{ idea.targetKeyword }}</div>
              <div v-if="idea.offerName" class="iv-truncate"><b>Offer:</b> {{ idea.offerName }}</div>
            </div>
            <div v-if="idea.topicTitle" class="iv-topic" :title="idea.topicDescription || idea.topicTitle">
              <Icon name="hash" :size="12" /><span>{{ idea.topicTitle }}</span>
            </div>
          </div>
          <div class="iv-card-foot">
            <template v-if="confirmDeleteId === idea.id">
              <span class="iv-confirm">Delete this idea?</span>
              <div class="iv-confirm-actions">
                <button class="iv-confirm-no" @click="confirmDeleteId = null">Keep</button>
                <button class="iv-confirm-yes" @click="deleteIdea(idea)">Delete</button>
              </div>
            </template>
            <template v-else>
              <button class="iv-write" @click="openIdeaId = idea.id">{{ actionLabel(idea) }} <Icon name="arrowRight" :size="12" /></button>
              <button class="iv-trash" title="Delete idea" aria-label="Delete idea" @click="confirmDeleteId = idea.id">
                <Icon name="trash" :size="14" />
              </button>
            </template>
          </div>
        </article>
      </div>

      <Pagination
        v-if="ideas"
        :page="page"
        :last-page="lastPage"
        :total="total"
        :noun="['idea', 'ideas']"
        :busy="loading"
        @change="goToPage"
      />
    </div>
  </div>
</template>
