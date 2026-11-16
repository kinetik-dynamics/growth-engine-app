<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, errorMessage, ideaCountFor, type Offer, type OfferIdeaCount } from "../api";
import Pagination from "../components/Pagination.vue";

const offers = ref<Offer[] | null>(null);
const ideaCounts = ref<OfferIdeaCount[]>([]);
const error = ref("");
const page = ref(1);
const lastPage = ref(1);
const total = ref(0);

/** Loads one page of offers; the server sends the newest first. */
async function load(target = page.value) {
  error.value = "";
  offers.value = null;
  try {
    const [result, counts] = await Promise.all([api.listOffers(target), api.offerIdeaCounts()]);
    // An offer deleted on the site can leave this page past the end.
    if (!result.offers.length && result.meta.current_page > result.meta.last_page) {
      return load(result.meta.last_page);
    }
    offers.value = result.offers;
    ideaCounts.value = counts;
    page.value = result.meta.current_page;
    lastPage.value = result.meta.last_page;
    total.value = result.meta.total;
    document.querySelector(".main")?.scrollTo({ top: 0 });
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

onMounted(() => load());
</script>

<template>
  <header class="page-head">
    <h1>Offers <span v-if="offers" class="muted">({{ total }})</span></h1>
    <button class="btn" @click="load()">Refresh</button>
  </header>

  <div class="page-body">
    <div v-if="error" class="alert alert-error" role="alert">
      {{ error }}
      <div style="margin-top: 8px"><button class="btn btn-sm" @click="load()">Try again</button></div>
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

    <Pagination v-if="offers" :page="page" :last-page="lastPage" :total="total" :noun="['offer', 'offers']" @change="load" />
  </div>
</template>
