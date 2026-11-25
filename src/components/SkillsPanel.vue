<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { api, errorMessage, type ChannelSkill, type Skill } from "../api";
import { channelsFor } from "../channels";
import Icon from "./Icon.vue";
import PlatformIcon from "./PlatformIcon.vue";

const props = defineProps<{ websiteUrl: string | null; kind?: "idea" | "offer" }>();
/** Tells Settings whether an edit is unsaved, for its side panel. */
const emit = defineEmits<{ dirty: [dirty: boolean]; count: [count: number] }>();

const isOffer = computed(() => props.kind === "offer");
const label = computed(() => (isOffer.value ? "Offer skills" : "Idea skills"));
const channels = channelsFor(props.websiteUrl);

const skills = ref<Skill[] | null>(null);
const channelSkills = ref<ChannelSkill[]>([]);
const error = ref("");
const notice = ref("");

/* Editor state: null shows the cards, "new" a blank skill, else a skill id. */
const editing = ref<string | null>(null);
const name = ref("");
const content = ref("");
const saved = ref({ name: "", content: "" });
const saving = ref(false);
const confirmingDelete = ref(false);
const confirmingLeave = ref(false);

const editingSkill = computed(() => skills.value?.find((skill) => skill.id === editing.value) ?? null);
const dirty = computed(() => editing.value !== null && (name.value !== saved.value.name || content.value !== saved.value.content));
const stats = computed(() => {
  const words = content.value.trim() ? content.value.trim().split(/\s+/).length : 0;
  return `${content.value.split("\n").length} lines · ${words} words`;
});
const defaultSkill = computed(() => skills.value?.find((skill) => skill.isDefault) ?? null);
watch(dirty, (value) => emit("dirty", value));

async function load() {
  error.value = "";
  try {
    if (isOffer.value) {
      skills.value = await api.listOfferSkills();
    } else {
      [skills.value, channelSkills.value] = await Promise.all([api.listSkills(), api.listChannelSkills()]);
    }
    emit("count", skills.value.length);
  } catch (caught) {
    error.value = errorMessage(caught);
    skills.value ??= [];
  }
}

onMounted(load);

/** The skill picked for a planner channel, or "" for the default skill. */
function channelSkillId(channelId: string) {
  return channelSkills.value.find((row) => row.channelId === channelId)?.skillId ?? "";
}

function channelsUsing(skill: Skill) {
  return channels.filter((channel) => {
    const picked = channelSkillId(channel.id);
    return picked ? picked === skill.id : skill.isDefault;
  });
}

