<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { homeDir } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { useApp } from "../composables/useApp";
import { useDashboardDrag } from "../composables/useDashboardDrag";
import { useTabs } from "../composables/useTabs";
import { STANDALONE_GROUP_ID, type RepoEntry, type RepoGroup } from "../types";
import Modal from "../components/Modal.vue";
import RepoGroupCard from "../components/RepoGroupCard.vue";
import RepoRow from "../components/RepoRow.vue";

const DRAFT_GROUP: RepoGroup = {
  id: "__draft__",
  name: "",
  expanded: true,
  pullFromBranch: "develop",
  checkoutFallbacks: ["develop"],
  headerColor: "#16323c",
  repos: [],
};

type DashboardEntry =
  | { kind: "group"; id: string; group: RepoGroup }
  | { kind: "repo"; id: string; repo: RepoEntry };

const {
  groups,
  standaloneRepos,
  dashboardIds,
  setAllGroupsExpanded,
  addStandaloneRepo,
  cloneStandaloneRepo,
  removeStandaloneRepo,
  refreshAll,
  cancelRefresh,
  pullAll,
  cancelPullAll,
  refreshingAll,
  refreshCancelled,
  refreshProgressLabel,
  pullingAll,
  pullAllCancelled,
  pullProgress,
  pullProgressLabel,
  reorderDashboard,
  reorderGroupRepos,
  moveRepo,
  busy,
  repoDisplayName,
  showToast,
} = useApp();
const { hasTab, closeRepos } = useTabs();
const creating = ref(false);

const refreshAllProgress = computed(
  () => refreshProgressLabel.value.replace(/^Fetching\s+/, "") || "…",
);

const pullAllProgress = computed(
  () => pullProgressLabel.value.replace(/^Pulling\s+/, "") || "…",
);

const hasRepos = computed(
  () =>
    standaloneRepos.value.length > 0 || groups.value.some((group) => group.repos.length > 0),
);

const canStartPullAll = computed(
  () =>
    hasRepos.value &&
    !pullAllCancelled.value &&
    !refreshingAll.value &&
    (pullingAll.value || !Object.keys(pullProgress.value).length),
);

const groupById = computed(() => new Map(groups.value.map((group) => [group.id, group])));
const repoById = computed(() => {
  const map = new Map<string, RepoEntry>();
  standaloneRepos.value.forEach((repo) => map.set(repo.id, repo));
  groups.value.forEach((group) => group.repos.forEach((repo) => map.set(repo.id, repo)));
  return map;
});

/** Group-wide pulls, checkouts, and fetches track progress by each group's repo count. */
function repoListLocked(listId: string) {
  return Boolean(busy.value[listId]) || refreshingAll.value || pullingAll.value;
}

async function commitGroupRepos(repoId: string, groupId: string, ids: string[]) {
  const from =
    groups.value.find((group) => group.repos.some((repo) => repo.id === repoId))?.id ??
    STANDALONE_GROUP_ID;
  if (from === groupId) {
    await reorderGroupRepos(groupId, ids);
  } else {
    await moveRepo(repoId, from, groupId, ids.indexOf(repoId));
  }
}

const drag = useDashboardDrag({
  layout: () => ({
    root: [...dashboardIds.value],
    groups: Object.fromEntries(
      groups.value.map((group) => [group.id, group.repos.map((repo) => repo.id)]),
    ),
  }),
  itemSelector: "[data-repo-id]",
  itemKey: "repoId",
  isCollapsed: (groupId) => !groupById.value.get(groupId)?.expanded,
  canMove: (from, to) =>
    !repoListLocked(from ?? STANDALONE_GROUP_ID) && !repoListLocked(to ?? STANDALONE_GROUP_ID),
  commitRoot: reorderDashboard,
  commitGroup: commitGroupRepos,
  onError: (err) => window.alert(String(err)),
});

