<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from "vue";
import { homeDir } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { useApp } from "../composables/useApp";
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

const {
  groups,
  standaloneRepos,
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
  reorderGroups,
  reorderStandaloneRepos,
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

const canSortStandalone = computed(() => standaloneRepos.value.length > 1);
const canDragRepos = computed(() => groups.value.length > 0 || canSortStandalone.value);
const draggingRepo = ref<{ id: string; from: string } | null>(null);
const draftRepoLists = ref<Record<string, string[]> | null>(null);
const repoById = computed(() => {
  const map = new Map<string, RepoEntry>();
  standaloneRepos.value.forEach((repo) => map.set(repo.id, repo));
  groups.value.forEach((group) => group.repos.forEach((repo) => map.set(repo.id, repo)));
  return map;
});

function draftRepos(listId: string, saved: RepoEntry[]) {
  const ids = draftRepoLists.value?.[listId];
  if (!ids) {
    return saved;
  }
  return ids.flatMap((id) => {
    const repo = repoById.value.get(id);
    return repo ? [repo] : [];
  });
}

const visibleStandalone = computed(() => draftRepos(STANDALONE_GROUP_ID, standaloneRepos.value));
const standaloneIds = computed(() => visibleStandalone.value.map((repo) => repo.id));
const groupRepos = computed(() =>
  Object.fromEntries(groups.value.map((group) => [group.id, draftRepos(group.id, group.repos)])),
);

const isEmpty = computed(() => !groups.value.length && !standaloneRepos.value.length);

const hasGroups = computed(() => groups.value.length > 0);

const canExpandAll = computed(
  () => hasGroups.value && groups.value.some((group) => !group.expanded),
);

const canCollapseAll = computed(
  () => hasGroups.value && groups.value.some((group) => group.expanded),
);

const canSortGroups = computed(() => groups.value.length > 1);
const draggingGroupId = ref<string | null>(null);
const draftGroupIds = ref<string[] | null>(null);
const visibleGroups = computed(() => {
  const ids = draftGroupIds.value ?? groups.value.map((group) => group.id);
  const byId = new Map(groups.value.map((group) => [group.id, group]));
  return ids.flatMap((id) => {
    const group = byId.get(id);
    return group ? [group] : [];
  });
});
const alphaGroupIds = computed(() =>
  [...groups.value]
    .sort((left, right) =>
      left.name.localeCompare(right.name, undefined, { sensitivity: "base", numeric: true }),
    )
    .map((group) => group.id),
);
const canSortGroupsAlpha = computed(
  () =>
    canSortGroups.value &&
    alphaGroupIds.value.join("\0") !== groups.value.map((group) => group.id).join("\0"),
);

function standaloneSortKey(repo: (typeof standaloneRepos.value)[number]) {
  return `${repoDisplayName(repo.id, repo.path)}\0${repo.label ?? ""}`;
}

const alphaStandaloneIds = computed(() =>
  [...standaloneRepos.value]
    .sort((left, right) =>
      standaloneSortKey(left).localeCompare(standaloneSortKey(right), undefined, {
        sensitivity: "base",
        numeric: true,
      }),
    )
    .map((repo) => repo.id),
);
const canSortStandaloneAlpha = computed(
  () =>
    canSortStandalone.value &&
    alphaStandaloneIds.value.join("\0") !==
      standaloneRepos.value.map((repo) => repo.id).join("\0"),
);
const canSortAlpha = computed(() => canSortGroupsAlpha.value || canSortStandaloneAlpha.value);

function moveDraggingGroupTo(targetId: string, before: boolean) {
  const dragging = draggingGroupId.value;
  if (!dragging || dragging === targetId) {
    return;
  }
  const ids = [...(draftGroupIds.value ?? groups.value.map((group) => group.id))];
  const from = ids.indexOf(dragging);
  if (from === -1) {
    return;
  }
  ids.splice(from, 1);
  let to = ids.indexOf(targetId);
  if (to === -1) {
    return;
  }
  if (!before) {
    to += 1;
  }
  ids.splice(to, 0, dragging);
  if (ids.join("\0") !== (draftGroupIds.value ?? []).join("\0")) {
    draftGroupIds.value = ids;
  }
}

function onReorderGroupsMove(event: PointerEvent) {
  if (!draggingGroupId.value) {
    return;
  }
  const node = document.elementFromPoint(event.clientX, event.clientY);
  const group = node instanceof Element ? node.closest("[data-group-id]") : null;
  if (!(group instanceof HTMLElement) || !group.dataset.groupId) {
    return;
  }
  const header = group.querySelector(".group-header");
  let before = true;
  if (header instanceof HTMLElement) {
    const rect = header.getBoundingClientRect();
    before =
      event.clientY <= rect.bottom
        ? event.clientY < rect.top + rect.height / 2
        : false;
  } else {
    const rect = group.getBoundingClientRect();
    before = event.clientY < rect.top + rect.height / 2;
  }
  moveDraggingGroupTo(group.dataset.groupId, before);
}

async function finishReorderGroups() {
  window.removeEventListener("pointermove", onReorderGroupsMove);
  window.removeEventListener("pointerup", finishReorderGroups);
  window.removeEventListener("pointercancel", finishReorderGroups);
  document.body.classList.remove("reordering-groups");
  const ids = draftGroupIds.value;
  draggingGroupId.value = null;
  draftGroupIds.value = null;
  if (!ids || ids.join("\0") === groups.value.map((group) => group.id).join("\0")) {
    return;
  }
  try {
    await reorderGroups(ids);
  } catch (err) {
    window.alert(String(err));
  }
}

function savedRepoLists(): Record<string, string[]> {
  return {
    [STANDALONE_GROUP_ID]: standaloneRepos.value.map((repo) => repo.id),
    ...Object.fromEntries(groups.value.map((group) => [group.id, group.repos.map((repo) => repo.id)])),
  };
}

function draftListOf(lists: Record<string, string[]>, repoId: string) {
  return Object.keys(lists).find((listId) => lists[listId].includes(repoId)) ?? null;
}

/** Group-wide pulls, checkouts, and fetches track progress by each group's repo count. */
function repoListLocked(listId: string) {
  return Boolean(busy.value[listId]) || refreshingAll.value || pullingAll.value;
}

function placeDraggingRepo(listId: string, targetId: string | null, before: boolean) {
  const drag = draggingRepo.value;
  const lists = draftRepoLists.value;
  if (!drag || !lists || !(listId in lists) || targetId === drag.id) {
    return;
  }
  const current = draftListOf(lists, drag.id);
  if (!current) {
    return;
  }
  if (listId !== drag.from && (repoListLocked(listId) || repoListLocked(drag.from))) {
    return;
  }
  const next = { ...lists, [current]: lists[current].filter((id) => id !== drag.id) };
  const ids = [...next[listId]];
  let to = targetId ? ids.indexOf(targetId) : ids.length;
  if (to === -1) {
    return;
  }
  if (targetId && !before) {
    to += 1;
  }
  ids.splice(to, 0, drag.id);
  next[listId] = ids;
  if (current === listId && ids.join("\0") === lists[listId].join("\0")) {
    return;
  }
  draftRepoLists.value = next;
}

function onRepoDragMove(event: PointerEvent) {
  const drag = draggingRepo.value;
  const lists = draftRepoLists.value;
  if (!drag || !lists) {
    return;
  }
  const node = document.elementFromPoint(event.clientX, event.clientY);
  if (!(node instanceof Element)) {
    return;
  }
  const list = node.closest("[data-repo-list]");
  if (!(list instanceof HTMLElement) || !list.dataset.repoList) {
    return;
  }
  const listId = list.dataset.repoList;
  const row = node.closest("[data-repo-id]");
  if (row instanceof HTMLElement && row.dataset.repoId && list.contains(row)) {
    const rect = row.getBoundingClientRect();
    placeDraggingRepo(listId, row.dataset.repoId, event.clientY < rect.top + rect.height / 2);
  } else if (draftListOf(lists, drag.id) !== listId) {
    placeDraggingRepo(listId, null, false);
  }
}

function stopRepoDrag() {
  window.removeEventListener("pointermove", onRepoDragMove);
  window.removeEventListener("pointerup", finishRepoDrag);
  window.removeEventListener("pointercancel", finishRepoDrag);
  document.body.classList.remove("reordering-repos");
}

async function finishRepoDrag() {
  stopRepoDrag();
  const drag = draggingRepo.value;
  const lists = draftRepoLists.value;
  draggingRepo.value = null;
  draftRepoLists.value = null;
  if (!drag || !lists) {
    return;
  }
  const target = draftListOf(lists, drag.id);
  if (!target) {
    return;
  }
  const ids = lists[target];
  try {
    if (target !== drag.from) {
      await moveRepo(drag.id, drag.from, target, ids.indexOf(drag.id));
    } else if (ids.join("\0") !== savedRepoLists()[target]?.join("\0")) {
      if (target === STANDALONE_GROUP_ID) {
        await reorderStandaloneRepos(ids);
      } else {
        await reorderGroupRepos(target, ids);
      }
    }
  } catch (err) {
    window.alert(String(err));
  }
}

function onRepoDragStart(event: PointerEvent, repoId: string, from: string) {
  if (event.button !== 0 || !canDragRepos.value) {
    return;
  }
  event.preventDefault();
  draggingRepo.value = { id: repoId, from };
  draftRepoLists.value = savedRepoLists();
  document.body.classList.add("reordering-repos");
  window.addEventListener("pointermove", onRepoDragMove);
  window.addEventListener("pointerup", finishRepoDrag);
  window.addEventListener("pointercancel", finishRepoDrag);
}

function onReorderGroupsStart(event: PointerEvent, groupId: string) {
  if (event.button !== 0 || !canSortGroups.value) {
    return;
  }
  event.preventDefault();
  draggingGroupId.value = groupId;
  draftGroupIds.value = groups.value.map((group) => group.id);
  document.body.classList.add("reordering-groups");
  window.addEventListener("pointermove", onReorderGroupsMove);
  window.addEventListener("pointerup", finishReorderGroups);
  window.addEventListener("pointercancel", finishReorderGroups);
}

async function sortAlphabetically() {
  if (!canSortAlpha.value) {
    return;
  }
  try {
    if (canSortGroupsAlpha.value) {
      await reorderGroups(alphaGroupIds.value);
    }
    if (canSortStandaloneAlpha.value) {
      await reorderStandaloneRepos(alphaStandaloneIds.value);
    }
  } catch (err) {
    window.alert(String(err));
  }
}

onUnmounted(() => {
  window.removeEventListener("pointermove", onReorderGroupsMove);
  window.removeEventListener("pointerup", finishReorderGroups);
  window.removeEventListener("pointercancel", finishReorderGroups);
  stopRepoDrag();
  document.body.classList.remove("reordering-groups");
});

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
            <button
              class="primary"
              type="button"
              :disabled="creating"
              @click="startCreate"
            >
              New group
            </button>
            <button class="ghost" type="button" @click="pickStandaloneRepo">Add repository</button>
            <button class="ghost" type="button" @click="openClone">Clone repository</button>
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
          v-if="visibleStandalone.length || draggingRepo"
          class="standalone-list"
          :class="{ reordering: Boolean(draggingRepo) }"
          :data-repo-list="STANDALONE_GROUP_ID"
        >
          <RepoRow
            v-for="repo in visibleStandalone"
            :key="repo.id"
            :repo="repo"
            :sibling-ids="standaloneIds"
            flush
            :sortable="canDragRepos"
            :dragging="draggingRepo?.id === repo.id"
            @remove="removeStandalone"
            @reorder-start="(event, id) => onRepoDragStart(event, id, STANDALONE_GROUP_ID)"
          />
          <p v-if="!visibleStandalone.length" class="muted tiny standalone-drop">
            Drop here to remove from group
          </p>
        </div>

        <div class="groups-list" :class="{ reordering: Boolean(draggingGroupId) }">
          <RepoGroupCard
            v-if="creating"
            :group="DRAFT_GROUP"
            draft
            @cancel="cancelCreate"
            @created="cancelCreate"
          />
          <RepoGroupCard
            v-for="group in visibleGroups"
            :key="group.id"
            :group="group"
            :sortable="canSortGroups"
            :dragging="draggingGroupId === group.id"
            :repos="groupRepos[group.id]"
            :repos-sortable="canDragRepos"
            :dragging-repo-id="draggingRepo?.id ?? null"
            @reorder-start="onReorderGroupsStart"
            @repo-drag-start="(event, id) => onRepoDragStart(event, id, group.id)"
          />
        </div>
      </div>
    </div>
  </div>
  <Modal v-if="cloningOpen" title="Clone repository" medium @close="closeClone">
    <label class="modal-label">
      <span class="muted tiny">Repository URL</span>
      <input
        ref="cloneUrlInput"
        v-model="cloneUrl"
        type="text"
        placeholder="https://github.com/owner/repo.git"
        autocapitalize="off"
        autocorrect="off"
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
