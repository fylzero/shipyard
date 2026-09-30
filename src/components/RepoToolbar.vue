<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import BranchIcon from "./BranchIcon.vue";
import SplitAction from "./SplitAction.vue";
import { useApp } from "../composables/useApp";
import { useOverflowMenu } from "../composables/useOverflowMenu";
import type { BranchTracking } from "../types";

const props = defineProps<{
  repoId: string;
  name: string;
  branch: string;
  path: string;
  branches: string[];
  branchTracking?: BranchTracking[];
  busy: boolean;
  busyLabel: string;
  busyBranch?: string;
}>();

const emit = defineEmits<{
  fetch: [];
  pull: [];
  pullOptions: [];
  push: [];
  undoUnpushed: [];
  checkout: [branch: string];
  create: [];
  merge: [];
  refreshBranches: [];
}>();

const { statuses } = useApp();
const { isOpen, toggle, close } = useOverflowMenu(() => `branch-${props.repoId}`);
const branchQuery = ref("");
const branchSearchInput = ref<HTMLInputElement | null>(null);
const branchMenuEl = ref<HTMLElement | null>(null);
const branchListEl = ref<HTMLElement | null>(null);
const branchActiveIndex = ref(0);
const branchSearchMoved = ref(false);

const currentBranch = computed(
  () => statuses.value[props.repoId]?.branch || props.branch,
);

const pullTitle = computed(() =>
  currentBranch.value ? `Pull from ${currentBranch.value}` : "Pull from current branch",
);

const pushTitle = computed(() =>
  currentBranch.value ? `Push to ${currentBranch.value}` : "Push current branch",
);

const branchItems = computed(() => {
  const tracking = new Map((props.branchTracking ?? []).map((item) => [item.name, item]));
  return props.branches.map((name) => {
    const item = tracking.get(name);
    const remote = item?.upstream?.trim() || "";
    return {
      name,
      localOnly: item?.localOnly ?? false,
      ahead: item?.ahead ?? 0,
      behind: item?.behind ?? 0,
      aheadTitle: remote
        ? `${item?.ahead ?? 0} commits ahead of ${remote}`
        : `${item?.ahead ?? 0} commits ahead`,
      behindTitle: remote
        ? `${item?.behind ?? 0} commits behind ${remote}`
        : `${item?.behind ?? 0} commits behind`,
    };
  });
});

const filteredBranchItems = computed(() => {
  const needle = branchQuery.value.trim().toLowerCase();
  if (!needle) {
    return branchItems.value;
  }
  return branchItems.value.filter((item) => item.name.toLowerCase().includes(needle));
});

const unpushedCount = computed(() => statuses.value[props.repoId]?.ahead ?? 0);
const canUndoUnpushed = computed(
  () => unpushedCount.value > 0 && !statuses.value[props.repoId]?.operation,
);
const undoUnpushedTitle = computed(() => {
  const count = unpushedCount.value;
  const label = count === 1 ? "1 unpushed commit" : `${count} unpushed commits`;
  return `Undo ${label}. Changes stay staged.`;
});

const progressLabel = computed(() => props.busyLabel.replace(/…$/, "").trim());
const progressBranch = computed(() => props.busyBranch?.trim() || "");

function selectBranch(branch: string) {
  close();
  if (branch === props.branch) {
    return;
  }
  emit("checkout", branch);
}

function currentBranchIndex() {
  const index = branchItems.value.findIndex((item) => item.name === props.branch);
  return index >= 0 ? index : 0;
}

function scrollBranchIntoView(selector: string) {
  void nextTick(() => {
    const list = branchListEl.value;
    const item = list?.querySelector<HTMLElement>(selector);
    if (!list || !item) {
      return;
    }
    const listRect = list.getBoundingClientRect();
    const itemRect = item.getBoundingClientRect();
    if (itemRect.top < listRect.top) {
      list.scrollTop -= listRect.top - itemRect.top;
    } else if (itemRect.bottom > listRect.bottom) {
      list.scrollTop += itemRect.bottom - listRect.bottom;
    }
  });
}

