import { ref } from "vue";

export type SettingsSection = "skills" | "agents" | "images" | "updates";

/** The Settings section to open next, when another page sends the user there. */
export const settingsSection = ref<SettingsSection | null>(null);
