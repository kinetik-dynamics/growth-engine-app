import { ref } from "vue";
import { api, errorMessage, type Overview } from "./api";
import { createAutoRefresh } from "./autoRefresh";

/** Lives outside the Overview page so the data and the auto-refresh timer survive page switches. */
export const overview = ref<Overview | null>(null);
export const overviewLoaded = ref(false);
export const overviewError = ref("");
export const overviewRefreshing = ref(false);

export const overviewAutoRefresh = createAutoRefresh("overview.autoRefresh", () => loadOverview(), () => overviewRefreshing.value);

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
    overviewAutoRefresh.restart();
  }
}

/** Starts the auto-refresh clock; call while signed in. */
export function startOverviewTimer() {
  overviewAutoRefresh.start();
}

/** Stops the clock and forgets the data, so the next account starts clean. */
export function stopOverviewTimer() {
  overviewAutoRefresh.stop();
  overview.value = null;
  overviewLoaded.value = false;
  overviewError.value = "";
}
