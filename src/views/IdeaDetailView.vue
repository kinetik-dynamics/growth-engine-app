<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type ComposerChannel, type Idea, type IdeaEdit, type RemoteComposer, type Session } from "../api";
import { CONNECTIONS_PATH, DEFAULT_POST_TIME, localDateFromToday } from "../publish";
import { formatPostPreviewHtml, type FormatMode } from "../textFormatter";
import Icon from "../components/Icon.vue";
import SectionCard from "../components/SectionCard.vue";
import ChannelPicker from "../components/ChannelPicker.vue";
import ChannelAvatar from "../components/ChannelAvatar.vue";
import FormatToolbar from "../components/FormatToolbar.vue";
import PlatformIcon from "../components/PlatformIcon.vue";
import { plannedChannel } from "../channels";

const props = defineProps<{ idea: Idea; session: Session }>();
const emit = defineEmits<{ close: []; saved: [idea: Idea]; deleted: [] }>();

interface Schedule {
  date: string;
  time: string;
}

const details = reactive({
  title: props.idea.title,
  shortNotes: props.idea.shortNotes,
  painAddressed: props.idea.painAddressed,
  targetKeyword: props.idea.targetKeyword,
  tag: props.idea.tag,
  imagePrompt: props.idea.imagePrompt ?? "",
});
const text = ref(props.idea.suggestedDraft || [props.idea.title, props.idea.shortNotes].filter(Boolean).join("\n\n"));
const pattern = ref(props.idea.pattern);
const selectedIds = ref<string[]>(props.idea.publishChannels.map((channel) => channel.id));
const schedules = reactive<Record<string, Schedule>>({});
for (const channel of props.idea.publishChannels) {
  schedules[channel.id] = { date: channel.scheduledDate ?? "", time: channel.scheduledTime ?? "" };
}

const channels = ref<ComposerChannel[]>([]);
const patterns = ref<string[]>([]);
const loading = ref(true);
const remote = ref<RemoteComposer | null>(null);
const remoteDiffers = ref(false);
const previewId = ref<string | null>(null);
const formatMode = ref<FormatMode>("unicode");
const textarea = ref<HTMLTextAreaElement | null>(null);
const busy = ref<"" | "saving" | "sending" | "publishing" | "deleting">("");
const errors = ref<string[]>([]);
const notice = ref("");
const loadError = ref("");
const confirmingDelete = ref(false);
const current = ref<Idea>(props.idea);

/** A slot the idea was planned for is the first suggestion, else tomorrow at 9. */
function defaultSchedule(): Schedule {
  return { date: current.value.scheduledDate ?? localDateFromToday(1), time: current.value.scheduledTime ?? DEFAULT_POST_TIME };
}

function scheduleFor(id: string): Schedule {
  if (!schedules[id]) schedules[id] = defaultSchedule();
  return schedules[id];
}

/** The channel the agent wrote this idea for; it stays fixed whatever channels the post goes out on. */
const planned = computed(() => plannedChannel(current.value));
const plannedStyle = computed(() => (planned.value ? { "--ch": planned.value.color, "--ch-soft": planned.value.soft } : undefined));

const selectedChannels = computed(() => channels.value.filter((channel) => selectedIds.value.includes(channel.id)));
const previewChannel = computed(
  () => selectedChannels.value.find((channel) => channel.id === previewId.value) ?? selectedChannels.value[0] ?? null,
);
const patternOptions = computed(() => {
  const names = [...patterns.value];
  if (pattern.value && !names.includes(pattern.value)) names.unshift(pattern.value);
  return names;
});
const facts = computed(() =>
  [
    { label: "Source", value: current.value.source },
    { label: "Written by", value: current.value.provider },
    { label: "Offer", value: current.value.offerName },
    {
      label: "Planned for",
      value: current.value.scheduledDate ? `${current.value.scheduledDate}${current.value.scheduledTime ? ` · ${current.value.scheduledTime}` : ""}` : null,
    },
  ].filter((fact) => fact.value),
);
const status = computed(() => {
  if (current.value.publishedAt) return { label: "In queue", tone: "queue" };
  if (current.value.sentAt) return { label: "On website", tone: "sent" };
  return { label: "Draft", tone: "draft" };
});
const canSave = computed(() => busy.value === "" && details.title.trim() !== "");

