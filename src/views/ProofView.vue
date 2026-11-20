<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, errorMessage, type Named, type SiteProof } from "../api";
import Icon from "../components/Icon.vue";
import RefreshControls from "../components/RefreshControls.vue";
import { createAutoRefresh } from "../autoRefresh";

type SourceFilter = "all" | "agent" | "manual";
type DisclosureFilter = "all" | "named" | "anonymous";
type SortKey = "drafted" | "title" | "recent";

const DISCLOSURE_OPTIONS: { value: DisclosureFilter; label: string }[] = [
  { value: "all", label: "Any disclosure" },
  { value: "named", label: "Client can be named" },
  { value: "anonymous", label: "Anonymous only" },
];
const SOURCE_OPTIONS: { value: SourceFilter; label: string }[] = [
  { value: "all", label: "Any source" },
  { value: "agent", label: "Agent-drafted" },
  { value: "manual", label: "Added manually" },
];
const SORT_OPTIONS: { value: SortKey; label: string }[] = [
  { value: "drafted", label: "Sort: as drafted" },
  { value: "title", label: "Sort: title A–Z" },
  { value: "recent", label: "Sort: recently edited" },
];

const proof = ref<SiteProof[] | null>(null);
const offerOptions = ref<Named[]>([]);
const error = ref("");
const busy = ref(false);

const search = ref("");
const offerFilter = ref("all");
const disclosure = ref<DisclosureFilter>("all");
const source = ref<SourceFilter>("all");
const sort = ref<SortKey>("drafted");

/** Keeps the current list on screen while a refresh is in flight. */
async function load() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await api.sitePage("proof");
    proof.value = result.proof;
    offerOptions.value = result.offerOptions;
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = false;
    autoRefresh.restart();
  }
}

// Leaving the page stops the clock; coming back loads fresh case studies and starts it over.
const autoRefresh = createAutoRefresh("proof.autoRefresh", () => load(), () => busy.value);
const { enabled: autoRefreshOn, countdown } = autoRefresh;

const visible = computed(() => {
  const needle = search.value.trim().toLowerCase();
  const rows = (proof.value ?? []).filter(
    (item) =>
      (!needle ||
        [item.title, item.result, item.client_name, item.client_type, item.industry].some((field) => field?.toLowerCase().includes(needle))) &&
      (source.value === "all" || (source.value === "agent") === item.drafted_by_agent) &&
      (disclosure.value === "all" || (disclosure.value === "named") === item.can_name_client) &&
      (offerFilter.value === "all" || (offerFilter.value === "none" ? item.offer_id === null : String(item.offer_id) === offerFilter.value)),
  );
  if (sort.value === "title") return [...rows].sort((a, b) => a.title.localeCompare(b.title));
  if (sort.value === "recent") return [...rows].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
  return rows;
});

const filtersActive = computed(() => search.value !== "" || source.value !== "all" || disclosure.value !== "all" || offerFilter.value !== "all");

const stats = computed(() => {
  const all = proof.value ?? [];
  const linked = all.filter((item) => item.offer_id !== null).length;
  return {
    total: all.length,
    named: all.filter((item) => item.can_name_client).length,
    anonymous: all.filter((item) => !item.can_name_client).length,
    linked,
    unlinked: all.length - linked,
  };
});

function clearFilters() {
  search.value = "";
  source.value = "all";
  disclosure.value = "all";
  offerFilter.value = "all";
}

function clientLine(item: SiteProof) {
  return [item.client_name, item.client_type, item.industry].filter(Boolean).join(" · ") || "Client not described";
}

onMounted(() => {
  load();
  autoRefresh.start();
});
onUnmounted(() => autoRefresh.stop());
</script>

