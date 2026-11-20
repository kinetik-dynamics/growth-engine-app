<script setup lang="ts">
import type { IconName } from "../icons";
import Icon from "./Icon.vue";

defineProps<{ icon: IconName; title: string; hint?: string }>();
</script>

<template>
  <section class="sc">
    <header class="sc-head">
      <div class="sc-title">
        <span class="sc-icon"><Icon :name="icon" :size="14" :stroke="2" /></span>
        <div style="min-width: 0">
          <h2>{{ title }}</h2>
          <p v-if="hint">{{ hint }}</p>
        </div>
      </div>
      <slot name="action" />
    </header>
    <div class="sc-body"><slot /></div>
  </section>
</template>
