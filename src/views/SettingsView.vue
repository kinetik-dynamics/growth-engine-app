<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { api, errorMessage, type CliStatus, type ImageApiStatus, type Permission, type Session } from "../api";
import {
  checkForUpdate,
  installUpdate,
  updateAvailable,
  updateChecking,
  updateError,
  updateInstalling,
  updateProgress,
} from "../updater";
import Icon from "../components/Icon.vue";
import SkillsPanel from "../components/SkillsPanel.vue";
import ImageApiPanel from "../components/ImageApiPanel.vue";
import type { IconName } from "../icons";
import { settingsSection, type SettingsSection } from "../settingsNav";

type Tab = SettingsSection;

defineProps<{ session: Session }>();

const tab = ref<Tab>(settingsSection.value ?? (updateAvailable.value ? "updates" : "skills"));
settingsSection.value = null;
const appVersion = ref("");

/* Idea skills and image API: their panels report back for the side panel */
const skillCount = ref<number | null>(null);
const skillDirty = ref(false);
const imageApi = ref<ImageApiStatus | null>(null);

/* AI agents */
const clis = ref<CliStatus[] | null>(null);
const cliError = ref("");
const tests = reactive<Record<string, { state: "running" | "ok" | "failed"; detail: string }>>({});
const connectedCount = computed(() => (clis.value ?? []).filter((c) => c.installed).length);

async function scan() {
  cliError.value = "";
  clis.value = null;
  try {
    clis.value = await api.detectClis();
  } catch (caught) {
    cliError.value = errorMessage(caught);
    clis.value = [];
  }
}

async function test(cli: CliStatus) {
  tests[cli.id] = { state: "running", detail: "" };
  try {
    const reply = await api.testCli(cli.id);
    tests[cli.id] = { state: "ok", detail: `Replied: ${reply.slice(0, 80)}` };
  } catch (caught) {
    tests[cli.id] = { state: "failed", detail: errorMessage(caught) };
  }
}

/* Tool permissions granted to agents from the planner */
const permissions = ref<Permission[]>([]);

function permissionsFor(cli: CliStatus) {
  return permissions.value.filter((p) => p.provider === cli.id);
}

async function loadPermissions() {
  try {
    permissions.value = await api.listPermissions();
  } catch (caught) {
    cliError.value = errorMessage(caught);
  }
}

async function revoke(permission: Permission) {
  try {
    await api.revokePermission(permission.provider, permission.tool);
    await loadPermissions();
  } catch (caught) {
    cliError.value = errorMessage(caught);
  }
}

/* Side navigation */
const channelTokens = ref<Record<string, string>>({});
const tokenInputs = reactive<Record<string, string>>({
  linkedin_personal: "",
  facebook_page: "",
  x_profile: "",
});
const savingTokens = reactive<Record<string, boolean>>({});
const tokenMessages = reactive<Record<string, string>>({});

async function loadChannelTokens() {
  try {
    channelTokens.value = await api.getChannelTokens();
    for (const [ch, val] of Object.entries(channelTokens.value)) {
      tokenInputs[ch] = val;
    }
  } catch {}
}

async function saveToken(channelId: string) {
  savingTokens[channelId] = true;
  tokenMessages[channelId] = "";
  try {
    await api.saveChannelToken(channelId, tokenInputs[channelId] || "");
    await loadChannelTokens();
    tokenMessages[channelId] = "Saved!";
    setTimeout(() => (tokenMessages[channelId] = ""), 2500);
  } catch (caught) {
    tokenMessages[channelId] = errorMessage(caught);
  } finally {
    savingTokens[channelId] = false;
  }
}

const NAV = computed<{ id: Tab; label: string; icon: IconName; sub: string; dot?: "ok" | "warn" | "info" }[]>(() => [
  {
    id: "skills",
    label: "Idea skills",
    icon: "fileText",
    sub: skillDirty.value
      ? "Unsaved changes"
      : skillCount.value === null
        ? "Instructions for plans"
        : `${skillCount.value} ${skillCount.value === 1 ? "skill" : "skills"}`,
    dot: skillDirty.value ? "warn" : undefined,
  },
  {
    id: "agents",
    label: "AI agents",
    icon: "bot",
    sub: !clis.value ? "Scanning…" : connectedCount.value ? `${connectedCount.value} connected` : "None found",
    dot: clis.value ? (connectedCount.value ? "ok" : "warn") : undefined,
  },
  {
    id: "images",
    label: "Image API",
    icon: "sparkles",
    sub: !imageApi.value ? "Cover images" : imageApi.value.configured ? imageApi.value.model : "Not set",
    dot: imageApi.value?.configured ? "ok" : undefined,
  },
  {
    id: "tokens",
    label: "Channel tokens",
    icon: "link",
    sub: Object.keys(channelTokens.value).length ? `${Object.keys(channelTokens.value).length} configured` : "Social media APIs",
    dot: Object.keys(channelTokens.value).length ? "ok" : undefined,
  },
  {
    id: "updates",
    label: "Updates",
    icon: "download",
    sub: updateAvailable.value ? `v${updateAvailable.value.version} available` : appVersion.value ? `v${appVersion.value}` : "App version",
    dot: updateAvailable.value ? "info" : undefined,
  },
]);

