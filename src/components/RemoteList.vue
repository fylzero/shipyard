<script setup lang="ts">
import { computed, ref, watch } from "vue";
import BranchContextMenu from "./BranchContextMenu.vue";
import BranchFolderRow from "./BranchFolderRow.vue";
import BranchIcon from "./BranchIcon.vue";
import { filterBranches } from "./BranchesHeader.vue";
import PinButton from "./PinButton.vue";
import RemotePicker from "./RemotePicker.vue";
import {
  buildBranchTree,
  flattenBranchTree,
  loadOpenFolders,
  saveOpenFolders,
  type BranchRow,
} from "../branchTree";
import { formatCommitDate } from "../graphLayout";
import { pinnedBranches, setBranchPinned } from "../pinnedBranches";
import type { RemoteBranch, RemoteEntry, RemoteOverview } from "../types";

const props = defineProps<{
  remotes: RemoteEntry[];
  selected: string;
  overview: RemoteOverview | null;
  busy: boolean;
  loading: boolean;
  fetching: boolean;
  query: string;
  repoPath: string;
}>();

const emit = defineEmits<{
  select: [name: string];
  fetch: [];
  add: [];
  edit: [remote: RemoteEntry];
  remove: [remote: RemoteEntry];
  open: [url: string];
  checkout: [branch: RemoteBranch];
  merge: [branch: RemoteBranch];
  delete: [branch: RemoteBranch];
}>();

const nameMenu = ref<{ name: string; x: number; y: number } | null>(null);

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

const selectedRemote = computed(
  () => props.remotes.find((remote) => remote.name === props.selected) ?? null,
);

const branches = computed(() =>
  props.overview?.remote === props.selected ? props.overview.branches : [],
);

const visibleBranches = computed(() => filterBranches(branches.value, props.query));
const pinned = computed(() => new Set(pinnedBranches(props.repoPath)));
const openFolders = ref<Set<string>>(loadOpenFolders("remote", props.repoPath));

function onTop(branch: RemoteBranch) {
  return branch.isDefault || pinned.value.has(branch.name);
}

const topBranches = computed(() => branches.value.filter(onTop));
const tree = computed(() => buildBranchTree(branches.value.filter((branch) => !onTop(branch))));

