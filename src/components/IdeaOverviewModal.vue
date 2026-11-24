<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { api, errorMessage, type Idea } from "../api";
import { plannedChannel } from "../channels";
import { formatPostPreviewHtml } from "../textFormatter";
import Icon from "./Icon.vue";
import PlatformIcon from "./PlatformIcon.vue";

/** A read-only look at an idea's full content, with its cover image tools close at hand. */
const props = defineProps<{ idea: Idea; actionLabel: string }>();
const emit = defineEmits<{ close: []; write: []; saved: [idea: Idea] }>();

const current = ref<Idea>(props.idea);
watch(
  () => props.idea,
  (idea) => (current.value = idea),
);

const coverImage = ref<string | null>(null);
const imageBusy = ref<"" | "uploading" | "generating" | "removing">("");
const imageError = ref("");
const imageApiReady = ref(false);
const promptCopied = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);
const MAX_IMAGE_BYTES = 8 * 1024 * 1024;

const planned = computed(() => plannedChannel(current.value));
const content = computed(() => current.value.suggestedDraft.trim());
const imagePrompt = computed(() => current.value.imagePrompt?.trim() ?? "");
const details = computed(() =>
  [
    { label: "Pain", value: current.value.painAddressed },
    { label: "Pattern", value: current.value.pattern },
    { label: "Keyword", value: current.value.targetKeyword },
    { label: "Offer", value: current.value.offerName },
    { label: "Topic", value: current.value.topicTitle },
    { label: "Skill", value: current.value.skillName },
    { label: "Written by", value: current.value.provider },
    {
      label: "Planned for",
      value: current.value.scheduledDate ? `${current.value.scheduledDate}${current.value.scheduledTime ? ` · ${current.value.scheduledTime}` : ""}` : null,
    },
  ].filter((fact) => fact.value),
);

async function loadCoverImage() {
  coverImage.value = current.value.imageFile ? await api.ideaImage(current.value.id) : null;
}

function pickImage() {
  imageError.value = "";
  fileInput.value?.click();
}

function readAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject("The image couldn't be read.");
    reader.readAsDataURL(file);
  });
}

async function imageAction(kind: "uploading" | "generating" | "removing", action: () => Promise<Idea>) {
  imageError.value = "";
  imageBusy.value = kind;
  try {
    current.value = await action();
    emit("saved", current.value);
    await loadCoverImage();
  } catch (caught) {
    imageError.value = errorMessage(caught);
  } finally {
    imageBusy.value = "";
  }
}

async function onImagePicked(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  if (!file.type.startsWith("image/")) {
    imageError.value = "Pick a PNG, JPG, WebP or GIF image.";
    return;
  }
  if (file.size > MAX_IMAGE_BYTES) {
    imageError.value = "The image is larger than 8 MB. Pick a smaller one.";
    return;
  }
  await imageAction("uploading", async () => api.setIdeaImage(current.value.id, await readAsDataUrl(file)));
}

const generateImage = () => imageAction("generating", () => api.generateIdeaImage(current.value.id, imagePrompt.value || null));
const removeImage = () => imageAction("removing", () => api.removeIdeaImage(current.value.id));

async function copyPrompt() {
  try {
    await navigator.clipboard.writeText(imagePrompt.value);
    promptCopied.value = true;
    window.setTimeout(() => (promptCopied.value = false), 1500);
  } catch {
    imageError.value = "Couldn't copy. Select the prompt and copy it by hand.";
  }
}

/** A busy image upload keeps the modal open, so its result isn't lost. */
function close() {
  if (imageBusy.value === "") emit("close");
}

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape") close();
}

