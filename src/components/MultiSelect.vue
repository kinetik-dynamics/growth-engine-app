<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import Icon from "./Icon.vue";

const props = defineProps<{
  options: { id: number; label: string }[];
  modelValue: number[];
  placeholder?: string;
  disabled?: boolean;
}>();
const emit = defineEmits<{ "update:modelValue": [value: number[]] }>();

const root = ref<HTMLElement | null>(null);
const input = ref<HTMLInputElement | null>(null);
const query = ref("");
const open = ref(false);

const selected = computed(() =>
  props.modelValue.flatMap((id) => props.options.filter((option) => option.id === id)),
);
const matches = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return props.options.filter((option) => !needle || option.label.toLowerCase().includes(needle));
});

function toggle(id: number) {
  const next = props.modelValue.includes(id) ? props.modelValue.filter((x) => x !== id) : [...props.modelValue, id];
  emit("update:modelValue", next);
  input.value?.focus();
}

function remove(id: number) {
  emit("update:modelValue", props.modelValue.filter((x) => x !== id));
}

function onBackspace() {
  if (!query.value && props.modelValue.length) remove(props.modelValue[props.modelValue.length - 1]);
}

function outside(event: MouseEvent) {
  if (root.value && !root.value.contains(event.target as Node)) open.value = false;
}

onMounted(() => document.addEventListener("mousedown", outside));
onUnmounted(() => document.removeEventListener("mousedown", outside));
</script>

<template>
  <div ref="root" class="ms" :class="{ disabled }">
    <div class="ms-box" @click="!disabled && ((open = true), input?.focus())">
      <span v-for="item in selected" :key="item.id" class="ms-chip">
        <span class="ms-chip-text">{{ item.label }}</span>
        <button type="button" class="ms-chip-x" :aria-label="`Remove ${item.label}`" :disabled="disabled" @click.stop="remove(item.id)">
          <Icon name="x" :size="11" />
        </button>
      </span>
      <input
        ref="input"
        v-model="query"
        class="ms-input"
        :disabled="disabled"
        :placeholder="modelValue.length ? '' : placeholder"
        role="combobox"
        :aria-expanded="open"
        @focus="open = true"
        @keydown.esc="open = false"
        @keydown.delete="onBackspace"
      />
      <Icon name="chevronDown" :size="14" class="ms-caret" />
    </div>
    <ul v-if="open && !disabled" class="ms-list" role="listbox" aria-multiselectable="true">
      <li v-if="!matches.length" class="ms-empty">No matches</li>
      <li
        v-for="option in matches"
        :key="option.id"
        class="ms-option"
        :class="{ on: modelValue.includes(option.id) }"
        role="option"
        :aria-selected="modelValue.includes(option.id)"
        @mousedown.prevent="toggle(option.id)"
      >
        <span class="ms-option-text">{{ option.label }}</span>
        <Icon v-if="modelValue.includes(option.id)" name="check" :size="13" />
      </li>
    </ul>
  </div>
</template>

<style scoped>
.ms { position: relative; }
.ms-box { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; min-height: 34px; padding: 4px 28px 4px 8px; border: 1px solid #d1d5db; border-radius: 6px; background: #fff; cursor: text; position: relative; }
.ms:focus-within .ms-box { border-color: #3b82f6; }
.ms.disabled .ms-box { background: #f9fafb; cursor: not-allowed; }
.ms-chip { display: inline-flex; align-items: center; gap: 4px; max-width: 100%; padding: 2px 4px 2px 8px; border-radius: 999px; background: #eff6ff; border: 1px solid #bfdbfe; color: #1d4ed8; font-size: 12px; }
.ms-chip-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 260px; }
.ms-chip-x { display: flex; align-items: center; justify-content: center; width: 16px; height: 16px; border: 0; border-radius: 50%; background: transparent; color: inherit; cursor: pointer; }
.ms-chip-x:hover { background: #dbeafe; }
.ms-input { flex: 1; min-width: 120px; border: 0; outline: none; background: transparent; font: inherit; font-size: 12px; color: #111827; padding: 4px 0; }
.ms-caret { position: absolute; right: 8px; top: 50%; transform: translateY(-50%); color: #6b7280; pointer-events: none; }
.ms-list { position: absolute; z-index: 20; left: 0; right: 0; top: calc(100% + 4px); max-height: 240px; overflow-y: auto; margin: 0; padding: 4px; list-style: none; border: 1px solid #e5e7eb; border-radius: 8px; background: #fff; box-shadow: 0 8px 24px rgb(0 0 0 / 0.12); }
.ms-option { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 7px 10px; border-radius: 6px; font-size: 12px; color: #111827; cursor: pointer; }
.ms-option:hover { background: #f3f4f6; }
.ms-option.on { color: #1d4ed8; font-weight: 600; }
.ms-option-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ms-empty { padding: 10px; font-size: 12px; color: #6b7280; text-align: center; }
</style>
