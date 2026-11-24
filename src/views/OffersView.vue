<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  api,
  errorMessage,
  ideaCountFor,
  type OfferDraft,
  type OfferIdeaCount,
  type OfferPage,
  type OfferPricing,
  type OfferSort,
  type OfferSource,
  type Session,
  type SiteProof,
} from "../api";
import Icon from "../components/Icon.vue";
import OfferDraftModal from "../components/OfferDraftModal.vue";
import Pagination from "../components/Pagination.vue";
import RefreshControls from "../components/RefreshControls.vue";
import { createAutoRefresh } from "../autoRefresh";
import GenerateOffersView from "./GenerateOffersView.vue";

const props = defineProps<{ session: Session }>();
const emit = defineEmits<{ openSettings: [] }>();

const PRICING_OPTIONS: { value: OfferPricing; label: string }[] = [
  { value: "all", label: "Any pricing" },
  { value: "priced", label: "Price stated" },
  { value: "unpriced", label: "No price yet" },
];
const SOURCE_OPTIONS: { value: OfferSource; label: string }[] = [
  { value: "all", label: "Any source" },
  { value: "agent", label: "Agent-drafted" },
  { value: "manual", label: "Added manually" },
];
const SORT_OPTIONS: { value: OfferSort; label: string }[] = [
  { value: "drafted", label: "Sort: as drafted" },
  { value: "name", label: "Sort: name A–Z" },
  { value: "proof", label: "Sort: most proof" },
  { value: "recent", label: "Sort: recently edited" },
];

const result = ref<OfferPage | null>(null);
const ideaCounts = ref<OfferIdeaCount[]>([]);
const error = ref("");
const busy = ref(false);
const page = ref(1);

const search = ref("");
const icpFilter = ref("all");
const pricing = ref<OfferPricing>("all");
const source = ref<OfferSource>("all");
const sort = ref<OfferSort>("drafted");

const filtersActive = computed(() => search.value.trim() !== "" || icpFilter.value !== "all" || pricing.value !== "all" || source.value !== "all");

let requestId = 0;

/** Loads one page; a slower, older request never overwrites a newer one. */
async function load(target = page.value) {
  const id = ++requestId;
  busy.value = true;
  error.value = "";
  try {
    const [next, counts] = await Promise.all([
      api.listOffers(target, {
        search: search.value,
        icpId: icpFilter.value === "all" ? null : Number(icpFilter.value),
        pricing: pricing.value,
        source: source.value,
        sort: sort.value,
      }),
      api.offerIdeaCounts(),
    ]);
    if (id !== requestId) return;
    // An offer deleted on the site can leave this page past the end.
    if (!next.offers.length && next.meta.current_page > next.meta.last_page) return load(next.meta.last_page);
    result.value = next;
    ideaCounts.value = counts;
    page.value = next.meta.current_page;
    document.querySelector(".main")?.scrollTo({ top: 0 });
  } catch (caught) {
    if (id === requestId) error.value = errorMessage(caught);
  } finally {
    if (id === requestId) {
      busy.value = false;
      autoRefresh.restart();
    }
  }
}

// Leaving the page stops the clock; coming back loads fresh offers and starts it over.
const autoRefresh = createAutoRefresh("offers.autoRefresh", () => load(), () => busy.value);
const { enabled: autoRefreshOn, countdown } = autoRefresh;

let searchTimer: number | undefined;
watch(search, () => {
  window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => load(1), 300);
});
watch([icpFilter, pricing, source, sort], () => load(1));

function clearFilters() {
  search.value = "";
  icpFilter.value = "all";
  pricing.value = "all";
  source.value = "all";
}

function openSite(path: string) {
  openUrl(`${props.session.baseUrl}${path}`);
}

/* Offers drafted by the AI agent, kept in the app until they are synced */

const drafts = ref<OfferDraft[]>([]);
/** The site's case studies, to name the ones a draft links and to pick them when editing. */
const proof = ref<SiteProof[]>([]);
const generatingOffers = ref(false);
const editingDraft = ref<OfferDraft | null>(null);
const confirmDeleteDraftId = ref<string | null>(null);
const syncing = ref(false);
const notice = ref("");
const draftError = ref("");

// A refresh scrolls the page to the top, so it waits while the generate page is open.
watch(generatingOffers, (open) => (open ? autoRefresh.stop() : autoRefresh.start()));

