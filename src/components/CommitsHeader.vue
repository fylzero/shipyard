<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";

const props = defineProps<{
  index: number;
  total: number;
  older: number;
  searching: boolean;
  truncated: boolean;
}>();

const emit = defineEmits<{
  step: [delta: number];
  open: [];
}>();

const query = defineModel<string>("query", { required: true });
const searchInput = ref<HTMLInputElement | null>(null);

const status = computed(() => {
  if (!query.value.trim()) {
    return "";
  }
  if (props.total) {
    return `${props.index + 1} of ${props.total}${props.truncated ? "+" : ""}`;
  }
  if (props.searching) {
    return "Searching…";
  }
  return props.older ? "Not in graph" : "No matches";
});

function clearSearch() {
  query.value = "";
  searchInput.value?.focus();
}

function onSearchKeydown(event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    if (event.metaKey || event.ctrlKey) {
      emit("open");
      return;
    }
    emit("step", event.shiftKey ? -1 : 1);
    return;
  }
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
    <label class="branch-search-field">
      <span class="sr-only">Search commits</span>
      <svg class="branch-search-icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
      </svg>
      <input
        ref="searchInput"
        v-model="query"
        type="text"
        class="branch-search-query"
        placeholder="Search by SHA or message"
        title="Enter for next match, Shift+Enter for previous, ⌘/Ctrl+Enter to open"
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
    <template v-if="query.trim()">
      <span class="muted tiny branch-search-count" aria-live="polite">{{ status }}</span>
      <div class="commit-search-steps">
        <button
          class="ghost tiny"
          type="button"
          title="Previous match (Shift+Enter)"
          aria-label="Previous match"
          :disabled="total < 2"
          @click="emit('step', -1)"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4.5 15.75l7.5-7.5 7.5 7.5" />
          </svg>
        </button>
        <button
          class="ghost tiny"
          type="button"
          title="Next match (Enter)"
          aria-label="Next match"
          :disabled="total < 2"
          @click="emit('step', 1)"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
          </svg>
        </button>
      </div>
    </template>
  </div>
</template>
