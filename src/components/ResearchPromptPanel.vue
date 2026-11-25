<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { api, errorMessage } from "../api";
import Icon from "./Icon.vue";

/** Tells Settings whether an edit is unsaved, for its side panel. */
const emit = defineEmits<{ dirty: [dirty: boolean] }>();

const loaded = ref(false);
const prompt = ref("");
const saved = ref("");
const isDefault = ref(true);
const saving = ref(false);
const error = ref("");
const notice = ref("");
const confirmingReset = ref(false);

const dirty = computed(() => loaded.value && prompt.value.trim() !== saved.value.trim());
const stats = computed(() => {
  const words = prompt.value.trim() ? prompt.value.trim().split(/\s+/).length : 0;
  return `${prompt.value.split("\n").length} lines · ${words} words`;
});
watch(dirty, (value) => emit("dirty", value));

function apply(result: { prompt: string; isDefault: boolean }) {
  prompt.value = saved.value = result.prompt;
  isDefault.value = result.isDefault;
}

onMounted(async () => {
  try {
    apply(await api.getResearchPrompt());
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    loaded.value = true;
  }
});

async function save() {
  error.value = notice.value = "";
  saving.value = true;
  try {
    apply(await api.saveResearchPrompt(prompt.value));
    notice.value = "Prompt saved. The next lead research uses it.";
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    saving.value = false;
  }
}

async function reset() {
  error.value = notice.value = "";
  confirmingReset.value = false;
  try {
    apply(await api.resetResearchPrompt());
    notice.value = "Back to the built-in prompt.";
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

defineExpose({ dirty });
</script>

<template>
  <div class="pub-section sk-editor-page">
    <div class="pub-section-head st-section-head">
      <div>
        <h2>Lead research prompt</h2>
        <p>
          The instructions the AI agent follows when you press <b>Research leads</b> on the Queue. The app adds the leads and the reply format
          after your text, so you can rewrite the rest freely.
        </p>
      </div>
      <span class="st-pill" :class="isDefault ? 'info' : 'ok'">{{ isDefault ? "Built-in" : "Customised" }}</span>
    </div>

    <div v-if="!loaded" class="skeleton" style="height: 360px" aria-busy="true"></div>

    <div v-else class="st-editor">
      <div class="st-editor-bar">
        <label for="research-prompt">Prompt</label>
        <span>{{ stats }}<template v-if="dirty"> · <b>Unsaved</b></template></span>
      </div>
      <textarea id="research-prompt" v-model="prompt" class="st-textarea" rows="24" spellcheck="false" @input="notice = ''"></textarea>
    </div>

    <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="alert alert-success" role="status">{{ notice }}</div>

    <div class="st-actions">
      <div class="sk-actions-left">
        <div v-if="confirmingReset" class="st-confirm">
          <span>Replace your prompt with the built-in one?</span>
          <button class="btn btn-sm btn-danger-solid" @click="reset">Reset prompt</button>
          <button class="btn btn-sm" @click="confirmingReset = false">Keep mine</button>
        </div>
        <button v-else-if="!isDefault || dirty" class="st-btn" @click="confirmingReset = true"><Icon name="refresh" :size="14" /> Reset to built-in</button>
      </div>
      <div class="sk-actions-right">
        <button class="btn btn-primary" :disabled="saving || !dirty || !prompt.trim()" @click="save">
          <span v-if="saving" class="spinner"></span>
          {{ saving ? "Saving…" : dirty ? "Save prompt" : "Saved" }}
        </button>
      </div>
    </div>
  </div>
</template>
