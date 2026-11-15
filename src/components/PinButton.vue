<script setup lang="ts">
defineProps<{
  name: string;
  pinned: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
}>();
</script>

<template>
  <button
    class="branch-pin"
    :class="{ pinned }"
    type="button"
    :aria-pressed="pinned"
    :title="pinned ? `Unpin ${name}` : `Pin ${name} to the top`"
    @click="emit('toggle')"
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 17v5" />
      <path
        d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"
      />
    </svg>
  </button>
</template>
