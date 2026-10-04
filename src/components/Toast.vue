<script setup lang="ts">
defineProps<{
  message: string;
  kind?: "success" | "error";
  hasDetails?: boolean;
}>();
const emit = defineEmits<{ dismiss: []; details: []; pause: []; resume: [] }>();
</script>

<template>
  <div class="toast-layer" role="status" aria-live="polite">
    <div
      class="toast"
      :class="kind ?? 'success'"
      @mouseenter="emit('pause')"
      @mouseleave="emit('resume')"
    >
      <span class="toast-icon" aria-hidden="true">
        <svg v-if="kind === 'error'" viewBox="0 0 16 16" fill="currentColor">
          <path
            d="M8 16A8 8 0 1 1 8 0a8 8 0 0 1 0 16ZM7.25 4.75a.75.75 0 0 1 1.5 0v4.5a.75.75 0 0 1-1.5 0v-4.5Zm.75 8.25a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"
          />
        </svg>
        <svg v-else viewBox="0 0 16 16" fill="currentColor">
          <path
            d="M8 16A8 8 0 1 1 8 0a8 8 0 0 1 0 16Zm3.78-9.72a.75.75 0 0 0-1.06-1.06L6.75 9.19 5.28 7.72a.75.75 0 0 0-1.06 1.06l2 2a.75.75 0 0 0 1.06 0Z"
          />
        </svg>
      </span>
      <span class="toast-copy">{{ message }}</span>
      <button v-if="hasDetails" class="toast-action" type="button" @click="emit('details')">
        View details
      </button>
      <button class="toast-close" type="button" aria-label="Dismiss" @click="emit('dismiss')">
        <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <path
            d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z"
          />
        </svg>
      </button>
    </div>
  </div>
</template>