const entries = computed(() =>
  drag.rootIds(dashboardIds.value).flatMap((id): DashboardEntry[] => {
    const group = groupById.value.get(id);
    if (group) {
      return [{ kind: "group", id, group }];
    }
    const repo = repoById.value.get(id);
    return repo ? [{ kind: "repo", id, repo }] : [];
  }),
);
const topLevelRepoIds = computed(() =>
  entries.value.filter((entry) => entry.kind === "repo").map((entry) => entry.id),
);

function groupRepos(group: RepoGroup) {
  const ids = drag.groupItemIds(group.id);
  if (!ids) {
    return group.repos;
  }
  return ids.flatMap((id) => {
    const repo = repoById.value.get(id);
    return repo ? [repo] : [];
  });
}

const isEmpty = computed(() => !dashboardIds.value.length);

const hasGroups = computed(() => groups.value.length > 0);

const canExpandAll = computed(
  () => hasGroups.value && groups.value.some((group) => !group.expanded),
);

const canCollapseAll = computed(
  () => hasGroups.value && groups.value.some((group) => group.expanded),
);

const canSortEntries = computed(() => dashboardIds.value.length > 1);
const canDragRepos = computed(() => hasGroups.value || canSortEntries.value);

function entrySortKey(id: string) {
  const group = groupById.value.get(id);
  if (group) {
    return group.name;
  }
  const repo = repoById.value.get(id);
  return repo ? `${repoDisplayName(repo.id, repo.path)}\0${repo.label ?? ""}` : id;
}

const alphaIds = computed(() =>
  [...dashboardIds.value].sort((left, right) =>
    entrySortKey(left).localeCompare(entrySortKey(right), undefined, {
      sensitivity: "base",
      numeric: true,
    }),
  ),
);
const canSortAlpha = computed(
  () => canSortEntries.value && alphaIds.value.join("\0") !== dashboardIds.value.join("\0"),
);

async function sortAlphabetically() {
  if (!canSortAlpha.value) {
    return;
  }
  try {
    await reorderDashboard(alphaIds.value);
  } catch (err) {
    window.alert(String(err));
  }
}

function startCreate() {
  creating.value = true;
}

function cancelCreate() {
  creating.value = false;
}

async function pickStandaloneRepo() {
  const selected = await open({
    directory: true,
    multiple: true,
    title: "Add Git repositories",
  });
  const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
  const failures: string[] = [];
  for (const path of paths) {
    try {
      await addStandaloneRepo(path);
    } catch (err) {
      failures.push(`${path}: ${String(err)}`);
    }
  }
  if (failures.length) {
    window.alert(failures.join("\n"));
  }
}

async function removeStandalone(repoId: string) {
  await removeStandaloneRepo(repoId);
  if (hasTab(repoId)) {
    closeRepos([repoId]);
  }
}

const cloningOpen = ref(false);
const cloning = ref(false);
const cloneUrl = ref("");
const cloneParent = ref("");
const cloneName = ref("");
const cloneNameEdited = ref(false);
const cloneError = ref("");
const cloneUrlInput = ref<HTMLInputElement | null>(null);

const canClone = computed(
  () =>
    !cloning.value &&
    Boolean(cloneUrl.value.trim() && cloneParent.value.trim() && cloneName.value.trim()),
);

const cloneDestination = computed(() => {
  const parent = cloneParent.value.trim().replace(/\/+$/, "");
  const name = cloneName.value.trim();
  return parent && name ? `${parent}/${name}` : "";
});

function repoNameFromUrl(url: string) {
  const trimmed = url.trim().replace(/[/\\]+$/, "").replace(/\/\.git$/, "");
  const last = trimmed.split(/[/:\\]/).pop() ?? "";
  return last.replace(/\.git$/i, "");
}

function parentDir(path: string) {
  const index = path.replace(/\/+$/, "").lastIndexOf("/");
  return index > 0 ? path.slice(0, index) : "";
}

async function defaultCloneParent() {
  const repos = [...groups.value.flatMap((group) => group.repos), ...standaloneRepos.value];
  const recent = repos.length ? parentDir(repos[repos.length - 1].path) : "";
  if (recent) {
    return recent;
  }
  try {
    return await homeDir();
  } catch {
    return "";
  }
}

