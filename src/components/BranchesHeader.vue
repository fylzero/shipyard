<script lang="ts">
export type BranchesView = "local" | "remotes";

export function filterBranches<T extends { name: string }>(branches: T[], query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return branches;
  }
  return branches.filter((branch) => branch.name.toLowerCase().includes(needle));
}
</script>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

defineProps<{
  view: BranchesView;
  busy: boolean;
  total: number;
  visible: number;
}>();

const emit = defineEmits<{
  select: [view: BranchesView];
}>();

const query = defineModel<string>("query", { required: true });
const searchInput = ref<HTMLInputElement | null>(null);

function clearSearch() {
  query.value = "";
  searchInput.value?.focus();
}

function onSearchKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") {
    return;
  }
  event.stopPropagation();
  if (query.value) {
    query.value = "";
    return;
  }
  searchInput.value?.blur();
}

function typingElsewhere(target: EventTarget | null) {
  if (!(target instanceof HTMLElement) || target === searchInput.value) {
    return false;
  }
  return target.isContentEditable || target.closest("input, textarea, select") !== null;
}

function onKeydown(event: KeyboardEvent) {
  if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "f") {
    return;
  }
  // Repo tabs stay mounted while hidden; only the visible header should respond.
  if (!searchInput.value?.offsetParent || typingElsewhere(event.target)) {
    return;
  }
  event.preventDefault();
  searchInput.value?.focus();
  searchInput.value?.select();
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="branch-search">
    <div class="segmented" role="group" aria-label="Branches view">
      <button
        type="button"
        :class="{ active: view === 'local' }"
        :aria-pressed="view === 'local'"
        :disabled="busy"
        title="Branches in this repository"
        @click="emit('select', 'local')"
      >
        Local
      </button>
      <button
        type="button"
        :class="{ active: view === 'remotes' }"
        :aria-pressed="view === 'remotes'"
        :disabled="busy"
        title="Remotes and their branches"
        @click="emit('select', 'remotes')"
      >
        Remote
      </button>
    </div>
    <label class="branch-search-field">
      <span class="sr-only">Filter branches</span>
      <svg class="branch-search-icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
      </svg>
      <input
        ref="searchInput"
        v-model="query"
        type="text"
        class="branch-search-query"
        placeholder="Filter branches"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown="onSearchKeydown"
      />
      <button
        v-if="query"
        class="file-tree-clear"
        type="button"
        title="Clear search"
        @click="clearSearch"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 18 18 6M6 6l12 12" />
        </svg>
      </button>
    </label>
    <span v-if="query.trim()" class="muted tiny branch-search-count">
      {{ visible }} of {{ total }}
    </span>
  </div>
</template>
