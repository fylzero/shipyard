<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, useId } from "vue";
import BranchIcon from "./BranchIcon.vue";
import { useOverflowMenu } from "../composables/useOverflowMenu";
import { formatCommitDate } from "../graphLayout";
import type { RemoteBranch, RemoteEntry, RemoteOverview } from "../types";

const props = defineProps<{
  remotes: RemoteEntry[];
  selected: string;
  overview: RemoteOverview | null;
  busy: boolean;
  loading: boolean;
  fetching: boolean;
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

const menuId = `remote-picker-${useId()}`;
const { isOpen, toggle, close } = useOverflowMenu(() => menuId);
const query = ref("");
const searchInput = ref<HTMLInputElement | null>(null);

const selectedRemote = computed(
  () => props.remotes.find((remote) => remote.name === props.selected) ?? null,
);

const branches = computed(() =>
  props.overview?.remote === props.selected ? props.overview.branches : [],
);

const visibleBranches = computed(() => {
  const needle = query.value.trim().toLowerCase();
  if (!needle) {
    return branches.value;
  }
  return branches.value.filter((branch) => branch.name.toLowerCase().includes(needle));
});

function selectFromMenu(name: string) {
  close();
  if (name === props.selected) {
    return;
  }
  query.value = "";
  emit("select", name);
}

function addFromMenu() {
  close();
  emit("add");
}

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
  // Repo tabs stay mounted while hidden; only the visible list should respond.
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
                class="branch-pill"
                :title="`${remote.branchCount} branches on ${remote.name}`"
              >
                {{ remote.branchCount }}
              </span>
            </button>
          </div>
        </div>
      </div>
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
    <div v-if="branches.length" class="branch-search">
      <label class="branch-search-field">
        <span class="sr-only">Filter remote branches</span>
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
        {{ visibleBranches.length }} of {{ branches.length }}
      </span>
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
      <div
        v-for="branch in visibleBranches"
        :key="branch.name"
        class="branch-row remote-row"
        :class="{ current: branch.current }"
      >
        <BranchIcon />
        <span class="branch-row-name" :title="`${branch.remote}/${branch.name}`">
          {{ branch.name }}
        </span>
        <span v-if="branch.isDefault" class="branch-pill" title="Default branch on this remote">
          Default
        </span>
        <span
          v-if="branch.local"
          class="branch-pill"
          :class="{ tracked: branch.tracked }"
          :title="localTitle(branch)"
        >
          {{ branch.tracked ? `Tracked by ${branch.local}` : `Local ${branch.local}` }}
        </span>
        <span v-if="branch.local && (branch.behind || branch.ahead)" class="sync-counts">
          <span v-if="branch.behind" class="sync-count behind" :title="behindTitle(branch)">
            ↓{{ branch.behind }}
          </span>
          <span v-if="branch.ahead" class="sync-count ahead" :title="aheadTitle(branch)">
            ↑{{ branch.ahead }}
          </span>
        </span>
        <span v-if="branch.subject" class="tag-row-message" :title="branch.subject">
          {{ branch.subject }}
        </span>
        <span v-if="branch.date" class="stash-row-date">{{ formatCommitDate(branch.date) }}</span>
        <div class="branch-row-actions">
          <button
            class="ghost tiny"
            type="button"
            :disabled="busy || branch.current"
            :title="checkoutTitle(branch)"
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
            :disabled="busy"
            :title="`Bring ${branch.remote}/${branch.name} into a local branch`"
            @click="emit('merge', branch)"
          >
            Merge into…
          </button>
          <button
            class="ghost tiny danger"
            type="button"
            :disabled="busy || branch.isDefault"
            :title="
              branch.isDefault
                ? 'The default branch cannot be deleted from here'
                : `Delete ${branch.name} on ${branch.remote}`
            "
            @click="emit('delete', branch)"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
