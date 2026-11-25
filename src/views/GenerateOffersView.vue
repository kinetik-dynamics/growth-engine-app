<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api, errorMessage, type CliStatus, type OfferDraft, type PlanProgress, type Session, type SiteIcp, type SiteProof, type Skill } from "../api";
import Icon from "../components/Icon.vue";
import SectionCard from "../components/SectionCard.vue";
import MultiSelect from "../components/MultiSelect.vue";

defineProps<{ session: Session }>();
const emit = defineEmits<{ close: []; generated: [offers: OfferDraft[], requested: number]; openSettings: [] }>();

const MAX_OFFERS = 10;
const FOCUS_MAX = 400;

const loading = ref(true);
const error = ref("");
const clis = ref<CliStatus[]>([]);
const icps = ref<SiteIcp[]>([]);
const proof = ref<SiteProof[]>([]);

const skills = ref<Skill[]>([]);
/* Picked material; nothing picked means the agent reads all of it. */
const pickedIcps = ref<number[]>([]);
const pickedProof = ref<number[]>([]);
const skillId = ref("");

const count = ref(3);
const focus = ref("");
const provider = ref("");

/* Generation state */
const generating = ref(false);
const stopping = ref(false);
const activity = ref<PlanProgress[]>([]);
const liveText = ref("");
const liveBox = ref<HTMLElement | null>(null);
const elapsed = ref(0);
const neededTools = ref<string[]>([]);
let timer: number | undefined;
let unlisten: UnlistenFn | undefined;

const installed = computed(() => clis.value.filter((c) => c.installed));
const providerName = computed(() => clis.value.find((c) => c.id === provider.value)?.name ?? "The AI agent");
const elapsedLabel = computed(() => `${Math.floor(elapsed.value / 60)}:${String(elapsed.value % 60).padStart(2, "0")}`);
const thinMaterial = computed(() => !loading.value && !icps.value.length && !proof.value.length);

