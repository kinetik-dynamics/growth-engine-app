<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type Named, type Session, type SiteIcp } from "../api";
import Icon from "../components/Icon.vue";
import RefreshControls from "../components/RefreshControls.vue";
import { createAutoRefresh } from "../autoRefresh";

type SourceFilter = "all" | "agent" | "manual";
type ActivityFilter = "all" | "with_leads" | "no_leads" | "no_searches";
type SortKey = "drafted" | "name" | "leads" | "recent";

const props = defineProps<{ session: Session }>();

const ACTIVITY_OPTIONS: { value: ActivityFilter; label: string }[] = [
  { value: "all", label: "Any activity" },
  { value: "with_leads", label: "Has leads" },
  { value: "no_leads", label: "No leads yet" },
  { value: "no_searches", label: "No searches yet" },
];
const SOURCE_OPTIONS: { value: SourceFilter; label: string }[] = [
  { value: "all", label: "Any source" },
  { value: "agent", label: "Agent-drafted" },
  { value: "manual", label: "Added manually" },
];
const SORT_OPTIONS: { value: SortKey; label: string }[] = [
  { value: "drafted", label: "Sort: as drafted" },
  { value: "name", label: "Sort: name A–Z" },
  { value: "leads", label: "Sort: most leads" },
  { value: "recent", label: "Sort: recently edited" },
];

const icps = ref<SiteIcp[] | null>(null);
const offerOptions = ref<Named[]>([]);
const error = ref("");
const busy = ref(false);

const search = ref("");
const offerFilter = ref("all");
const activity = ref<ActivityFilter>("all");
const source = ref<SourceFilter>("all");
const sort = ref<SortKey>("drafted");

/** Keeps the current list on screen while a refresh is in flight. */
async function load() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await api.sitePage("icps");
    icps.value = result.icps;
    offerOptions.value = result.offerOptions;
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = false;
    autoRefresh.restart();
  }
}

// Leaving the page stops the clock; coming back loads fresh profiles and starts it over.
const autoRefresh = createAutoRefresh("icps.autoRefresh", () => load(), () => busy.value);
const { enabled: autoRefreshOn, countdown } = autoRefresh;

function matchesActivity(icp: SiteIcp, filter: ActivityFilter) {
  if (filter === "with_leads") return icp.leads_count > 0;
  if (filter === "no_leads") return icp.leads_count === 0;
  if (filter === "no_searches") return icp.searches_count === 0;
  return true;
}

const visible = computed(() => {
  const needle = search.value.trim().toLowerCase();
  const rows = (icps.value ?? []).filter(
    (icp) =>
      (!needle ||
        [icp.name, icp.who_they_are, icp.roles_to_contact, icp.buying_triggers, icp.pain, ...(icp.criteria ?? [])].some((field) =>
          field?.toLowerCase().includes(needle),
        )) &&
      (source.value === "all" || (source.value === "agent") === icp.drafted_by_agent) &&
      (offerFilter.value === "all" || icp.offers.some((offer) => String(offer.id) === offerFilter.value)) &&
      matchesActivity(icp, activity.value),
  );
  if (sort.value === "name") return [...rows].sort((a, b) => a.name.localeCompare(b.name));
  if (sort.value === "leads") return [...rows].sort((a, b) => b.leads_count - a.leads_count);
  if (sort.value === "recent") return [...rows].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
  return rows;
});

const filtersActive = computed(() => search.value !== "" || source.value !== "all" || offerFilter.value !== "all" || activity.value !== "all");

const stats = computed(() => {
  const all = icps.value ?? [];
  return {
    total: all.length,
    agent: all.filter((icp) => icp.drafted_by_agent).length,
    prospects: all.reduce((sum, icp) => sum + icp.prospects_count, 0),
    leads: all.reduce((sum, icp) => sum + icp.leads_count, 0),
  };
});

function clearFilters() {
  search.value = "";
  source.value = "all";
  offerFilter.value = "all";
  activity.value = "all";
}

