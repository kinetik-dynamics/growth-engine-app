import { ref, readonly } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const _update = ref<Update | null>(null);
const _checking = ref(false);
const _error = ref("");
const _progress = ref(0);
const _installing = ref(false);

export const updateAvailable = readonly(_update);
export const updateChecking = readonly(_checking);
export const updateError = readonly(_error);
export const updateProgress = readonly(_progress);
export const updateInstalling = readonly(_installing);

export async function checkForUpdate(): Promise<void> {
  if (_checking.value) return;
  _checking.value = true;
  _error.value = "";
  try {
    _update.value = await check();
  } catch (e) {
    _error.value = typeof e === "string" ? e : String(e);
  } finally {
    _checking.value = false;
  }
}

export async function installUpdate(): Promise<void> {
  if (!_update.value || _installing.value) return;
  _installing.value = true;
  _progress.value = 0;
  _error.value = "";
  try {
    let downloaded = 0;
    let total: number | undefined;
    await _update.value.downloadAndInstall((event) => {
      if (event.event === "Started") total = event.data.contentLength;
      if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        if (total) _progress.value = Math.round((downloaded / total) * 100);
      }
      if (event.event === "Finished") _progress.value = 100;
    });
    await relaunch();
  } catch (e) {
    _error.value = typeof e === "string" ? e : String(e);
    _installing.value = false;
  }
}