async function pickChannelSkill(channelId: string, skillId: string) {
  error.value = notice.value = "";
  try {
    await api.setChannelSkill(channelId, skillId || null);
    channelSkills.value = await api.listChannelSkills();
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

function preview(text: string) {
  return text
    .split("\n")
    .map((line) => line.replace(/^#+\s*/, "").trim())
    .filter(Boolean)
    .slice(0, 4)
    .join(" · ");
}

function updated(skill: Skill) {
  return new Date(skill.updatedAt).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}

function open(skill: Skill) {
  editing.value = skill.id;
  name.value = skill.name;
  content.value = skill.content;
  saved.value = { name: skill.name, content: skill.content };
  confirmingDelete.value = confirmingLeave.value = false;
  error.value = notice.value = "";
}

async function startNew() {
  editing.value = "new";
  name.value = "";
  content.value = "";
  saved.value = { name: "", content: "" };
  confirmingDelete.value = confirmingLeave.value = false;
  error.value = notice.value = "";
}

async function useTemplate() {
  try {
    content.value = await (isOffer.value ? api.offerSkillTemplate() : api.skillTemplate());
    onInput();
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

function onInput() {
  notice.value = "";
}

function close(force = false) {
  if (dirty.value && !force) {
    confirmingLeave.value = true;
    return;
  }
  editing.value = null;
  confirmingLeave.value = false;
  error.value = "";
}

async function save() {
  error.value = notice.value = "";
  saving.value = true;
  try {
    const id = editing.value as string;
    const skill = isOffer.value
      ? id === "new"
        ? await api.createOfferSkill(name.value, content.value)
        : await api.updateOfferSkill(id, name.value, content.value)
      : id === "new"
        ? await api.createSkill(name.value, content.value)
        : await api.updateSkill(id, name.value, content.value);
    await load();
    open(skill);
    notice.value = isOffer.value ? "Skill saved. Pick it when you generate offers." : "Skill saved. The next plan uses it.";
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    saving.value = false;
  }
}

async function makeDefault() {
  if (!editingSkill.value) return;
  error.value = notice.value = "";
  try {
    await api.setDefaultSkill(editingSkill.value.id);
    await load();
    notice.value = `"${editingSkill.value?.name}" is now the default skill.`;
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

async function remove() {
  if (!editingSkill.value) return;
  error.value = "";
  try {
    await (isOffer.value ? api.deleteOfferSkill(editingSkill.value.id) : api.deleteSkill(editingSkill.value.id));
    await load();
    close(true);
  } catch (caught) {
    error.value = errorMessage(caught);
    confirmingDelete.value = false;
  }
}

defineExpose({ dirty });
</script>

<template>
  <!-- Skill editor: its own page inside Settings -->
  <div v-if="editing" class="pub-section sk-editor-page">
    <div class="sk-editor-head">
      <button class="sk-back" title="Back to skills" aria-label="Back to skills" @click="close()">
        <Icon name="arrowLeft" :size="16" />
      </button>
      <div class="sk-editor-title">
        <h2>{{ editing === "new" ? "New skill" : saved.name }}</h2>
        <p>{{ editing === "new" ? (isOffer ? "Name it and write the instructions the agent follows when drafting offers." : "Name it and write the instructions the agent follows."): `Updated ${editingSkill ? updated(editingSkill) : ""}` }}</p>
      </div>
      <span v-if="editingSkill?.isDefault" class="st-pill info">Default</span>
    </div>

    <div v-if="confirmingLeave" class="st-confirm" role="alert">
      <span>You have unsaved changes. Leave without saving?</span>
      <button class="btn btn-sm btn-danger-solid" @click="close(true)">Discard changes</button>
      <button class="btn btn-sm" @click="confirmingLeave = false">Keep editing</button>
    </div>

    <label class="sk-field">
      <span class="st-label">Skill name</span>
      <input v-model="name" class="sk-name" maxlength="80" placeholder="e.g. LinkedIn thought leadership" @input="onInput" />
    </label>

    <div class="st-editor">
      <div class="st-editor-bar">
        <label for="skill-content">SKILL.md</label>
        <span>
          <button v-if="!content.trim()" class="sk-template" type="button" @click="useTemplate">Start from the built-in template</button>
          <template v-else>{{ stats }}<template v-if="dirty"> · <b>Unsaved</b></template></template>
        </span>
      </div>
      <textarea
        id="skill-content"
        v-model="content"
        class="st-textarea"
        rows="22"
        spellcheck="false"
        placeholder="# Writing rules&#10;&#10;Describe the tone, structure, hooks and CTAs the agent should use…"
        @input="onInput"
      ></textarea>
    </div>

    <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="alert alert-success" role="status">{{ notice }}</div>

    <div class="st-actions">
      <div class="sk-actions-left">
        <template v-if="editingSkill && (isOffer || !editingSkill.isDefault)">
          <div v-if="confirmingDelete" class="st-confirm">
            <span>Delete "{{ editingSkill.name }}"?{{ isOffer ? "" : " Channels using it switch to the default skill." }}</span>
            <button class="btn btn-sm btn-danger-solid" @click="remove">Delete skill</button>
            <button class="btn btn-sm" @click="confirmingDelete = false">Keep it</button>
          </div>
          <template v-else>
            <button v-if="!isOffer" class="st-btn" @click="makeDefault"><Icon name="check" :size="14" /> Make default</button>
            <button class="st-btn sk-danger" @click="confirmingDelete = true"><Icon name="trash" :size="14" /> Delete</button>
          </template>
        </template>
      </div>
      <div class="sk-actions-right">
        <button class="st-btn" @click="close()">Cancel</button>
        <button class="btn btn-primary" :disabled="saving || !dirty || !name.trim() || !content.trim()" @click="save">
          <span v-if="saving" class="spinner"></span>
          {{ saving ? "Saving…" : editing === "new" ? "Create skill" : dirty ? "Save changes" : "Saved" }}
        </button>
      </div>
    </div>
  </div>

  <!-- Skill cards + channel defaults -->
  <div v-else class="sk-layout">
    <div class="pub-section sk-cards-col">
      <div class="pub-section-head st-section-head">
        <div>
          <h2>{{ label }}{{ skills ? ` (${skills.length})` : "" }}</h2>
          <p v-if="isOffer">Instructions the AI agent follows when it drafts offers. Pick one on the Generate offers page.</p>
          <p v-else>Writing instructions sent to the AI agent with every plan. Open a skill to edit it.</p>
        </div>
        <button class="st-btn" @click="startNew"><Icon name="plus" :size="14" /> New skill</button>
      </div>

      <div v-if="error" class="alert alert-error" role="alert">{{ error }}</div>

      <div v-if="!skills" class="sk-grid" aria-busy="true">
        <div v-for="n in 2" :key="n" class="skeleton" style="height: 150px"></div>
      </div>

      <div v-else-if="!skills.length" class="pub-empty">
        <Icon name="fileText" :size="32" />
        <p class="pub-empty-title">No {{ label.toLowerCase() }} yet</p>
        <p>{{ isOffer ? "Create one to tell the agent how to draft offers." : "Create one to tell the agent how to write ideas." }}</p>
      </div>

      <div v-else class="sk-grid">
        <button v-for="skill in skills" :key="skill.id" type="button" class="sk-card" :class="{ default: skill.isDefault }" @click="open(skill)">
          <span class="sk-card-head">
            <span class="sk-card-icon"><Icon name="fileText" :size="16" /></span>
            <span class="sk-card-name">{{ skill.name }}</span>
            <span v-if="skill.isDefault" class="st-pill info">Default</span>
          </span>
          <span class="sk-card-preview">{{ preview(skill.content) || "Empty skill" }}</span>
          <span class="sk-card-foot">
            <span v-if="!isOffer" class="sk-card-channels">
              <span v-for="channel in channelsUsing(skill)" :key="channel.id" :title="channel.name"><PlatformIcon :platform="channel.platform" size="sm" /></span>
              <span v-if="!channelsUsing(skill).length" class="sk-card-unused">No channel</span>
            </span>
            <span v-else></span>
            <span class="sk-card-date">Updated {{ updated(skill) }}</span>
          </span>
        </button>
        <button type="button" class="sk-card sk-card-new" @click="startNew">
          <Icon name="plus" :size="20" />
          <span>New skill</span>
        </button>
      </div>
    </div>

    <aside v-if="!isOffer" class="sk-defaults" aria-label="Skill per channel">
      <div class="sk-defaults-head">
        <span class="st-label">Skill per channel</span>
        <p>Picked by default when you plan next week. You can still change it on the plan page.</p>
      </div>
      <div class="sk-defaults-list">
        <label v-for="channel in channels" :key="channel.id" class="sk-default-row">
          <span class="sk-default-channel">
            <PlatformIcon :platform="channel.platform" size="sm" />
            <span>{{ channel.name }}</span>
          </span>
          <select
            class="sk-select"
            :value="channelSkillId(channel.id)"
            :disabled="!skills?.length"
            :aria-label="`Skill for ${channel.name}`"
            @change="pickChannelSkill(channel.id, ($event.target as HTMLSelectElement).value)"
          >
            <option value="">Default{{ defaultSkill ? ` (${defaultSkill.name})` : "" }}</option>
            <option v-for="skill in skills ?? []" :key="skill.id" :value="skill.id">{{ skill.name }}</option>
          </select>
        </label>
      </div>
    </aside>
  </div>
</template>