function openIcp(id: number) {
  openUrl(`${props.session.baseUrl}/brain/icps/${id}`);
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
      <h1 class="bp-h1">Ideal client profiles</h1>
      <RefreshControls v-model="autoRefreshOn" :countdown="countdown" :refreshing="busy" size="sm" @refresh="load()" />
    </div>

    <div v-if="error" class="alert alert-error" role="alert">
      {{ error }}
      <div style="margin-top: 8px"><button class="btn btn-sm" :disabled="busy" @click="load()">Try again</button></div>
    </div>

    <template v-if="!icps && !error">
      <div class="skeleton" style="height: 82px"></div>
      <div class="skeleton" style="height: 320px"></div>
    </template>

    <div v-else-if="icps && !icps.length" class="bp-empty">
      <span class="bp-empty-icon"><Icon name="target" :stroke="1.5" /></span>
      <p class="bp-empty-title">No ideal client profiles yet</p>
      <p class="bp-empty-detail">Add your first profile in the web app, or sync from your website and the research agent will draft them.</p>
    </div>

    <template v-else-if="icps">
      <dl class="bp-stats">
        <div>
          <dt>Profiles</dt>
          <dd>{{ stats.total }}</dd>
        </div>
        <div>
          <dt>Drafted by agent</dt>
          <dd>{{ stats.agent }}</dd>
        </div>
        <div>
          <dt>Companies found</dt>
          <dd>{{ stats.prospects }}</dd>
          <p>matched by the website scan</p>
        </div>
        <div>
          <dt>Leads imported</dt>
          <dd>{{ stats.leads }}</dd>
          <p>from LinkedIn searches</p>
        </div>
      </dl>

      <div class="bp-toolbar">
        <div class="bp-search">
          <Icon name="search" :size="14" :stroke="1.5" />
          <input v-model="search" type="search" aria-label="Search profiles" placeholder="Search profiles…" />
        </div>
        <select v-if="offerOptions.length" v-model="offerFilter" class="bp-select" aria-label="Filter by offer">
          <option value="all">Any offer</option>
          <option v-for="offer in offerOptions" :key="offer.id" :value="String(offer.id)">{{ offer.name }}</option>
        </select>
        <select v-model="activity" class="bp-select" aria-label="Filter by activity">
          <option v-for="option in ACTIVITY_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <select v-model="source" class="bp-select" aria-label="Filter by source">
          <option v-for="option in SOURCE_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <select v-model="sort" class="bp-select" aria-label="Sort profiles">
          <option v-for="option in SORT_OPTIONS" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <button v-if="filtersActive" class="bp-clear" @click="clearFilters"><Icon name="x" :size="12" :stroke="2" /> Clear filters</button>
        <span class="bp-count">
          {{ filtersActive ? `${visible.length} of ${stats.total}` : stats.total }} {{ stats.total === 1 ? "profile" : "profiles" }}
        </span>
      </div>

      <div v-if="!visible.length" class="bp-empty">
        <span class="bp-empty-icon"><Icon name="target" :stroke="1.5" /></span>
        <p class="bp-empty-title">No profiles match these filters</p>
        <p class="bp-empty-detail">Try a different search or clear the filters to see every profile.</p>
        <button class="bp-btn" style="margin-top: 16px" @click="clearFilters">Clear filters</button>
      </div>

      <div v-else class="bp-table-wrap" :class="{ 'bp-busy': busy }">
        <table class="bp-table">
          <thead>
            <tr>
              <th scope="col" style="width: 38%">Profile</th>
              <th scope="col">Roles to contact</th>
              <th scope="col">Offers</th>
              <th scope="col" class="bp-num-col">Companies</th>
              <th scope="col" class="bp-num-col">Searches</th>
              <th scope="col" class="bp-num-col">Leads</th>
              <th scope="col">Source</th>
              <th scope="col"><span class="sr-only">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="icp in visible" :key="icp.id">
              <td>
                <button class="bp-name bp-name-sm" @click="openIcp(icp.id)">{{ icp.name }}</button>
                <p class="bp-clamp2 bp-gray-600" style="margin-top: 2px">{{ icp.who_they_are }}</p>
                <p v-if="icp.pain" class="bp-clamp1 bp-pain">Pain: {{ icp.pain }}</p>
              </td>
              <td class="bp-gray-600">{{ icp.roles_to_contact || "—" }}</td>
              <td>
                <div v-if="icp.offers.length" class="bp-chips" style="gap: 4px; max-width: 208px">
                  <span v-for="offer in icp.offers" :key="offer.id" class="bp-chip">{{ offer.name }}</span>
                </div>
                <span v-else style="color: #9ca3af">Not linked</span>
              </td>
              <td class="bp-num-col bp-mono bp-gray-600">{{ icp.prospects_count }}</td>
              <td class="bp-num-col bp-mono bp-gray-600">{{ icp.searches_count }}</td>
              <td class="bp-num-col bp-mono" style="font-weight: 600; color: #111827">{{ icp.leads_count }}</td>
              <td>
                <span
                  class="bp-source"
                  :class="{ 'bp-source-agent': icp.drafted_by_agent }"
                  :title="icp.drafted_by_agent ? 'Drafted by the research agent from your website' : 'Added by hand'"
                >
                  <Icon :name="icp.drafted_by_agent ? 'bot' : 'user'" :size="12" :stroke="1.5" />
                  {{ icp.drafted_by_agent ? "Agent" : "Manual" }}
                </span>
              </td>
              <td style="text-align: right">
                <button class="bp-open" @click="openIcp(icp.id)">Open <Icon name="arrowRight" :size="12" /></button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>
