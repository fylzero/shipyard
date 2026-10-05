<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import BranchContextMenu from "./BranchContextMenu.vue";
import BranchIcon from "./BranchIcon.vue";
import BranchPicker from "./BranchPicker.vue";
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
  checkedOutBranch?: string;
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
  pullBranch: [branch: string];
  mergeIntoCurrent: [branch: string];
  createPullRequest: [branch: string];
  refreshBranches: [];
}>();

const { statuses } = useApp();
const { isOpen, toggle, close } = useOverflowMenu(() => `branch-${props.repoId}`);
const branchMenuEl = ref<HTMLElement | null>(null);
const pickerRef = ref<InstanceType<typeof BranchPicker> | null>(null);
const branchActions = ref<{ branch: string; x: number; y: number } | null>(null);

const branchActionsTracking = computed(() => {
  const name = branchActions.value?.branch;
  return props.branchTracking?.find((item) => item.name === name);
});

const currentBranch = computed(
  () => statuses.value[props.repoId]?.branch || props.branch,
);

const pullTitle = computed(() =>
  currentBranch.value ? `Pull from ${currentBranch.value}` : "Pull from current branch",
);

const pushTitle = computed(() =>
  currentBranch.value ? `Push to ${currentBranch.value}` : "Push current branch",
);

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

function openBranchActions(branch: string, x: number, y: number) {
  const tracking = props.branchTracking?.find((item) => item.name === branch);
  const canPull = Boolean(tracking?.upstream) && (tracking?.behind ?? 0) > 0;
  const checkedOut = props.checkedOutBranch ?? "";
  const canMerge = Boolean(checkedOut) && branch !== checkedOut;
  const canCreatePullRequest = Boolean(tracking?.upstream?.startsWith("origin/"));
  branchActions.value = canPull || canMerge || canCreatePullRequest ? { branch, x, y } : null;
}

function closeBranchActions() {
  branchActions.value = null;
  pickerRef.value?.focus();
}

function pullFromMenu() {
  const branch = branchActions.value?.branch;
  branchActions.value = null;
  close();
  if (branch) {
    emit("pullBranch", branch);
  }
}

function mergeFromMenu() {
  const branch = branchActions.value?.branch;
  branchActions.value = null;
  close();
  if (branch) {
    emit("mergeIntoCurrent", branch);
  }
}

function createPullRequestFromMenu() {
  const branch = branchActions.value?.branch;
  branchActions.value = null;
  close();
  if (branch) {
    emit("createPullRequest", branch);
  }
}

watch(isOpen, async (open) => {
  if (!open) {
    branchActions.value = null;
    return;
  }
  await nextTick();
  const menu = branchMenuEl.value;
  if (menu) {
    menu.style.minWidth = `${menu.getBoundingClientRect().width}px`;
  }
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
          <BranchPicker
            ref="pickerRef"
            :branches="branches"
            :branch-tracking="branchTracking"
            :selected="branch"
            @select="selectBranch"
            @close="close"
            @branch-menu="openBranchActions"
          />
        </div>
        <BranchContextMenu
          v-if="isOpen && branchActions"
          :branch="branchActions.branch"
          :current="checkedOutBranch ?? ''"
          :behind="branchActionsTracking?.behind ?? 0"
          :ahead="branchActionsTracking?.ahead ?? 0"
          :upstream="branchActionsTracking?.upstream ?? ''"
          :x="branchActions.x"
          :y="branchActions.y"
          :busy="busy"
          @pull="pullFromMenu"
          @merge-into-current="mergeFromMenu"
          @create-pull-request="createPullRequestFromMenu"
          @close="closeBranchActions"
        />
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
