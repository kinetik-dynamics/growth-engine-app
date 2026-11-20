<script setup lang="ts" generic="K extends keyof SitePages">
import { onMounted, ref } from "vue";
import { api, errorMessage, type SitePages } from "../api";

const props = defineProps<{ page: K; title: string }>();

const data = ref<SitePages[K] | null>(null);
const error = ref("");

async function load() {
  error.value = "";
  data.value = null;
  try {
    data.value = await api.sitePage(props.page);
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

onMounted(load);
</script>

<template>
  <header class="page-head">
    <div>
      <h1>{{ title }} <slot name="count" v-if="data" :data="data" /></h1>
      <p class="muted small">Read-only view from the Growth Engine web app.</p>
    </div>
    <button class="btn" @click="load()">Refresh</button>
  </header>

  <div class="page-body">
    <div v-if="error" class="alert alert-error" role="alert">
      {{ error }}
      <div style="margin-top: 8px"><button class="btn btn-sm" @click="load()">Try again</button></div>
    </div>

    <div v-else-if="!data" class="grid" aria-busy="true">
      <div v-for="n in 3" :key="n" class="skeleton" style="height: 140px"></div>
    </div>

    <slot v-else :data="data" />
  </div>
</template>
