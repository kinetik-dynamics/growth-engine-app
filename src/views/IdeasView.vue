<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, errorMessage, type Idea, type IdeaTab, type Session } from "../api";
import Pagination from "../components/Pagination.vue";
import PlanWeekView from "./PlanWeekView.vue";
import IdeaDetailView from "./IdeaDetailView.vue";

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

const openIdea = computed(() => ideas.value?.find((idea) => idea.id === openIdeaId.value) ?? null);

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
}

/** Sending moves an idea to the other tab, so the page is re-read on the way back. */
function closeIdea() {
  openIdeaId.value = null;
  return load();
}

async function onDeleted() {
  await closeIdea();
  notice.value = "Idea deleted.";
}

function sentLabel(idea: Idea): string {
  return new Date(idea.sentAt ?? 0).toLocaleDateString("en-US", { month: "short", day: "numeric" });
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

  <template v-else>
    <header class="page-head">
      <div class="row" style="gap: 20px">
        <h1>Ideas</h1>
        <div class="tabs" role="tablist" aria-label="Ideas">
          <button class="tab" :class="{ active: tab === 'drafts' }" role="tab" :aria-selected="tab === 'drafts'" @click="showTab('drafts')">
            Drafts <span class="tab-count">{{ draftCount }}</span>
          </button>
          <button class="tab" :class="{ active: tab === 'sent' }" role="tab" :aria-selected="tab === 'sent'" @click="showTab('sent')">
            Sent <span class="tab-count">{{ sentCount }}</span>
          </button>
        </div>
      </div>
      <button class="btn btn-primary" @click="planning = true">Plan next week</button>
    </header>

    <div class="page-body">
      <form v-if="tab === 'drafts'" class="row" @submit.prevent="addIdea">
        <label for="new-idea" class="label" style="white-space: nowrap">New idea</label>
        <input id="new-idea" v-model="newTitle" class="input" placeholder="Press Enter to save" />
        <button class="btn" type="submit" :disabled="!newTitle.trim()">Add idea</button>
      </form>
      <p v-else class="muted">
        These ideas are on the main site's Ideas page. Open one to edit it and send the update.
      </p>

      <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>
      <div v-if="notice" class="alert alert-success row-between" role="status">
        {{ notice }}
        <button class="btn btn-sm btn-ghost" @click="notice = ''">Dismiss</button>
      </div>

      <div v-if="!ideas" class="grid" aria-busy="true">
        <div v-for="n in 3" :key="n" class="skeleton" style="height: 220px"></div>
      </div>

      <div v-else-if="!ideas.length && tab === 'drafts'" class="card empty">
        <h2>{{ sentCount ? "No drafts left" : "No ideas yet" }}</h2>
        <p class="muted">Let your AI agent draft next week's ideas from one of your offers.</p>
        <button class="btn btn-primary" @click="planning = true">Plan next week</button>
      </div>

      <div v-else-if="!ideas.length" class="card empty">
        <h2>Nothing sent yet</h2>
        <p class="muted">Open a draft, polish it, then send it to the main site. It will show up here.</p>
        <button class="btn" @click="showTab('drafts')">View drafts</button>
      </div>

      <div v-else class="grid" :aria-busy="loading">
        <button
          v-for="idea in ideas"
          :key="idea.id"
          type="button"
          class="card idea idea-card"
          :aria-label="`Open idea: ${idea.title}`"
          @click="openIdeaId = idea.id"
        >
          <span class="row-between">
            <span class="muted small">{{ idea.source }}</span>
            <span class="row" style="gap: 6px; flex-wrap: wrap; justify-content: flex-end">
              <span v-if="idea.sentAt" class="badge badge-success">Sent {{ sentLabel(idea) }}</span>
              <span v-if="idea.tag" class="badge">{{ idea.tag }}</span>
            </span>
          </span>
          <span class="idea-title">{{ idea.title }}</span>
          <span v-if="idea.shortNotes" class="small idea-notes">{{ idea.shortNotes }}</span>

          <span v-if="idea.channelName || idea.pattern || idea.offerName" class="idea-meta">
            <span v-if="idea.channelName"><b>Channel:</b> {{ idea.channelName }}</span>
            <span v-if="idea.pattern"><b>Pattern:</b> {{ idea.pattern }}</span>
            <span v-if="idea.offerName"><b>Offer:</b> {{ idea.offerName }}</span>
          </span>
        </button>
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
  </template>
</template>