function toggleChannel(id: string) {
  selectedIds.value = selectedIds.value.includes(id) ? selectedIds.value.filter((c) => c !== id) : [...selectedIds.value, id];
  scheduleFor(id);
}

function applyFirstScheduleToAll() {
  const first = { ...scheduleFor(selectedChannels.value[0].id) };
  for (const channel of selectedChannels.value) schedules[channel.id] = { ...first };
}

/** The website's copy wins only when the user asks for it. */
function useRemoteVersion() {
  const data = remote.value;
  if (!data) return;
  details.title = data.idea.title;
  details.shortNotes = data.idea.shortNotes ?? "";
  details.painAddressed = data.idea.painAddressed ?? "";
  details.targetKeyword = data.idea.targetKeyword ?? "";
  details.tag = data.idea.tag ?? "";
  details.imagePrompt = data.idea.imagePrompt ?? "";
  text.value = data.posts[0]?.text ?? data.idea.suggestedDraft ?? text.value;
  pattern.value = data.posts[0]?.pattern ?? data.idea.pattern ?? "";
  if (data.posts.length) {
    selectedIds.value = data.posts.map((post) => post.channelId).filter((id) => channels.value.some((c) => c.id === id));
    for (const post of data.posts) {
      schedules[post.channelId] = { date: post.scheduledDate ?? defaultSchedule().date, time: post.scheduledTime ?? defaultSchedule().time };
    }
  }
  remoteDiffers.value = false;
}

function differsFromRemote(data: RemoteComposer) {
  const remoteText = data.posts[0]?.text ?? data.idea.suggestedDraft ?? "";
  return (
    data.idea.title !== details.title ||
    (data.idea.shortNotes ?? "") !== details.shortNotes ||
    (data.idea.painAddressed ?? "") !== details.painAddressed ||
    (data.idea.targetKeyword ?? "") !== details.targetKeyword ||
    (data.idea.tag ?? "") !== details.tag ||
    (remoteText !== "" && remoteText !== text.value) ||
    (data.posts.length > 0 && data.posts.map((post) => post.channelId).sort().join() !== [...selectedIds.value].sort().join())
  );
}

