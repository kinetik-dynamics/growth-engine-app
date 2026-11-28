<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type Named, type QueueItem } from "../api";
import { externalUrl, primaryLabel, ratingLabel, secondaryLabel, timeAgo } from "../leads";
import CompanyAvatar from "../components/CompanyAvatar.vue";
import Icon from "../components/Icon.vue";
import InfoRow from "../components/InfoRow.vue";
import RefreshControls from "../components/RefreshControls.vue";
import ResearchLeadsView from "./ResearchLeadsView.vue";
import { createAutoRefresh } from "../autoRefresh";

const emit = defineEmits<{ openSettings: [] }>();

/** The research page replaces the queue while it is open. */
const researching = ref(false);

/* Research found in the app waits here until it is synced to the website. */
const pendingCount = ref(0);
const syncing = ref(false);
const syncError = ref("");
const syncNotice = ref("");

async function sync() {
  syncError.value = syncNotice.value = "";
  syncing.value = true;
  try {
    const summary = await api.syncLeadResearch();
    syncNotice.value = `Sent ${summary.sent} ${summary.sent === 1 ? "lead" : "leads"} to the website: ${summary.updated} updated, ${summary.fieldsFilled} ${summary.fieldsFilled === 1 ? "field" : "fields"} filled.`;
  } catch (caught) {
    syncError.value = errorMessage(caught);
  } finally {
    syncing.value = false;
    await load();
  }
}
type Target = { type: "lead" | "prospect"; id: number; name: string };
/** The leads picked for research; empty means the whole queue. */
const researchTargets = ref<Target[]>([]);

function openResearch(picked?: QueueItem | QueueItem[]) {
  const list = picked === undefined ? [] : Array.isArray(picked) ? picked : [picked];
  researchTargets.value = list.map((item) => ({ type: item.type, id: item.id, name: primaryLabel(item) }));
  researching.value = true;
}

type Presence = "all" | "has" | "missing";
type Enrichment = "all" | "enriched" | "not_enriched";
type SortKey = "newest" | "oldest" | "name";

const items = ref<QueueItem[] | null>(null);
const icps = ref<Named[]>([]);
const error = ref("");
const busy = ref(false);
const selectedKey = ref<string | null>(null);

const search = ref("");
const icpFilter = ref("all");
const enrichment = ref<Enrichment>("all");
const emailFilter = ref<Presence>("all");
const phoneFilter = ref<Presence>("all");
const websiteFilter = ref<Presence>("all");
const source = ref("all");
const sort = ref<SortKey>("newest");

function keyOf(item: Pick<QueueItem, "id" | "type">) {
  return `${item.type}-${item.id}`;
}

async function load() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await api.sitePage("queue");
    items.value = result.items;
    icps.value = result.icps;
    pendingCount.value = await api.pendingResearchCount().catch(() => pendingCount.value);
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = false;
    autoRefresh.restart();
  }
}

const autoRefresh = createAutoRefresh("queue.autoRefresh", () => load(), () => busy.value);
const { enabled: autoRefreshOn, countdown } = autoRefresh;

watch(researching, (open) => (open ? autoRefresh.stop() : autoRefresh.start()));

function closeResearch() {
  researching.value = false;
  researchTargets.value = [];
  load();
}

/** Blank strings count as missing. */
function passes(filter: Presence, value: string | null | undefined) {
  if (filter === "all") return true;
  const present = !!value && value.trim() !== "";
  return filter === "has" ? present : !present;
}

const icpOptions = computed(() => [...new Set((items.value ?? []).filter((item) => item.icp).map((item) => item.icp!.name))].sort());

const visible = computed(() => {
  const needle = search.value.trim().toLowerCase();
  const rows = (items.value ?? []).filter((item) => {
    if (source.value !== "all" && item.source_label !== source.value) return false;
    if (icpFilter.value === "none" ? item.icp !== null : icpFilter.value !== "all" && item.icp?.name !== icpFilter.value) return false;
    if (!passes(emailFilter.value, item.email) || !passes(phoneFilter.value, item.phone)) return false;
    // Scanned companies carry a domain instead of a website.
    if (!passes(websiteFilter.value, item.website || item.company_domain)) return false;
    if (enrichment.value === "enriched" && !item.enriched_at) return false;
    if (enrichment.value === "not_enriched" && (item.type !== "lead" || item.enriched_at)) return false;
    if (!needle) return true;
    return [item.contact_name, item.company_name, item.contact_title, item.email, item.location, item.headline].some((field) =>
      field?.toLowerCase().includes(needle),
    );
  });
  if (sort.value === "name") return [...rows].sort((a, b) => primaryLabel(a).localeCompare(primaryLabel(b)));
  if (sort.value === "oldest") return [...rows].reverse();
  return rows;
});

