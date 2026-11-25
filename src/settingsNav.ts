import { ref } from "vue";

export type SettingsSection = "skills" | "offerSkills" | "prompts" | "agents" | "images" | "tokens" | "updates";

/** The Settings section to open next, when another page sends the user there. */
export const settingsSection = ref<SettingsSection | null>(null);
