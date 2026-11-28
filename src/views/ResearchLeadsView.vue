<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api, errorMessage, type CliStatus, type PlanProgress, type ResearchPlan, type ResearchTarget } from "../api";
import Icon from "../components/Icon.vue";
import SectionCard from "../components/SectionCard.vue";

/** When not empty, only these leads are researched. */
const props = defineProps<{ targets?: { type: "lead" | "prospect"; id: number; name: string }[] }>();
const emit = defineEmits<{ close: []; changed: []; openSettings: [] }>();

const LIMITS = [10, 20, 30, 50, 100, 0];
const LIMIT_KEY = "research.limit";

function savedLimit() {
  try {
    const value = Number(localStorage.getItem(LIMIT_KEY));
    return LIMITS.includes(value) && localStorage.getItem(LIMIT_KEY) !== null ? value : 10;
  } catch {
    return 10;
  }
}

const picked = computed(() => props.targets ?? []);
const single = computed(() => picked.value.length === 1);
/** Hand-picked leads (one or many): no limit, history ignored. */
const handPicked = computed(() => picked.value.length > 0);
const only = computed<ResearchTarget[]>(() => picked.value.map(({ type, id }) => ({ type, id })));
/** The first few picked names, for the summary under the numbers. */
const pickedNames = computed(() => {
  const names = picked.value.slice(0, 5).map((item) => item.name).join(", ");
  const more = picked.value.length - 5;
  return more > 0 ? `${names} +${more} more` : names;
});
/** 0 means every lead. */
const limit = ref(savedLimit());
const includeResearched = ref(false);

const FIELD_LABELS: Record<string, string> = {
  email: "Email",
  phone: "Phone",
  website: "Website",
  location: "Location",
  category: "Category",
  rating: "Rating",
  company_name: "Company",
  company_details: "Company details",
  contacts: "Key people",
};

const loading = ref(true);
const error = ref("");
const clis = ref<CliStatus[]>([]);
const plan = ref<ResearchPlan | null>(null);
const provider = ref("");

const generating = ref(false);
const stopping = ref(false);
const activity = ref<PlanProgress[]>([]);
const activityBox = ref<HTMLElement | null>(null);
const errorBox = ref<HTMLElement | null>(null);
const liveText = ref("");
const liveBox = ref<HTMLElement | null>(null);
const elapsed = ref(0);
/** Elapsed seconds at the agent's last sign of life. */
const lastSign = ref(0);
const neededTools = ref<string[]>([]);
const result = ref<{ checked: number; updated: number; fieldsFilled: number; failed: number; cancelled: boolean } | null>(null);
let timer: number | undefined;
let unlisten: UnlistenFn | undefined;

const installed = computed(() => clis.value.filter((c) => c.installed));
const providerName = computed(() => clis.value.find((c) => c.id === provider.value)?.name ?? "The AI agent");
const elapsedLabel = computed(() => `${Math.floor(elapsed.value / 60)}:${String(elapsed.value % 60).padStart(2, "0")}`);
/** How many leads this run will take. */
const toResearch = computed(() => {
  const needing = plan.value?.needing ?? 0;
  return handPicked.value || !limit.value ? needing : Math.min(limit.value, needing);
});
const silentFor = computed(() => elapsed.value - lastSign.value);
const silentLabel = computed(() => `${Math.floor(silentFor.value / 60)}:${String(silentFor.value % 60).padStart(2, "0")}`);
const batches = computed(() => Math.ceil(toResearch.value / 5));

watch(limit, (value) => {
  try {
    localStorage.setItem(LIMIT_KEY, String(value));
  } catch {
    /* remembering the choice is optional */
  }
});

