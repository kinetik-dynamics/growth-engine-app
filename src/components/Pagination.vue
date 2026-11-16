<script setup lang="ts">
defineProps<{
  page: number;
  lastPage: number;
  total: number;
  /** What is being paged, singular and plural: ["offer", "offers"]. */
  noun: [string, string];
  busy?: boolean;
}>();
const emit = defineEmits<{ change: [page: number] }>();
</script>

<template>
  <nav v-if="lastPage > 1" class="pagination" aria-label="Pagination">
    <button class="btn btn-sm" :disabled="busy || page <= 1" @click="emit('change', page - 1)">Previous</button>
    <span class="muted small" aria-live="polite">
      Page {{ page }} of {{ lastPage }} · {{ total }} {{ total === 1 ? noun[0] : noun[1] }}
    </span>
    <button class="btn btn-sm" :disabled="busy || page >= lastPage" @click="emit('change', page + 1)">Next</button>
  </nav>
</template>
