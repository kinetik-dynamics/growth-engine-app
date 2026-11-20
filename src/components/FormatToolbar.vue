<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { applyFormatToTextarea, type FormatAction, type FormatMode } from "../textFormatter";
import type { IconName } from "../icons";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    textarea: HTMLTextAreaElement | null;
    text: string;
    formatMode?: FormatMode;
    showModeSwitch?: boolean;
    disabled?: boolean;
    /** Shows an "Add image" button that emits `image`. */
    showImage?: boolean;
  }>(),
  { formatMode: "unicode", showModeSwitch: false, disabled: false, showImage: false },
);
const emit = defineEmits<{ "update:text": [text: string]; "update:formatMode": [mode: FormatMode]; image: [] }>();

/** Markdown has no underline, so that button only works in Unicode mode. */
function unavailable(action: FormatAction) {
  return action === "underline" && props.formatMode === "markdown";
}

const EMOJI_GROUPS = [
  { name: "Tech & Work", emojis: ["🚀", "⚡", "💡", "🛠️", "💻", "📱", "⚙️", "🤖", "📈", "📊"] },
  { name: "Signal & Focus", emojis: ["🎯", "📌", "📍", "✅", "❌", "⚠️", "🔍", "💎", "🔑", "🏷️"] },
  { name: "Social & Clicks", emojis: ["👇", "👉", "🧵", "💬", "📣", "🔔", "🤝", "👏", "🔥", "💯"] },
];

const STUDIO_HASHTAGS = ["#b2b", "#saas", "#automation", "#mvp", "#softwaredevelopment", "#ai", "#engineering", "#startups"];

const HOOK_TEMPLATES = [
  { label: "📊 Statistic Hook", template: "📊 In 2026, 73% of B2B founders struggle with [bottleneck]. Here is why:" },
  { label: "⚠️ Contrarian Teardown", template: "⚠️ Most software studios tell you [myth].\n\nOur client data proves the exact opposite:" },
  { label: "🛠️ 6-Week Case Study", template: "🛠️ How we took [client] from [legacy bottleneck] to a production [solution] in 6 weeks without technical debt:" },
  { label: "👇 3 Key Lessons", template: "👇 3 production engineering lessons we learned shipping [system]:\n\n1. \n2. \n3. " },
];

const FORMAT_BUTTONS: { action: FormatAction; icon: IconName; title: string }[] = [
  { action: "bold", icon: "bold", title: "Bold" },
  { action: "italic", icon: "italic", title: "Italic" },
  { action: "underline", icon: "underline", title: "Underline (Unicode low line)" },
  { action: "strikethrough", icon: "strikethrough", title: "Strikethrough" },
  { action: "mono", icon: "code", title: "Monospace / Code" },
];
const LIST_BUTTONS: { action: FormatAction; icon: IconName; title: string }[] = [
  { action: "bullets", icon: "list", title: "Bullet list (•)" },
  { action: "numbers", icon: "listOrdered", title: "Numbered list (1. 2. 3.)" },
  { action: "quote", icon: "quote", title: "Quote / Callout (>)" },
];

type Menu = "" | "case" | "emoji" | "hashtag" | "hook";
const open = ref<Menu>("");
const root = ref<HTMLElement | null>(null);

function toggle(menu: Menu) {
  open.value = open.value === menu ? "" : menu;
}

function onOutside(event: MouseEvent) {
  const menu = root.value?.querySelector(".ft-menu-open");
  if (menu && !menu.contains(event.target as Node)) open.value = "";
}
onMounted(() => document.addEventListener("mousedown", onOutside));
onUnmounted(() => document.removeEventListener("mousedown", onOutside));

/** Puts focus and the selection back after the text changes. */
function restore(start: number, end: number) {
  const el = props.textarea;
  if (!el) return;
  setTimeout(() => {
    el.focus();
    el.setSelectionRange(start, end);
  }, 10);
}

function apply(action: FormatAction) {
  const el = props.textarea;
  if (!el) return;
  const { newText, newStart, newEnd } = applyFormatToTextarea(props.text, el.selectionStart, el.selectionEnd, action, props.formatMode);
  emit("update:text", newText);
  restore(newStart, newEnd);
  open.value = "";
}

function insert(value: string) {
  const el = props.textarea;
  if (!el) {
    emit("update:text", props.text ? `${props.text} ${value}` : value);
    return;
  }
  const start = el.selectionStart;
  emit("update:text", props.text.substring(0, start) + value + props.text.substring(el.selectionEnd));
  restore(start + value.length, start + value.length);
  open.value = "";
}
</script>