<template>
  <div class="bp">
    <div class="bp-header">
      <h1 class="bp-h1">Proof &amp; case studies</h1>
      <RefreshControls v-model="autoRefreshOn" :countdown="countdown" :refreshing="busy" size="sm" @refresh="load()" />
    </div>

    <div v-if="error" class="alert alert-error" role="alert">
      {{ error }}
      <div style="margin-top: 8px"><button class="btn btn-sm" :disabled="busy" @click="load()">Try again</button></div>
    </div>

    <template v-if="!proof && !error">
      <div class="skeleton" style="height: 82px"></div>
      <div class="skeleton" style="height: 320px"></div>
    </template>

    <div v-else-if="proof && !proof.length" class="bp-empty">
      <span class="bp-empty-icon"><Icon name="fileCheck" :stroke="1.5" /></span>
      <p class="bp-empty-title">No case studies yet</p>
      <p class="bp-empty-detail">Add a client result in the web app, import a CSV or Excel file there, or sync from your website.</p>
    </div>

    <template v-else-if="proof">
      <dl class="bp-stats">
        <div>
          <dt>Case studies</dt>
          <dd>{{ stats.total }}</dd>
        </div>
        <div>
          <dt>Client can be named</dt>
          <dd>{{ stats.named }}</dd>
        </div>
        <div>
          <dt>Anonymous only</dt>
          <dd>{{ stats.anonymous }}</dd>
        </div>
        <div>
          <dt>Linked to an offer</dt>
          <dd>{{ stats.linked }}</dd>
          <p>{{ stats.unlinked }} not linked</p>
        </div>
      </dl>

      <div class="bp-toolbar">
        <div class="bp-search">
          <Icon name="search" :size="14" :stroke="1.5" />
          <input v-model="search" type="search" aria-label="Search case studies" placeholder="Search case studies…" />
        </div>
        <select v-model="offerFilter" class="bp-select" aria-label="Filter by offer">
          <option value="all">Any offer</option>
          <option value="none">Not linked to an offer</option>
          <option v-for="offer in offerOptions" :key="offer.id" :value="String(offer.id)">{{ offer.name }}</option>
        </select>
        <select v-model="disclosure" class="bp-select" aria-label="Filter by disclosure">
          <option v-for="option in DISCLOSURE_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <select v-model="source" class="bp-select" aria-label="Filter by source">
          <option v-for="option in SOURCE_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <select v-model="sort" class="bp-select" aria-label="Sort case studies">
          <option v-for="option in SORT_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <button v-if="filtersActive" class="bp-clear" @click="clearFilters"><Icon name="x" :size="12" :stroke="2" /> Clear filters</button>
        <span class="bp-count">
          {{ filtersActive ? `${visible.length} of ${stats.total}` : stats.total }} {{ stats.total === 1 ? "case study" : "case studies" }}
        </span>
      </div>

      <div v-if="!visible.length" class="bp-empty">
        <span class="bp-empty-icon"><Icon name="fileCheck" :stroke="1.5" /></span>
        <p class="bp-empty-title">No case studies match these filters</p>
        <p class="bp-empty-detail">Try a different search or clear the filters to see every case study.</p>
        <button class="bp-btn" style="margin-top: 16px" @click="clearFilters">Clear filters</button>
      </div>

      <div v-else class="bp-table-wrap" :class="{ 'bp-busy': busy }">
        <table class="bp-table" style="min-width: 820px">
          <thead>
            <tr>
              <th scope="col" style="width: 30%">Case study</th>
              <th scope="col" style="width: 32%">Result</th>
              <th scope="col">Offer</th>
              <th scope="col">Disclosure</th>
              <th scope="col">Source</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in visible" :key="item.id">
              <td>
                <p style="font-weight: 600; color: #111827">{{ item.title }}</p>
                <p class="bp-pain" style="margin-top: 2px">{{ clientLine(item) }}</p>
              </td>
              <td style="color: #1f2937">{{ item.result }}</td>
              <td>
                <span v-if="item.offer" class="bp-chip">{{ item.offer.name }}</span>
                <span v-else style="color: #9ca3af">Not linked</span>
              </td>
              <td>
                <span class="bp-source" :class="{ 'bp-source-named': item.can_name_client }">
                  <Icon :name="item.can_name_client ? 'eye' : 'eyeOff'" :size="12" :stroke="1.5" />
                  {{ item.can_name_client ? "Can name" : "Anonymous" }}
                </span>
              </td>
              <td>
                <span
                  class="bp-source"
                  :class="{ 'bp-source-agent': item.drafted_by_agent }"
                  :title="item.drafted_by_agent ? 'Drafted by the research agent from your website' : 'Added by hand'"
                >
                  <Icon :name="item.drafted_by_agent ? 'bot' : 'user'" :size="12" :stroke="1.5" />
                  {{ item.drafted_by_agent ? "Agent" : "Manual" }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>
