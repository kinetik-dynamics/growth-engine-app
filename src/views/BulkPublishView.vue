<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type ComposerChannel, type Idea, type Session } from "../api";
import { CONNECTIONS_PATH, DEFAULT_POST_TIME, addDays, localDateFromToday } from "../publish";
import { plannedChannel } from "../channels";
import Icon from "../components/Icon.vue";
import ChannelAvatar from "../components/ChannelAvatar.vue";
import SectionCard from "../components/SectionCard.vue";
import ChannelPicker from "../components/ChannelPicker.vue";
import FormatToolbar from "../components/FormatToolbar.vue";

const props = defineProps<{ ideas: Idea[]; session: Session }>();
const emit = defineEmits<{ close: [published: boolean]; saved: [idea: Idea] }>();

interface Schedule {
  date: string;
  time: string;
}

/** The text each idea goes out with, edited here without touching the others. */
const items = ref(props.ideas.map((idea) => ({ idea, text: idea.suggestedDraft || [idea.title, idea.shortNotes].filter(Boolean).join("\n\n") })));
const edited = ref<string[]>([]);
const channels = ref<ComposerChannel[]>([]);
/** Idea id → the channels that idea goes out on. Each row picks its own. */
const rowChannels = reactive<Record<string, string[]>>({});
const loading = ref(true);
const busy = ref<"" | "saving" | "sending">("");
const errors = ref<string[]>([]);

const startDate = ref(localDateFromToday(1));
const startTime = ref(DEFAULT_POST_TIME);
const gapDays = ref(1);
const schedules = reactive<Record<string, Schedule>>({});

function spread() {
  items.value.forEach((item, index) => {
    schedules[item.idea.id] = { date: addDays(startDate.value, index * gapDays.value), time: startTime.value || DEFAULT_POST_TIME };
  });
}
spread();

const editingId = ref<string | null>(null);
const editingText = ref("");
const editTextarea = ref<HTMLTextAreaElement | null>(null);
const editingItem = computed(() => items.value.find((item) => item.idea.id === editingId.value) ?? null);

function channelsOf(id: string): string[] {
  return rowChannels[id] ?? [];
}

/** Channels picked on at least one row. */
const usedIds = computed(() => channels.value.filter((c) => items.value.some((item) => channelsOf(item.idea.id).includes(c.id))).map((c) => c.id));
/** Channels picked on every row: these show as selected in the "Publish to" card. */
const everyRowIds = computed(() =>
  items.value.length ? channels.value.filter((c) => items.value.every((item) => channelsOf(item.idea.id).includes(c.id))).map((c) => c.id) : [],
);
const postCount = computed(() => items.value.reduce((sum, item) => sum + channelsOf(item.idea.id).length, 0));
const missingChannel = computed(() => items.value.some((item) => !channelsOf(item.idea.id).length));
const canSubmit = computed(() => postCount.value > 0 && items.value.length > 0 && busy.value === "" && !loading.value);

/** The connected channel that best matches the one the idea was written for. */
function plannedTarget(idea: Idea): ComposerChannel | undefined {
  const planned = plannedChannel(idea);
  return (
    channels.value.find((c) => c.id === idea.channelId) ??
    channels.value.find((c) => !!idea.channelName && c.name === idea.channelName) ??
    channels.value.find((c) => !!planned && c.platform === planned.platform)
  );
}

/** Channels already picked for the idea, else the one it was planned for, else the first connected one. */
function defaultChannels(idea: Idea): string[] {
  const known = idea.publishChannels.map((c) => c.id).filter((id) => channels.value.some((c) => c.id === id));
  if (known.length) return known;
  const target = plannedTarget(idea) ?? channels.value[0];
  return target ? [target.id] : [];
}

onMounted(async () => {
  try {
    channels.value = (await api.listChannels()).channels;
    for (const { idea } of items.value) rowChannels[idea.id] = defaultChannels(idea);
  } catch (caught) {
    errors.value = [errorMessage(caught)];
  } finally {
    loading.value = false;
  }
});

/** One row's channel chip. */
function toggleRowChannel(ideaId: string, channelId: string) {
  const current = channelsOf(ideaId);
  rowChannels[ideaId] = current.includes(channelId) ? current.filter((c) => c !== channelId) : [...current, channelId];
}

/** The "Publish to" card: adds the channel to every row, or takes it off every row when all rows have it. */
function toggleChannel(channelId: string) {
  const onAll = everyRowIds.value.includes(channelId);
  for (const { idea } of items.value) {
    const current = channelsOf(idea.id).filter((c) => c !== channelId);
    rowChannels[idea.id] = onAll ? current : [...current, channelId];
  }
}

function removeItem(id: string) {
  items.value = items.value.filter((item) => item.idea.id !== id);
}

