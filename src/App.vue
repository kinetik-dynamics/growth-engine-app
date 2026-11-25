<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { api, errorMessage, type Session } from "./api";
import LoginView from "./views/LoginView.vue";
import OffersView from "./views/OffersView.vue";
import IdeasView from "./views/IdeasView.vue";
import SettingsView from "./views/SettingsView.vue";
import OverviewView from "./views/OverviewView.vue";
import IcpsView from "./views/IcpsView.vue";
import ProofView from "./views/ProofView.vue";
import PublishView from "./views/PublishView.vue";
import QueueView from "./views/QueueView.vue";
import PipelineView from "./views/PipelineView.vue";
import Icon from "./components/Icon.vue";
import type { IconName } from "./icons";
import { checkForUpdate, updateAvailable, updateChecking, updateInstalling } from "./updater";
import { startOverviewTimer, stopOverviewTimer } from "./overviewStore";

type Page = "overview" | "offers" | "icps" | "proof" | "queue" | "pipeline" | "ideas" | "publish" | "settings";

const PAGE_ICONS: Record<Page, IconName> = {
  overview: "layoutDashboard",
  offers: "layers",
  icps: "target",
  proof: "award",
  queue: "users",
  pipeline: "layers",
  ideas: "lightbulb",
  publish: "calendar",
  settings: "settings",
};

const NAV: { title: string | null; items: { id: Page; label: string }[] }[] = [
  { title: null, items: [{ id: "overview", label: "Overview" }] },
  { title: "Brain", items: [{ id: "offers", label: "Offers" }, { id: "icps", label: "ICPs" }, { id: "proof", label: "Proof" }] },
  { title: "Leads", items: [{ id: "queue", label: "Queue" }, { id: "pipeline", label: "Pipeline" }] },
  { title: "Content", items: [{ id: "ideas", label: "Ideas" }, { id: "publish", label: "Publish" }] },
  { title: null, items: [{ id: "settings", label: "Settings" }] },
];

const loading = ref(true);
const startupError = ref("");
const session = ref<Session | null>(null);
const page = ref<Page>("overview");
const appVersion = ref("");

watch(session, (current) => (current ? startOverviewTimer() : stopOverviewTimer()));

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
    page.value = "overview";
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
      <div class="brand">
        <img src="/logo-sm.png" alt="Growth Engine icon" class="brand-icon" />
        <div class="brand-text">
          <span class="brand-name">Kinetik Dynamics</span>
          <span class="brand-sub">Growth Engine</span>
        </div>
      </div>
      <nav class="nav">
        <div v-for="(group, index) in NAV" :key="index" class="nav-group" :class="{ 'nav-group-last': index === NAV.length - 1 }">
          <div v-if="group.title" class="nav-title">{{ group.title }}</div>
          <button
            v-for="item in group.items"
            :key="item.id"
            class="nav-item"
            :class="{ active: page === item.id }"
            :aria-current="page === item.id ? 'page' : undefined"
            @click="page = item.id"
          >
            <Icon class="nav-icon" :name="PAGE_ICONS[item.id]" :stroke="1.5" />
            <span>{{ item.label }}</span>
          </button>
        </div>
      </nav>

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
      <OverviewView v-if="page === 'overview'" :session="session" @navigate="page = $event" />
      <OffersView v-else-if="page === 'offers'" :session="session" @open-settings="page = 'settings'" />
      <IcpsView v-else-if="page === 'icps'" :session="session" />
      <ProofView v-else-if="page === 'proof'" />
      <QueueView v-else-if="page === 'queue'" @open-settings="page = 'settings'" />
      <PipelineView v-else-if="page === 'pipeline'" :session="session" />
      <IdeasView v-else-if="page === 'ideas'" :session="session" @open-settings="page = 'settings'" />
      <PublishView v-else-if="page === 'publish'" :session="session" />
      <SettingsView v-else :session="session" />
    </main>
  </div>
</template>
