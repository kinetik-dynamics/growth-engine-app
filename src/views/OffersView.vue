<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, errorMessage, ideaCountFor, type Offer, type OfferIdeaCount } from "../api";

const offers = ref<Offer[] | null>(null);
const ideaCounts = ref<OfferIdeaCount[]>([]);
const error = ref("");

async function load() {
  error.value = "";
  offers.value = null;
  try {
    [offers.value, ideaCounts.value] = await Promise.all([api.listOffers(), api.offerIdeaCounts()]);
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

onMounted(load);
</script>

<template>
  <header class="page-head">
    <h1>Offers <span v-if="offers" class="muted">({{ offers.length }})</span></h1>
    <button class="btn" @click="load">Refresh</button>
  </header>

  <div class="page-body">
    <div v-if="error" class="alert alert-error" role="alert">
      {{ error }}
      <div style="margin-top: 8px"><button class="btn btn-sm" @click="load">Try again</button></div>
    </div>

    <div v-else-if="!offers" class="grid" aria-busy="true">
      <div v-for="n in 3" :key="n" class="skeleton" style="height: 180px"></div>
    </div>

    <div v-else-if="!offers.length" class="card empty">
      <h2>No offers yet</h2>
      <p class="muted">Offers come from the Brain in the Growth Engine web app. Add one there, then refresh.</p>
    </div>

    <div v-else class="grid">
      <article v-for="offer in offers" :key="offer.id" class="card stack">
        <div>
          <div class="row-between" style="align-items: flex-start">
            <h2>{{ offer.name }}</h2>
            <span
              class="badge"
              :class="{ 'badge-success': ideaCountFor(offer, ideaCounts) > 0 }"
              :title="`Ideas generated from this offer: ${ideaCountFor(offer, ideaCounts)}`"
            >
              {{ ideaCountFor(offer, ideaCounts) }} {{ ideaCountFor(offer, ideaCounts) === 1 ? "idea" : "ideas" }}
            </span>
          </div>
          <p class="muted">{{ offer.promise }}</p>
        </div>
        <p v-if="offer.description" class="small">{{ offer.description }}</p>
        <div class="row" style="flex-wrap: wrap; gap: 6px">
          <span v-if="offer.price_range" class="badge">{{ offer.price_range }}</span>
          <span v-if="offer.timeline" class="badge">{{ offer.timeline }}</span>
          <span v-for="icp in offer.icps" :key="icp.id" class="badge badge-primary">{{ icp.name }}</span>
        </div>
        <ul v-if="offer.deliverables.length" class="small muted" style="margin: 0; padding-left: 18px">
          <li v-for="item in offer.deliverables" :key="item">{{ item }}</li>
        </ul>
        <p v-if="offer.case_studies.length" class="small muted">
          {{ offer.case_studies.length }} case {{ offer.case_studies.length === 1 ? "study" : "studies" }}
        </p>
      </article>
    </div>
  </div>
</template>