async function loadPlan() {
  try {
    plan.value = await api.researchPlan(includeResearched.value, only.value);
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

watch(includeResearched, loadPlan);
/* Keep the newest step in view while the agent works. */
watch(
  () => activity.value.length,
  () => nextTick(() => activityBox.value?.scrollTo({ top: activityBox.value.scrollHeight })),
);

onMounted(async () => {
  unlisten = await listen<PlanProgress>("plan-progress", ({ payload }) => onProgress(payload));
  try {
    const [detected, found] = await Promise.all([api.detectClis(), api.researchPlan(includeResearched.value, only.value)]);
    clis.value = detected;
    plan.value = found;
    provider.value = installed.value[0]?.id ?? "";
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
});

onUnmounted(() => {
  unlisten?.();
  window.clearInterval(timer);
});

function onProgress(progress: PlanProgress) {
  if (!generating.value) return;
  lastSign.value = elapsed.value;
  if (progress.kind === "text") {
    liveText.value += progress.message;
    nextTick(() => liveBox.value?.scrollTo({ top: liveBox.value.scrollHeight }));
  } else {
    activity.value.push(progress);
  }
}

async function start() {
  error.value = "";
  neededTools.value = [];
  activity.value = [];
  liveText.value = "";
  result.value = null;
  elapsed.value = 0;
  lastSign.value = 0;
  stopping.value = false;
  generating.value = true;
  timer = window.setInterval(() => elapsed.value++, 1000);
  try {
    const outcome = await api.researchLeads({
      provider: provider.value,
      limit: handPicked.value || !limit.value ? null : limit.value,
      includeResearched: includeResearched.value,
      only: only.value,
    });
    if (outcome.kind === "done") {
      result.value = outcome;
      emit("changed");
      await loadPlan();
    } else {
      neededTools.value = outcome.tools;
    }
  } catch (caught) {
    error.value = errorMessage(caught);
    // The message sits at the top, so bring it into view.
    nextTick(() => errorBox.value?.scrollIntoView({ block: "center", behavior: "smooth" }));
    // Batches saved before the failure are already on the site.
    emit("changed");
  } finally {
    window.clearInterval(timer);
    generating.value = false;
  }
}

async function stop() {
  stopping.value = true;
  await api.cancelPlan();
}

async function allowAndRetry() {
  try {
    await api.grantPermissions(provider.value, neededTools.value);
  } catch (caught) {
    error.value = errorMessage(caught);
    return;
  }
  await start();
}
</script>

<template>
  <div class="wp pw">
    <div class="wp-header">
      <div class="wp-header-left">
        <button class="wp-back" title="Back to Queue" aria-label="Back to Queue" :disabled="generating" @click="emit('close')">
          <Icon name="arrowLeft" />
        </button>
        <div style="min-width: 0">
          <div class="wp-title-row">
            <h1>{{ single ? "Research lead" : handPicked ? `Research ${picked.length} selected leads` : "Research leads" }}</h1>
            <span class="pw-agent-badge"><Icon name="sparkles" :size="11" /> Research agent</span>
          </div>
          <p class="wp-subtitle">
            {{
              single
                ? `Your AI agent looks up ${picked[0].name} and fills the fields that are still empty.`
                : handPicked
                  ? `Your AI agent looks up the ${picked.length} leads you selected and fills the fields that are still empty.`
                  : "Your AI agent checks the leads in the Queue and fills the fields that are still empty."
            }}
          </p>
        </div>
      </div>
    </div>

    <div class="wp-layout">
      <main class="wp-main">
        <div class="wp-column">
          <div v-if="error" ref="errorBox" class="wp-alert wp-alert-error" role="alert">
            <Icon name="alertCircle" :size="16" />
            <span style="white-space: pre-wrap; word-break: break-word">{{ error }}</span>
          </div>

          <div v-if="loading" class="pw-skeletons" aria-busy="true">
            <div class="skeleton" style="height: 150px"></div>
            <div class="skeleton" style="height: 120px"></div>
          </div>

          <template v-else>
            <SectionCard
              icon="users"
              :title="single ? 'The lead' : handPicked ? 'Selected leads' : 'What the agent checks'"
              :hint="
                single
                  ? 'Only this lead is researched, even if it was researched before.'
                  : handPicked
                    ? 'Only these leads are researched, even if they were researched before.'
                    : 'The Queue is read from the main site each time you start. Findings stay in this app until you sync them.'
              "
            >
              <dl v-if="!single" class="go-sources">
                <div>
                  <dt><Icon name="users" :size="12" /> {{ handPicked ? "Selected" : "In the queue" }}</dt>
                  <dd>{{ handPicked ? picked.length : (plan?.total ?? 0) }}</dd>
                </div>
                <div>
                  <dt><Icon name="search" :size="12" /> To research</dt>
                  <dd>{{ toResearch }}</dd>
                </div>
                <div>
                  <dt><Icon name="layers" :size="12" /> Agent runs</dt>
                  <dd>{{ batches }}</dd>
                </div>
              </dl>
              <p v-else class="rl-single">{{ picked[0]?.name }}</p>
              <p v-if="handPicked && !single" class="wp-muted" style="margin-top: 10px">{{ pickedNames }}</p>
              <div v-if="plan?.fields.length" class="rl-fields">
                <span v-for="item in plan.fields" :key="item.field" class="bp-chip">
                  {{ FIELD_LABELS[item.field] ?? item.field }}{{ single ? "" : ` · ${item.count}` }}
                </span>
              </div>
              <div v-if="plan && !plan.needing" class="wp-alert wp-alert-info" role="status" style="margin-top: 12px">
                <Icon name="checkCircle" :size="16" />
                <span v-if="single">This lead already has all of its fields. Nothing to research.</span>
                <span v-else-if="handPicked">The selected leads already have all of their fields. Nothing to research.</span>
                <span v-else-if="plan.alreadyResearched">
                  Every lead with gaps was researched before ({{ plan.alreadyResearched }}). Tick "Include leads researched before" to try them again.
                </span>
                <span v-else>Every lead already has all of its fields. Nothing to research.</span>
              </div>
              <p v-else-if="!single" class="wp-muted" style="margin-top: 12px">
                Leads are researched five at a time, and each batch is kept in this app as soon as it finishes. Only empty fields are filled; nothing
                you already have is overwritten. The instructions come from Settings &gt; Prompts.
              </p>
            </SectionCard>

            <SectionCard
              v-if="!handPicked"
              icon="hash"
              title="Leads per run"
              hint="Research searches the web, so a big queue takes a long time. Leads left over stay for the next run."
            >
              <div class="rl-run">
                <label class="wp-field rl-limit">
                  <span>Research up to</span>
                  <select v-model.number="limit" class="wp-input" :disabled="generating">
                    <option v-for="option in LIMITS" :key="option" :value="option">{{ option ? `${option} leads` : "All leads" }}</option>
                  </select>
                </label>
                <label class="rl-check">
                  <input v-model="includeResearched" type="checkbox" :disabled="generating" />
                  <span>
                    Include leads researched before
                    <small v-if="plan?.alreadyResearched || includeResearched">Searched leads are marked, so they are skipped by default.</small>
                  </span>
                </label>
              </div>
              <p v-if="plan?.alreadyResearched && !includeResearched" class="wp-muted" style="margin-top: 10px">
                {{ plan.alreadyResearched }} {{ plan.alreadyResearched === 1 ? "lead was" : "leads were" }} researched before and will be skipped.
              </p>
            </SectionCard>

            <SectionCard icon="bot" title="AI agent" hint="Agents come from the CLIs detected in Settings > AI agents. Research needs one that can search the web.">
              <label v-if="installed.length" class="wp-field">
                <span>Agent</span>
                <select v-model="provider" class="wp-input" :disabled="generating">
                  <option v-for="cli in installed" :key="cli.id" :value="cli.id">
                    {{ cli.name }}{{ cli.version ? ` (${cli.version})` : "" }}
                  </option>
                </select>
              </label>
              <div v-else class="cp-empty">
                <p class="cp-empty-title">No AI agent CLI found</p>
                <p>Install Claude Code, Antigravity or OpenCode, then check Settings.</p>
                <button type="button" class="cp-connect" @click="emit('openSettings')">Open Settings</button>
              </div>
            </SectionCard>

            <SectionCard
              v-if="generating || activity.length"
              icon="radio"
              :title="generating ? `${providerName} is working` : 'Last run'"
              :hint="`Elapsed ${elapsedLabel}`"
            >
              <template v-if="generating" #action>
                <button class="wp-btn" :disabled="stopping" @click="stop">{{ stopping ? "Stopping…" : "Stop researching" }}</button>
              </template>
              <div class="wp-details">
                <ol ref="activityBox" class="activity rl-activity" aria-live="polite">
                  <li v-for="(entry, index) in activity" :key="index" :class="`activity-${entry.kind}`">
                    <span class="activity-mark" aria-hidden="true">
                      {{ entry.kind === "denied" ? "!" : entry.kind === "stage" && (!generating || index < activity.length - 1) ? "✓" : "•" }}
                    </span>
                    <span>{{ entry.message }}</span>
                  </li>
                </ol>
                <div v-if="liveText" class="wp-field">
                  <span>Live output from {{ providerName }}</span>
                  <pre ref="liveBox" class="live-output" tabindex="0">{{ liveText }}</pre>
                </div>
                <p v-else-if="generating" class="wp-muted">Nothing written yet. Research runs search the web, so the first batch can take a few minutes.</p>
                <p v-if="generating && silentFor >= 30" class="wp-muted" role="status">
                  No new activity for {{ silentLabel }}. {{ providerName }} may be waiting on its model. If it stays silent for 4 minutes the run is stopped
                  and you can try again or pick another agent.
                </p>
              </div>
            </SectionCard>

            <div v-if="result && !generating" class="wp-alert wp-alert-info" role="status">
              <Icon name="checkCircle" :size="16" />
              <div class="wp-details">
                <b>{{ result.cancelled ? "Stopped early" : "Research finished" }}</b>
                <p>
                  Checked {{ result.checked }} {{ result.checked === 1 ? "lead" : "leads" }}, updated {{ result.updated }} and filled
                  {{ result.fieldsFilled }} {{ result.fieldsFilled === 1 ? "field" : "fields" }}. The findings are saved in this app; press
                  <b>Sync to website</b> on the Queue to send them.
                  <template v-if="result.failed">{{ result.failed }} could not be researched; run it again to retry them.</template>
                </p>
              </div>
            </div>

            <div v-if="neededTools.length && !generating" class="wp-alert wp-alert-info pw-permission" role="alert">
              <Icon name="alertTriangle" :size="16" />
              <div class="wp-details">
                <b>{{ providerName }} needs your permission</b>
                <p>
                  {{ providerName }} asked to use
                  <template v-for="(tool, index) in neededTools" :key="tool"><code class="wp-code">{{ tool }}</code>{{ index < neededTools.length - 1 ? ", " : "" }}</template>.
                  It runs in the background and can't ask you directly, so the request was denied and no research came back.
                </p>
                <p class="wp-muted">You can revoke this later in Settings &gt; AI agents.</p>
                <div class="wp-actions">
                  <button class="wp-btn wp-btn-primary" @click="allowAndRetry">Allow and run again</button>
                  <button class="wp-btn" @click="neededTools = []">Don't allow</button>
                </div>
              </div>
            </div>
          </template>
        </div>
      </main>
    </div>

    <footer class="pw-foot">
      <button class="wp-btn" :disabled="generating" @click="emit('close')">{{ result ? "Back to Queue" : "Cancel" }}</button>
      <div class="pw-foot-right">
        <span v-if="!loading && !provider" class="wp-muted">Pick an AI agent to research the leads.</span>
        <button class="wp-btn wp-btn-primary" :disabled="loading || generating || !provider || !toResearch" @click="start">
          <span v-if="generating" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
          <Icon v-else name="sparkles" :size="14" />
          {{ generating ? "Researching…" : error || neededTools.length ? "Try again" : `Research ${toResearch} ${toResearch === 1 ? "lead" : "leads"}` }}
        </button>
      </div>
    </footer>
  </div>
</template>