const icpNames = computed(() => new Map((result.value?.icp_options ?? []).map((icp) => [icp.id, icp.name])));

async function loadDrafts() {
  try {
    drafts.value = await api.listOfferDrafts();
    // Only needed to show and edit drafts; a failure here must not hide them.
    if (drafts.value.length) proof.value = (await api.sitePage("proof").catch(() => ({ proof: proof.value }))).proof;
  } catch (caught) {
    draftError.value = errorMessage(caught);
  }
}

async function onGenerated(created: OfferDraft[], requested: number) {
  generatingOffers.value = false;
  await Promise.all([loadDrafts(), load()]);
  notice.value =
    created.length === requested
      ? `Drafted ${created.length} new ${created.length === 1 ? "offer" : "offers"}. Review them, then sync them to the website.`
      : `The AI returned ${created.length} of the ${requested} offers you asked for. Generate again for more.`;
}

function onDraftSaved(saved: OfferDraft) {
  drafts.value = drafts.value.map((draft) => (draft.id === saved.id ? saved : draft));
  editingDraft.value = null;
}

async function deleteDraft(draft: OfferDraft) {
  draftError.value = "";
  try {
    await api.deleteOfferDraft(draft.id);
    drafts.value = drafts.value.filter((item) => item.id !== draft.id);
  } catch (caught) {
    draftError.value = errorMessage(caught);
  } finally {
    confirmDeleteDraftId.value = null;
  }
}

async function syncDrafts() {
  draftError.value = "";
  notice.value = "";
  syncing.value = true;
  try {
    const synced = await api.syncOfferDrafts();
    notice.value = `Synced ${synced} ${synced === 1 ? "offer" : "offers"} to the website.`;
    await Promise.all([loadDrafts(), load(1)]);
  } catch (caught) {
    draftError.value = errorMessage(caught);
  } finally {
    syncing.value = false;
  }
}

onMounted(() => {
  load(1);
  loadDrafts();
  autoRefresh.start();
});
onUnmounted(() => {
  autoRefresh.stop();
  window.clearTimeout(searchTimer);
});
</script>

