<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import BranchContextMenu from "./BranchContextMenu.vue";
import BranchFolderRow from "./BranchFolderRow.vue";
import BranchIcon from "./BranchIcon.vue";
import { filterBranches } from "./BranchesHeader.vue";
import PinButton from "./PinButton.vue";
import {
  buildBranchTree,
  flattenBranchTree,
  loadOpenFolders,
  saveOpenFolders,
  type BranchRow,
} from "../branchTree";
import { pinnedBranches, setBranchPinned } from "../pinnedBranches";
import { commandKeyName } from "../platform";
import { rangeIds, toggleId } from "../selection";
import type { BranchOverview, LocalBranch } from "../types";

const props = defineProps<{
  overview: BranchOverview | null;
  busy: boolean;
  query: string;
  repoPath: string;
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
const nameMenu = ref<{ name: string; x: number; y: number } | null>(null);

const openFolders = ref<Set<string>>(loadOpenFolders("local", props.repoPath));

const visibleBranches = computed(() => filterBranches(props.overview?.branches ?? [], props.query));
const pinned = computed(() => new Set(pinnedBranches(props.repoPath)));

const topBranches = computed(() =>
  (props.overview?.branches ?? []).filter(
    (branch) => branch.current || pinned.value.has(branch.name),
  ),
);

const tree = computed(() =>
  buildBranchTree(
    (props.overview?.branches ?? []).filter(
      (branch) => !branch.current && !pinned.value.has(branch.name),
    ),
  ),
);

// Searching shows a flat list so matches never hide inside collapsed folders.
const rows = computed((): BranchRow<LocalBranch>[] => {
  const flat = (branch: LocalBranch): BranchRow<LocalBranch> => ({
    kind: "branch",
    name: branch.name,
    depth: 0,
    branch,
  });
  if (props.query.trim()) {
    return visibleBranches.value.map(flat);
  }
  return [...topBranches.value.map(flat), ...flattenBranchTree(tree.value, openFolders.value)];
});

const lastTopName = computed(() =>
  props.query.trim() || !tree.value.length ? null : topBranches.value[topBranches.value.length - 1]?.name ?? null,
);

const rowBranches = computed(() =>
  rows.value.flatMap((row) => (row.kind === "branch" ? [row.branch] : [])),
);

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
  rowBranches.value.filter((branch) => canSelect(branch)).map((branch) => branch.name),
);

const selectedBranches = computed(() =>
  rowBranches.value.filter(
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

function leftoverIn(branches: LocalBranch[]) {
  return branches.filter((branch) => isLeftover(branch)).length;
}

function toggleFolder(path: string) {
  const next = new Set(openFolders.value);
  if (next.has(path)) {
    next.delete(path);
  } else {
    next.add(path);
  }
  openFolders.value = next;
  saveOpenFolders("local", props.repoPath, next);
}

function togglePin(name: string) {
  setBranchPinned(props.repoPath, name, !pinned.value.has(name));
}

function keepNameUnselected(event: MouseEvent) {
  if (event.button === 2) {
    event.preventDefault();
  }
}

function openNameMenu(event: MouseEvent, name: string) {
  event.preventDefault();
  event.stopPropagation();
  window.getSelection()?.removeAllRanges();
  nameMenu.value = { name, x: event.clientX, y: event.clientY };
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

watch(
  () => props.repoPath,
  (path) => {
    openFolders.value = loadOpenFolders("local", path);
  },
);

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
    <div class="graph-scroll branch-list">
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
      <p v-else-if="!overview.mergeTarget" class="muted tiny branch-list-hint">
        Couldn’t find origin/develop, develop, main, or master to compare against.
      </p>
      <p v-if="overview && !overview.branches.length" class="muted tiny empty-files">
        No local branches.
      </p>
      <p v-else-if="overview && !visibleBranches.length" class="muted tiny empty-files">
        No branches match that search.
      </p>
      <template
        v-for="row in rows"
        :key="row.kind === 'folder' ? `folder:${row.path}` : row.branch.name"
      >
        <BranchFolderRow
          v-if="row.kind === 'folder'"
          :path="row.path"
          :name="row.name"
          :depth="row.depth"
          :open="row.open"
          :count="row.branches.length"
          @toggle="toggleFolder"
        >
          <span
            v-if="leftoverIn(row.branches)"
            class="branch-pill merged"
            :title="`${leftoverIn(row.branches)} merged branches in ${row.path}/`"
          >
            {{ leftoverIn(row.branches) }} merged
          </span>
        </BranchFolderRow>
        <div
          v-else
          class="branch-row"
          :class="{
            current: row.branch.current,
            leftover: isLeftover(row.branch),
            partial: isPartial(row.branch),
            selected: isSelected(row.branch),
            'section-end': row.branch.name === lastTopName,
          }"
          :style="row.depth ? { paddingLeft: `${0.85 + row.depth * 1.3}rem` } : undefined"
          :title="row.name !== row.branch.name ? row.branch.name : undefined"
          :aria-selected="isSelected(row.branch)"
          @click="onRowClick($event, row.branch)"
          @mousedown="keepNameUnselected"
          @contextmenu="openNameMenu($event, row.branch.name)"
        >
          <BranchIcon />
          <span class="branch-row-name">{{ row.name }}</span>
          <PinButton
            :name="row.branch.name"
            :pinned="pinned.has(row.branch.name)"
            @toggle="togglePin(row.branch.name)"
          />
          <span v-if="row.branch.current" class="branch-pill">Current</span>
          <span
            v-if="row.branch.pending"
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
            v-else-if="isLeftover(row.branch)"
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
            v-else-if="isPartial(row.branch)"
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
              :disabled="busy || row.branch.current"
              :title="row.branch.current ? 'Already on this branch' : `Check out ${row.branch.name}`"
              @click="emit('checkout', row.branch)"
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
                  ? `Merge ${row.branch.name} into another local branch`
                  : 'Need another local branch to merge into'
              "
              @click="emit('merge', row.branch)"
            >
              Merge
            </button>
            <button
              class="ghost tiny"
              type="button"
              :disabled="busy"
              :title="`Rename ${row.branch.name}`"
              @click="emit('rename', row.branch)"
            >
              Rename
            </button>
            <button
              class="ghost tiny danger"
              type="button"
              :disabled="busy || row.branch.current"
              :title="row.branch.current ? 'Cannot delete the current branch' : `Delete ${row.branch.name}`"
              @click="emit('delete', row.branch)"
            >
              Delete
            </button>
          </div>
        </div>
      </template>
    </div>
    <BranchContextMenu
      v-if="nameMenu"
      :branch="nameMenu.name"
      :x="nameMenu.x"
      :y="nameMenu.y"
      :pinned="pinned.has(nameMenu.name)"
      @toggle-pin="togglePin(nameMenu.name)"
      @close="nameMenu = null"
    />
    <div class="branch-footer">
      <button
        class="ghost tiny danger"
        type="button"
        :disabled="busy || !selectedBranches.length"
        :title="
          selectedBranches.length
            ? `Delete ${selectedBranches.length} selected branches`
            : `${commandKeyName}-click or Shift-click branches to select them`
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
