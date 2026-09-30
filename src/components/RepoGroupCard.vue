<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useApp } from "../composables/useApp";
import { useOverflowMenu } from "../composables/useOverflowMenu";
import { useTabs } from "../composables/useTabs";
import { contrastingText, DEFAULT_HEADER_COLOR } from "../color";
import type { RepoEntry, RepoGroup } from "../types";
import BranchIcon from "./BranchIcon.vue";
import Modal from "./Modal.vue";
import RepoRow from "./RepoRow.vue";
import SplitAction from "./SplitAction.vue";

const DEFAULT_HEADER = DEFAULT_HEADER_COLOR;
const LAST_FALLBACKS = ["develop", "master", "main"] as const;
type LastFallback = (typeof LAST_FALLBACKS)[number];

function isLastFallback(value: string): value is LastFallback {
  return (LAST_FALLBACKS as readonly string[]).includes(value);
}

const props = defineProps<{
  group: RepoGroup;
  draft?: boolean;
  sortable?: boolean;
  dragging?: boolean;
  repos?: RepoEntry[];
  reposSortable?: boolean;
  draggingRepoId?: string | null;
  dropTarget?: boolean;
}>();

const emit = defineEmits<{
  created: [];
  cancel: [];
  reorderStart: [event: PointerEvent, groupId: string];
  repoDragStart: [event: PointerEvent, repoId: string];
}>();

const {
  busy,
  toggleGroup,
  renameGroup,
  createGroup,
  deleteGroup,
  saveSettings,
  addRepo,
  removeRepo,
  reorderGroupRepos,
  repoDisplayName,
  pullGroup,
  pullProgress,
  pullBranchByGroup,
  pullCancelled,
  cancelPull,
  refreshGroup,
  cancelRefresh,
  refreshingAll,
  pullingAll,
  refreshCancelled,
  refreshProgress,
  isGroupRefreshing,
  checkoutGroup,
  checkoutProgress,
  checkoutCancelled,
  cancelCheckout,
  statuses,
} = useApp();
const { hasTab, closeRepos } = useTabs();
const {
  isOpen: groupMenuOpen,
  toggle: toggleGroupMenu,
  close: closeMenus,
} = useOverflowMenu(() => `group:${props.group.id}`);

const siblingIds = computed(() => visibleRepos.value.map((repo) => repo.id));
const canSort = computed(() => !props.draft && props.group.repos.length > 1);
const alphaIds = computed(() =>
  [...props.group.repos]
    .sort((left, right) =>
      repoDisplayName(left.id, left.path).localeCompare(
        repoDisplayName(right.id, right.path),
        undefined,
        { sensitivity: "base", numeric: true },
      ),
    )
    .map((repo) => repo.id),
);
const canSortAlpha = computed(
  () =>
    canSort.value &&
    alphaIds.value.join("\0") !== props.group.repos.map((repo) => repo.id).join("\0"),
);
const visibleRepos = computed(() => props.repos ?? props.group.repos);

async function sortAlphabetically() {
  closeMenus();
  if (!canSortAlpha.value) {
    return;
  }
  try {
    await reorderGroupRepos(props.group.id, alphaIds.value);
  } catch (err) {
    window.alert(String(err));
  }
}

const nameInput = ref<HTMLInputElement | null>(null);
const renaming = ref(Boolean(props.draft));
const name = ref(props.draft ? "" : props.group.name);
const pullFromBranch = ref(props.group.pullFromBranch);
const extraFallbacks = ref<string[]>([]);
const lastFallback = ref<LastFallback>("develop");
loadFallbacks(props.group.checkoutFallbacks);
const checkoutSource = ref<"develop" | "specify" | "master" | "main">("develop");
const checkoutTarget = ref("");
const headerColor = ref(props.group.headerColor || DEFAULT_HEADER);
const modal = ref<"pull" | "checkout" | null>(null);
const pullSource = ref<"current" | "develop" | "master" | "main" | "specify">("current");
const specifyBranch = ref("");

