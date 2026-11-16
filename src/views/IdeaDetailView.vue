<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { api, errorMessage, type Idea, type IdeaEdit, type Session } from "../api";
import { channelsFor } from "../channels";

const props = defineProps<{ idea: Idea; session: Session }>();
const emit = defineEmits<{ close: []; saved: [idea: Idea]; deleted: [] }>();

const channels = channelsFor(props.session.websiteUrl);

function editable(idea: Idea): IdeaEdit {
  return {
    id: idea.id,
    title: idea.title,
    shortNotes: idea.shortNotes,
    painAddressed: idea.painAddressed,
    targetKeyword: idea.targetKeyword,
    pattern: idea.pattern,
    tag: idea.tag,
    channelId: idea.channelId,
    channelName: idea.channelName,
    suggestedDraft: idea.suggestedDraft,
    imagePrompt: idea.imagePrompt,
  };
}

const form = reactive<IdeaEdit>(editable(props.idea));
const savedSnapshot = ref(JSON.stringify(editable(props.idea)));
const busy = ref<"" | "saving" | "sending" | "deleting">("");
const error = ref("");
const notice = ref("");
const titleError = ref("");
const confirmingDelete = ref(false);

const dirty = computed(() => JSON.stringify(form) !== savedSnapshot.value);
const wasSent = computed(() => props.idea.sentAt !== null);
const sentOn = computed(() =>
  new Date(props.idea.sentAt ?? 0).toLocaleString("en-US", { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" }),
);
const sendLabel = computed(() => {
  if (busy.value === "sending") return "Sending…";
  if (!wasSent.value) return "Send to main site";
  return dirty.value ? "Save and send update" : "Send again";
});

function validateTitle(): boolean {
  titleError.value = form.title.trim() ? "" : "Give the idea a title.";
  return !titleError.value;
}

/** Saves the form if it changed. Returns false when it could not be saved. */
async function persist(): Promise<boolean> {
  if (!validateTitle()) return false;
  if (!dirty.value) return true;
  form.channelName = channels.find((channel) => channel.id === form.channelId)?.name ?? null;
  const saved = await api.updateIdea({ ...form, imagePrompt: form.imagePrompt?.trim() || null });
  Object.assign(form, editable(saved));
  savedSnapshot.value = JSON.stringify(form);
  emit("saved", saved);
  return true;
}

async function save() {
  error.value = notice.value = "";
  busy.value = "saving";
  try {
    if (await persist()) notice.value = "Changes saved.";
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = "";
  }
}

async function send() {
  error.value = notice.value = "";
  busy.value = "sending";
  try {
    if (!(await persist())) return;
    const sent = await api.sendIdea(form.id);
    emit("saved", sent);
    notice.value = `Sent. It's now on the Ideas page at ${props.session.baseUrl}/ideas.`;
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = "";
  }
}

async function remove() {
  error.value = "";
  busy.value = "deleting";
  try {
    await api.deleteIdea(form.id);
    emit("deleted");
  } catch (caught) {
    error.value = errorMessage(caught);
    busy.value = "";
  }
}
</script>

<template>
  <header class="page-head">
    <div class="row" style="gap: 12px; min-width: 0">
      <button class="btn" :disabled="busy !== ''" @click="emit('close')">Back to ideas</button>
      <h1 style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">Edit idea</h1>
      <span v-if="wasSent" class="badge badge-success">Sent {{ sentOn }}</span>
      <span v-else class="badge">Draft</span>
      <span v-if="dirty" class="badge badge-primary">Unsaved changes</span>
    </div>
  </header>

  <form class="page-body plan-body" novalidate @submit.prevent="save">
    <div class="field">
      <label for="idea-title">Title / hook</label>
      <textarea
        id="idea-title"
        v-model="form.title"
        class="input prose"
        rows="2"
        :aria-invalid="titleError !== ''"
        aria-describedby="idea-title-error"
        @blur="validateTitle"
        @input="titleError = ''"
      ></textarea>
      <span v-if="titleError" id="idea-title-error" class="field-error">{{ titleError }}</span>
    </div>

    <div class="field">
      <label for="idea-notes">Angle and audience</label>
      <textarea id="idea-notes" v-model="form.shortNotes" class="input prose" rows="2"></textarea>
    </div>

    <div class="field">
      <label for="idea-pain">Pain it addresses</label>
      <textarea id="idea-pain" v-model="form.painAddressed" class="input prose" rows="2"></textarea>
    </div>

    <div class="form-grid">
      <div class="field">
        <label for="idea-channel">Channel</label>
        <select id="idea-channel" v-model="form.channelId" class="input">
          <option :value="null">No channel</option>
          <option v-for="channel in channels" :key="channel.id" :value="channel.id">{{ channel.name }}</option>
        </select>
      </div>
      <div class="field">
        <label for="idea-pattern">Pattern</label>
        <input id="idea-pattern" v-model="form.pattern" class="input" />
      </div>
      <div class="field">
        <label for="idea-keyword">Keyword</label>
        <input id="idea-keyword" v-model="form.targetKeyword" class="input" />
      </div>
      <div class="field">
        <label for="idea-tag">Tag</label>
        <input id="idea-tag" v-model="form.tag" class="input" />
      </div>
    </div>

    <div class="field">
      <label for="idea-draft">Draft</label>
      <textarea id="idea-draft" v-model="form.suggestedDraft" class="input prose" rows="14"></textarea>
    </div>

    <div class="field">
      <label for="idea-image">Cover image brief <span class="muted" style="font-weight: 400">(optional)</span></label>
      <textarea id="idea-image" v-model="form.imagePrompt" class="input prose" rows="2"></textarea>
    </div>

    <p class="muted small">
      {{ idea.source }}<template v-if="idea.offerName"> · Offer: {{ idea.offerName }}</template>
      <template v-if="idea.provider"> · Written by {{ idea.provider }}</template>
    </p>

    <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="alert alert-success" role="status">{{ notice }}</div>
  </form>

  <footer class="plan-foot">
    <div v-if="confirmingDelete" class="row">
      <span>Delete this idea from the app?{{ wasSent ? " The copy on the main site stays." : "" }}</span>
      <button class="btn btn-danger-solid" :disabled="busy !== ''" @click="remove">
        {{ busy === "deleting" ? "Deleting…" : "Delete idea" }}
      </button>
      <button class="btn" :disabled="busy !== ''" @click="confirmingDelete = false">Keep idea</button>
    </div>
    <button v-else class="btn btn-ghost btn-danger" :disabled="busy !== ''" @click="confirmingDelete = true">Delete</button>

    <div class="row">
      <button class="btn" :disabled="busy !== '' || !dirty" @click="save">
        {{ busy === "saving" ? "Saving…" : dirty ? "Save changes" : "Saved" }}
      </button>
      <button class="btn btn-primary" :disabled="busy !== ''" @click="send">
        <span v-if="busy === 'sending'" class="spinner"></span>
        {{ sendLabel }}
      </button>
    </div>
  </footer>
</template>
