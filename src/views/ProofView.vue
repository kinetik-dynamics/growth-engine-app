<script setup lang="ts">
import SitePage from "../components/SitePage.vue";
</script>

<template>
  <SitePage page="proof" title="Proof">
    <template #count="{ data }"><span class="muted">({{ data.proof.length }})</span></template>
    <template #default="{ data: { proof } }">
      <div v-if="!proof.length" class="card empty">
        <h2>No case studies yet</h2>
        <p class="muted">Case studies come from the Brain in the Growth Engine web app.</p>
      </div>

      <div v-else class="grid">
        <article v-for="item in proof" :key="item.id" class="card stack">
          <div>
            <h2>{{ item.title }}</h2>
            <p class="muted small">
              {{ [item.can_name_client ? item.client_name : "Anonymous client", item.client_type, item.industry].filter(Boolean).join(" · ") }}
            </p>
          </div>
          <p class="small">{{ item.result }}</p>
          <div v-if="item.offer"><span class="badge badge-primary">{{ item.offer.name }}</span></div>
        </article>
      </div>
    </template>
  </SitePage>
</template>