function openEditor(id: string, text: string) {
  editingId.value = id;
  editingText.value = text;
}

function saveEditedText() {
  const id = editingId.value;
  if (!id || !editingText.value.trim()) return;
  items.value = items.value.map((item) => (item.idea.id === id ? { ...item, text: editingText.value } : item));
  if (!edited.value.includes(id)) edited.value.push(id);
  editingId.value = null;
}

/** Saves every idea's text and its channels and time in the app. */
async function saveAll() {
  for (const { idea, text } of items.value) {
    const schedule = schedules[idea.id];
    const picked = channelsOf(idea.id);
    const first = channels.value.find((c) => c.id === picked[0]);
    const saved = await api.updateIdea({
      id: idea.id,
      title: idea.title,
      shortNotes: idea.shortNotes,
      painAddressed: idea.painAddressed,
      targetKeyword: idea.targetKeyword,
      pattern: idea.pattern,
      tag: idea.tag,
      channelId: first?.id ?? idea.channelId,
      channelName: first?.name ?? idea.channelName,
      suggestedDraft: text,
      imagePrompt: idea.imagePrompt,
      publishChannels: picked.map((id) => ({ id, scheduledDate: schedule.date || null, scheduledTime: schedule.time || null })),
    });
    emit("saved", saved);
  }
}

async function submit(action: "saving" | "sending") {
  errors.value = [];
  if (action === "sending" && items.value.some((item) => !schedules[item.idea.id].date || !schedules[item.idea.id].time)) {
    errors.value = ["Set a date and time for every idea before sending."];
    return;
  }
  if (action === "sending" && missingChannel.value) {
    errors.value = ["Pick at least one channel for every idea before sending, or leave that idea out."];
    return;
  }
  busy.value = action;
  try {
    await saveAll();
    if (action === "sending") {
      const published = await api.publishIdeas(
        items.value.map((item) => ({
          id: item.idea.id,
          scheduledDate: schedules[item.idea.id].date,
          scheduledTime: schedules[item.idea.id].time,
          channels: channelsOf(item.idea.id),
        })),
      );
      published.forEach((idea) => emit("saved", idea));
      emit("close", true);
    } else {
      emit("close", false);
    }
  } catch (caught) {
    errors.value = [errorMessage(caught)];
  } finally {
    busy.value = "";
  }
}

function openConnections() {
  openUrl(`${props.session.baseUrl}${CONNECTIONS_PATH}`);
}
</script>