// Searching shows a flat list so matches never hide inside collapsed folders.
const rows = computed((): BranchRow<RemoteBranch>[] => {
  const flat = (branch: RemoteBranch): BranchRow<RemoteBranch> => ({
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

function toggleFolder(path: string) {
  const next = new Set(openFolders.value);
  if (next.has(path)) {
    next.delete(path);
  } else {
    next.add(path);
  }
  openFolders.value = next;
  saveOpenFolders("remote", props.repoPath, next);
}

function togglePin(name: string) {
  setBranchPinned(props.repoPath, name, !pinned.value.has(name));
}

watch(
  () => props.repoPath,
  (path) => {
    openFolders.value = loadOpenFolders("remote", path);
  },
);

function localTitle(branch: RemoteBranch) {
  const full = `${branch.remote}/${branch.name}`;
  if (branch.tracked) {
    return `Local branch ${branch.local} tracks ${full}`;
  }
  return `Local branch ${branch.local} has the same name but tracks something else. Counts compare it with ${full}.`;
}

function behindTitle(branch: RemoteBranch) {
  return `${branch.remote}/${branch.name} has ${branch.behind} commits that ${branch.local} doesn't`;
}

function aheadTitle(branch: RemoteBranch) {
  return `${branch.local} has ${branch.ahead} commits that ${branch.remote}/${branch.name} doesn't`;
}

function checkoutTitle(branch: RemoteBranch) {
  if (branch.current) {
    return `Already on ${branch.local}`;
  }
  if (branch.tracked) {
    return `Switch to ${branch.local}`;
  }
  if (branch.local) {
    return `Create a new local branch that tracks ${branch.remote}/${branch.name}`;
  }
  return `Create local ${branch.name} tracking ${branch.remote}/${branch.name}`;
}

</script>

<template>
  <div class="branch-pane">
    <div v-if="!remotes.length" class="tag-pane-header">
      <p class="muted tiny">
        This repository has no remotes. Add one to fetch its branches, like
        <strong>upstream</strong> for the project you forked from.
      </p>
      <button class="ghost tiny commit" type="button" :disabled="busy" @click="emit('add')">
        Add remote
      </button>
    </div>
    <div v-else class="remote-header">
      <RemotePicker
        :remotes="remotes"
        :selected="selected"
        :busy="busy"
        @select="emit('select', $event)"
        @add="emit('add')"
      />
      <span
        v-if="selectedRemote"
        class="remote-url muted tiny"
        :title="
          selectedRemote.pushUrl !== selectedRemote.fetchUrl
            ? `Fetch: ${selectedRemote.fetchUrl}\nPush: ${selectedRemote.pushUrl}`
            : selectedRemote.fetchUrl
        "
      >
        {{ selectedRemote.fetchUrl }}
      </span>
      <div class="remote-header-actions">
        <button
          class="ghost tiny"
          type="button"
          :disabled="busy || fetching || !selectedRemote"
          :title="`Fetch branches from ${selected}`"
          @click="emit('fetch')"
        >
          Fetch
          <span v-if="fetching" class="spinner" aria-hidden="true" />
        </button>
        <button
          v-if="selectedRemote?.browseUrl"
          class="ghost tiny"
          type="button"
          :title="`Open ${selectedRemote.browseUrl}`"
          @click="emit('open', selectedRemote.browseUrl)"
        >
          Open
        </button>
        <button
          class="ghost tiny"
          type="button"
          :disabled="busy || !selectedRemote"
          :title="`Rename ${selected} or change its URL`"
          @click="selectedRemote && emit('edit', selectedRemote)"
        >
          Edit
        </button>
        <button
          class="ghost tiny danger"
          type="button"
          :disabled="busy || !selectedRemote"
          :title="`Remove ${selected} from this repository`"
          @click="selectedRemote && emit('remove', selectedRemote)"
        >
          Remove
        </button>
      </div>
    </div>
    <div v-if="remotes.length" class="graph-scroll branch-list">
      <p v-if="loading && !branches.length" class="muted tiny branch-list-hint">
        <span class="spinner" aria-hidden="true" />
        <span>Loading branches on {{ selected }}…</span>
      </p>
      <p v-else-if="fetching" class="muted tiny branch-list-hint">
        <span class="spinner" aria-hidden="true" />
        <span>Fetching {{ selected }}. The list updates when it finishes.</span>
      </p>
      <p v-if="!loading && !fetching && !branches.length" class="muted tiny empty-files">
        No branches on {{ selected }} yet. Fetch it to load them.
      </p>
      <p v-else-if="branches.length && !visibleBranches.length" class="muted tiny empty-files">
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
        />
        <div
          v-else
          class="branch-row remote-row"
          :class="{ current: row.branch.current, 'section-end': row.branch.name === lastTopName }"
          :style="row.depth ? { paddingLeft: `${0.85 + row.depth * 1.3}rem` } : undefined"
          @mousedown="keepNameUnselected"
          @contextmenu="openNameMenu($event, row.branch.name)"
        >
          <BranchIcon />
          <span class="branch-row-name" :title="`${row.branch.remote}/${row.branch.name}`">
            {{ row.name }}
          </span>
          <PinButton
            :name="row.branch.name"
            :pinned="pinned.has(row.branch.name)"
            @toggle="togglePin(row.branch.name)"
          />
          <span v-if="row.branch.isDefault" class="branch-pill" title="Default branch on this remote">
            Default
          </span>
          <span
            v-if="row.branch.local"
            class="branch-pill"
            :class="{ tracked: row.branch.tracked }"
            :title="localTitle(row.branch)"
          >
            {{ row.branch.tracked ? `Tracked by ${row.branch.local}` : `Local ${row.branch.local}` }}
          </span>
          <span v-if="row.branch.local && (row.branch.behind || row.branch.ahead)" class="sync-counts">
            <span v-if="row.branch.behind" class="sync-count behind" :title="behindTitle(row.branch)">
              ↓{{ row.branch.behind }}
            </span>
            <span v-if="row.branch.ahead" class="sync-count ahead" :title="aheadTitle(row.branch)">
              ↑{{ row.branch.ahead }}
            </span>
          </span>
          <span v-if="row.branch.subject" class="tag-row-message" :title="row.branch.subject">
            {{ row.branch.subject }}
          </span>
          <span v-if="row.branch.date" class="stash-row-date">{{ formatCommitDate(row.branch.date) }}</span>
          <div class="branch-row-actions">
            <button
              class="ghost tiny"
              type="button"
              :disabled="busy || row.branch.current"
              :title="checkoutTitle(row.branch)"
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
              :disabled="busy"
              :title="`Bring ${row.branch.remote}/${row.branch.name} into a local branch`"
              @click="emit('merge', row.branch)"
            >
              Merge into…
            </button>
            <button
              class="ghost tiny danger"
              type="button"
              :disabled="busy || row.branch.isDefault"
              :title="
                row.branch.isDefault
                  ? 'The default branch cannot be deleted from here'
                  : `Delete ${row.branch.name} on ${row.branch.remote}`
              "
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
  </div>
</template>