onMounted(async () => {
  unlisten = await listen<PlanProgress>("plan-progress", ({ payload }) => onProgress(payload));
  try {
    const [detected, icpPage, proofPage, offerSkills] = await Promise.all([
      api.detectClis(),
      api.sitePage("icps"),
      api.sitePage("proof"),
      api.listOfferSkills(),
    ]);
    clis.value = detected;
    skills.value = offerSkills;
    skillId.value = offerSkills[0]?.id ?? "";
    icps.value = icpPage.icps;
    proof.value = proofPage.proof;
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
  if (progress.kind === "text") {
    liveText.value += progress.message;
    nextTick(() => liveBox.value?.scrollTo({ top: liveBox.value.scrollHeight }));
  } else {
    activity.value.push(progress);
  }
}

async function generate() {
  error.value = "";
  neededTools.value = [];
  activity.value = [];
  liveText.value = "";
  elapsed.value = 0;
  stopping.value = false;
  generating.value = true;
  timer = window.setInterval(() => elapsed.value++, 1000);
  try {
    const outcome = await api.generateOffers({ provider: provider.value, count: count.value,
      focus: focus.value.trim(),
      icpIds: [...pickedIcps.value],
      proofIds: [...pickedProof.value],
      skillId: skillId.value || null,
    });
    if (outcome.kind === "created") emit("generated", outcome.offers, count.value);
    else neededTools.value = outcome.tools;
  } catch (caught) {
    error.value = errorMessage(caught);
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
  await generate();
}
</script>

<template>
  <div class="wp pw">
    <div class="wp-header">
      <div class="wp-header-left">
        <button class="wp-back" title="Back to Offers" aria-label="Back to Offers" :disabled="generating" @click="emit('close')">
          <Icon name="arrowLeft" />
        </button>
        <div style="min-width: 0">
          <div class="wp-title-row">
            <h1>Generate offers</h1>
            <span class="pw-agent-badge"><Icon name="sparkles" :size="11" /> Offer agent</span>
          </div>
          <p class="wp-subtitle">Your AI agent drafts outcome-led offers from your company overview, ICPs and case studies.</p>
        </div>
      </div>
    </div>

    <div class="wp-layout">
      <main class="wp-main">
        <div class="wp-column">
          <div v-if="error" class="wp-alert wp-alert-error" role="alert">
            <Icon name="alertCircle" :size="16" />
            <span style="white-space: pre-wrap; word-break: break-word">{{ error }}</span>
          </div>

          <div v-if="loading" class="pw-skeletons" aria-busy="true">
            <div class="skeleton" style="height: 150px"></div>
            <div class="skeleton" style="height: 170px"></div>
          </div>

          <template v-else>
            <SectionCard icon="layers" title="What the agent reads" hint="Read from the main site each time you generate.">
              <dl class="go-sources">
                <div>
                  <dt><Icon name="building" :size="12" /> Company overview</dt>
                  <dd>{{ session.companyName || "Your company profile" }}</dd>
                </div>
                <div>
                  <dt><Icon name="target" :size="12" /> ICPs</dt>
                  <dd>{{ icps.length }}</dd>
                </div>
                <div>
                  <dt><Icon name="award" :size="12" /> Case studies</dt>
                  <dd>{{ proof.length }}</dd>
                </div>
              </dl>
              <div v-if="thinMaterial" class="wp-alert wp-alert-info" role="status" style="margin-top: 12px">
                <Icon name="alertCircle" :size="16" />
                <span>No ICPs or case studies yet, so the offers will rest on the company overview alone. Add them in the web app for sharper offers.</span>
              </div>
            </SectionCard>

            <SectionCard icon="target" title="ICPs" hint="Pick one or more. Leave all unpicked to use every ICP.">
              <MultiSelect
                v-if="icps.length"
                v-model="pickedIcps"
                :options="icps.map((icp) => ({ id: icp.id, label: icp.name }))"
                :disabled="generating"
                placeholder="Search ICPs… (all if none picked)"
              />
              <p v-else class="wp-muted">No ICPs on the main site yet.</p>
            </SectionCard>

            <SectionCard icon="award" title="Proof" hint="Pick one or more case studies. Leave all unpicked to use every one.">
              <MultiSelect
                v-if="proof.length"
                v-model="pickedProof"
                :options="proof.map((study) => ({ id: study.id, label: study.title }))"
                :disabled="generating"
                placeholder="Search case studies… (all if none picked)"
              />
              <p v-else class="wp-muted">No case studies on the main site yet.</p>
            </SectionCard>

            <SectionCard icon="fileText" title="Offer skill" hint="The instructions the agent follows. Manage them in Settings > Offer skills.">
              <label v-if="skills.length" class="wp-field">
                <span>Skill</span>
                <select v-model="skillId" class="wp-input" :disabled="generating">
                  <option value="">None (built-in guidance only)</option>
                  <option v-for="skill in skills" :key="skill.id" :value="skill.id">{{ skill.name }}</option>
                </select>
              </label>
              <div v-else class="cp-empty">
                <p class="cp-empty-title">No offer skills yet</p>
                <p>Add one in Settings to steer how the agent writes offers.</p>
                <button type="button" class="cp-connect" @click="emit('openSettings')">Open Settings</button>
              </div>
            </SectionCard>

            <SectionCard icon="package" title="How many offers" hint="Each offer takes a different angle: a different audience, outcome or problem.">
              <template #action>
                <div class="pw-stepper">
                  <button type="button" :disabled="generating || count <= 1" aria-label="Fewer offers" @click="count--">−</button>
                  <output aria-label="Offers to draft">{{ count }}</output>
                  <button type="button" :disabled="generating || count >= MAX_OFFERS" aria-label="More offers" @click="count++">+</button>
                </div>
              </template>
            </SectionCard>

            <SectionCard icon="hash" title="Focus" hint="Optional. Steer the agent toward an audience, industry or problem.">
              <label class="wp-field">
                <span class="pw-label-row">
                  Focus
                  <span class="pw-count">{{ focus.length }}/{{ FOCUS_MAX }}</span>
                </span>
                <textarea
                  v-model="focus"
                  rows="3"
                  class="wp-input"
                  :maxlength="FOCUS_MAX"
                  :disabled="generating"
                  placeholder="e.g. Offers for construction firms that still estimate jobs in spreadsheets. Leave blank to let the agent pick."
                ></textarea>
              </label>
            </SectionCard>

            <SectionCard icon="bot" title="AI agent" hint="Agents come from the CLIs detected in Settings > AI agents.">
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

            <!-- Live activity: what the agent is doing right now -->
            <SectionCard
              v-if="generating || activity.length"
              icon="radio"
              :title="generating ? `${providerName} is working` : 'Last run'"
              :hint="`Elapsed ${elapsedLabel}`"
            >
              <template v-if="generating" #action>
                <button class="wp-btn" :disabled="stopping" @click="stop">{{ stopping ? "Stopping…" : "Stop generating" }}</button>
              </template>
              <div class="wp-details">
                <ol class="activity" aria-live="polite">
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
                <p v-else-if="generating" class="wp-muted">Nothing written yet. Agents usually think for a while before the first words appear.</p>
              </div>
            </SectionCard>

            <!-- Permission request the headless agent could not ask for itself -->
            <div v-if="neededTools.length && !generating" class="wp-alert wp-alert-info pw-permission" role="alert">
              <Icon name="alertTriangle" :size="16" />
              <div class="wp-details">
                <b>{{ providerName }} needs your permission</b>
                <p>
                  {{ providerName }} asked to use
                  <template v-for="(tool, index) in neededTools" :key="tool"><code class="wp-code">{{ tool }}</code>{{ index < neededTools.length - 1 ? ", " : "" }}</template>.
                  It runs in the background and can't ask you directly, so the request was denied and no offers came back.
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
      <button class="wp-btn" :disabled="generating" @click="emit('close')">Cancel</button>
      <div class="pw-foot-right">
        <span v-if="!loading && !provider" class="wp-muted">Pick an AI agent to draft the offers.</span>
        <button class="wp-btn wp-btn-primary" :disabled="loading || generating || !provider" @click="generate">
          <span v-if="generating" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
          <Icon v-else name="sparkles" :size="14" />
          {{ generating ? "Generating…" : error || neededTools.length ? "Try again" : `Generate ${count} ${count === 1 ? "offer" : "offers"}` }}
        </button>
      </div>
    </footer>
  </div>
</template>