<template>
  <div class="wp">
    <div class="wp-header">
      <div class="wp-header-left">
        <button class="wp-back" title="Back to Ideas" aria-label="Back to Ideas" :disabled="busy !== ''" @click="emit('close', false)">
          <Icon name="arrowLeft" />
        </button>
        <div style="min-width: 0">
          <h1 class="wp-h1">Publish {{ items.length }} {{ items.length === 1 ? "idea" : "ideas" }}</h1>
          <p class="wp-subtitle">
            {{
              postCount > 0
                ? `${postCount} ${postCount === 1 ? "post" : "posts"} across ${usedIds.length} ${usedIds.length === 1 ? "channel" : "channels"}`
                : "Pick a channel to publish to"
            }}
          </p>
        </div>
      </div>
      <div class="wp-actions">
        <button class="wp-btn" :disabled="!canSubmit" @click="submit('saving')">
          <Icon name="fileText" :size="14" /> {{ busy === "saving" ? "Saving…" : "Save all as drafts" }}
        </button>
        <button class="wp-btn wp-btn-primary" :disabled="!canSubmit" @click="submit('sending')">
          <span v-if="busy === 'sending'" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
          <Icon v-else name="send" :size="14" />
          {{ busy === "sending" ? "Sending…" : "Send all" }}
        </button>
      </div>
    </div>

    <div class="wp-layout">
      <main class="wp-main">
        <div class="wp-column" style="max-width: 896px">
          <div v-if="errors.length" class="wp-alert wp-alert-error" role="alert">
            <Icon name="alertCircle" :size="16" />
            <ul>
              <li v-for="message in errors" :key="message">{{ message }}</li>
            </ul>
          </div>

          <SectionCard icon="share" title="Publish to" hint="Pick a channel to add it to every idea, or pick it again to take it off. Fine-tune each idea's channels below.">
            <div v-if="loading" class="skeleton" style="height: 64px"></div>
            <ChannelPicker v-else :channels="channels" :selected-ids="everyRowIds" @toggle="toggleChannel" @connect="openConnections" />
          </SectionCard>

          <SectionCard icon="calendarClock" title="Schedule" hint="Spread the ideas out, then fine-tune any single one below.">
            <div class="bk-spread">
              <label class="wp-field">
                <span>First post on</span>
                <input v-model="startDate" type="date" class="wp-input wp-input-sm" />
              </label>
              <label class="wp-field">
                <span>At</span>
                <input v-model="startTime" type="time" class="wp-input wp-input-sm" />
              </label>
              <label class="wp-field">
                <span>Then one</span>
                <select v-model.number="gapDays" class="wp-select">
                  <option :value="0">at the same time</option>
                  <option :value="1">every day</option>
                  <option :value="2">every 2 days</option>
                  <option :value="3">every 3 days</option>
                  <option :value="7">every week</option>
                </select>
              </label>
              <button class="bk-apply" :disabled="!startDate" @click="spread">Apply to all</button>
            </div>
          </SectionCard>

          <SectionCard icon="penLine" :title="`Ideas (${items.length})`" hint="Each idea is published with its own draft text.">
            <div v-if="!items.length" class="cp-empty">
              <p class="cp-empty-title">No ideas left to publish</p>
              <button class="wp-link" @click="emit('close', false)">Back to Ideas</button>
            </div>
            <ul v-else class="bk-list">
              <li v-for="(item, index) in items" :key="item.idea.id" class="bk-item">
                <span class="bk-num">{{ index + 1 }}</span>
                <div class="bk-text">
                  <div class="bk-title-row">
                    <h3>{{ item.idea.title }}</h3>
                    <span v-if="item.idea.publishedAt" class="iv-badge iv-badge-sent">Already sent</span>
                    <span v-else-if="item.idea.publishChannels.length" class="iv-badge iv-badge-draft">Draft</span>
                  </div>
                  <p>{{ item.text }}</p>
                  <div class="bk-meta">
                    <span v-if="item.idea.pattern">{{ item.idea.pattern }}</span>
                    <button class="wp-link" @click="openEditor(item.idea.id, item.text)">Edit text</button>
                    <span v-if="edited.includes(item.idea.id)" class="bk-edited">Edited</span>
                  </div>
                  <div v-if="channels.length" class="bk-channels" role="group" :aria-label="`Channels for ${item.idea.title}`">
                    <button
                      v-for="channel in channels"
                      :key="channel.id"
                      type="button"
                      class="bk-chip"
                      :class="{ selected: channelsOf(item.idea.id).includes(channel.id) }"
                      :aria-pressed="channelsOf(item.idea.id).includes(channel.id)"
                      :title="channel.handleOrSub ? `${channel.name} · ${channel.handleOrSub}` : channel.name"
                      @click="toggleRowChannel(item.idea.id, channel.id)"
                    >
                      <ChannelAvatar :channel="channel" size="sm" />
                      <span class="bk-chip-name">{{ channel.name }}</span>
                      <span v-if="plannedTarget(item.idea)?.id === channel.id" class="cp-planned">Planned</span>
                      <Icon v-if="channelsOf(item.idea.id).includes(channel.id)" name="check" :size="11" :stroke="3" />
                    </button>
                    <span v-if="!channelsOf(item.idea.id).length" class="bk-no-channel">Pick a channel</span>
                  </div>
                </div>
                <div class="bk-when">
                  <input v-model="schedules[item.idea.id].date" type="date" class="wp-input wp-input-sm" :aria-label="`Date for ${item.idea.title}`" />
                  <input v-model="schedules[item.idea.id].time" type="time" class="wp-input wp-input-sm" :aria-label="`Time for ${item.idea.title}`" />
                  <button class="iv-trash" title="Leave this idea out" aria-label="Leave this idea out" @click="removeItem(item.idea.id)">
                    <Icon name="x" :size="14" />
                  </button>
                </div>
              </li>
            </ul>
          </SectionCard>
        </div>
      </main>
    </div>

    <div v-if="editingItem" class="pd" role="dialog" aria-modal="true" aria-label="Edit post text" style="justify-content: center; align-items: center">
      <div class="pd-backdrop" @click="editingId = null"></div>
      <div class="bk-modal">
        <div class="bk-modal-head">
          <div style="min-width: 0">
            <h2>Edit post text</h2>
            <p>{{ editingItem.idea.title }}</p>
          </div>
          <button class="pd-close" aria-label="Close" @click="editingId = null"><Icon name="x" /></button>
        </div>
        <FormatToolbar v-model:text="editingText" :textarea="editTextarea" />
        <textarea ref="editTextarea" v-model="editingText" rows="14" class="wp-editor" aria-label="Post text"></textarea>
        <div class="bk-modal-foot">
          <span class="wp-chars">{{ editingText.length.toLocaleString() }} chars</span>
          <div class="wp-actions">
            <button class="wp-btn" @click="editingId = null">Cancel</button>
            <button class="wp-btn wp-btn-primary" :disabled="!editingText.trim()" @click="saveEditedText">Save text</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
