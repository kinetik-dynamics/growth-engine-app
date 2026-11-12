<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, errorMessage, type Session } from "./api";
import LoginView from "./views/LoginView.vue";
import OffersView from "./views/OffersView.vue";
import IdeasView from "./views/IdeasView.vue";
import SettingsView from "./views/SettingsView.vue";

type Page = "offers" | "ideas" | "settings";

const NAV: { id: Page; label: string }[] = [
  { id: "offers", label: "Offers" },
  { id: "ideas", label: "Ideas" },
  { id: "settings", label: "Settings" },
];

const loading = ref(true);
const startupError = ref("");
const session = ref<Session | null>(null);
const page = ref<Page>("ideas");

onMounted(async () => {
  try {
    session.value = await api.getSession();
  } catch (error) {
    startupError.value = errorMessage(error);
  } finally {
    loading.value = false;
  }
});

async function signOut() {
  try {
    await api.logout();
    session.value = null;
    page.value = "ideas";
  } catch (error) {
    startupError.value = errorMessage(error);
  }
}
</script>

<template>
  <div v-if="loading" class="login"><span class="spinner" aria-label="Loading"></span></div>

  <LoginView v-else-if="!session" :notice="startupError" @signed-in="session = $event" />

  <div v-else class="shell">
    <aside class="sidebar">
      <div class="brand"><span class="brand-mark">GE</span> Growth Engine</div>
      <button
        v-for="item in NAV"
        :key="item.id"
        class="nav-item"
        :class="{ active: page === item.id }"
        :aria-current="page === item.id ? 'page' : undefined"
        @click="page = item.id"
      >
        {{ item.label }}
      </button>

      <div class="sidebar-foot">
        <div class="who">
          <div class="name">{{ session.companyName || session.userName }}</div>
          <div class="muted small">{{ session.userEmail }}</div>
        </div>
        <button class="btn btn-sm" @click="signOut">Sign out</button>
      </div>
    </aside>

    <main class="main">
      <OffersView v-if="page === 'offers'" />
      <IdeasView v-else-if="page === 'ideas'" :session="session" @open-settings="page = 'settings'" />
      <SettingsView v-else :session="session" />
    </main>
  </div>
</template>
