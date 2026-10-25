<script setup lang="ts">
import { useId } from "vue";
import { useOverflowMenu } from "../composables/useOverflowMenu";
import type { RemoteEntry } from "../types";

const props = withDefaults(
  defineProps<{
    remotes: RemoteEntry[];
    selected: string;
    busy: boolean;
    showCounts?: boolean;
  }>(),
  { showCounts: true },
);

const emit = defineEmits<{
  select: [name: string];
  add: [];
}>();

const menuId = `remote-picker-${useId()}`;
const { isOpen, toggle, close } = useOverflowMenu(() => menuId);

function selectFromMenu(name: string) {
  close();
  if (name === props.selected) {
    return;
  }
  emit("select", name);
}

function addFromMenu() {
  close();
  emit("add");
}
</script>

<template>
  <div class="overflow-menu branch-menu">
    <button
      class="branch-switch"
      type="button"
      :disabled="busy"
      :aria-expanded="isOpen"
      aria-haspopup="menu"
      :title="`Switch remote from ${selected}`"
      @click="toggle"
    >
      <span class="branch-switch-name">{{ selected || "No remote" }}</span>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M4.2 6.2L8 10l3.8-3.8" />
      </svg>
    </button>
    <div
      v-if="isOpen"
      class="overflow-menu-dropdown branch-menu-dropdown"
      role="menu"
      aria-label="Remotes"
    >
      <button
        class="overflow-menu-item"
        type="button"
        role="menuitem"
        :disabled="busy"
        @click="addFromMenu"
      >
        Add remote…
      </button>
      <div class="context-menu-sep" />
      <div class="branch-menu-list">
        <button
          v-for="remote in remotes"
          :key="remote.name"
          class="overflow-menu-item branch-menu-branch"
          :class="{ active: remote.name === selected }"
          type="button"
          role="menuitem"
          :title="remote.fetchUrl"
          @click="selectFromMenu(remote.name)"
        >
          <span class="branch-menu-name">{{ remote.name }}</span>
          <span
            v-if="showCounts"
            class="branch-pill"
            :title="`${remote.branchCount} branches on ${remote.name}`"
          >
            {{ remote.branchCount }}
          </span>
        </button>
      </div>
    </div>
  </div>
</template>