function clearFilters() {
  search.value = "";
  icpFilter.value = "all";
  enrichment.value = "all";
  emailFilter.value = phoneFilter.value = websiteFilter.value = "all";
  source.value = "all";
}

/** Leads ticked in the list, by keyOf(). Always replaced, never mutated, so Vue re-renders. */
const checked = ref<Set<string>>(new Set());

function toggleChecked(item: QueueItem) {
  const key = keyOf(item);
  const next = new Set(checked.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  checked.value = next;
}

const checkedItems = computed(() => visible.value.filter((item) => checked.value.has(keyOf(item))));
const allChecked = computed(() => visible.value.length > 0 && checkedItems.value.length === visible.value.length);
const someChecked = computed(() => checkedItems.value.length > 0 && !allChecked.value);

function toggleAll() {
  checked.value = allChecked.value ? new Set() : new Set(visible.value.map(keyOf));
}

function clearChecked() {
  checked.value = new Set();
}

/* Keep a valid row selected as the list changes, and drop ticks on rows that are no longer shown. */
watch(
  visible,
  (rows) => {
    if (!rows.some((item) => keyOf(item) === selectedKey.value)) selectedKey.value = rows[0] ? keyOf(rows[0]) : null;
    const keys = new Set(rows.map(keyOf));
    if ([...checked.value].some((key) => !keys.has(key))) checked.value = new Set([...checked.value].filter((key) => keys.has(key)));
  },
  { immediate: true },
);

const selected = computed(() => visible.value.find((item) => keyOf(item) === selectedKey.value) ?? null);
const selectedIndex = computed(() => visible.value.findIndex((item) => keyOf(item) === selectedKey.value));

const stats = computed(() => {
  const all = items.value ?? [];
  const leads = all.filter((item) => item.type === "lead");
  return [
    { label: "Total", value: all.length },
    { label: "With email", value: leads.filter((item) => item.email).length },
    { label: "With phone", value: leads.filter((item) => item.phone).length },
    { label: "Enriched", value: leads.filter((item) => item.enriched_at).length },
  ];
});

function onKeyDown(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null;
  if (target && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName)) return;
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  const step = event.key === "ArrowDown" ? 1 : -1;
  const next = visible.value[Math.min(Math.max(selectedIndex.value + step, 0), visible.value.length - 1)];
  if (next) {
    event.preventDefault();
    selectedKey.value = keyOf(next);
  }
}

onMounted(() => {
  load();
  autoRefresh.start();
  window.addEventListener("keydown", onKeyDown);
});
onUnmounted(() => {
  autoRefresh.stop();
  window.removeEventListener("keydown", onKeyDown);
});
</script>

