<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { api, errorMessage, type CliStatus, type Permission, type Session } from "../api";
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
import type { IconName } from "../icons";

type Tab = "agents" | "skill" | "updates";

defineProps<{ session: Session }>();

const tab = ref<Tab>(updateAvailable.value ? "updates" : "agents");
const appVersion = ref("");

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

/* Idea skill */
const skill = ref("");
const savedSkill = ref("");
const skillLoaded = ref(false);
const skillError = ref("");
const skillNotice = ref("");
const saving = ref(false);
const confirmingReset = ref(false);
const dirty = computed(() => skill.value !== savedSkill.value);
const skillStats = computed(() => {
  const text = skill.value;
  const words = text.trim() ? text.trim().split(/\s+/).length : 0;
  return `${text.split("\n").length} lines · ${words} words`;
});

async function loadSkill() {
  try {
    skill.value = savedSkill.value = await api.getSkill();
  } catch (caught) {
    skillError.value = errorMessage(caught);
  } finally {
    skillLoaded.value = true;
  }
}

async function saveSkill() {
  skillError.value = skillNotice.value = "";
  saving.value = true;
  try {
    await api.saveSkill(skill.value);
    savedSkill.value = skill.value;
    skillNotice.value = "Skill saved. The next plan uses it.";
  } catch (caught) {
    skillError.value = errorMessage(caught);
  } finally {
    saving.value = false;
  }
}

async function resetSkill() {
  skillError.value = skillNotice.value = "";
  confirmingReset.value = false;
  try {
    skill.value = savedSkill.value = await api.resetSkill();
    skillNotice.value = "Skill reset to the default.";
  } catch (caught) {
    skillError.value = errorMessage(caught);
  }
}

/* Side navigation */
const NAV = computed<{ id: Tab; label: string; icon: IconName; sub: string; dot?: "ok" | "warn" | "info" }[]>(() => [
  {
    id: "agents",
    label: "AI agents",
    icon: "bot",
    sub: !clis.value ? "Scanning…" : connectedCount.value ? `${connectedCount.value} connected` : "None found",
    dot: clis.value ? (connectedCount.value ? "ok" : "warn") : undefined,
  },
  {
    id: "skill",
    label: "Idea skill",
    icon: "fileText",
    sub: dirty.value ? "Unsaved changes" : "Instructions for plans",
    dot: dirty.value ? "warn" : undefined,
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
  loadSkill();
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

        <!-- Idea skill -->
        <div v-else-if="tab === 'skill'" class="pub-section">
          <div class="pub-section-head st-section-head">
            <div>
              <h2>Idea skill</h2>
              <p>These instructions are sent to the AI agent with every plan. Edit them to change how ideas are written.</p>
            </div>
          </div>

          <div class="st-editor">
            <div class="st-editor-bar">
              <label for="skill">SKILL.md</label>
              <span v-if="skillLoaded">{{ skillStats }}<template v-if="dirty"> · <b>Unsaved</b></template></span>
            </div>
            <div v-if="!skillLoaded" class="skeleton" style="height: 420px; border-radius: 0"></div>
            <textarea v-else id="skill" v-model="skill" class="st-textarea" rows="24" spellcheck="false" @input="skillNotice = ''"></textarea>
          </div>

          <div v-if="skillError" class="alert alert-error" role="alert">{{ skillError }}</div>
          <div v-if="skillNotice" class="alert alert-success" role="status">{{ skillNotice }}</div>

          <div class="st-actions">
            <div v-if="confirmingReset" class="st-confirm">
              <span>Replace your skill with the default? Your edits will be lost.</span>
              <button class="btn btn-sm btn-danger-solid" @click="resetSkill">Reset skill</button>
              <button class="btn btn-sm" @click="confirmingReset = false">Keep my skill</button>
            </div>
            <button v-else class="st-btn" @click="confirmingReset = true"><Icon name="rotateCcw" :size="14" /> Reset to default</button>
            <button class="btn btn-primary" :disabled="saving || !skillLoaded || !dirty" @click="saveSkill">
              <span v-if="saving" class="spinner"></span>
              {{ saving ? "Saving…" : dirty ? "Save skill" : "Saved" }}
            </button>
          </div>
        </div>

        <!-- Updates -->
        <div v-else class="pub-section st-narrow">
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
