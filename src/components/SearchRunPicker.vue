<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import type { SearchRun } from "../api";
import Icon from "./Icon.vue";

const props = defineProps<{ runs: SearchRun[]; modelValue: number | null; totalLeads: number }>();
const emit = defineEmits<{ "update:modelValue": [value: number | null] }>();

const WEEK_MS = 7 * 24 * 60 * 60 * 1000;
const MAX_MONTH_CHIPS = 3;

const root = ref<HTMLElement | null>(null);
const input = ref<HTMLInputElement | null>(null);
const open = ref(false);
const query = ref("");
/** "all", "week", or a month key like "2026-11". */
const period = ref("all");

const selectedRun = computed(() => props.runs.find((run) => run.id === props.modelValue) ?? null);

function runDate(run: SearchRun) {
  return new Date(run.created_at).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

function monthKey(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
}

function inPeriod(run: SearchRun, key: string) {
  if (key === "all") return true;
  const created = new Date(run.created_at);
  if (key === "week") return Date.now() - created.getTime() <= WEEK_MS;
  return monthKey(created) === key;
}

const chips = computed(() => {
  const months = new Map<string, { label: string; count: number }>();
  for (const run of props.runs) {
    const created = new Date(run.created_at);
    const key = monthKey(created);
    const month = months.get(key);
    if (month) month.count++;
    else months.set(key, { label: created.toLocaleDateString(undefined, { month: "short", year: "numeric" }), count: 1 });
  }
  const monthChips = [...months.entries()]
    .sort(([a], [b]) => b.localeCompare(a))
    .slice(0, MAX_MONTH_CHIPS)
    .map(([key, month]) => ({ key, label: `${month.label} (${month.count})` }));
  return [
    { key: "all", label: `All runs (${props.runs.length})` },
    { key: "week", label: `This week (${props.runs.filter((run) => inPeriod(run, "week")).length})` },
    ...monthChips,
  ];
});

const matches = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return props.runs.filter((run) => {
    if (!inPeriod(run, period.value)) return false;
    if (!needle) return true;
    return [run.label, run.icp?.name, runDate(run)].some((field) => field?.toLowerCase().includes(needle));
  });
});

function toggle() {
  if (open.value) return close();
  query.value = "";
  period.value = "all";
  open.value = true;
  nextTick(() => input.value?.focus());
}

function close() {
  open.value = false;
}

function pick(value: number | null) {
  emit("update:modelValue", value);
  close();
}

function outside(event: MouseEvent) {
  if (root.value && !root.value.contains(event.target as Node)) close();
}

function onKeyDown(event: KeyboardEvent) {
  if (open.value && event.key === "Escape") close();
}

onMounted(() => {
  document.addEventListener("mousedown", outside);
  document.addEventListener("keydown", onKeyDown);
});
onUnmounted(() => {
  document.removeEventListener("mousedown", outside);
  document.removeEventListener("keydown", onKeyDown);
});
</script>

<template>
  <div ref="root" class="sr">
    <button type="button" class="sr-trigger" :aria-expanded="open" aria-haspopup="listbox" @click="toggle">
      <Icon name="search" :size="13" class="sr-muted" />
      <span class="sr-muted">Search run:</span>
      <span class="sr-trigger-label">{{ selectedRun ? selectedRun.label : "All leads" }}</span>
      <span class="sr-pill">{{ selectedRun ? selectedRun.leads_count : totalLeads }} leads</span>
      <Icon name="chevronDown" :size="13" class="sr-caret" :class="{ open }" />
    </button>

    <div v-if="open" class="sr-panel">
      <div class="sr-head">
        <div class="sr-search">
          <Icon name="search" :size="13" />
          <input ref="input" v-model="query" type="search" placeholder="Search runs by name, ICP or date…" aria-label="Search runs" />
        </div>
        <div class="sr-chips">
          <button
            v-for="chip in chips"
            :key="chip.key"
            type="button"
            class="sr-chip"
            :class="{ active: period === chip.key }"
            @click="period = chip.key"
          >
            {{ chip.label }}
          </button>
        </div>
      </div>

      <div class="sr-list" role="listbox">
        <div v-if="!runs.length" class="sr-empty">No search runs yet. Import leads with the Chrome extension and each import shows up here.</div>
        <div v-else-if="!matches.length" class="sr-empty">No search runs match.</div>
        <button
          v-for="run in matches"
          :key="run.id"
          type="button"
          class="sr-row"
          :class="{ active: run.id === modelValue }"
          role="option"
          :aria-selected="run.id === modelValue"
          @click="pick(run.id)"
        >
          <span class="sr-row-main">
            <span class="sr-row-top">
              <span class="sr-row-label">{{ run.label }}</span>
              <span v-if="run.id === modelValue" class="sr-active">ACTIVE</span>
            </span>
            <span class="sr-row-meta">
              <Icon name="calendar" :size="11" />
              <span>{{ runDate(run) }}</span>
              <span>· {{ run.platform === "google_maps" ? "Google Maps" : "LinkedIn" }}</span>
              <span v-if="run.icp" class="sr-row-icp">· {{ run.icp.name }}</span>
            </span>
          </span>
          <span class="sr-row-counts">
            <span class="sr-row-leads">{{ run.leads_count }} leads</span>
            <span class="sr-row-new">{{ run.imported_count }} new<template v-if="run.skipped_count > 0"> · {{ run.skipped_count }} already in queue</template></span>
          </span>
        </button>
      </div>

      <div class="sr-footer">
        <span>Each import from the Chrome extension is one run</span>
        <button type="button" class="lq-linkbtn" @click="pick(null)">Show all {{ totalLeads }} leads</button>
      </div>
    </div>
  </div>
</template>
