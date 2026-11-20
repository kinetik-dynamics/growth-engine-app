<script setup lang="ts">
import SitePage from "../components/SitePage.vue";
</script>

<template>
  <SitePage page="icps" title="ICPs">
    <template #count="{ data }"><span class="muted">({{ data.icps.length }})</span></template>
    <template #default="{ data: { icps } }">
      <div v-if="!icps.length" class="card empty">
        <h2>No ICPs yet</h2>
        <p class="muted">ICPs come from the Brain in the Growth Engine web app.</p>
      </div>

      <div v-else class="grid">
        <article v-for="icp in icps" :key="icp.id" class="card stack">
          <div>
            <h2>{{ icp.name }}</h2>
            <p class="muted">{{ icp.who_they_are }}</p>
          </div>
          <p v-if="icp.pain" class="small"><b>Pain:</b> {{ icp.pain }}</p>
          <p v-if="icp.roles_to_contact" class="small"><b>Contact:</b> {{ icp.roles_to_contact }}</p>
          <p v-if="icp.buying_triggers" class="small"><b>Triggers:</b> {{ icp.buying_triggers }}</p>
          <div v-if="icp.offers.length" class="row" style="flex-wrap: wrap; gap: 6px">
            <span v-for="offer in icp.offers" :key="offer.id" class="badge badge-primary">{{ offer.name }}</span>
          </div>
          <p class="small muted">
            {{ icp.leads_count }} leads · {{ icp.prospects_count }} companies · {{ icp.searches_count }} searches
          </p>
        </article>
      </div>
    </template>
  </SitePage>
</template>
