<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  api,
  errorMessage,
  ideaCountFor,
  type OfferIdeaCount,
  type OfferPage,
  type OfferPricing,
  type OfferSort,
  type OfferSource,
  type Session,
} from "../api";
import Icon from "../components/Icon.vue";
import Pagination from "../components/Pagination.vue";
import RefreshControls from "../components/RefreshControls.vue";
import { createAutoRefresh } from "../autoRefresh";

const props = defineProps<{ session: Session }>();

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

onMounted(() => {
  load(1);
  autoRefresh.start();
});
onUnmounted(() => {
  autoRefresh.stop();
  window.clearTimeout(searchTimer);
});
</script>

<template>
  <div class="bp">
    <div class="bp-header">
      <h1 class="bp-h1">Offers</h1>
      <RefreshControls v-model="autoRefreshOn" :countdown="countdown" :refreshing="busy" size="sm" @refresh="load()" />
    </div>

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
      <p class="bp-empty-detail">Add your first package in the web app, or sync from your website and the research agent will draft them.</p>
      <button class="bp-btn" style="margin-top: 16px" @click="openSite('/brain/offers')">Open the web app</button>
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