<template>
  <div ref="root" class="ft">
    <div class="ft-tools">
      <button
        v-for="button in FORMAT_BUTTONS"
        :key="button.action"
        type="button"
        class="ft-btn"
        :title="unavailable(button.action) ? 'Underline needs Unicode mode (Markdown has no underline)' : button.title"
        :aria-label="button.title"
        :disabled="disabled || unavailable(button.action)"
        @click="apply(button.action)"
      >
        <Icon :name="button.icon" :size="14" :stroke="2" />
      </button>
      <span class="ft-sep"></span>
      <button
        v-for="button in LIST_BUTTONS"
        :key="button.action"
        type="button"
        class="ft-btn"
        :title="button.title"
        :aria-label="button.title"
        :disabled="disabled"
        @click="apply(button.action)"
      >
        <Icon :name="button.icon" :size="14" :stroke="2" />
      </button>
      <span class="ft-sep"></span>
      <template v-if="showImage">
        <button type="button" class="ft-btn" title="Add a cover image" aria-label="Add image" :disabled="disabled" @click="emit('image')">
          <Icon name="image" :size="14" :stroke="2" />
        </button>
        <span class="ft-sep"></span>
      </template>

      <div class="ft-menu" :class="{ 'ft-menu-open': open === 'case' }">
        <button type="button" class="ft-btn ft-btn-text" title="Text case transformation" :disabled="disabled" @click="toggle('case')">
          <Icon name="type" :size="12" :stroke="2" /> Case <Icon name="chevronDown" :size="10" :stroke="2" />
        </button>
        <div v-if="open === 'case'" class="ft-pop ft-pop-case">
          <button type="button" class="ft-pop-item" @click="apply('titlecase')">Title Case</button>
          <button type="button" class="ft-pop-item" @click="apply('uppercase')">UPPERCASE</button>
        </div>
      </div>

      <div class="ft-menu" :class="{ 'ft-menu-open': open === 'emoji' }">
        <button type="button" class="ft-btn" title="Insert business emoji" aria-label="Insert emoji" :disabled="disabled" @click="toggle('emoji')">
          <Icon name="smile" :size="14" :stroke="2" />
        </button>
        <div v-if="open === 'emoji'" class="ft-pop ft-pop-emoji">
          <div class="ft-pop-title">High-Signal Emojis</div>
          <div v-for="group in EMOJI_GROUPS" :key="group.name">
            <div class="ft-pop-group">{{ group.name }}</div>
            <div class="ft-emoji-grid">
              <button v-for="emoji in group.emojis" :key="emoji" type="button" class="ft-emoji" @click="insert(emoji)">{{ emoji }}</button>
            </div>
          </div>
        </div>
      </div>

      <div class="ft-menu" :class="{ 'ft-menu-open': open === 'hashtag' }">
        <button type="button" class="ft-btn" title="Insert studio hashtags" aria-label="Insert hashtag" :disabled="disabled" @click="toggle('hashtag')">
          <Icon name="hash" :size="14" :stroke="2" />
        </button>
        <div v-if="open === 'hashtag'" class="ft-pop ft-pop-tags">
          <div class="ft-pop-title">Studio Hashtags</div>
          <div class="ft-tags">
            <button v-for="tag in STUDIO_HASHTAGS" :key="tag" type="button" class="ft-tag" @click="insert(` ${tag}`)">{{ tag }}</button>
          </div>
        </div>
      </div>

      <div class="ft-menu" :class="{ 'ft-menu-open': open === 'hook' }">
        <button type="button" class="ft-btn ft-btn-text" title="Insert proven social hook template" :disabled="disabled" @click="toggle('hook')">
          <span class="ft-amber"><Icon name="sparkles" :size="12" :stroke="2" /></span> Hooks <Icon name="chevronDown" :size="10" :stroke="2" />
        </button>
        <div v-if="open === 'hook'" class="ft-pop ft-pop-hooks">
          <div class="ft-pop-title">Editorial Hook Formulas</div>
          <button
            v-for="hook in HOOK_TEMPLATES"
            :key="hook.label"
            type="button"
            class="ft-hook"
            @click="insert(text ? `\n\n${hook.template}` : hook.template)"
          >
            <span class="ft-hook-label">{{ hook.label }}</span>
            <span class="ft-hook-text">{{ hook.template.split("\n")[0] }}</span>
          </button>
        </div>
      </div>

      <button
        type="button"
        class="ft-btn ft-clear"
        title="Clear formatting (strip Unicode / Markdown styling)"
        aria-label="Clear formatting"
        :disabled="disabled"
        @click="apply('clear')"
      >
        <Icon name="rotateCcw" :size="14" :stroke="2" />
      </button>
    </div>

    <div v-if="showModeSwitch" class="ft-mode">
      <span>Mode:</span>
      <button
        type="button"
        :class="{ active: formatMode === 'unicode' }"
        title="Unicode bold/italic renders natively in LinkedIn, X, and Facebook feeds"
        @click="emit('update:formatMode', 'unicode')"
      >
        Unicode
      </button>
      <button
        type="button"
        :class="{ active: formatMode === 'markdown' }"
        title="Markdown (**bold**, *italic*) for Website/Blog CMS"
        @click="emit('update:formatMode', 'markdown')"
      >
        Markdown
      </button>
    </div>
  </div>
</template>
