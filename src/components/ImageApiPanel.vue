<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, errorMessage, type ImageApiStatus, type ImageProvider } from "../api";
import Icon from "./Icon.vue";

const emit = defineEmits<{ status: [status: ImageApiStatus] }>();

const PROVIDERS: { id: ImageProvider; name: string; models: string[]; keyUrl: string }[] = [
  { id: "gemini", name: "Google Gemini", models: ["gemini-2.5-flash-image", "gemini-3-pro-image-preview"], keyUrl: "aistudio.google.com/api-keys" },
  { id: "openai", name: "OpenAI", models: ["gpt-image-1", "gpt-image-1-mini", "dall-e-3"], keyUrl: "platform.openai.com/api-keys" },
];

const status = ref<ImageApiStatus | null>(null);
const provider = ref<ImageProvider>("gemini");
const model = ref("");
const apiKey = ref("");
const showKey = ref(false);
const busy = ref<"" | "saving" | "testing" | "removing">("");
const error = ref("");
const notice = ref("");
const testImage = ref<string | null>(null);

const providerInfo = computed(() => PROVIDERS.find((item) => item.id === provider.value) ?? PROVIDERS[0]);
const providerChanged = computed(() => status.value?.configured && status.value.provider !== provider.value);
const canSave = computed(() => busy.value === "" && (apiKey.value.trim() !== "" || (status.value?.configured && !providerChanged.value)));

function apply(next: ImageApiStatus) {
  status.value = next;
  provider.value = next.provider;
  model.value = next.model;
  apiKey.value = "";
  emit("status", next);
}

onMounted(async () => {
  try {
    apply(await api.getImageApi());
  } catch (caught) {
    error.value = errorMessage(caught);
  }
});

function pickProvider(id: ImageProvider) {
  provider.value = id;
  model.value = PROVIDERS.find((item) => item.id === id)?.models[0] ?? "";
  notice.value = "";
}

async function run(kind: "saving" | "testing" | "removing", action: () => Promise<void>) {
  error.value = notice.value = "";
  busy.value = kind;
  try {
    await action();
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = "";
  }
}

const save = () =>
  run("saving", async () => {
    apply(await api.saveImageApi(provider.value, model.value, apiKey.value.trim() || null));
    notice.value = "Saved. Plans can now draw cover images.";
  });

const test = () =>
  run("testing", async () => {
    testImage.value = null;
    testImage.value = await api.testImageApi();
    notice.value = "It works. Here is a test image.";
  });

const remove = () =>
  run("removing", async () => {
    apply(await api.removeImageApi());
    testImage.value = null;
    notice.value = "The API key was removed.";
  });
</script>

<template>
  <div class="pub-section st-narrow">
    <div class="pub-section-head st-section-head">
      <div>
        <h2>Image API</h2>
        <p>Draws cover images when you plan with "Generate with the image API", or from an idea's Write post page.</p>
      </div>
    </div>

    <div v-if="!status && !error" class="skeleton" style="height: 260px"></div>

    <div v-else class="st-card">
      <div class="st-card-head">
        <div class="st-card-title">
          <span class="st-card-icon" :class="{ info: status?.configured }"><Icon name="sparkles" :size="16" /></span>
          <div class="st-card-meta">
            <div class="st-card-name">
              <span>{{ status?.configured ? `${PROVIDERS.find((item) => item.id === status?.provider)?.name} is set up` : "No image API yet" }}</span>
              <span class="st-pill" :class="status?.configured ? 'ok' : 'off'">
                <span class="st-dot" :class="status?.configured ? 'ok' : 'off'"></span>
                {{ status?.configured ? "Connected" : "Not set" }}
              </span>
            </div>
            <div class="st-card-path">
              {{ status?.configured ? `${status.model} · key ${status.keyHint}` : "Without it, plans write an image prompt you can use in any image tool." }}
            </div>
          </div>
        </div>
        <button v-if="status?.configured" class="st-btn" :disabled="busy !== ''" @click="test">
          <span v-if="busy === 'testing'" class="spinner"></span>
          <Icon v-else name="play" :size="12" />
          {{ busy === "testing" ? "Drawing…" : "Test" }}
        </button>
      </div>

      <div class="ia-form">
        <div class="ia-field">
          <span class="st-label">Service</span>
          <div class="ia-providers" role="radiogroup" aria-label="Image service">
            <button
              v-for="item in PROVIDERS"
              :key="item.id"
              type="button"
              class="ia-provider"
              :class="{ active: provider === item.id }"
              role="radio"
              :aria-checked="provider === item.id"
              @click="pickProvider(item.id)"
            >
              {{ item.name }}
            </button>
          </div>
        </div>

        <label class="ia-field">
          <span class="st-label">Model</span>
          <input v-model="model" class="sk-name" list="image-models" :placeholder="providerInfo.models[0]" />
          <datalist id="image-models">
            <option v-for="item in providerInfo.models" :key="item" :value="item"></option>
          </datalist>
        </label>

        <label class="ia-field">
          <span class="st-label">API key</span>
          <span class="ia-key">
            <input
              v-model="apiKey"
              class="sk-name"
              :type="showKey ? 'text' : 'password'"
              autocomplete="off"
              spellcheck="false"
              :placeholder="status?.configured && !providerChanged ? `Saved (${status.keyHint}). Paste a new key to replace it.` : 'Paste your API key'"
            />
            <button type="button" class="ia-eye" :aria-label="showKey ? 'Hide key' : 'Show key'" @click="showKey = !showKey">
              <Icon :name="showKey ? 'eyeOff' : 'eye'" :size="14" />
            </button>
          </span>
          <span class="st-hint">Get a key at {{ providerInfo.keyUrl }}. It is stored only on this computer.</span>
        </label>
      </div>

      <div v-if="testImage" class="ia-test">
        <img :src="testImage" alt="Test image drawn by the image API" />
      </div>

      <div class="st-card-foot ia-foot">
        <button v-if="status?.configured" class="st-btn sk-danger" :disabled="busy !== ''" @click="remove">
          <Icon name="trash" :size="14" /> Remove key
        </button>
        <span v-else></span>
        <button class="btn btn-primary btn-sm" :disabled="!canSave" @click="save">
          <span v-if="busy === 'saving'" class="spinner"></span>
          {{ busy === "saving" ? "Saving…" : "Save" }}
        </button>
      </div>
    </div>

    <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="alert alert-success" role="status">{{ notice }}</div>
  </div>
</template>
