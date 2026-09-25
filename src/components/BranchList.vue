<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import BranchIcon from "./BranchIcon.vue";
import { rangeIds, toggleId } from "../selection";
import type { BranchOverview, LocalBranch } from "../types";

const props = defineProps<{
  overview: BranchOverview | null;
  busy: boolean;
}>();

const emit = defineEmits<{
  checkout: [branch: LocalBranch];
  merge: [branch: LocalBranch];
  rename: [branch: LocalBranch];
  delete: [branch: LocalBranch];
  deleteMerged: [];
  deleteSelected: [branches: LocalBranch[]];
}>();

const selected = ref<string[]>([]);
const anchor = ref<string | null>(null);
const query = ref("");
const searchInput = ref<HTMLInputElement | null>(null);
const listEl = ref<HTMLElement | null>(null);

const visibleBranches = computed(() => {
  const branches = props.overview?.branches ?? [];
  const needle = query.value.trim().toLowerCase();
  if (!needle) {
    return branches;
  }
  return branches.filter((branch) => branch.name.toLowerCase().includes(needle));
});

const leftoverCount = computed(
  () =>
    props.overview?.branches.filter(
      (branch) => branch.merged && !branch.current && !branch.protected && !branch.pending,
    ).length ?? 0,
);

const canMerge = computed(() => (props.overview?.branches.length ?? 0) >= 2);

const classifying = computed(
  () => props.overview?.branches.some((branch) => branch.pending) ?? false,
);

const selectableNames = computed(() =>
  visibleBranches.value.filter((branch) => canSelect(branch)).map((branch) => branch.name),
);

const selectedBranches = computed(() =>
  visibleBranches.value.filter(
    (branch) => selected.value.includes(branch.name) && canSelect(branch),
  ),
);

function canSelect(branch: LocalBranch) {
  return !branch.current && !branch.protected;
}

function isLeftover(branch: LocalBranch) {
  return branch.merged && !branch.current && !branch.protected && !branch.pending;
}

function isPartial(branch: LocalBranch) {
  return branch.partial && !branch.protected && !branch.pending;
}

function isSelected(branch: LocalBranch) {
  return selected.value.includes(branch.name);
}

function onRowClick(event: MouseEvent, branch: LocalBranch) {
  const target = event.target;
  if (target instanceof Element && target.closest("button")) {
    return;
  }
  if (!canSelect(branch)) {
    return;
  }
  if (event.shiftKey && anchor.value) {
    selected.value = rangeIds(selectableNames.value, anchor.value, branch.name);
    return;
  }
  if (event.metaKey || event.ctrlKey) {
    selected.value = toggleId(selected.value, branch.name);
    anchor.value = branch.name;
    return;
  }
  selected.value = [branch.name];
  anchor.value = branch.name;
}

function deleteSelected() {
  if (!selectedBranches.value.length) {
    return;
  }
  emit("deleteSelected", selectedBranches.value);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && selected.value.length) {
    selected.value = [];
  }
}

function clearQuery() {
  query.value = "";
  searchInput.value?.focus();
}

function onSearchKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && query.value) {
    event.stopPropagation();
    event.preventDefault();
    query.value = "";
  }
}

watch(query, () => {
  listEl.value?.scrollTo({ top: 0 });
});