function clearBranchQuery() {
  branchQuery.value = "";
  branchSearchInput.value?.focus();
}

function onBranchHover(index: number) {
  branchSearchMoved.value = true;
  branchActiveIndex.value = index;
}

function onBranchSearchKeydown(event: KeyboardEvent) {
  const items = filteredBranchItems.value;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (!items.length) {
      return;
    }
    const delta = event.key === "ArrowDown" ? 1 : -1;
    branchSearchMoved.value = true;
    branchActiveIndex.value = Math.min(items.length - 1, Math.max(0, branchActiveIndex.value + delta));
    scrollBranchIntoView(`[data-branch-index="${branchActiveIndex.value}"]`);
    return;
  }
  if (event.key === "Enter") {
    const item = items[branchActiveIndex.value];
    event.preventDefault();
    if (!item || (!branchQuery.value.trim() && !branchSearchMoved.value)) {
      close();
      return;
    }
    selectBranch(item.name);
    return;
  }
  if (event.key === "Escape" && branchQuery.value) {
    event.stopPropagation();
    event.preventDefault();
    branchQuery.value = "";
  }
}

watch(branchQuery, async (value) => {
  if (!value.trim()) {
    branchActiveIndex.value = currentBranchIndex();
    return;
  }
  branchActiveIndex.value = 0;
  await nextTick();
  branchListEl.value?.scrollTo({ top: 0 });
});

watch(filteredBranchItems, (items) => {
  if (branchActiveIndex.value >= items.length) {
    branchActiveIndex.value = Math.max(0, items.length - 1);
  }
});

watch(isOpen, async (open) => {
  if (!open) {
    branchQuery.value = "";
    branchSearchMoved.value = false;
    return;
  }
  branchSearchMoved.value = false;
  branchActiveIndex.value = currentBranchIndex();
  await nextTick();
  const menu = branchMenuEl.value;
  if (menu) {
    menu.style.minWidth = `${menu.getBoundingClientRect().width}px`;
  }
  branchSearchInput.value?.focus();
  scrollBranchIntoView(".branch-menu-branch.active");
});

function createBranch() {
  close();
  emit("create");
}

function mergeBranch() {
  close();
  if (props.branches.length < 2) {
    return;
  }
  emit("merge");
}

async function toggleBranches() {
  if (!isOpen.value) {
    emit("refreshBranches");
  }
  toggle();
}
</script>