async function load() {
  loading.value = true;
  loadError.value = "";
  try {
    const [loadedPatterns, composer] = await Promise.all([
      api.listPatterns(),
      current.value.remoteId ? api.getRemoteIdea(current.value.remoteId).catch(() => null) : Promise.resolve(null),
    ]);
    patterns.value = loadedPatterns;
    if (composer) {
      remote.value = composer;
      channels.value = composer.channels;
    } else {
      channels.value = (await api.listChannels()).channels;
    }

    if (!selectedIds.value.length) {
      const target =
        channels.value.find((channel) => channel.id === current.value.channelId) ??
        channels.value.find((channel) => channel.platform === planned.value?.platform) ??
        channels.value[0];
      if (target) selectedIds.value = [target.id];
    }
    selectedIds.value = selectedIds.value.filter((id) => channels.value.some((channel) => channel.id === id));
    selectedIds.value.forEach(scheduleFor);

    if (composer) {
      if (!current.value.publishChannels.length && composer.posts.length) useRemoteVersion();
      else remoteDiffers.value = differsFromRemote(composer);
    }
  } catch (caught) {
    loadError.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
}

onMounted(load);

function edit(): IdeaEdit {
  return {
    id: current.value.id,
    title: details.title.trim(),
    shortNotes: details.shortNotes,
    painAddressed: details.painAddressed,
    targetKeyword: details.targetKeyword,
    pattern: pattern.value,
    tag: details.tag,
    // The planned channel is kept; the picked ones live in publishChannels.
    channelId: current.value.channelId,
    channelName: current.value.channelName,
    suggestedDraft: text.value,
    imagePrompt: details.imagePrompt.trim() || null,
    publishChannels: selectedChannels.value.map((channel) => ({
      id: channel.id,
      scheduledDate: scheduleFor(channel.id).date || null,
      scheduledTime: scheduleFor(channel.id).time || null,
    })),
  };
}

async function persist() {
  const saved = await api.updateIdea(edit());
  current.value = saved;
  emit("saved", saved);
}

function check(forPublish: boolean): boolean {
  const problems: string[] = [];
  if (!details.title.trim()) problems.push("Give the idea a title.");
  if (forPublish) {
    if (!selectedChannels.value.length) problems.push("Pick at least one channel to publish to.");
    if (!text.value.trim()) problems.push("Write the post before sending it.");
    if (selectedChannels.value.some((channel) => !scheduleFor(channel.id).date || !scheduleFor(channel.id).time)) {
      problems.push("Set a date and time for every channel before sending.");
    }
  }
  errors.value = problems;
  return problems.length === 0;
}

async function run(kind: "saving" | "sending" | "publishing", forPublish: boolean, action: () => Promise<string>) {
  notice.value = "";
  if (!check(forPublish)) return;
  busy.value = kind;
  try {
    await persist();
    notice.value = await action();
  } catch (caught) {
    errors.value = [errorMessage(caught)];
  } finally {
    busy.value = "";
  }
}

const saveDraft = () => run("saving", false, async () => "Saved as a draft in the app.");

const sendToIdeas = () =>
  run("sending", false, async () => {
    current.value = await api.sendIdea(current.value.id);
    emit("saved", current.value);
    return `Sent. It's on the Ideas page at ${props.session.baseUrl}/ideas.`;
  });

const sendToPublish = () =>
  run("publishing", true, async () => {
    current.value = await api.publishIdea(current.value.id);
    emit("saved", current.value);
    const count = selectedChannels.value.length;
    return `Sent to the publish queue on ${count} ${count === 1 ? "channel" : "channels"}.`;
  });

async function remove() {
  busy.value = "deleting";
  try {
    await api.deleteIdea(current.value.id);
    emit("deleted");
  } catch (caught) {
    errors.value = [errorMessage(caught)];
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
        <button class="wp-back" title="Back to Ideas" aria-label="Back to Ideas" :disabled="busy !== ''" @click="emit('close')">
          <Icon name="arrowLeft" />
        </button>
        <div style="min-width: 0">
          <div class="wp-title-row">
            <h1>Write post</h1>
            <span class="wp-status" :class="`wp-status-${status.tone}`">{{ status.label }}</span>
            <span v-if="planned" class="ch-chip" :title="`Planned for ${planned.name}`">
              <PlatformIcon :platform="planned.platform" size="sm" />
              <b>{{ planned.platform }}</b>
              <span v-if="planned.account" class="ch-account">{{ planned.account }}</span>
            </span>
          </div>
          <p class="wp-subtitle">{{ details.title }}</p>
        </div>
      </div>
      <div class="wp-actions">
        <button class="wp-btn" :disabled="!canSave" @click="saveDraft">
          <Icon name="fileText" :size="14" /> {{ busy === "saving" ? "Saving…" : "Save as draft" }}
        </button>
        <button class="wp-btn" :disabled="!canSave" @click="sendToIdeas">
          <Icon name="lightbulb" :size="14" /> {{ busy === "sending" ? "Sending…" : "Send to website ideas" }}
        </button>
        <button class="wp-btn wp-btn-primary" :disabled="!canSave || loading" @click="sendToPublish">
          <span v-if="busy === 'publishing'" class="spinner" style="width: 13px; height: 13px; border-width: 1.5px"></span>
          <Icon v-else name="send" :size="14" />
          {{ busy === "publishing" ? "Sending…" : "Send to published" }}
        </button>
      </div>
    </div>

    <div class="wp-layout">
      <main class="wp-main">
        <div class="wp-column">
          <div v-if="errors.length" class="wp-alert wp-alert-error" role="alert">
            <Icon name="alertCircle" :size="16" />
            <ul>
              <li v-for="message in errors" :key="message">{{ message }}</li>
            </ul>
          </div>
          <div v-if="notice" class="wp-alert wp-alert-success" role="status">
            <Icon name="check" :size="16" /> <span>{{ notice }}</span>
          </div>
          <div v-if="loadError" class="wp-alert wp-alert-error" role="alert">
            <Icon name="alertCircle" :size="16" />
            <span>{{ loadError }} <button class="wp-link" @click="load">Try again</button></span>
          </div>
          <div v-if="remoteDiffers" class="wp-alert wp-alert-info" role="status">
            <Icon name="alertCircle" :size="16" />
            <span>
              This idea was changed on the website.
              <button class="wp-link" @click="useRemoteVersion">Load the website's version</button>
            </span>
          </div>

          <SectionCard icon="lightbulb" title="Idea details" hint="The hook, angle and audience this post is written from.">
            <div class="wp-details">
              <div v-if="planned || current.topicTitle" class="wp-planned" :style="plannedStyle ?? { '--ch': '#9ca3af', '--ch-soft': '#f9fafb' }">
                <span v-if="planned" class="wp-planned-icon"><PlatformIcon :platform="planned.platform" /></span>
                <span v-else class="wp-planned-icon"><Icon name="hash" :size="14" /></span>
                <div class="wp-planned-text">
                  <span v-if="planned" class="wp-planned-label">Written for</span>
                  <span v-if="planned" class="wp-planned-name">
                    <b>{{ planned.platform }}</b><template v-if="planned.account"> · {{ planned.account }}</template>
                  </span>
                  <span v-if="current.topicTitle" class="wp-planned-topic">
                    Topic: <b>{{ current.topicTitle }}</b><template v-if="current.topicDescription"> · {{ current.topicDescription }}</template>
                  </span>
                </div>
              </div>
              <label class="wp-field">
                <span>Title / hook</span>
                <textarea v-model="details.title" rows="2" class="wp-input"></textarea>
              </label>
              <div class="wp-grid2">
                <label class="wp-field">
                  <span>Angle and audience</span>
                  <textarea v-model="details.shortNotes" rows="2" class="wp-input"></textarea>
                </label>
                <label class="wp-field">
                  <span>Pain it addresses</span>
                  <textarea v-model="details.painAddressed" rows="2" class="wp-input"></textarea>
                </label>
                <label class="wp-field">
                  <span>Keyword</span>
                  <input v-model="details.targetKeyword" class="wp-input" />
                </label>
                <label class="wp-field">
                  <span>Tag</span>
                  <input v-model="details.tag" class="wp-input" />
                </label>
              </div>
              <label class="wp-field">
                <span>Cover image brief (optional)</span>
                <textarea v-model="details.imagePrompt" rows="2" class="wp-input"></textarea>
              </label>
              <dl v-if="facts.length" class="wp-facts">
                <div v-for="fact in facts" :key="fact.label">
                  <dt>{{ fact.label }}:</dt>
                  <dd>{{ fact.value }}</dd>
                </div>
              </dl>
              <div class="wp-delete-row">
                <template v-if="confirmingDelete">
                  <span>Delete this idea from the app?{{ current.sentAt ? " The copy on the website stays." : "" }}</span>
                  <button class="wp-btn wp-btn-danger" :disabled="busy !== ''" @click="remove">{{ busy === "deleting" ? "Deleting…" : "Delete idea" }}</button>
                  <button class="wp-btn" :disabled="busy !== ''" @click="confirmingDelete = false">Keep idea</button>
                </template>
                <button v-else class="wp-delete" :disabled="busy !== ''" @click="confirmingDelete = true">
                  <Icon name="trash" :size="14" /> Delete idea
                </button>
              </div>
            </div>
          </SectionCard>

          <SectionCard icon="share" title="Publish to" hint="Pick one or more of your connected channels.">
            <div v-if="loading" class="skeleton" style="height: 64px"></div>
            <ChannelPicker v-else :channels="channels" :selected-ids="selectedIds" :planned-platform="planned?.platform" @toggle="toggleChannel" @connect="openConnections" />
          </SectionCard>

          <SectionCard icon="layers" title="Pattern" hint="The writing structure this post follows.">
            <template #action>
              <select v-model="pattern" class="wp-select" aria-label="Pattern">
                <option value="">No pattern</option>
                <option v-for="name in patternOptions" :key="name" :value="name">{{ name }}</option>
              </select>
            </template>
            <p class="wp-muted">{{ pattern ? "This pattern has no saved structure." : "Choose a pattern to see its structure." }}</p>
          </SectionCard>

          <SectionCard icon="penLine" title="Post">
            <template #action>
              <span class="wp-chars">{{ text.length.toLocaleString() }} chars</span>
            </template>
            <FormatToolbar v-model:text="text" v-model:format-mode="formatMode" :textarea="textarea" show-mode-switch />
            <textarea
              ref="textarea"
              v-model="text"
              rows="16"
              class="wp-editor"
              placeholder="What do you want to share with your audience?"
              aria-label="Post"
            ></textarea>
          </SectionCard>

          <SectionCard icon="calendarClock" title="Schedule" hint="When each channel's post goes out. Used when you press Send to published.">
            <template v-if="selectedChannels.length > 1" #action>
              <button class="wp-link" @click="applyFirstScheduleToAll">Use first time for all</button>
            </template>
            <p v-if="!selectedChannels.length" class="wp-muted">Pick a channel above to set its schedule.</p>
            <div v-else class="wp-schedules">
              <div v-for="channel in selectedChannels" :key="channel.id" class="wp-schedule">
                <div class="wp-schedule-name">
                  <ChannelAvatar :channel="channel" size="sm" />
                  <span>{{ channel.name }}</span>
                </div>
                <div class="wp-schedule-inputs">
                  <input v-model="scheduleFor(channel.id).date" type="date" class="wp-input wp-input-sm" :aria-label="`Date for ${channel.name}`" />
                  <input v-model="scheduleFor(channel.id).time" type="time" class="wp-input wp-input-sm" :aria-label="`Time for ${channel.name}`" />
                </div>
              </div>
            </div>
          </SectionCard>
        </div>
      </main>

      <aside class="wp-preview">
        <div class="wp-preview-head">
          <span class="wp-preview-label"><Icon name="eye" :size="14" /> Live preview</span>
          <div v-if="selectedChannels.length > 1" class="wp-preview-tabs">
            <button
              v-for="channel in selectedChannels"
              :key="channel.id"
              :class="{ active: previewChannel?.id === channel.id }"
              @click="previewId = channel.id"
            >
              {{ channel.name.split(" · ")[1] ?? channel.name }}
            </button>
          </div>
        </div>
        <div v-if="previewChannel" class="wp-preview-card">
          <div class="wp-preview-author">
            <ChannelAvatar :channel="previewChannel" />
            <div style="min-width: 0">
              <div class="wp-preview-name">{{ previewChannel.handleOrSub }}</div>
              <div class="wp-preview-sub">{{ previewChannel.name }}</div>
            </div>
          </div>
          <!-- formatPostPreviewHtml escapes the text before adding its own tags -->
          <div v-if="text.trim()" class="wp-preview-text" v-html="formatPostPreviewHtml(text)"></div>
          <div v-else class="wp-preview-empty">Your post will appear here as you type…</div>
          <div class="wp-preview-foot">
            <span>Like · Comment · Repost</span>
            <span>{{ scheduleFor(previewChannel.id).date }} · {{ scheduleFor(previewChannel.id).time }}</span>
          </div>
        </div>
        <div v-else class="wp-preview-none">Pick a channel to preview the post.</div>
      </aside>
    </div>
  </div>
</template>
