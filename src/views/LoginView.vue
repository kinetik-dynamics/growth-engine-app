<script setup lang="ts">
import { ref } from "vue";
import { api, errorMessage, type Session } from "../api";

defineProps<{ notice?: string }>();
const emit = defineEmits<{ signedIn: [session: Session] }>();

const baseUrl = ref("http://growth-engine.test");
const token = ref("");
const busy = ref(false);
const error = ref("");

async function activate() {
  error.value = "";
  busy.value = true;
  try {
    emit("signedIn", await api.login(baseUrl.value, token.value));
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="login">
    <form class="card login-card" @submit.prevent="activate">
      <div class="brand" style="padding: 0"><span class="brand-mark">GE</span> Growth Engine</div>
      <div>
        <h1>Activate this app</h1>
        <p class="muted">
          Paste an access token from Growth Engine. Create one in the web app under Settings &gt; Access Tokens.
        </p>
      </div>

      <div v-if="notice" class="alert alert-error">{{ notice }}</div>

      <div class="field">
        <label for="base-url">Server URL</label>
        <input id="base-url" v-model="baseUrl" class="input" type="url" placeholder="http://growth-engine.test" required />
      </div>

      <div class="field">
        <label for="token">Access token</label>
        <input id="token" v-model="token" class="input" type="password" autocomplete="off" placeholder="1|abc123…" @input="error = ''" />
      </div>

      <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>

      <button class="btn btn-primary" type="submit" :disabled="busy">
        <span v-if="busy" class="spinner"></span>
        {{ busy ? "Checking token…" : "Activate" }}
      </button>
    </form>
  </div>
</template>