<template>
  <GenerateOffersView
    v-if="generatingOffers"
    :session="session"
    @close="generatingOffers = false"
    @generated="onGenerated"
    @open-settings="generatingOffers = false; emit('openSettings')"
  />

  <OfferDraftModal
    v-if="editingDraft"
    :draft="editingDraft"
    :icp-options="result?.icp_options ?? []"
    :proof-options="proof"
    @close="editingDraft = null"
    @saved="onDraftSaved"
  />

  <div v-show="!generatingOffers" class="bp">
    <div class="bp-header">
      <h1 class="bp-h1">Offers</h1>
      <div class="od-header-actions">
        <RefreshControls v-model="autoRefreshOn" :countdown="countdown" :refreshing="busy" size="sm" @refresh="load()" />
        <button class="iv-plan" @click="generatingOffers = true"><Icon name="sparkles" :size="14" /> Generate offers</button>
      </div>
    </div>

    <div v-if="notice" class="alert alert-success row-between" role="status">
      {{ notice }}
      <button class="btn btn-sm btn-ghost" @click="notice = ''">Dismiss</button>
    </div>

    <!-- Offers the AI agent drafted that are not on the website yet -->
    <section v-if="drafts.length || draftError" class="od" aria-label="New offers from your AI agent">
      <div class="od-head">
        <div style="min-width: 0">
          <h2 class="od-title"><Icon name="sparkles" :size="14" /> New offers from your AI agent</h2>
          <p class="od-sub">
            {{ drafts.length }} {{ drafts.length === 1 ? "offer is" : "offers are" }} only in this app. Edit or delete any you don't want, then sync the rest to the website.
          </p>
        </div>
        <button v-if="drafts.length" class="iv-plan" :disabled="syncing" @click="syncDrafts">
          <span v-if="syncing" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
          <Icon v-else name="upload" :size="14" />
          {{ syncing ? "Syncing…" : `Sync ${drafts.length} to website` }}
        </button>
      </div>

      <div v-if="draftError" class="alert alert-error" role="alert">{{ draftError }}</div>

      <ul v-if="drafts.length" class="bp-grid">
        <li v-for="draft in drafts" :key="draft.id" class="bp-card od-card">
          <div class="bp-card-body">
            <div class="bp-card-top">
              <div style="min-width: 0">
                <button class="bp-name" @click="editingDraft = draft">{{ draft.name }}</button>
                <div class="bp-meta">
                  <span class="bp-price">{{ draft.priceRange || "No price stated" }}</span>
                  <span v-if="draft.timeline" class="bp-inline"><Icon name="clock" :size="12" :stroke="1.5" /> {{ draft.timeline }}</span>
                </div>
              </div>
              <span class="bp-source od-pending" title="Drafted in this app and not synced to the website yet">
                <Icon name="bot" :size="12" :stroke="1.5" /> Not synced
              </span>
            </div>

            <p class="bp-promise">{{ draft.promise }}</p>

            <ul v-if="draft.deliverables.length" class="bp-deliverables">
              <li v-for="item in draft.deliverables.slice(0, 3)" :key="item"><span class="bp-bullet"></span>{{ item }}</li>
              <li v-if="draft.deliverables.length > 3" class="bp-more">+{{ draft.deliverables.length - 3 }} more</li>
            </ul>

            <div v-if="draft.icpIds.some((id) => icpNames.has(id))" class="bp-chips">
              <span class="bp-chip-icon"><Icon name="target" :size="12" :stroke="1.5" /></span>
              <template v-for="id in draft.icpIds" :key="id">
                <span v-if="icpNames.has(id)" class="bp-chip">{{ icpNames.get(id) }}</span>
              </template>
            </div>

            <p v-if="draft.rationale" class="od-why"><b>Why:</b> {{ draft.rationale }}</p>
          </div>

          <div class="bp-card-foot">
            <div class="bp-foot-stats">
              <span class="bp-inline" title="Case studies that will be attached to this offer">
                <Icon name="fileCheck" :size="12" :stroke="1.5" /> {{ draft.caseStudyIds.length }} proof
              </span>
            </div>
            <div v-if="confirmDeleteDraftId === draft.id" class="od-actions">
              <span>Delete this offer?</span>
              <button class="bp-open" @click="confirmDeleteDraftId = null">Keep</button>
              <button class="bp-open od-danger" @click="deleteDraft(draft)">Delete</button>
            </div>
            <div v-else class="od-actions">
              <button class="bp-open" :disabled="syncing" @click="editingDraft = draft"><Icon name="pencil" :size="12" /> Edit</button>
              <button class="bp-open od-danger" :disabled="syncing" @click="confirmDeleteDraftId = draft.id"><Icon name="trash" :size="12" /> Delete</button>
            </div>
          </div>
        </li>
      </ul>
    </section>

    <div v-if="error" class="alert alert-error" role="alert">
      {{ error }}
      <div style="margin-top: 8px"><button class="btn btn-sm" :disabled="busy" @click="load()">Try again</button></div>
    </div>

    <template v-if="!result && !error">
      <div class="skeleton" style="height: 82px"></div>
      <div class="bp-grid" aria-busy="true">
        <div v-for="n in 4" :key="n" class="skeleton" style="height: 260px"></div>
      </div>
    </template>

    <div v-else-if="result && result.stats.total === 0" class="bp-empty">
      <span class="bp-empty-icon"><Icon name="package" :stroke="1.5" /></span>
      <p class="bp-empty-title">No offers yet</p>
      <p class="bp-empty-detail">Let your AI agent draft offers from your company overview, ICPs and case studies, or add one in the web app.</p>
      <div class="od-empty-actions">
        <button class="iv-plan" @click="generatingOffers = true"><Icon name="sparkles" :size="14" /> Generate offers</button>
        <button class="bp-btn" @click="openSite('/brain/offers')">Open the web app</button>
      </div>
    </div>

    <template v-else-if="result">
      <dl class="bp-stats">
        <div>
          <dt>Offers</dt>
          <dd>{{ result.stats.total }}</dd>
        </div>
        <div>
          <dt>Drafted by agent</dt>
          <dd>{{ result.stats.drafted_by_agent }}</dd>
        </div>
        <div>
          <dt>With a stated price</dt>
          <dd>{{ result.stats.priced }}</dd>
        </div>
        <div>
          <dt>Backed by proof</dt>
          <dd>{{ result.stats.with_proof }}</dd>
          <p>have at least one case study</p>
        </div>
      </dl>

      <div class="bp-toolbar">
        <div class="bp-search">
          <Icon name="search" :size="14" :stroke="1.5" />
          <input v-model="search" type="search" aria-label="Search offers" placeholder="Search offers…" />
        </div>
        <select v-if="result.icp_options.length" v-model="icpFilter" class="bp-select" aria-label="Filter by ICP">
          <option value="all">Any ICP</option>
          <option v-for="icp in result.icp_options" :key="icp.id" :value="String(icp.id)">{{ icp.name }}</option>
        </select>
        <select v-model="pricing" class="bp-select" aria-label="Filter by pricing">
          <option v-for="option in PRICING_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <select v-model="source" class="bp-select" aria-label="Filter by source">
          <option v-for="option in SOURCE_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <select v-model="sort" class="bp-select" aria-label="Sort offers">
          <option v-for="option in SORT_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <button v-if="filtersActive" class="bp-clear" @click="clearFilters"><Icon name="x" :size="12" :stroke="2" /> Clear filters</button>
        <span class="bp-count">
          {{ filtersActive ? `${result.meta.total} of ${result.stats.total}` : result.stats.total }}
          {{ result.stats.total === 1 ? "offer" : "offers" }}
        </span>
      </div>

      <div v-if="!result.offers.length" class="bp-empty">
        <span class="bp-empty-icon"><Icon name="package" :stroke="1.5" /></span>
        <p class="bp-empty-title">No offers match these filters</p>
        <p class="bp-empty-detail">Try a different search or clear the filters to see every offer.</p>
        <button class="bp-btn" style="margin-top: 16px" @click="clearFilters">Clear filters</button>
      </div>

      <ul v-else class="bp-grid" :class="{ 'bp-busy': busy }">
        <li v-for="offer in result.offers" :key="offer.id" class="bp-card">
          <div class="bp-card-body">
            <div class="bp-card-top">
              <div style="min-width: 0">
                <button class="bp-name" @click="openSite(`/brain/offers/${offer.id}`)">{{ offer.name }}</button>
                <div class="bp-meta">
                  <span class="bp-price">{{ offer.price_range || "No price stated" }}</span>
                  <span v-if="offer.timeline" class="bp-inline"><Icon name="clock" :size="12" :stroke="1.5" /> {{ offer.timeline }}</span>
                </div>
              </div>
              <span
                class="bp-source"
                :class="{ 'bp-source-agent': offer.drafted_by_agent }"
                :title="offer.drafted_by_agent ? 'Drafted by the research agent from your website' : 'Added by hand'"
              >
                <Icon :name="offer.drafted_by_agent ? 'bot' : 'user'" :size="12" :stroke="1.5" />
                {{ offer.drafted_by_agent ? "Agent" : "Manual" }}
              </span>
            </div>

            <p class="bp-promise">{{ offer.promise }}</p>

            <ul v-if="offer.deliverables.length" class="bp-deliverables">
              <li v-for="item in offer.deliverables.slice(0, 3)" :key="item"><span class="bp-bullet"></span>{{ item }}</li>
              <li v-if="offer.deliverables.length > 3" class="bp-more">+{{ offer.deliverables.length - 3 }} more</li>
            </ul>

            <div v-if="offer.icps.length" class="bp-chips">
              <span class="bp-chip-icon"><Icon name="target" :size="12" :stroke="1.5" /></span>
              <span v-for="icp in offer.icps" :key="icp.id" class="bp-chip">{{ icp.name }}</span>
            </div>
          </div>

          <div class="bp-card-foot">
            <div class="bp-foot-stats">
              <span class="bp-inline" title="Case studies attached to this offer"><Icon name="fileCheck" :size="12" :stroke="1.5" /> {{ offer.case_studies_count }} proof</span>
              <span class="bp-inline" title="Leads matched to this offer"><Icon name="users" :size="12" :stroke="1.5" /> {{ offer.leads_count }} leads</span>
              <span
                class="bp-inline"
                :class="{ 'bp-ideas': ideaCountFor(offer, ideaCounts) > 0 }"
                title="Ideas generated from this offer in this app"
              >
                <Icon name="lightbulb" :size="12" :stroke="1.5" /> {{ ideaCountFor(offer, ideaCounts) }} ideas
              </span>
            </div>
            <button class="bp-open" @click="openSite(`/brain/offers/${offer.id}`)">Open <Icon name="arrowRight" :size="12" /></button>
          </div>
        </li>
      </ul>

      <Pagination :page="page" :last-page="result.meta.last_page" :total="result.meta.total" :noun="['offer', 'offers']" :busy="busy" @change="load" />
    </template>
  </div>
</template>