<template>
  <ResearchLeadsView
    v-if="researching"
    :targets="researchTargets"
    @close="closeResearch"
    @changed="load()"
    @open-settings="researching = false; emit('openSettings')"
  />

  <div v-else class="lq-page">
    <div class="pub-header">
      <div class="pub-header-left"><h1 class="pub-title">Lead Approval Queue</h1></div>
      <div class="lq-head-actions">
        <button
          type="button"
          class="lq-research"
          :disabled="!items?.length"
          :title="checkedItems.length ? `Research only the ${checkedItems.length} selected leads` : 'Fill the empty fields of every lead with the AI agent'"
          @click="checkedItems.length ? openResearch(checkedItems) : openResearch()"
        >
          <Icon name="sparkles" :size="14" />
          {{ checkedItems.length ? `Research ${checkedItems.length} selected` : "Research leads" }}
        </button>
        <RefreshControls v-model="autoRefreshOn" :countdown="countdown" :refreshing="busy" size="sm" @refresh="load()" />
        <button
          type="button"
          class="lq-sync"
          :class="{ pending: pendingCount > 0 }"
          :disabled="syncing || !pendingCount"
          :title="pendingCount ? 'Send the research found in this app to the website' : 'Nothing to sync. Research changes show up here until they are sent.'"
          @click="sync"
        >
          <span v-if="syncing" class="spinner" style="width: 12px; height: 12px; border-width: 1.5px"></span>
          <Icon v-else name="upload" :size="14" />
          {{ syncing ? "Syncing…" : "Sync to website" }}
          <span v-if="pendingCount && !syncing" class="lq-sync-count">{{ pendingCount }}</span>
        </button>
      </div>
    </div>

    <div v-if="syncError || syncNotice" class="lq-sync-msg" :class="syncError ? 'bad' : 'ok'" :role="syncError ? 'alert' : 'status'">
      <span style="white-space: pre-wrap">{{ syncError || syncNotice }}</span>
      <button type="button" class="lq-linkbtn" @click="syncError = syncNotice = ''">Dismiss</button>
    </div>

    <div v-if="error" class="lq-state">
      <div class="alert alert-error" role="alert">
        {{ error }}
        <div style="margin-top: 8px"><button class="btn btn-sm" :disabled="busy" @click="load()">Try again</button></div>
      </div>
    </div>

    <div v-else-if="!items" class="lq-state" aria-busy="true">
      <div class="skeleton" style="height: 320px"></div>
    </div>

    <div v-else class="lq-body">
      <!-- Left: list -->
      <div class="lq-list">
        <div class="lq-filters">
          <div class="lq-stats">
            <div v-for="stat in stats" :key="stat.label" class="lq-stat">
              <div class="lq-stat-value">{{ stat.value }}</div>
              <div class="lq-stat-label">{{ stat.label }}</div>
            </div>
          </div>

          <div class="lq-search">
            <Icon name="search" :size="14" />
            <input v-model="search" type="search" placeholder="Search name, company, title, email…" aria-label="Search leads" />
          </div>

          <div class="lq-selects">
            <select v-model="icpFilter" aria-label="Filter by ICP">
              <option value="all">All ICPs</option>
              <option value="none">No ICP</option>
              <option v-for="name in icpOptions" :key="name" :value="name">{{ name }}</option>
            </select>
            <select v-model="enrichment" aria-label="Filter by contact info">
              <option value="all">Any enrichment</option>
              <option value="enriched">Enriched</option>
              <option value="not_enriched">Not enriched yet</option>
            </select>
          </div>
          <div class="lq-selects">
            <select v-model="emailFilter" aria-label="Filter by email">
              <option value="all">Email: any</option>
              <option value="has">Has email</option>
              <option value="missing">No email</option>
            </select>
            <select v-model="phoneFilter" aria-label="Filter by phone">
              <option value="all">Phone: any</option>
              <option value="has">Has phone</option>
              <option value="missing">No phone</option>
            </select>
            <select v-model="websiteFilter" aria-label="Filter by website">
              <option value="all">Website: any</option>
              <option value="has">Has website</option>
              <option value="missing">No website</option>
            </select>
          </div>
          <div class="lq-selects">
            <select v-model="source" aria-label="Filter by source">
              <option value="all">All sources</option>
              <option value="LinkedIn">LinkedIn</option>
              <option value="Onboarding Scan">Onboarding scan</option>
            </select>
            <select v-model="sort" aria-label="Sort leads">
              <option value="newest">Newest first</option>
              <option value="oldest">Oldest first</option>
              <option value="name">Name A–Z</option>
            </select>
          </div>
        </div>

        <div class="lq-bar">
          <label class="lq-check-all">
            <input
              type="checkbox"
              :checked="allChecked"
              :indeterminate="someChecked"
              :disabled="!visible.length"
              aria-label="Select all leads in the list"
              @change="toggleAll"
            />
            <span>{{ checkedItems.length ? `${checkedItems.length}/${visible.length} selected` : "Select all" }}</span>
          </label>
          <button v-if="checkedItems.length" type="button" class="lq-linkbtn" @click="clearChecked">Clear</button>
          <span class="lq-bar-count">{{ visible.length === items.length ? `${items.length} leads` : `${visible.length} of ${items.length}` }}</span>
        </div>

        <div class="lq-rows">
          <div v-if="!visible.length" class="lq-none">
            <template v-if="!items.length">
              <p>No leads yet. Open a saved search from an ICP and import the results with the Chrome extension.</p>
            </template>
            <template v-else>
              <p>No leads match these filters.</p>
              <button type="button" class="lq-linkbtn" @click="clearFilters">Clear filters</button>
            </template>
          </div>
          <div
            v-for="item in visible"
            :key="keyOf(item)"
            class="lq-row"
            :class="{ active: keyOf(item) === selectedKey, checked: checked.has(keyOf(item)) }"
            @click="selectedKey = keyOf(item)"
          >
            <input
              type="checkbox"
              class="lq-row-check"
              :checked="checked.has(keyOf(item))"
              :aria-label="`Select ${primaryLabel(item)}`"
              @click.stop
              @change="toggleChecked(item)"
            />
            <CompanyAvatar :label="primaryLabel(item)" :image-url="item.avatar_url" :domain="item.company_domain" round style="margin-top: 2px" />
            <div class="lq-row-main">
              <div class="lq-row-top">
                <span class="lq-row-name">{{ primaryLabel(item) }}</span>
                <span class="lq-row-age">{{ timeAgo(item.created_at) }}</span>
              </div>
              <div class="lq-row-sub">{{ secondaryLabel(item) || item.location || item.source_label }}</div>
              <div class="lq-row-tags">
                <span v-if="item.icp" class="lq-tag lq-tag-blue">{{ item.icp.name }}</span>
                <span v-if="item.type === 'prospect'" class="lq-tag lq-tag-gray">Company</span>
                <span class="lq-row-icons">
                  <button
                    type="button"
                    class="lq-row-research"
                    :title="item.researched_at ? `Researched ${timeAgo(item.researched_at)}. Research again` : 'Research this lead'"
                    :aria-label="`Research ${primaryLabel(item)}`"
                    @click.stop="openResearch(item)"
                  >
                    <Icon name="sparkles" :size="12" :style="item.researched_at ? 'color:#059669' : ''" />
                  </button>
                  <span v-if="item.email" title="Has email"><Icon name="mail" :size="12" style="color: #059669" /></span>
                  <span v-if="item.phone" title="Has phone"><Icon name="phone" :size="12" style="color: #059669" /></span>
                  <span v-if="item.enriched_at" title="Enriched from LinkedIn profile"><Icon name="sparkles" :size="12" style="color: #f59e0b" /></span>
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: detail -->
      <div class="lq-detail">
        <div v-if="!selected" class="lq-detail-empty">Select a lead to see its details.</div>
        <div v-else class="lq-detail-inner">
          <div class="ld-head">
            <CompanyAvatar :label="primaryLabel(selected)" :image-url="selected.avatar_url" :domain="selected.company_domain" round size="lg" />
            <div class="ld-head-main">
              <h2 class="ld-title">{{ primaryLabel(selected) }}</h2>
              <p class="ld-sub">{{ secondaryLabel(selected) || selected.source_label }}</p>
              <p v-if="selected.headline" class="ld-headline">{{ selected.headline }}</p>
              <div class="ld-badges">
                <span class="ld-badge ld-badge-gray">{{ selected.source_label }}</span>
                <span v-if="selected.icp" class="ld-badge ld-badge-blue">{{ selected.icp.name }}</span>
                <span v-if="selected.search_label" class="ld-badge ld-badge-outline">Search: {{ selected.search_label }}</span>
                <span v-if="selected.unsynced" class="ld-badge ld-badge-amber" title="Found in this app. Press Sync to website to send it.">Not synced yet</span>
                <span v-if="selected.researched_at" class="ld-badge ld-badge-green">
                  <Icon name="check" :size="12" /> Researched {{ timeAgo(selected.researched_at) }}
                </span>
                <span v-if="selected.enriched_at" class="ld-badge ld-badge-amber">
                  <Icon name="sparkles" :size="12" /> Enriched {{ timeAgo(selected.enriched_at) }}
                </span>
              </div>
            </div>
            <div class="ld-actions">
              <button type="button" class="ld-btn-outline" @click="openResearch(selected)">
                <Icon name="sparkles" :size="12" /> {{ selected.researched_at ? "Research again" : "Research" }}
              </button>
              <button v-if="selected.linkedin_url" type="button" class="ld-btn-primary" @click="openUrl(selected.linkedin_url)">
                LinkedIn <Icon name="externalLink" :size="12" />
              </button>
              <button v-if="selected.maps_url" type="button" class="ld-btn-primary" @click="openUrl(selected.maps_url)">
                Google Maps <Icon name="externalLink" :size="12" />
              </button>
            </div>
          </div>

          <div v-if="selected.type === 'lead' && !selected.enriched_at" class="ld-banner">
            <Icon name="sparkles" :size="14" style="margin-top: 2px; color: #d97706" />
            <p v-if="selected.maps_url">
              Only search-result details so far. Open the place on Google Maps and click <strong>Save place details</strong> in the extension to
              pull its phone, website and full address.
            </p>
            <p v-else>
              Only search-result details so far. Open their LinkedIn profile and click <strong>Save profile details</strong> in the extension to
              pull the headline, about, experience and any email, phone or website LinkedIn shows you.
            </p>
          </div>

          <div class="ld-grid">
            <section class="ld-section">
              <h3>Contact</h3>
              <div class="ld-rows">
                <InfoRow icon="mail" label="Email" :value="selected.email" :href="selected.email ? `mailto:${selected.email}` : undefined" copyable />
                <InfoRow icon="phone" label="Phone" :value="selected.phone" copyable />
                <InfoRow
                  icon="globe"
                  label="Website"
                  :value="selected.website || selected.company_domain"
                  :href="selected.website || selected.company_domain ? externalUrl((selected.website || selected.company_domain)!) : undefined"
                />
                <InfoRow icon="mapPin" label="Location" :value="selected.location" />
              </div>
            </section>

            <section class="ld-section">
              <h3>{{ selected.type === "lead" ? "Role" : "Company" }}</h3>
              <div class="ld-rows">
                <template v-if="selected.type === 'lead' && selected.maps_url">
                  <InfoRow label="Category" :value="selected.category" />
                  <InfoRow label="Rating" :value="ratingLabel(selected)" />
                </template>
                <template v-if="selected.type === 'lead' && !selected.maps_url">
                  <InfoRow label="First name" :value="selected.first_name" />
                  <InfoRow label="Last name" :value="selected.last_name" />
                  <InfoRow label="Title" :value="selected.contact_title" />
                </template>
                <InfoRow icon="building" label="Company" :value="selected.company_name" />
                <InfoRow label="ICP" :value="selected.icp?.name" />
                <InfoRow label="Added" :value="`${timeAgo(selected.created_at)} · ${new Date(selected.created_at).toLocaleDateString()}`" />
              </div>
            </section>
          </div>

          <section v-if="selected.about || selected.description" class="ld-section">
            <h3>{{ selected.type === "lead" ? "About" : "Lead brief" }}</h3>
            <p class="ld-text">{{ selected.about || selected.description }}</p>
          </section>

          <section v-if="selected.experience && selected.experience.length" class="ld-section">
            <h3>Experience</h3>
            <ul class="ld-rows">
              <li v-for="(entry, index) in selected.experience" :key="index" class="ld-li">{{ entry }}</li>
            </ul>
          </section>

          <section v-if="selected.type === 'lead'" class="ld-section">
            <h3>Notes</h3>
            <p v-if="selected.notes" class="ld-text">{{ selected.notes }}</p>
            <p v-else class="ld-text ld-dash">No notes. Add them in the web app.</p>
          </section>

          <section v-if="selected.company_details" class="ld-section">
            <h3>Company details</h3>
            <p class="ld-text">{{ selected.company_details }}</p>
          </section>

          <section v-if="selected.contacts && selected.contacts.length" class="ld-section">
            <h3>Key people</h3>
            <ul class="ld-rows">
              <li v-for="(person, index) in selected.contacts" :key="index" class="ld-person">
                <span class="ld-person-name">{{ person.name }}</span>
                <span v-if="person.role" class="ld-person-role">{{ person.role }}</span>
                <span class="ld-person-contact">
                  <a v-if="person.email" class="ld-link" href="#" @click.prevent="openUrl(`mailto:${person.email}`)">{{ person.email }}</a>
                  <span v-if="person.phone">{{ person.phone }}</span>
                  <a v-if="person.linkedin_url" class="ld-link" href="#" @click.prevent="openUrl(externalUrl(person.linkedin_url))">LinkedIn</a>
                </span>
              </li>
            </ul>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>
