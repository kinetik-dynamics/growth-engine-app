<script setup lang="ts">
import { computed, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{ label: string; imageUrl?: string | null; domain?: string | null; size?: "sm" | "md"; round?: boolean }>(),
  { imageUrl: null, domain: null, size: "md", round: false },
);

const TONES = ["tone-blue", "tone-emerald", "tone-amber", "tone-violet", "tone-rose", "tone-cyan"];

const sources = computed(() =>
  [props.imageUrl, props.domain ? `https://www.google.com/s2/favicons?domain=${encodeURIComponent(props.domain)}&sz=64` : null].filter(
    (s): s is string => Boolean(s),
  ),
);
const index = ref(0);
watch(sources, () => (index.value = 0));

const src = computed(() => sources.value[index.value]);

const tone = computed(() => {
  let hash = 0;
  for (let i = 0; i < props.label.length; i++) hash = (hash * 31 + props.label.charCodeAt(i)) | 0;
  return TONES[Math.abs(hash) % TONES.length];
});

const initials = computed(() => {
  const parts = props.label.trim().split(/\s+/).filter(Boolean);
  return parts.length ? (parts[0][0] + (parts[1]?.[0] ?? "")).toUpperCase() : "?";
});

/** Google answers unknown sites with a generic 16px globe, so treat that as missing. */
function loaded(event: Event) {
  const img = event.target as HTMLImageElement;
  if (src.value?.includes("/s2/favicons") && img.naturalWidth <= 16) index.value++;
}
</script>

<template>
  <img
    v-if="src"
    :src="src"
    alt=""
    loading="lazy"
    referrerpolicy="no-referrer"
    class="avatar avatar-img"
    :class="[`avatar-${size}`, { 'avatar-round': round }]"
    @error="index++"
    @load="loaded"
  />
  <span v-else class="avatar" :class="[`avatar-${size}`, tone, { 'avatar-round': round }]">{{ initials }}</span>
</template>
