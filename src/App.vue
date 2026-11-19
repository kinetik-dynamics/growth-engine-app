<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { api, errorMessage, type Session } from "./api";
import LoginView from "./views/LoginView.vue";
import OffersView from "./views/OffersView.vue";
import IdeasView from "./views/IdeasView.vue";
import SettingsView from "./views/SettingsView.vue";
import { checkForUpdate, updateAvailable, updateChecking, updateInstalling } from "./updater";

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
const appVersion = ref("");

onMounted(async () => {
  try {
    [session.value, appVersion.value] = await Promise.all([api.getSession(), getVersion()]);
  } catch (error) {
    startupError.value = errorMessage(error);
  } finally {
    loading.value = false;
  }
  checkForUpdate();
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
        <div class="version-row">
          <span class="muted small">v{{ appVersion }}</span>
          <button
            class="btn-icon"
            :class="{ 'has-update': updateAvailable }"
            :title="updateAvailable ? `Update v${updateAvailable.version} available — click to install` : 'Check for updates'"
            :disabled="updateChecking || updateInstalling"
            :aria-label="updateAvailable ? 'Update available' : 'Check for updates'"
            @click="updateAvailable ? (page = 'settings') : checkForUpdate()"
          >
            <span v-if="updateChecking || updateInstalling" class="spinner" style="width:12px;height:12px;border-width:1.5px"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/>
            </svg>
          </button>
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
