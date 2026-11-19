<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
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

defineProps<{ session: Session }>();

const tab = ref<"agents" | "skill" | "updates">(updateAvailable.value ? "updates" : "agents");

/* AI agents */
const clis = ref<CliStatus[] | null>(null);
const cliError = ref("");
const tests = reactive<Record<string, { state: "running" | "ok" | "failed"; detail: string }>>({});

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

onMounted(() => {
  scan();
  loadPermissions();
  loadSkill();
});
</script>

<template>
  <header class="page-head">
    <div class="row" style="gap: 20px">
      <h1>Settings</h1>
      <div class="tabs" role="tablist">
        <button class="tab" :class="{ active: tab === 'agents' }" role="tab" :aria-selected="tab === 'agents'" @click="tab = 'agents'">AI agents</button>
        <button class="tab" :class="{ active: tab === 'skill' }" role="tab" :aria-selected="tab === 'skill'" @click="tab = 'skill'">
          Idea skill{{ dirty ? " •" : "" }}
        </button>
        <button class="tab" :class="{ active: tab === 'updates' }" role="tab" :aria-selected="tab === 'updates'" @click="tab = 'updates'">
          Updates{{ updateAvailable ? " ●" : "" }}
        </button>
      </div>
    </div>
    <span class="muted small">Connected to {{ session.baseUrl }}</span>
  </header>

  <div v-if="tab === 'agents'" class="page-body" >
    <div class="row-between">
      <p class="muted">Ideas are written by AI agent CLIs installed on this computer. The app finds them on your PATH.</p>
      <button class="btn" :disabled="!clis" @click="scan">
        <span v-if="!clis" class="spinner"></span>
        {{ clis ? "Scan again" : "Scanning…" }}
      </button>
    </div>

    <div v-if="cliError" class="alert alert-error" role="alert">{{ cliError }}</div>

    <div v-if="!clis" class="stack" aria-busy="true">
      <div v-for="n in 4" :key="n" class="skeleton" style="height: 68px"></div>
    </div>

    <div v-else class="stack">
      <div v-for="cli in clis" :key="cli.id" class="card stack">
        <div class="row-between">
          <div>
            <div class="row" style="gap: 8px">
              <h2>{{ cli.name }}</h2>
              <span class="badge" :class="{ 'badge-success': cli.installed }">{{ cli.installed ? "Connected" : "Not installed" }}</span>
              <span v-if="cli.version" class="muted small">{{ cli.version }}</span>
            </div>
            <div class="muted small">
              {{ cli.installed ? cli.path : `No "${cli.command}" command found on PATH. Install it, sign in, then scan again.` }}
            </div>
          </div>
          <button v-if="cli.installed" class="btn btn-sm" :disabled="tests[cli.id]?.state === 'running'" @click="test(cli)">
            <span v-if="tests[cli.id]?.state === 'running'" class="spinner"></span>
            {{ tests[cli.id]?.state === "running" ? "Testing…" : "Test connection" }}
          </button>
        </div>
        <div v-if="cli.installed" class="row" style="flex-wrap: wrap; gap: 6px">
          <span class="label">Allowed tools:</span>
          <span v-if="!permissions.some((p) => p.provider === cli.id)" class="muted small">
            None. If {{ cli.name }} needs a tool while planning, the planner asks you to allow it.
          </span>
          <span v-for="permission in permissions.filter((p) => p.provider === cli.id)" :key="permission.tool" class="row" style="gap: 4px">
            <code>{{ permission.tool }}</code>
            <button class="btn btn-sm btn-ghost btn-danger" :aria-label="`Revoke ${permission.tool} for ${cli.name}`" @click="revoke(permission)">
              Revoke
            </button>
          </span>
        </div>
        <div v-if="tests[cli.id]?.state === 'ok'" class="alert alert-success" role="status">Working. {{ tests[cli.id].detail }}</div>
        <div v-if="tests[cli.id]?.state === 'failed'" class="alert alert-error" role="alert">{{ tests[cli.id].detail }}</div>
      </div>
    </div>
  </div>

  <div v-else-if="tab === 'updates'" class="page-body" style="max-width: 600px">
    <div class="card stack">
      <div class="row-between">
        <div>
          <h2>App updates</h2>
          <p class="muted small" style="margin-top: 4px">Growth Engine checks for updates automatically on launch.</p>
        </div>
        <button class="btn" :disabled="updateChecking || updateInstalling" @click="checkForUpdate">
          <span v-if="updateChecking" class="spinner"></span>
          {{ updateChecking ? "Checking…" : "Check now" }}
        </button>
      </div>

      <div v-if="!updateAvailable && !updateChecking" class="alert alert-success" role="status">
        You're up to date.
      </div>

      <template v-if="updateAvailable">
        <div class="update-info">
          <div class="row" style="gap: 10px">
            <span class="badge badge-primary">v{{ updateAvailable.version }} available</span>
          </div>
          <div v-if="updateAvailable.body" class="update-notes">{{ updateAvailable.body }}</div>
        </div>

        <div v-if="updateInstalling" class="stack" style="gap: 8px">
          <div class="row-between">
            <span class="small">Downloading update…</span>
            <span class="small muted">{{ updateProgress }}%</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: updateProgress + '%' }"></div>
          </div>
          <p class="muted small">The app will restart automatically when the update is ready.</p>
        </div>

        <button v-else class="btn btn-primary" style="align-self: flex-start" @click="installUpdate">
          Install update v{{ updateAvailable.version }}
        </button>
      </template>

      <div v-if="updateError" class="alert alert-error" role="alert">{{ updateError }}</div>
    </div>
  </div>

  <div v-else-if="tab === 'skill'" class="page-body">
    <div class="field">
      <label for="skill">Idea skill</label>
      <p class="muted small">These instructions are sent to the AI agent with every plan. Edit them to change how ideas are written.</p>
      <div v-if="!skillLoaded" class="skeleton" style="height: 420px"></div>
      <textarea v-else id="skill" v-model="skill" class="input" rows="24" spellcheck="false" @input="skillNotice = ''"></textarea>
    </div>

    <div v-if="skillError" class="alert alert-error" role="alert">{{ skillError }}</div>
    <div v-if="skillNotice" class="alert alert-success" role="status">{{ skillNotice }}</div>

    <div class="row-between">
      <div v-if="confirmingReset" class="row">
        <span>Replace your skill with the default? Your edits will be lost.</span>
        <button class="btn btn-sm btn-danger" @click="resetSkill">Reset skill</button>
        <button class="btn btn-sm" @click="confirmingReset = false">Keep my skill</button>
      </div>
      <button v-else class="btn" @click="confirmingReset = true">Reset to default</button>
      <button class="btn btn-primary" :disabled="saving || !skillLoaded" @click="saveSkill">
        <span v-if="saving" class="spinner"></span>
        {{ saving ? "Saving…" : dirty ? "Save skill" : "Saved" }}
      </button>
    </div>
  </div>
</template>