watch(cloneUrl, (url) => {
  if (!cloneNameEdited.value) {
    cloneName.value = repoNameFromUrl(url);
  }
});

async function openClone() {
  cloneUrl.value = "";
  cloneName.value = "";
  cloneNameEdited.value = false;
  cloneError.value = "";
  cloneParent.value = await defaultCloneParent();
  cloningOpen.value = true;
  await nextTick();
  cloneUrlInput.value?.focus();
}

function closeClone() {
  if (cloning.value) {
    return;
  }
  cloningOpen.value = false;
}

async function pickCloneParent() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Choose where to clone",
    defaultPath: cloneParent.value || undefined,
  });
  if (typeof selected === "string") {
    cloneParent.value = selected;
  }
}

async function cloneRepo() {
  if (!canClone.value) {
    return;
  }
  cloning.value = true;
  cloneError.value = "";
  try {
    const name = cloneName.value.trim();
    await cloneStandaloneRepo(cloneUrl.value.trim(), cloneParent.value.trim(), name);
    cloningOpen.value = false;
    showToast(`Cloned ${name}.`);
  } catch (err) {
    cloneError.value = String(err);
  } finally {
    cloning.value = false;
  }
}
</script>

<template>
  <div class="groups-page">
    <div class="groups-inner">
      <div class="groups-header">
        <div class="brand">
          <img class="brand-icon" src="/app-icon.png" alt="" width="72" height="72" />
          Shipyard
        </div>
      </div>

      <div class="groups-display">
        <div class="groups-toolbar">
          <div class="toolbar-start">
            <button class="primary" type="button" @click="pickStandaloneRepo">Add repository</button>
            <button class="ghost" type="button" @click="openClone">Clone repository</button>
            <button
              class="ghost"
              type="button"
              :disabled="creating"
              @click="startCreate"
            >
              New group
            </button>
          </div>
          <div class="toolbar-end">
            <button
              v-if="hasGroups"
              class="ghost"
              type="button"
              :disabled="!canExpandAll"
              @click="setAllGroupsExpanded(true)"
            >
              <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M19.5 5.25 12 12.75 4.5 5.25m15 6L12 18.75l-7.5-7.5" />
              </svg>
              Expand
            </button>
            <button
              v-if="hasGroups"
              class="ghost"
              type="button"
              :disabled="!canCollapseAll"
              @click="setAllGroupsExpanded(false)"
            >
              <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="m4.5 18.75 7.5-7.5 7.5 7.5m-15-6 7.5-7.5 7.5 7.5" />
              </svg>
              Collapse
            </button>
            <button
              class="ghost"
              type="button"
              :disabled="!canSortAlpha"
              @click="sortAlphabetically"
            >
              Sort A–Z
            </button>
            <div class="header-action">
              <span v-if="refreshingAll" class="action-progress">
                <span class="spinner" aria-hidden="true" />
                {{ refreshAllProgress }}
              </span>
              <button
                type="button"
                :class="{ danger: refreshingAll }"
                :disabled="!hasRepos || refreshCancelled || pullingAll"
                @click="refreshingAll ? cancelRefresh() : refreshAll()"
              >
                <svg
                  v-if="!refreshingAll"
                  class="button-icon"
                  viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <path
                    d="M12 9.75v6.75m0 0-3-3m3 3 3-3m-8.25 6a4.5 4.5 0 0 1-1.41-8.775 5.25 5.25 0 0 1 10.233-2.33 3 3 0 0 1 3.758 3.848A3.752 3.752 0 0 1 18 19.5H6.75Z"
                  />
                </svg>
                {{ refreshCancelled ? "Cancelling…" : refreshingAll ? "Cancel" : "Fetch" }}
              </button>
            </div>
            <div class="header-action">
              <span v-if="pullingAll" class="action-progress">
                <span class="spinner" aria-hidden="true" />
                {{ pullAllProgress }}
              </span>
              <button
                type="button"
                :class="{ danger: pullingAll }"
                :disabled="!canStartPullAll"
                :title="pullingAll ? undefined : 'Pull the current branch for every repository'"
                @click="pullingAll ? cancelPullAll() : pullAll()"
              >
                <svg
                  v-if="!pullingAll"
                  class="button-icon"
                  viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <path
                    d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3"
                  />
                </svg>
                {{ pullAllCancelled ? "Cancelling…" : pullingAll ? "Cancel" : "Pull" }}
              </button>
            </div>
          </div>
        </div>

        <p v-if="isEmpty && !creating" class="muted">
          Add a repository, or create a group for several at once.
        </p>

        <div
          class="groups-list"
          data-dashboard-root
          :class="{ reordering: Boolean(drag.draggingGroupId.value) }"
        >
          <RepoGroupCard
            v-if="creating"
            :group="DRAFT_GROUP"
            draft
            @cancel="cancelCreate"
            @created="cancelCreate"
          />
          <template v-for="entry in entries" :key="entry.id">
            <RepoGroupCard
              v-if="entry.kind === 'group'"
              :data-dashboard-id="entry.id"
              :group="entry.group"
              :sortable="canSortEntries"
              :dragging="drag.draggingGroupId.value === entry.id"
              :repos="groupRepos(entry.group)"
              :repos-sortable="canDragRepos"
              :dragging-repo-id="drag.draggingItemId.value"
              :drop-target="drag.dropGroupId.value === entry.id"
              @reorder-start="(event, id) => drag.start(event, 'group', id)"
              @repo-drag-start="(event, id) => drag.start(event, 'item', id)"
            />
            <RepoRow
              v-else
              :data-dashboard-id="entry.id"
              :repo="entry.repo"
              :sibling-ids="topLevelRepoIds"
              flush
              :sortable="canDragRepos"
              :dragging="drag.draggingItemId.value === entry.id"
              @remove="removeStandalone"
              @reorder-start="(event, id) => drag.start(event, 'item', id)"
            />
          </template>
        </div>
      </div>
    </div>
  </div>
  <Modal v-if="cloningOpen" title="Clone repository" medium @close="closeClone">
    <label class="modal-label">
      <span class="muted tiny">Repository URL or SSH address</span>
      <input
        ref="cloneUrlInput"
        v-model="cloneUrl"
        type="text"
        placeholder="git@github.com:owner/repo.git"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        :disabled="cloning"
        @keydown.enter.prevent="cloneRepo"
      />
    </label>
    <label class="modal-label">
      <span class="muted tiny">Location</span>
      <span class="clone-location">
        <input
          v-model="cloneParent"
          type="text"
          placeholder="/Users/me/Code"
          autocapitalize="off"
          autocorrect="off"
          autocomplete="off"
          spellcheck="false"
          :disabled="cloning"
          @keydown.enter.prevent="cloneRepo"
        />
        <button class="ghost" type="button" :disabled="cloning" @click="pickCloneParent">
          Browse…
        </button>
      </span>
    </label>
    <label class="modal-label">
      <span class="muted tiny">Folder name</span>
      <input
        v-model="cloneName"
        type="text"
        placeholder="repo"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        :disabled="cloning"
        @input="cloneNameEdited = true"
        @keydown.enter.prevent="cloneRepo"
      />
    </label>
    <p v-if="cloneError" class="settings-error clone-error">{{ cloneError }}</p>
    <p v-else-if="cloneDestination" class="muted tiny">
      Clones into {{ cloneDestination }} and adds it to Shipyard.
    </p>
    <template #actions>
      <button class="ghost" type="button" :disabled="cloning" @click="closeClone">Cancel</button>
      <button class="primary" type="button" :disabled="!canClone" @click="cloneRepo">
        <span v-if="cloning" class="spinner" aria-hidden="true" />
        {{ cloning ? "Cloning…" : "Clone" }}
      </button>
    </template>
  </Modal>
</template>
