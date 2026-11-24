<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { api, errorMessage, type Named, type OfferDraft, type SiteProof } from "../api";
import Icon from "./Icon.vue";

/** Edits an offer draft before it is synced to the main site. */
const props = defineProps<{ draft: OfferDraft; icpOptions: Named[]; proofOptions: SiteProof[] }>();
const emit = defineEmits<{ close: []; saved: [draft: OfferDraft] }>();

const form = reactive({
  name: props.draft.name,
  promise: props.draft.promise,
  description: props.draft.description,
  priceRange: props.draft.priceRange,
  timeline: props.draft.timeline,
  deliverables: props.draft.deliverables.join("\n"),
  icpIds: [...props.draft.icpIds],
  caseStudyIds: [...props.draft.caseStudyIds],
});
const saving = ref(false);
const error = ref("");

/** Case studies another offer already has stay with it when this one syncs. */
const proofChoices = computed(() =>
  props.proofOptions.map((study) => ({ ...study, taken: study.offer_id !== null && !form.caseStudyIds.includes(study.id) })),
);

function toggle(list: number[], id: number) {
  const index = list.indexOf(id);
  if (index === -1) list.push(id);
  else list.splice(index, 1);
}

async function save() {
  error.value = "";
  saving.value = true;
  try {
    const saved = await api.updateOfferDraft({
      id: props.draft.id,
      name: form.name,
      promise: form.promise,
      description: form.description,
      priceRange: form.priceRange,
      timeline: form.timeline,
      deliverables: form.deliverables.split("\n"),
      icpIds: form.icpIds,
      caseStudyIds: form.caseStudyIds,
    });
    emit("saved", saved);
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    saving.value = false;
  }
}

function close() {
  if (!saving.value) emit("close");
}

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape") close();
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="pd io" role="dialog" aria-modal="true" :aria-label="`Edit offer: ${draft.name}`">
    <div class="pd-backdrop" @click="close"></div>
    <form class="io-modal" @submit.prevent="save">
      <div class="io-head">
        <div class="io-head-text">
          <span class="io-source">Offer draft · not on the website yet</span>
          <h2>Edit offer</h2>
        </div>
        <button type="button" class="pd-close" title="Close" aria-label="Close" :disabled="saving" @click="close"><Icon name="x" /></button>
      </div>

      <div class="io-body">
        <div v-if="error" class="wp-alert wp-alert-error" role="alert"><Icon name="alertCircle" :size="16" /> <span>{{ error }}</span></div>

        <label class="wp-field">
          <span>Name</span>
          <input v-model="form.name" class="wp-input" maxlength="255" required />
        </label>
        <label class="wp-field">
          <span>Promise</span>
          <textarea v-model="form.promise" class="wp-input" rows="2" required></textarea>
        </label>
        <label class="wp-field">
          <span>Description</span>
          <textarea v-model="form.description" class="wp-input" rows="4"></textarea>
        </label>
        <div class="od-row">
          <label class="wp-field">
            <span>Price range</span>
            <input v-model="form.priceRange" class="wp-input" maxlength="255" placeholder="e.g. $8k–$15k" />
          </label>
          <label class="wp-field">
            <span>Timeline</span>
            <input v-model="form.timeline" class="wp-input" maxlength="255" placeholder="e.g. 4–6 weeks" />
          </label>
        </div>
        <label class="wp-field">
          <span>Deliverables (one per line)</span>
          <textarea v-model="form.deliverables" class="wp-input" rows="4"></textarea>
        </label>

        <fieldset v-if="icpOptions.length" class="od-picks">
          <legend>ICPs it is for</legend>
          <label v-for="icp in icpOptions" :key="icp.id" class="od-pick">
            <input type="checkbox" :checked="form.icpIds.includes(icp.id)" @change="toggle(form.icpIds, icp.id)" />
            <span>{{ icp.name }}</span>
          </label>
        </fieldset>

        <fieldset v-if="proofChoices.length" class="od-picks">
          <legend>Case studies that prove it</legend>
          <label v-for="study in proofChoices" :key="study.id" class="od-pick" :class="{ taken: study.taken }">
            <input type="checkbox" :checked="form.caseStudyIds.includes(study.id)" @change="toggle(form.caseStudyIds, study.id)" />
            <span>
              {{ study.title }}
              <small v-if="study.taken" :title="`Already proves ${study.offer?.name}; it stays with that offer`">· with {{ study.offer?.name }}</small>
            </span>
          </label>
        </fieldset>
      </div>

      <div class="pd-foot">
        <button type="button" class="pd-btn" :disabled="saving" @click="close">Cancel</button>
        <button type="submit" class="pd-btn pd-btn-primary" :disabled="saving">
          <span v-if="saving" class="spinner" style="width: 12px; height: 12px; border-width: 1.5px"></span>
          Save offer
        </button>
      </div>
    </form>
  </div>
</template>
