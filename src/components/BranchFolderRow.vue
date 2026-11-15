<script setup lang="ts">
defineProps<{
  path: string;
  name: string;
  depth: number;
  open: boolean;
  count: number;
}>();

const emit = defineEmits<{
  toggle: [path: string];
}>();
</script>

<template>
  <button
    class="branch-row branch-folder-row"
    type="button"
    :style="{ paddingLeft: `${0.85 + depth * 1.3}rem` }"
    :aria-expanded="open"
    :title="`${path}/`"
    @click="emit('toggle', path)"
  >
    <svg class="file-tree-twist" :class="{ open }" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M9 6.75l6.75 5.25L9 17.25" />
    </svg>
    <span class="branch-row-name">{{ name }}/</span>
    <span class="branch-pill" :title="`${count} branches in ${path}/`">{{ count }}</span>
    <slot />
  </button>
</template>