<template>
  <div class="pane-header repo-toolbar">
    <div class="repo-toolbar-meta">
      <strong class="repo-toolbar-name">{{ name }}</strong>
      <div class="overflow-menu branch-menu">
        <button
          class="branch-switch"
          type="button"
          :disabled="busy"
          :aria-expanded="isOpen"
          aria-haspopup="menu"
          :title="branch ? `Switch branch from ${branch}` : 'Switch branch'"
          @click="toggleBranches"
        >
          <span class="branch-switch-name">{{ branch || "No branch" }}</span>
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M4.2 6.2L8 10l3.8-3.8" />
          </svg>
        </button>
        <div
          v-if="isOpen"
          ref="branchMenuEl"
          class="overflow-menu-dropdown branch-menu-dropdown"
          role="menu"
          aria-label="Branch actions"
        >
          <button
            class="overflow-menu-item"
            type="button"
            role="menuitem"
            :disabled="busy"
            @click="createBranch"
          >
            New branch
          </button>
          <button
            class="overflow-menu-item"
            type="button"
            role="menuitem"
            :disabled="busy || branches.length < 2"
            :title="
              branches.length < 2
                ? 'Need another local branch to merge into'
                : 'Merge a local branch into another'
            "
            @click="mergeBranch"
          >
            Merge into…
          </button>
          <div class="context-menu-sep" />
          <label class="branch-menu-search">
            <span class="sr-only">Filter branches</span>
            <svg class="branch-menu-search-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
              />
            </svg>
            <input
              ref="branchSearchInput"
              v-model="branchQuery"
              type="text"
              placeholder="Filter branches"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              @keydown="onBranchSearchKeydown"
            />
            <button
              v-if="branchQuery"
              class="branch-menu-search-clear"
              type="button"
              title="Clear search"
              @mousedown.prevent
              @click="clearBranchQuery"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </label>
          <div ref="branchListEl" class="branch-menu-list">
            <p v-if="!branches.length" class="muted tiny empty-branches">No local branches.</p>
            <p v-else-if="!filteredBranchItems.length" class="muted tiny empty-branches">
              No branches match that search.
            </p>
            <button
              v-for="(item, index) in filteredBranchItems"
              :key="item.name"
              class="overflow-menu-item branch-menu-branch"
              :class="{
                active: item.name === branch,
                highlighted: index === branchActiveIndex && item.name !== branch,
              }"
              :data-branch-index="index"
              type="button"
              role="menuitem"
              @mouseenter="onBranchHover(index)"
              @click="selectBranch(item.name)"
            >
            <span class="branch-menu-name">{{ item.name }}</span>
            <span
              v-if="item.localOnly"
              class="branch-pill"
              title="Local only — no remote counterpart"
            >
              Local
            </span>
            <span v-else-if="item.behind || item.ahead" class="sync-counts">
              <span
                v-if="item.behind"
                class="sync-count behind"
                :title="item.behindTitle"
              >
                ↓{{ item.behind }}
              </span>
              <span v-if="item.ahead" class="sync-count ahead" :title="item.aheadTitle">
                ↑{{ item.ahead }}
              </span>
            </span>
            </button>
          </div>
        </div>
      </div>
      <span class="repo-path" :title="path">{{ path }}</span>
      <span v-if="busyLabel" class="action-progress repo-toolbar-progress">
        {{ progressLabel }}
        <span v-if="progressBranch" class="action-branch-badge" :title="progressBranch">
          <BranchIcon />
          <span class="action-branch-name">{{ progressBranch }}</span>
        </span>
        <span class="spinner" aria-hidden="true" />
      </span>
    </div>
    <div class="repo-toolbar-bar repo-toolbar-actions">
      <div class="repo-toolbar-work">
        <button
          v-if="canUndoUnpushed"
          class="ghost tiny"
          type="button"
          :disabled="busy"
          :title="undoUnpushedTitle"
          @click="emit('undoUnpushed')"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M9 15L3 9m0 0l6-6M3 9h10.5a6 6 0 010 12H12" />
          </svg>
          Undo unpushed
          <span class="file-count-badge">{{ unpushedCount }}</span>
        </button>
        <button
          class="ghost tiny"
          type="button"
          :disabled="busy"
          title="Fetch remotes for this repository"
          @click="emit('fetch')"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M12 9.75v6.75m0 0-3-3m3 3 3-3m-8.25 6a4.5 4.5 0 0 1-1.41-8.775 5.25 5.25 0 0 1 10.233-2.33 3 3 0 0 1 3.758 3.848A3.752 3.752 0 0 1 18 19.5H6.75Z"
            />
          </svg>
          Fetch
        </button>
        <SplitAction
          :primary-title="pullTitle"
          more-title="Pull from another branch"
          :disabled="busy"
          @primary="emit('pull')"
          @more="emit('pullOptions')"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3"
            />
          </svg>
          Pull
        </SplitAction>
        <button
          class="ghost tiny"
          type="button"
          :disabled="busy"
          :title="pushTitle"
          @click="emit('push')"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M7.5 7.5 12 3m0 0 4.5 4.5M12 3v13.5"
            />
          </svg>
          Push
        </button>
      </div>
    </div>
  </div>
</template>