onMounted(() => {
  window.addEventListener("keydown", onKey);
  loadCoverImage().catch((caught) => (imageError.value = errorMessage(caught)));
  api.getImageApi().then((status) => (imageApiReady.value = status.configured)).catch(() => {});
});
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="pd io" role="dialog" aria-modal="true" :aria-label="`Idea: ${current.title}`">
    <div class="pd-backdrop" @click="close"></div>
    <div class="io-modal">
      <div class="io-head">
        <div class="io-head-text">
          <div class="io-head-meta">
            <span class="io-source">{{ current.source }}</span>
            <span v-if="planned" class="ch-chip" :title="`Planned for ${planned.name}`">
              <PlatformIcon :platform="planned.platform" size="sm" />
              <b>{{ planned.platform }}</b>
              <span v-if="planned.account" class="ch-account">{{ planned.account }}</span>
            </span>
            <span v-if="current.tag" class="iv-tag">{{ current.tag }}</span>
          </div>
          <h2>{{ current.title }}</h2>
        </div>
        <button class="pd-close" title="Close" aria-label="Close" :disabled="imageBusy !== ''" @click="close"><Icon name="x" /></button>
      </div>

      <div class="io-body">
        <p v-if="current.shortNotes" class="io-notes">{{ current.shortNotes }}</p>

        <section>
          <div class="pd-label">Content</div>
          <!-- formatPostPreviewHtml escapes the text before adding its own tags -->
          <div v-if="content" class="pd-content io-content" v-html="formatPostPreviewHtml(content)"></div>
          <p v-else class="wp-muted">No post written yet. Open the Write post page to draft it.</p>
        </section>

        <div v-if="details.length" class="pd-facts">
          <div v-for="fact in details" :key="fact.label" class="pd-fact io-fact">
            <span>{{ fact.label }}</span>
            <b>{{ fact.value }}</b>
          </div>
        </div>

        <section>
          <div class="pd-label">Cover image</div>
          <input
            ref="fileInput"
            type="file"
            accept="image/png,image/jpeg,image/webp,image/gif"
            class="ci-file"
            tabindex="-1"
            aria-hidden="true"
            @change="onImagePicked"
          />
          <div class="ci">
            <div v-if="coverImage" class="ci-preview" :class="{ busy: imageBusy !== '' }">
              <img :src="coverImage" alt="Cover image" />
            </div>
            <button v-else type="button" class="ci-empty" :disabled="imageBusy !== ''" @click="pickImage">
              <span v-if="imageBusy" class="spinner"></span>
              <Icon v-else name="image" :size="28" />
              <b>{{ imageBusy === "generating" ? "Drawing the image…" : imageBusy === "uploading" ? "Uploading…" : "No cover image yet" }}</b>
              <span>Copy the prompt below, make the image in any tool, then upload it here.</span>
            </button>

            <div class="ci-actions">
              <button type="button" class="wp-btn" :disabled="imageBusy !== ''" @click="pickImage">
                <Icon name="upload" :size="14" /> {{ coverImage ? "Replace image" : "Upload image" }}
              </button>
              <button
                v-if="imageApiReady"
                type="button"
                class="wp-btn"
                :disabled="imageBusy !== '' || !imagePrompt"
                :title="!imagePrompt ? 'Write an image prompt on the Write post page first' : 'Draw a new image from the prompt'"
                @click="generateImage"
              >
                <span v-if="imageBusy === 'generating'" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
                <Icon v-else name="sparkles" :size="14" />
                {{ imageBusy === "generating" ? "Generating…" : coverImage ? "Regenerate" : "Generate" }}
              </button>
              <button v-if="coverImage" type="button" class="wp-btn wp-btn-danger-ghost" :disabled="imageBusy !== ''" @click="removeImage">
                <Icon name="trash" :size="14" /> Remove
              </button>
            </div>

            <div v-if="imageError" class="wp-alert wp-alert-error" role="alert">
              <Icon name="alertCircle" :size="16" /> <span>{{ imageError }}</span>
            </div>

            <div class="wp-field">
              <span class="ci-prompt-label">
                Image prompt
                <button type="button" class="wp-link" :disabled="!imagePrompt" @click="copyPrompt">
                  <Icon :name="promptCopied ? 'check' : 'copy'" :size="12" /> {{ promptCopied ? "Copied" : "Copy" }}
                </button>
              </span>
              <p v-if="imagePrompt" class="io-prompt">{{ imagePrompt }}</p>
              <p v-else class="wp-muted">No image prompt yet. Add one on the Write post page.</p>
            </div>
          </div>
        </section>
      </div>

      <div class="pd-foot">
        <button class="pd-btn" :disabled="imageBusy !== ''" @click="close">Close</button>
        <button class="pd-btn pd-btn-primary" :disabled="imageBusy !== ''" @click="emit('write')">
          {{ actionLabel }} <Icon name="arrowRight" :size="12" />
        </button>
      </div>
    </div>
  </div>
</template>