watch(
  () => props.group,
  (group) => {
    if (props.draft || renaming.value) {
      return;
    }
    name.value = group.name;
    pullFromBranch.value = group.pullFromBranch;
    loadFallbacks(group.checkoutFallbacks);
    headerColor.value = group.headerColor || DEFAULT_HEADER;
  },
);

onMounted(() => {
  if (props.draft) {
    void nextTick(() => nameInput.value?.focus());
  }
});

const actionLabel = computed(() => busy.value[props.group.id] ?? "");

const headerStyle = computed(() => ({
  "--group-header": headerColor.value,
  "--group-header-fg": contrastingText(headerColor.value),
}));

function loadFallbacks(list: string[]) {
  const cleaned = list.map((item) => item.trim()).filter(Boolean);
  const last = cleaned[cleaned.length - 1];
  if (last && isLastFallback(last)) {
    lastFallback.value = last;
    extraFallbacks.value = cleaned.slice(0, -1);
    return;
  }
  lastFallback.value = "develop";
  extraFallbacks.value = cleaned;
}

function fallbackList() {
  return [
    ...extraFallbacks.value.map((item) => item.trim()).filter(Boolean),
    lastFallback.value,
  ];
}

async function persistSettings() {
  extraFallbacks.value = extraFallbacks.value.map((item) => item.trim()).filter(Boolean);
  const nextFallbacks = fallbackList();
  await saveSettings(
    props.group.id,
    pullFromBranch.value.trim() || "develop",
    nextFallbacks,
    headerColor.value,
  );
}

function onHeaderColor(event: Event) {
  headerColor.value = (event.target as HTMLInputElement).value;
}

function startRename() {
  closeMenus();
  name.value = props.group.name;
  headerColor.value = props.group.headerColor || DEFAULT_HEADER;
  renaming.value = true;
}

function cancelRename() {
  if (props.draft) {
    emit("cancel");
    return;
  }
  renaming.value = false;
  name.value = props.group.name;
  headerColor.value = props.group.headerColor || DEFAULT_HEADER;
}

async function finishRename() {
  const value = name.value.trim();
  if (props.draft) {
    if (!value) {
      return;
    }
    const group = await createGroup(value);
    if (headerColor.value !== DEFAULT_HEADER) {
      await saveSettings(group.id, "develop", ["develop"], headerColor.value);
    }
    emit("created");
    return;
  }
  renaming.value = false;
  if (!value || value === props.group.name) {
    name.value = props.group.name;
  } else {
    await renameGroup(props.group.id, value);
  }
  const savedColor = props.group.headerColor || DEFAULT_HEADER;
  if (headerColor.value !== savedColor) {
    await persistSettings();
  }
}

