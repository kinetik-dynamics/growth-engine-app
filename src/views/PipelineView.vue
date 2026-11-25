<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, errorMessage, type PipelineCard, type PipelineStage } from "../api";
import CompanyAvatar from "../components/CompanyAvatar.vue";
import Icon from "../components/Icon.vue";
import RefreshControls from "../components/RefreshControls.vue";
import { createAutoRefresh } from "../autoRefresh";

type TypeFilter = "all" | "lead" | "prospect";

const stages = ref<PipelineStage[] | null>(null);
const cards = ref<PipelineCard[]>([]);
const error = ref("");
const busy = ref(false);

const search = ref("");
const typeFilter = ref<TypeFilter>("all");
const icpFilter = ref("all");

async function load() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await api.sitePage("pipeline");
    stages.value = result.stages;
    cards.value = result.cards;
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = false;
    autoRefresh.restart();
  }
}

const autoRefresh = createAutoRefresh("pipeline.autoRefresh", () => load(), () => busy.value);
const { enabled: autoRefreshOn, countdown } = autoRefresh;

const hasBothTypes = computed(() => cards.value.some((card) => card.type === "lead") && cards.value.some((card) => card.type === "prospect"));
const icpNames = computed(() => [...new Set(cards.value.filter((card) => card.icp).map((card) => card.icp!.name))].sort());

const visibleCards = computed(() => {
  const needle = search.value.trim().toLowerCase();
  return cards.value.filter((card) => {
    if (typeFilter.value !== "all" && card.type !== typeFilter.value) return false;
    if (icpFilter.value === "none" ? card.icp !== null : icpFilter.value !== "all" && card.icp?.name !== icpFilter.value) return false;
    if (!needle) return true;
    return [card.name, card.company_name, card.contact_title, card.email, card.location].some((field) => field?.toLowerCase().includes(needle));
  });
});

const filtersActive = computed(() => search.value !== "" || typeFilter.value !== "all" || icpFilter.value !== "all");

function inStage(list: PipelineCard[], stage: PipelineStage) {
  return list.filter((card) => card.stage_id === stage.id);
}

function subtitle(card: PipelineCard) {
  if (card.type === "prospect") return card.company_domain || "Company";
  return [card.contact_title, card.company_name].filter(Boolean).join(" at ") || "No role details yet";
}

function clearFilters() {
  search.value = "";
  typeFilter.value = "all";
  icpFilter.value = "all";
}

onMounted(() => {
  load();
  autoRefresh.start();
});
onUnmounted(() => autoRefresh.stop());
</script>

<template>
  <div class="lp-page">
    <div class="pub-header">
      <div class="pub-header-left"><h1 class="pub-title">Lead Pipeline</h1></div>
      <RefreshControls v-model="autoRefreshOn" :countdown="countdown" :refreshing="busy" size="sm" @refresh="load()" />
    </div>

    <div v-if="error" class="lq-state">
      <div class="alert alert-error" role="alert">
        {{ error }}
        <div style="margin-top: 8px"><button class="btn btn-sm" :disabled="busy" @click="load()">Try again</button></div>
      </div>
    </div>

    <div v-else-if="!stages" class="lq-state" aria-busy="true">
      <div class="skeleton" style="height: 420px"></div>
    </div>

    <div v-else-if="!stages.length" class="lp-setup">
      <h2>Set up your business profile first</h2>
      <p>The pipeline tracks leads for your business. Finish onboarding in the web app, then import leads and they will show up here.</p>
    </div>

    <template v-else>
      <div class="lp-toolbar">
        <div class="lq-search lp-search">
          <Icon name="search" :size="14" />
          <input v-model="search" type="search" placeholder="Search name, company, title…" aria-label="Search the pipeline" />
        </div>
        <select v-if="hasBothTypes" v-model="typeFilter" class="lp-select" aria-label="Filter by type">
          <option value="all">People and companies</option>
          <option value="lead">People only</option>
          <option value="prospect">Companies only</option>
        </select>
        <select v-if="icpNames.length" v-model="icpFilter" class="lp-select" aria-label="Filter by ICP">
          <option value="all">All ICPs</option>
          <option value="none">No ICP</option>
          <option v-for="name in icpNames" :key="name" :value="name">{{ name }}</option>
        </select>
        <button v-if="filtersActive" type="button" class="lp-clear" @click="clearFilters"><Icon name="x" :size="14" /> Clear filters</button>
        <p class="lp-count">
          {{ filtersActive ? `${visibleCards.length} of ${cards.length} leads` : `${cards.length} ${cards.length === 1 ? "lead" : "leads"}` }}
          · {{ stages.length }} {{ stages.length === 1 ? "stage" : "stages" }}
        </p>
      </div>

      <div class="lp-board">
        <section
          v-for="stage in stages"
          :key="stage.id"
          class="lp-col"
          :aria-label="`${stage.name}, ${inStage(cards, stage).length} cards`"
        >
          <header class="lp-col-head">
            <span class="lp-dot" :class="`lp-dot-${stage.color}`" aria-hidden="true"></span>
            <h3 class="lp-col-name" :title="stage.name">{{ stage.name }}</h3>
            <span class="lp-col-count">
              {{ filtersActive && inStage(visibleCards, stage).length !== inStage(cards, stage).length ? `${inStage(visibleCards, stage).length}/${inStage(cards, stage).length}` : inStage(cards, stage).length }}
            </span>
            <span v-if="stage.is_default" class="lp-col-default" title="New leads start in this stage">Default</span>
          </header>

          <div v-if="!inStage(visibleCards, stage).length" class="lp-col-empty">
            <template v-if="filtersActive && inStage(cards, stage).length">
              <p>No matching leads here.</p>
              <button type="button" class="lq-linkbtn" @click="clearFilters">Clear filters</button>
            </template>
            <p v-else>No leads here</p>
          </div>

          <ul v-else class="lp-cards">
            <li v-for="card in inStage(visibleCards, stage)" :key="card.key">
              <article class="lp-card">
                <div class="lp-card-top">
                  <CompanyAvatar :label="card.name || '?'" :image-url="card.avatar_url" :domain="card.company_domain" :round="card.type === 'lead'" />
                  <div class="lp-card-main">
                    <h4 class="lp-card-name" :title="card.name ?? undefined">{{ card.name }}</h4>
                    <p class="lp-card-sub" :title="subtitle(card)">{{ subtitle(card) }}</p>
                  </div>
                </div>
                <div class="lp-card-foot">
                  <span v-if="card.type === 'prospect'" class="lq-tag lq-tag-gray lp-company"><Icon name="building" :size="12" /> Company</span>
                  <span v-if="card.icp" class="lq-tag lq-tag-blue" :title="`ICP: ${card.icp.name}`">{{ card.icp.name }}</span>
                  <span v-if="card.location" class="lp-loc" :title="card.location">
                    <Icon name="mapPin" :size="12" />
                    <span>{{ card.location }}</span>
                  </span>
                  <span v-if="card.type === 'lead' && !card.icp && !card.location" class="lp-src">{{ card.source_label }}</span>
                  <span class="lq-row-icons">
                    <span v-if="card.email" :title="card.email"><Icon name="mail" :size="12" style="color: #059669" /></span>
                    <span v-if="card.phone" :title="card.phone"><Icon name="phone" :size="12" style="color: #059669" /></span>
                  </span>
                </div>
              </article>
            </li>
          </ul>
        </section>
      </div>
    </template>
  </div>
</template>