onMounted(async () => {
  scan();
  loadPermissions();
  loadChannelTokens();
  api.getImageApi().then((status) => (imageApi.value = status)).catch(() => {});
  api.listSkills().then((skills) => (skillCount.value = skills.length)).catch(() => {});
  try {
    appVersion.value = await getVersion();
  } catch {
    appVersion.value = "";
  }
});
</script>

<template>
  <div class="st">
    <div class="pub-header">
      <div class="pub-header-left">
        <h1 class="pub-title">Settings</h1>
      </div>
      <span class="st-server" :title="session.baseUrl">
        <span class="st-dot ok"></span>
        <span>Connected to <b>{{ session.baseUrl }}</b></span>
      </span>
    </div>

    <div class="pub-layout">
      <aside class="pub-channels" aria-label="Settings sections">
        <div class="pub-channels-head"><span>Sections</span></div>
        <nav class="pub-channels-list">
          <button
            v-for="item in NAV"
            :key="item.id"
            class="st-nav"
            :class="{ active: tab === item.id }"
            :aria-current="tab === item.id ? 'page' : undefined"
            @click="tab = item.id"
          >
            <span class="st-nav-icon"><Icon :name="item.icon" :size="14" /></span>
            <span class="st-nav-text">
              <span class="st-nav-name">
                <span>{{ item.label }}</span>
                <span v-if="item.dot" class="st-dot" :class="item.dot"></span>
              </span>
              <span class="st-nav-sub">{{ item.sub }}</span>
            </span>
          </button>
        </nav>
        <div class="pub-channels-foot st-foot">
          <span>Growth Engine</span>
          <span v-if="appVersion" class="st-version">v{{ appVersion }}</span>
        </div>
      </aside>

      <main class="pub-main">
        <!-- Idea skills: kept mounted so an open edit survives switching sections -->
        <SkillsPanel
          v-show="tab === 'skills'"
          :website-url="session.websiteUrl"
          @dirty="skillDirty = $event"
          @count="skillCount = $event"
        />

        <!-- Image API -->
        <ImageApiPanel v-if="tab === 'images'" @status="imageApi = $event" />

        <!-- AI agents -->
        <div v-if="tab === 'agents'" class="pub-section">
          <div class="pub-section-head st-section-head">
            <div>
              <h2>AI agents{{ clis ? ` (${connectedCount} connected)` : "" }}</h2>
              <p>Ideas are written by AI agent CLIs installed on this computer. The app finds them on your PATH.</p>
            </div>
            <button class="st-btn" :disabled="!clis" @click="scan">
              <span v-if="!clis" class="spinner"></span>
              <Icon v-else name="refresh" :size="14" />
              {{ clis ? "Scan again" : "Scanning…" }}
            </button>
          </div>

          <div v-if="cliError" class="alert alert-error" role="alert">{{ cliError }}</div>

          <div v-if="!clis" class="pub-list" aria-busy="true">
            <div v-for="n in 3" :key="n" class="skeleton" style="height: 96px"></div>
          </div>

          <div v-else-if="!clis.length && !cliError" class="pub-empty">
            <Icon name="terminal" :size="32" />
            <p class="pub-empty-title">No supported agents</p>
            <p>Install an AI agent CLI, sign in, then scan again.</p>
          </div>

          <div v-else class="pub-list">
            <div v-for="cli in clis" :key="cli.id" class="st-card" :class="{ muted: !cli.installed }">
              <div class="st-card-head">
                <div class="st-card-title">
                  <span class="st-card-icon"><Icon name="terminal" :size="16" /></span>
                  <div class="st-card-meta">
                    <div class="st-card-name">
                      <span>{{ cli.name }}</span>
                      <span class="st-pill" :class="cli.installed ? 'ok' : 'off'">
                        <span class="st-dot" :class="cli.installed ? 'ok' : 'off'"></span>
                        {{ cli.installed ? "Connected" : "Not installed" }}
                      </span>
                      <span v-if="cli.version" class="st-card-version">{{ cli.version }}</span>
                    </div>
                    <div class="st-card-path" :title="cli.path ?? undefined">
                      {{ cli.installed ? cli.path : `No "${cli.command}" command found on PATH. Install it, sign in, then scan again.` }}
                    </div>
                  </div>
                </div>
                <button v-if="cli.installed" class="st-btn" :disabled="tests[cli.id]?.state === 'running'" @click="test(cli)">
                  <span v-if="tests[cli.id]?.state === 'running'" class="spinner"></span>
                  <Icon v-else name="play" :size="12" />
                  {{ tests[cli.id]?.state === "running" ? "Testing…" : "Test connection" }}
                </button>
              </div>

              <div v-if="cli.installed" class="st-card-foot">
                <span class="st-label">Allowed tools</span>
                <span v-if="!permissionsFor(cli).length" class="st-hint">
                  None. If {{ cli.name }} needs a tool while planning, the planner asks you to allow it.
                </span>
                <span v-for="permission in permissionsFor(cli)" :key="permission.tool" class="st-tool">
                  <code>{{ permission.tool }}</code>
                  <button class="st-tool-x" :title="`Revoke ${permission.tool}`" :aria-label="`Revoke ${permission.tool} for ${cli.name}`" @click="revoke(permission)">
                    <Icon name="x" :size="12" />
                  </button>
                </span>
              </div>

              <div v-if="tests[cli.id]?.state === 'ok'" class="st-result ok" role="status">
                <Icon name="checkCircle" :size="14" /> Working. {{ tests[cli.id].detail }}
              </div>
              <div v-if="tests[cli.id]?.state === 'failed'" class="st-result bad" role="alert">
                <Icon name="alertCircle" :size="14" /> {{ tests[cli.id].detail }}
              </div>
            </div>
          </div>
        </div>

        <!-- Channel Tokens -->
        <div v-else-if="tab === 'tokens'" class="pub-section st-narrow">
          <div class="pub-section-head st-section-head">
            <div>
              <h2>Channel tokens</h2>
              <p>Configure API access tokens for social channels to fetch live post analytics.</p>
            </div>
          </div>

          <!-- LinkedIn Personal -->
          <div class="st-card" style="margin-bottom: 16px">
            <div class="st-card-head">
              <div class="st-card-title">
                <span class="st-card-icon" style="background: #eff6ff; color: #0a66c2"><Icon name="link" :size="16" /></span>
                <div class="st-card-meta">
                  <div class="st-card-name">
                    <span>LinkedIn (Personal Profile)</span>
                    <span v-if="channelTokens.linkedin_personal" class="st-pill ok">Configured</span>
                    <span v-else class="st-pill">Not set</span>
                  </div>
                  <div class="st-card-path">Used for socialActions (likes, comments) and memberCreatorPostAnalytics</div>
                </div>
              </div>
            </div>
            <div class="st-card-foot" style="flex-direction: column; align-items: stretch; gap: 8px">
              <div style="display: flex; gap: 8px">
                <input
                  v-model="tokenInputs.linkedin_personal"
                  type="password"
                  class="input"
                  style="flex: 1"
                  placeholder="Paste LinkedIn Access Token (AQ...)"
                />
                <button
                  class="btn btn-sm btn-primary"
                  :disabled="savingTokens.linkedin_personal"
                  @click="saveToken('linkedin_personal')"
                >
                  {{ savingTokens.linkedin_personal ? "Saving…" : "Save" }}
                </button>
              </div>
              <span v-if="tokenMessages.linkedin_personal" class="st-hint" style="color: #059669">
                {{ tokenMessages.linkedin_personal }}
              </span>
            </div>
          </div>

          <!-- Facebook Page -->
          <div class="st-card" style="margin-bottom: 16px">
            <div class="st-card-head">
              <div class="st-card-title">
                <span class="st-card-icon" style="background: #eff6ff; color: #1877f2"><Icon name="link" :size="16" /></span>
                <div class="st-card-meta">
                  <div class="st-card-name">
                    <span>Facebook (Page)</span>
                    <span v-if="channelTokens.facebook_page" class="st-pill ok">Configured</span>
                    <span v-else class="st-pill">Not set</span>
                  </div>
                  <div class="st-card-path">Used for Graph API post insights (impressions, views) and reactions/comments</div>
                </div>
              </div>
            </div>
            <div class="st-card-foot" style="flex-direction: column; align-items: stretch; gap: 8px">
              <div style="display: flex; gap: 8px">
                <input
                  v-model="tokenInputs.facebook_page"
                  type="password"
                  class="input"
                  style="flex: 1"
                  placeholder="Paste Facebook Page Access Token (EA...)"
                />
                <button
                  class="btn btn-sm btn-primary"
                  :disabled="savingTokens.facebook_page"
                  @click="saveToken('facebook_page')"
                >
                  {{ savingTokens.facebook_page ? "Saving…" : "Save" }}
                </button>
              </div>
              <span v-if="tokenMessages.facebook_page" class="st-hint" style="color: #059669">
                {{ tokenMessages.facebook_page }}
              </span>
            </div>
          </div>

          <!-- X Profile -->
          <div class="st-card">
            <div class="st-card-head">
              <div class="st-card-title">
                <span class="st-card-icon" style="background: #f3f4f6; color: #111827"><Icon name="link" :size="16" /></span>
                <div class="st-card-meta">
                  <div class="st-card-name">
                    <span>X (Twitter Profile)</span>
                    <span v-if="channelTokens.x_profile" class="st-pill ok">Configured</span>
                    <span v-else class="st-pill">Not set</span>
                  </div>
                  <div class="st-card-path">Used for X API v2 public_metrics (impressions, likes, replies)</div>
                </div>
              </div>
            </div>
            <div class="st-card-foot" style="flex-direction: column; align-items: stretch; gap: 8px">
              <div style="display: flex; gap: 8px">
                <input
                  v-model="tokenInputs.x_profile"
                  type="password"
                  class="input"
                  style="flex: 1"
                  placeholder="Paste X Bearer Token (AAAA...)"
                />
                <button
                  class="btn btn-sm btn-primary"
                  :disabled="savingTokens.x_profile"
                  @click="saveToken('x_profile')"
                >
                  {{ savingTokens.x_profile ? "Saving…" : "Save" }}
                </button>
              </div>
              <span v-if="tokenMessages.x_profile" class="st-hint" style="color: #059669">
                {{ tokenMessages.x_profile }}
              </span>
            </div>
          </div>
        </div>

        <!-- Updates -->
        <div v-else-if="tab === 'updates'" class="pub-section st-narrow">
          <div class="pub-section-head st-section-head">
            <div>
              <h2>App updates</h2>
              <p>Growth Engine checks for updates automatically on launch.</p>
            </div>
            <button class="st-btn" :disabled="updateChecking || updateInstalling" @click="checkForUpdate">
              <span v-if="updateChecking" class="spinner"></span>
              <Icon v-else name="refresh" :size="14" />
              {{ updateChecking ? "Checking…" : "Check now" }}
            </button>
          </div>

          <div class="st-card">
            <div class="st-card-head">
              <div class="st-card-title">
                <span class="st-card-icon" :class="{ info: updateAvailable }"><Icon :name="updateAvailable ? 'download' : 'checkCircle'" :size="16" /></span>
                <div class="st-card-meta">
                  <div class="st-card-name">
                    <span>{{ updateAvailable ? `Version ${updateAvailable.version} is available` : updateChecking ? "Checking for updates…" : "You're up to date" }}</span>
                    <span v-if="updateAvailable" class="st-pill info">New</span>
                  </div>
                  <div class="st-card-path">Installed version {{ appVersion ? `v${appVersion}` : "—" }}</div>
                </div>
              </div>
              <button v-if="updateAvailable && !updateInstalling" class="btn btn-primary btn-sm" @click="installUpdate">
                Install v{{ updateAvailable.version }}
              </button>
            </div>

            <div v-if="updateAvailable?.body" class="st-notes">
              <span class="st-label">What's new</span>
              <div class="update-notes">{{ updateAvailable.body }}</div>
            </div>

            <div v-if="updateInstalling" class="st-progress">
              <div class="st-progress-row">
                <span>Downloading update…</span>
                <span>{{ updateProgress }}%</span>
              </div>
              <div class="progress-bar"><div class="progress-fill" :style="{ width: updateProgress + '%' }"></div></div>
              <p>The app will restart automatically when the update is ready.</p>
            </div>
          </div>

          <div v-if="updateError" class="alert alert-error" role="alert">{{ updateError }}</div>
        </div>
      </main>
    </div>
  </div>
</template>