async function confirmDelete() {
  closeMenus();
  const ok = await confirm(
    `Delete group “${props.group.name}”? Repositories on disk are not deleted.`,
    {
      title: "Delete group",
      kind: "warning",
      okLabel: "Delete",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  const selected = props.group.repos.filter((repo) => hasTab(repo.id)).map((repo) => repo.id);
  await deleteGroup(props.group.id);
  if (selected.length) {
    closeRepos(selected);
  }
}

async function pickRepo() {
  closeMenus();
  const selected = await open({
    directory: true,
    multiple: true,
    title: "Add Git repositories",
  });
  const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
  const failures: string[] = [];
  for (const path of paths) {
    try {
      await addRepo(props.group.id, path);
    } catch (err) {
      failures.push(`${path}: ${String(err)}`);
    }
  }
  if (failures.length) {
    window.alert(failures.join("\n"));
  }
}

function addFallback() {
  extraFallbacks.value = [...extraFallbacks.value, ""];
}

function removeFallback() {
  extraFallbacks.value = extraFallbacks.value.slice(0, -1);
}

async function removeAndLeave(repoId: string) {
  await removeRepo(props.group.id, repoId);
  if (hasTab(repoId)) {
    closeRepos([repoId]);
  }
}

const pulling = computed(() => Boolean(pullProgress.value[props.group.id]));
const checkingOut = computed(() => Boolean(checkoutProgress.value[props.group.id]));
const groupRefreshing = computed(() => isGroupRefreshing(props.group.id));
const canCancelRefresh = computed(() => groupRefreshing.value && !refreshingAll.value);

const pullLabel = computed(() => {
  if (pullCancelled.value[props.group.id]) {
    return "Cancelling…";
  }
  return pulling.value ? "Cancel" : "Pull";
});

const pullTitle = computed(() => {
  const branches = [
    ...new Set(
      props.group.repos
        .map((repo) => statuses.value[repo.id]?.branch?.trim())
        .filter((branch): branch is string => Boolean(branch)),
    ),
  ];
  if (branches.length === 1) {
    return `Pull from ${branches[0]}`;
  }
  if (branches.length > 1) {
    return `Pull from ${branches.join(", ")}`;
  }
  return "Pull from current branch";
});

const pullBranch = computed(() => {
  if (pullSource.value === "current") {
    return "";
  }
  if (pullSource.value === "specify") {
    return specifyBranch.value.trim();
  }
  return pullSource.value;
});

const canConfirmPull = computed(
  () => pullSource.value === "current" || Boolean(pullBranch.value),
);

const pullRemoteLabel = computed(() => {
  if (pullSource.value === "current") {
    return "current-branch";
  }
  return pullBranch.value || "…";
});

const pullDisplayBranch = computed(() => {
  return pullBranchByGroup.value[props.group.id] || pullBranch.value || "…";
});

const pullHint = computed(() =>
  pullSource.value === "current"
    ? "Use this to pick up others’ commits on the same branch."
    : "Brings that remote branch into this checkout. Conflicts appear in the files list so you can open them, mark them resolved, or abort.",
);

const checkoutLabel = computed(() => {
  if (checkoutCancelled.value[props.group.id]) {
    return "Cancelling…";
  }
  return checkingOut.value ? "Cancel" : "Checkout";
});

const checkoutBranch = computed(() => {
  if (checkoutSource.value === "specify") {
    return checkoutTarget.value.trim();
  }
  return checkoutSource.value;
});

const canConfirmCheckout = computed(() => Boolean(checkoutBranch.value));

const refreshLabel = computed(() => {
  if (canCancelRefresh.value && refreshCancelled.value) {
    return "Cancelling…";
  }
  return canCancelRefresh.value ? "Cancel" : "Fetch";
});

function openPull() {
  if (pulling.value) {
    cancelPull(props.group.id);
    return;
  }
  pullSource.value = "current";
  specifyBranch.value = props.group.pullFromBranch;
  modal.value = "pull";
}

function pullCurrent() {
  if (pulling.value) {
    cancelPull(props.group.id);
    return;
  }
  return pullGroup(props.group.id);
}

function openCheckout() {
  if (checkingOut.value) {
    cancelCheckout(props.group.id);
    return;
  }
  checkoutSource.value = "develop";
  checkoutTarget.value = "";
  loadFallbacks(props.group.checkoutFallbacks);
  modal.value = "checkout";
}

function closeModal() {
  modal.value = null;
}

async function confirmPull() {
  if (!canConfirmPull.value) {
    return;
  }
  const branch = pullBranch.value;
  modal.value = null;
  if (branch) {
    pullFromBranch.value = branch;
    await persistSettings();
  }
  return pullGroup(props.group.id, branch || undefined);
}

async function checkoutAll() {
  if (!canConfirmCheckout.value) {
    return;
  }
  const target = checkoutBranch.value;
  const fallbacks = checkoutSource.value === "specify" ? fallbackList() : [];
  modal.value = null;
  if (checkoutSource.value === "specify") {
    await persistSettings();
  }
  return checkoutGroup(props.group.id, target, fallbacks);
}

function onHeaderClick(event: MouseEvent) {
  if (props.draft) {
    return;
  }
  const target = event.target;
  if (!(target instanceof Element)) {
    return;
  }
  if (target.closest("button, input, label, select, textarea, a, .overflow-menu, .group-drag")) {
    return;
  }
  void toggleGroup(props.group.id);
}

</script>

<template>
  <section
    class="group"
    :class="{ dragging, sortable, 'drop-target': dropTarget }"
    :data-group-id="draft ? undefined : group.id"
  >
    <div
      class="group-header"
      :class="{ sortable }"
      :style="headerStyle"
      @click="onHeaderClick"
    >
      <span
        v-if="sortable"
        class="group-drag"
        role="button"
        title="Drag to reorder"
        aria-label="Drag to reorder"
        @click.stop
        @pointerdown.stop="emit('reorderStart', $event, group.id)"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="5.5" cy="4" r="1.15" />
          <circle cx="10.5" cy="4" r="1.15" />
          <circle cx="5.5" cy="8" r="1.15" />
          <circle cx="10.5" cy="8" r="1.15" />
          <circle cx="5.5" cy="12" r="1.15" />
          <circle cx="10.5" cy="12" r="1.15" />
        </svg>
      </span>
      <button
        class="chevron"
        type="button"
        :class="{ open: group.expanded }"
        :aria-expanded="group.expanded"
        @click="draft ? undefined : toggleGroup(group.id)"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M4 6l4 4 4-4" />
        </svg>
      </button>
      <div class="group-heading">
        <input
          v-if="renaming"
          ref="nameInput"
          v-model="name"
          type="text"
          placeholder="Group name"
          autocapitalize="off"
          autocorrect="off"
          autocomplete="off"
          spellcheck="false"
          @keydown.enter="finishRename"
          @keydown.escape="cancelRename"
        />
        <span v-else class="group-title">{{ group.name }}</span>
        <span v-if="!renaming" class="group-count">{{ visibleRepos.length }}</span>
        <label v-if="renaming" class="color-picker">
          <span class="color-picker-label">Color</span>
          <span class="color-picker-swatch" aria-hidden="true">
            <input type="color" :value="headerColor" @input="onHeaderColor" />
          </span>
        </label>
        <button
          v-if="renaming"
          class="ghost tiny"
          type="button"
          :disabled="draft && !name.trim()"
          @mousedown.prevent="finishRename"
        >
          Save
        </button>
        <button
          v-if="renaming"
          class="ghost tiny"
          type="button"
          @mousedown.prevent="cancelRename"
        >
          Cancel
        </button>
      </div>
      <div class="group-actions">
        <template v-if="group.repos.length">
          <div
            v-if="refreshingAll || groupRefreshing || (!pullingAll && !pulling && !checkingOut)"
            class="header-action"
          >
            <span v-if="refreshingAll || groupRefreshing" class="action-progress">
              Fetching
              <span class="spinner" aria-hidden="true" />
              {{ refreshProgress[group.id] || `0/${group.repos.length}` }}
            </span>
            <button
              v-if="!refreshingAll && !pulling && !checkingOut"
              class="tiny"
              type="button"
              :class="canCancelRefresh ? 'danger' : 'ghost'"
              :disabled="refreshCancelled || (!!actionLabel && !groupRefreshing)"
              @click="groupRefreshing ? cancelRefresh() : refreshGroup(group.id)"
            >
              <svg
                v-if="!groupRefreshing"
                class="button-icon"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  d="M12 9.75v6.75m0 0-3-3m3 3 3-3m-8.25 6a4.5 4.5 0 0 1-1.41-8.775 5.25 5.25 0 0 1 10.233-2.33 3 3 0 0 1 3.758 3.848A3.752 3.752 0 0 1 18 19.5H6.75Z"
                />
              </svg>
              {{ refreshLabel }}
            </button>
          </div>
          <div
            v-if="!refreshingAll && !groupRefreshing && !checkingOut"
            class="header-action"
          >
            <span v-if="pulling" class="action-progress">
              Pulling
              <span class="action-branch-badge" :title="pullDisplayBranch">
                <BranchIcon />
                <span class="action-branch-name">{{ pullDisplayBranch }}</span>
              </span>
              <span class="spinner" aria-hidden="true" />
              {{ pullProgress[group.id] }}
            </span>
            <SplitAction
              v-if="!pulling"
              :primary-title="pullTitle"
              more-title="Pull from another branch"
              :disabled="!!actionLabel"
              @primary="pullCurrent"
              @more="openPull"
            >
              <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
                <path
                  d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3"
                />
              </svg>
              Pull
            </SplitAction>
            <button
              v-else-if="!pullingAll"
              class="tiny danger"
              type="button"
              :disabled="pullCancelled[group.id]"
              @click="openPull"
            >
              {{ pullLabel }}
            </button>
          </div>
          <div
            v-if="!refreshingAll && !pullingAll && !groupRefreshing && !pulling"
            class="header-action"
          >
            <span v-if="checkingOut" class="action-progress">
              Checking out
              <span class="action-branch-badge" :title="checkoutBranch">
                <BranchIcon />
                <span class="action-branch-name">{{ checkoutBranch }}</span>
              </span>
              <span class="spinner" aria-hidden="true" />
              {{ checkoutProgress[group.id] }}
            </span>
            <button
              class="tiny"
              type="button"
              :class="checkingOut ? 'danger' : 'ghost'"
              :disabled="(!!actionLabel && !checkingOut) || checkoutCancelled[group.id]"
              @click="openCheckout"
            >
              <svg
                v-if="!checkingOut"
                class="button-icon"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"
                />
              </svg>
              {{ checkoutLabel }}
            </button>
          </div>
        </template>
        <div v-if="!draft" class="overflow-menu group-menu">
          <button
            class="ghost tiny overflow-menu-trigger"
            type="button"
            :aria-expanded="groupMenuOpen"
            aria-haspopup="menu"
            title="Group actions"
            @click.stop="toggleGroupMenu"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="8" cy="3.25" r="1.25" />
              <circle cx="8" cy="8" r="1.25" />
              <circle cx="8" cy="12.75" r="1.25" />
            </svg>
          </button>
          <div v-if="groupMenuOpen" class="overflow-menu-dropdown" role="menu">
            <button
              class="overflow-menu-item"
              type="button"
              role="menuitem"
              @click.stop="pickRepo"
            >
              Add repository
            </button>
            <button
              class="overflow-menu-item"
              type="button"
              role="menuitem"
              :disabled="!canSortAlpha"
              @click.stop="sortAlphabetically"
            >
              Sort A–Z
            </button>
            <button
              class="overflow-menu-item"
              type="button"
              role="menuitem"
              @click.stop="startRename"
            >
              Edit group
            </button>
            <button
              class="overflow-menu-item danger"
              type="button"
              role="menuitem"
              @click.stop="confirmDelete"
            >
              Delete group
            </button>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="group.expanded && visibleRepos.length"
      class="group-body"
      :class="{ reordering: Boolean(draggingRepoId) }"
    >
      <RepoRow
        v-for="repo in visibleRepos"
        :key="repo.id"
        :repo="repo"
        :sibling-ids="siblingIds"
        :sortable="!draft && reposSortable"
        :dragging="draggingRepoId === repo.id"
        @remove="removeAndLeave"
        @reorder-start="(event, id) => emit('repoDragStart', event, id)"
      />
    </div>
    <p v-else-if="group.expanded && !draft" class="muted tiny group-empty">
      No repositories yet.
      <button class="link-button" type="button" @click="pickRepo">Add one</button>
    </p>

    <Modal v-if="modal === 'pull'" title="Pull from remote" @close="closeModal">
      <p class="pull-summary">
        Merges <code>origin/{{ pullRemoteLabel }}</code> into the currently checked-out branch.
        Checkout does not change.
      </p>
      <fieldset class="radio-list">
        <legend class="muted tiny">Remote branch</legend>
        <label class="radio-option">
          <input v-model="pullSource" type="radio" value="current" />
          origin/current-branch
        </label>
        <label class="radio-option">
          <input v-model="pullSource" type="radio" value="develop" />
          origin/develop
        </label>
        <label class="radio-option">
          <input v-model="pullSource" type="radio" value="master" />
          origin/master
        </label>
        <label class="radio-option">
          <input v-model="pullSource" type="radio" value="main" />
          origin/main
        </label>
        <label class="radio-option">
          <input v-model="pullSource" type="radio" value="specify" />
          Specify
        </label>
        <input
          v-if="pullSource === 'specify'"
          v-model="specifyBranch"
          type="text"
          placeholder="branch name"
          autocapitalize="off"
          autocorrect="off"
          autocomplete="off"
          spellcheck="false"
          autofocus
          @keydown.enter="confirmPull"
        />
      </fieldset>
      <p class="muted tiny pull-hint">{{ pullHint }}</p>
      <template #actions>
        <button class="ghost" type="button" @click="closeModal">Cancel</button>
        <button class="primary" type="button" :disabled="!canConfirmPull" @click="confirmPull">
          Pull
        </button>
      </template>
    </Modal>

    <Modal v-if="modal === 'checkout'" title="Checkout" @close="closeModal">
      <fieldset class="radio-list">
        <legend class="muted tiny">Branch</legend>
        <label class="radio-option">
          <input v-model="checkoutSource" type="radio" value="develop" />
          develop
        </label>
        <label class="radio-option">
          <input v-model="checkoutSource" type="radio" value="specify" />
          Specify
        </label>
        <div v-if="checkoutSource === 'specify'" class="specify-panel">
          <label class="modal-label">
            <span class="muted tiny">Check out this branch</span>
            <input
              v-model="checkoutTarget"
              type="text"
              placeholder="feature/JIRA-123"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              autofocus
              @keydown.enter="checkoutAll"
            />
          </label>
          <div class="specify-fallbacks">
            <p class="muted tiny specify-fallbacks-title">Fallbacks</p>
            <div v-if="extraFallbacks.length" class="fallback-editor">
              <input
                v-for="(_fallback, index) in extraFallbacks"
                :key="index"
                v-model="extraFallbacks[index]"
                type="text"
                placeholder="branch name"
                autocapitalize="off"
                autocorrect="off"
                autocomplete="off"
                spellcheck="false"
              />
            </div>
            <div class="fallback-editor-actions">
              <button class="ghost tiny" type="button" @click="addFallback">
                {{ extraFallbacks.length ? "Add another fallback" : "Add a fallback" }}
              </button>
              <button
                v-if="extraFallbacks.length"
                class="ghost tiny"
                type="button"
                @click="removeFallback"
              >
                Remove
              </button>
            </div>
            <p class="muted tiny specify-fallbacks-title">Final fallback</p>
            <div class="specify-last-fallback">
              <label class="radio-option">
                <input v-model="lastFallback" type="radio" value="develop" />
                develop
              </label>
              <label class="radio-option">
                <input v-model="lastFallback" type="radio" value="master" />
                master
              </label>
              <label class="radio-option">
                <input v-model="lastFallback" type="radio" value="main" />
                main
              </label>
            </div>
          </div>
        </div>
        <label class="radio-option">
          <input v-model="checkoutSource" type="radio" value="master" />
          master
        </label>
        <label class="radio-option">
          <input v-model="checkoutSource" type="radio" value="main" />
          main
        </label>
      </fieldset>
      <template #actions>
        <button class="ghost" type="button" @click="closeModal">Cancel</button>
        <button class="primary" type="button" :disabled="!canConfirmCheckout" @click="checkoutAll">
          Checkout
        </button>
      </template>
    </Modal>
  </section>
</template>
