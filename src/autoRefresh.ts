import { computed, ref, watch } from "vue";

export const AUTO_REFRESH_MS = 30 * 60 * 1000;

/**
 * A remembered on/off switch plus a once-a-second clock that calls
 * `refresh` every 30 minutes while switched on. Call `restart()` after each
 * refresh, manual or automatic, so the countdown starts over.
 */
export function createAutoRefresh(storageKey: string, refresh: () => unknown, isBusy: () => boolean) {
  const enabled = ref(read());
  const nextAt = ref(Date.now() + AUTO_REFRESH_MS);
  const now = ref(Date.now());
  let ticker: number | undefined;

  function read() {
    try {
      return localStorage.getItem(storageKey) === "1";
    } catch {
      return false;
    }
  }

  watch(enabled, (on) => {
    try {
      localStorage.setItem(storageKey, on ? "1" : "0");
    } catch {
      // Storage can be unavailable; the toggle still works for this session.
    }
    if (on) restart();
  });

  function restart() {
    nextAt.value = Date.now() + AUTO_REFRESH_MS;
  }

  function start() {
    if (ticker !== undefined) return;
    restart();
    ticker = window.setInterval(() => {
      now.value = Date.now();
      if (enabled.value && !isBusy() && now.value >= nextAt.value) {
        restart();
        refresh();
      }
    }, 1000);
  }

  function stop() {
    window.clearInterval(ticker);
    ticker = undefined;
  }

  const countdown = computed(() => {
    const seconds = Math.max(0, Math.ceil((nextAt.value - now.value) / 1000));
    return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
  });

  return { enabled, countdown, restart, start, stop };
}