watch(
  selectableNames,
  (names) => {
    const live = new Set(names);
    selected.value = selected.value.filter((name) => live.has(name));
    if (anchor.value && !live.has(anchor.value)) {
      anchor.value = selected.value[selected.value.length - 1] ?? null;
    }
  },
);

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="branch-pane">
    <div class="branch-list-header">
      <label class="branch-list-search">
        <span class="sr-only">Filter branches</span>
        <svg class="branch-menu-search-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
          />
        </svg>
        <input
          ref="searchInput"
          v-model="query"
          type="text"
          placeholder="Filter branches"
          autocomplete="off"
          spellcheck="false"
          @keydown="onSearchKeydown"
        />
        <button
          v-if="query"
          class="branch-menu-search-clear"
          type="button"
          title="Clear search"
          @mousedown.prevent
          @click="clearQuery"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </label>
      <span v-if="overview && query.trim()" class="muted tiny">
        {{ visibleBranches.length }} of {{ overview.branches.length }}
      </span>
    </div>
    <div ref="listEl" class="graph-scroll branch-list">
      <p v-if="!overview" class="muted tiny branch-list-hint">
        <span class="spinner" aria-hidden="true" />
        <span>Loading branches…</span>
      </p>
      <p v-else-if="classifying && overview.mergeTarget" class="muted tiny branch-list-hint">
        <span class="spinner" aria-hidden="true" />
        <span>
          Listed local branches. Checking leftover work against
          <strong>{{ overview.mergeTarget }}</strong>…
        </span>
      </p>
      <p v-else-if="overview.mergeTarget" class="muted tiny branch-list-hint">
        Merged marks leftover local work already contained in
        <strong>{{ overview.mergeTarget }}</strong>. A new branch that still points there is
        not leftover. Partial means some commits are in that branch and some are still unique.
        Pull first if you want the latest remote picture.
      </p>
      <p v-else class="muted tiny branch-list-hint">
        Couldn’t find origin/develop, develop, main, or master to compare against.
      </p>
      <p v-if="overview && !overview.branches.length" class="muted tiny empty-files">
        No local branches.
      </p>
      <p v-else-if="overview && !visibleBranches.length" class="muted tiny empty-files">
        No branches match that search.
      </p>
      <div
        v-for="branch in visibleBranches"
        :key="branch.name"
        class="branch-row"
        :class="{
          current: branch.current,
          leftover: isLeftover(branch),
          partial: isPartial(branch),
          selected: isSelected(branch),
        }"
        :aria-selected="isSelected(branch)"
        @click="onRowClick($event, branch)"
      >
        <BranchIcon />
        <span class="branch-row-name">{{ branch.name }}</span>
        <span v-if="branch.current" class="branch-pill">Current</span>
        <span
          v-if="branch.pending"
          class="action-progress muted tiny"
          :title="
            overview?.mergeTarget
              ? `Checking whether this is already contained in ${overview.mergeTarget}`
              : 'Checking whether this leftover work is already merged'
          "
        >
          Checking
          <span class="spinner" aria-hidden="true" />
        </span>
        <span
          v-else-if="isLeftover(branch)"
          class="branch-pill merged"
          :title="
            overview?.mergeTarget
              ? `Already contained in ${overview.mergeTarget}`
              : 'Already contained in the integration branch'
          "
        >
          Merged
        </span>
        <span
          v-else-if="isPartial(branch)"
          class="branch-pill partial"
          :title="
            overview?.mergeTarget
              ? `Some commits are in ${overview.mergeTarget}; others are still unique`
              : 'Some commits are already merged; others are still unique'
          "
        >
          Partial
        </span>
        <div class="branch-row-actions">
          <button
            class="ghost tiny"
            type="button"
            :disabled="busy || branch.current"
            :title="branch.current ? 'Already on this branch' : `Check out ${branch.name}`"
            @click="emit('checkout', branch)"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"
              />
            </svg>
            Checkout
          </button>
          <button
            class="ghost tiny"
            type="button"
            :disabled="busy || !canMerge"
            :title="
              canMerge
                ? `Merge ${branch.name} into another local branch`
                : 'Need another local branch to merge into'
            "
            @click="emit('merge', branch)"
          >
            Merge
          </button>
          <button
            class="ghost tiny"
            type="button"
            :disabled="busy"
            :title="`Rename ${branch.name}`"
            @click="emit('rename', branch)"
          >
            Rename
          </button>
          <button
            class="ghost tiny danger"
            type="button"
            :disabled="busy || branch.current"
            :title="branch.current ? 'Cannot delete the current branch' : `Delete ${branch.name}`"
            @click="emit('delete', branch)"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
    <div class="branch-footer">
      <button
        class="ghost tiny danger"
        type="button"
        :disabled="busy || !selectedBranches.length"
        :title="
          selectedBranches.length
            ? `Delete ${selectedBranches.length} selected branches`
            : 'Command-click or Shift-click branches to select them'
        "
        @click="deleteSelected"
      >
        Delete selected
        <span v-if="selectedBranches.length" class="file-count-badge">{{
          selectedBranches.length
        }}</span>
      </button>
      <button
        class="ghost tiny danger"
        type="button"
        :disabled="busy || leftoverCount === 0"
        @click="emit('deleteMerged')"
      >
        Delete merged branches
        <span v-if="leftoverCount" class="file-count-badge">{{ leftoverCount }}</span>
      </button>
      <p class="muted tiny branch-footer-hint">
        Delete merged only removes leftover merged branches. Partial and unique work stay. Keeps
        develop, main, master, and the branch you’re on.
      </p>
    </div>
  </div>
</template>
