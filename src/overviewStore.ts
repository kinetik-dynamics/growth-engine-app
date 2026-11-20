import { computed, ref, watch } from "vue";
import { api, errorMessage, type Overview } from "./api";

const AUTO_REFRESH_MS = 30 * 60 * 1000;
const AUTO_REFRESH_KEY = "overview.autoRefresh";

/** Lives outside the Overview page so the data and the auto-refresh timer survive page switches. */
export const overview = ref<Overview | null>(null);
export const overviewLoaded = ref(false);
export const overviewError = ref("");
export const overviewRefreshing = ref(false);
export const autoRefresh = ref(readAutoRefresh());

const nextRefreshAt = ref(0);
const now = ref(Date.now());
let ticker: number | undefined;

function readAutoRefresh() {
  try {
    return localStorage.getItem(AUTO_REFRESH_KEY) === "1";
  } catch {
    return false;
  }
}

watch(autoRefresh, (on) => {
  try {
    localStorage.setItem(AUTO_REFRESH_KEY, on ? "1" : "0");
  } catch {
    // Storage can be unavailable; the toggle still works for this session.
  }
  if (on) nextRefreshAt.value = Date.now() + AUTO_REFRESH_MS;
});

export const refreshCountdown = computed(() => {
  const seconds = Math.max(0, Math.ceil((nextRefreshAt.value - now.value) / 1000));
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
});

/** Keeps the current numbers on screen while a refresh is in flight. */
export async function loadOverview() {
  if (overviewRefreshing.value) return;
  overviewRefreshing.value = true;
  overviewError.value = "";
  try {
    overview.value = (await api.sitePage("overview")).overview;
    overviewLoaded.value = true;
  } catch (caught) {
    overviewError.value = errorMessage(caught);
  } finally {
    overviewRefreshing.value = false;
    nextRefreshAt.value = Date.now() + AUTO_REFRESH_MS;
  }
}

/** Starts the once-a-second clock behind the countdown; call while signed in. */
export function startOverviewTimer() {
  if (ticker !== undefined) return;
  nextRefreshAt.value = Date.now() + AUTO_REFRESH_MS;
  ticker = window.setInterval(() => {
    now.value = Date.now();
    if (autoRefresh.value && !overviewRefreshing.value && now.value >= nextRefreshAt.value) loadOverview();
  }, 1000);
}

/** Stops the clock and forgets the data, so the next account starts clean. */
export function stopOverviewTimer() {
  window.clearInterval(ticker);
  ticker = undefined;
  overview.value = null;
  overviewLoaded.value = false;
  overviewError.value = "";
}
