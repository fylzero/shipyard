<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "../api";
import { contrastingText, DEFAULT_HEADER_COLOR } from "../color";
import { rangeIds } from "../selection";
import { useApp } from "../composables/useApp";
import { conflictCountLabel } from "../gitOperation";
import { useOverflowMenu } from "../composables/useOverflowMenu";
import { useTabs } from "../composables/useTabs";
import type { RepoEntry } from "../types";
import BranchIcon from "./BranchIcon.vue";
import FileIcon from "./FileIcon.vue";
import RepoIcon from "./RepoIcon.vue";
import StandaloneRepoActions from "./StandaloneRepoActions.vue";

const lastRepoClick = ref<string | null>(null);

const props = defineProps<{
  repo: RepoEntry;
  siblingIds: string[];
  flush?: boolean;
  sortable?: boolean;
  dragging?: boolean;
}>();

const emit = defineEmits<{
  remove: [repoId: string];
  reorderStart: [event: PointerEvent, repoId: string];
}>();

const { statuses, isRepoRefreshing, updateStandaloneRepo, showToast } = useApp();
const { activeId, hasTab, openRepo, openRepos } = useTabs();
const { isOpen: menuOpen, toggle: toggleMenu, close: closeMenu } = useOverflowMenu(
  () => `repo:${props.repo.id}`,
);

const editing = ref(false);
const labelDraft = ref("");
const colorDraft = ref(DEFAULT_HEADER_COLOR);
const labelInput = ref<HTMLInputElement | null>(null);

const colored = computed(() => Boolean(props.flush && (props.repo.headerColor || editing.value)));
const headerStyle = computed(() => {
  if (!colored.value) {
    return undefined;
  }
  const color = editing.value ? colorDraft.value : props.repo.headerColor || DEFAULT_HEADER_COLOR;
  return {
    "--group-header": color,
    "--group-header-fg": contrastingText(color),
  };
});

function folderName(path: string) {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function handleClick(event: MouseEvent) {
  if (editing.value) {
    return;
  }
  const target = event.target;
  if (
    target instanceof Element &&
    target.closest("button, input, label, .overflow-menu, .repo-drag, .repo-row-actions")
  ) {
    return;
  }
  if (event.shiftKey && lastRepoClick.value) {
    openRepos(rangeIds(props.siblingIds, lastRepoClick.value, props.repo.id), props.repo.id);
  } else {
    openRepo(props.repo.id);
  }
  lastRepoClick.value = props.repo.id;
}

function onRemove() {
  closeMenu();
  emit("remove", props.repo.id);
}

async function onOpenRemote() {
  closeMenu();
  try {
    const url = await api.repoRemoteUrl(props.repo.path);
    await openUrl(url);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function onOpenInFinder() {
  closeMenu();
  try {
    await api.openRepoInFinder(props.repo.path);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function startEdit() {
  closeMenu();
  labelDraft.value = props.repo.label ?? "";
  colorDraft.value = props.repo.headerColor || DEFAULT_HEADER_COLOR;
  editing.value = true;
  await nextTick();
  labelInput.value?.focus();
  labelInput.value?.select();
}

function cancelEdit() {
  editing.value = false;
  labelDraft.value = props.repo.label ?? "";
  colorDraft.value = props.repo.headerColor || DEFAULT_HEADER_COLOR;
}

function onHeaderColor(event: Event) {
  colorDraft.value = (event.target as HTMLInputElement).value;
}

async function saveEdit() {
  await updateStandaloneRepo(props.repo.id, labelDraft.value.trim(), colorDraft.value);
  editing.value = false;
}
</script>

<template>
  <div
    class="repo-row"
    :class="{
      active: activeId === repo.id,
      open: hasTab(repo.id),
      refreshing: isRepoRefreshing(repo.id),
      flush,
      sortable,
      dragging,
      colored,
      editing,
    }"
    :style="headerStyle"
    :data-repo-id="repo.id"
    @click="handleClick"
  >
    <span
      class="repo-drag"
      :class="{ spacer: !sortable }"
      role="button"
      title="Drag to reorder"
      aria-label="Drag to reorder"
      :aria-hidden="!sortable"
      @click.stop
      @pointerdown.stop="sortable && emit('reorderStart', $event, repo.id)"
    >
      <svg v-if="sortable" viewBox="0 0 16 16" aria-hidden="true">
        <circle cx="5.5" cy="4" r="1.15" />
        <circle cx="10.5" cy="4" r="1.15" />
        <circle cx="5.5" cy="8" r="1.15" />
        <circle cx="10.5" cy="8" r="1.15" />
        <circle cx="5.5" cy="12" r="1.15" />
        <circle cx="10.5" cy="12" r="1.15" />
      </svg>
    </span>
    <span class="repo-identity">
      <RepoIcon />
      <span class="repo-name">{{ statuses[repo.id]?.name ?? folderName(repo.path) }}</span>
      <span v-if="!editing && repo.label" class="repo-label">{{ repo.label }}</span>
      <input
        v-if="editing"
        ref="labelInput"
        v-model="labelDraft"
        type="text"
        class="repo-label-input"
        placeholder="Label, e.g. code review"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @click.stop
        @keydown.enter="saveEdit"
        @keydown.escape="cancelEdit"
      />
      <label v-if="editing" class="color-picker" @click.stop>
        <span class="color-picker-label">Color</span>
        <span class="color-picker-swatch" aria-hidden="true">
          <input type="color" :value="colorDraft" @input="onHeaderColor" />
        </span>
      </label>
      <button
        v-if="editing"
        class="ghost tiny"
        type="button"
        @mousedown.prevent="saveEdit"
      >
        Save
      </button>
      <button
        v-if="editing"
        class="ghost tiny"
        type="button"
        @mousedown.prevent="cancelEdit"
      >
        Cancel
      </button>
    </span>
    <span v-if="!editing" class="branch">
      <span v-if="isRepoRefreshing(repo.id)" class="spinner" aria-label="Fetching repository" />
      <span class="branch-name">
        <BranchIcon />
        {{ statuses[repo.id]?.branch ?? "…" }}
      </span>
      <span
        v-if="(statuses[repo.id]?.behind ?? 0) > 0 || (statuses[repo.id]?.ahead ?? 0) > 0"
        class="sync-counts"
      >
        <span
          v-if="(statuses[repo.id]?.behind ?? 0) > 0"
          class="sync-count behind"
          :title="`${statuses[repo.id]?.behind} commits behind`"
        >↓{{ statuses[repo.id]?.behind }}</span>
        <span
          v-if="(statuses[repo.id]?.ahead ?? 0) > 0"
          class="sync-count ahead"
          :title="`${statuses[repo.id]?.ahead} commits ahead`"
        >↑{{ statuses[repo.id]?.ahead }}</span>
      </span>
      <span
        v-if="(statuses[repo.id]?.conflictedFiles ?? 0) > 0"
        class="sync-count conflicted"
        :title="`${conflictCountLabel(statuses[repo.id]?.conflictedFiles ?? 0)}${
          statuses[repo.id]?.operation ? ` · ${statuses[repo.id]?.operation} in progress` : ''
        }`"
      >
        {{ conflictCountLabel(statuses[repo.id]?.conflictedFiles ?? 0) }}
      </span>
      <span
        v-if="(statuses[repo.id]?.changedFiles ?? 0) > 0"
        class="diff-stat"
        :title="`${statuses[repo.id]?.changedFiles} uncommitted files`"
      >
        <span class="file-count">
          <FileIcon />
          {{ statuses[repo.id]?.changedFiles }}
        </span>
        <span class="line-changes">
          <span v-if="(statuses[repo.id]?.insertions ?? 0) > 0" class="line-ins">
            +{{ statuses[repo.id]?.insertions }}
          </span>
          <span v-if="(statuses[repo.id]?.deletions ?? 0) > 0" class="line-del">
            −{{ statuses[repo.id]?.deletions }}
          </span>
        </span>
      </span>
    </span>
    <StandaloneRepoActions v-if="flush && !editing" :repo="repo" />
    <div v-else-if="!editing" class="repo-row-actions" aria-hidden="true" />
    <div class="overflow-menu repo-menu" @click.stop>
      <button
        class="ghost tiny overflow-menu-trigger"
        type="button"
        :aria-expanded="menuOpen"
        aria-haspopup="menu"
        title="Repository actions"
        @click.stop="toggleMenu"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="3.25" r="1.25" />
          <circle cx="8" cy="8" r="1.25" />
          <circle cx="8" cy="12.75" r="1.25" />
        </svg>
      </button>
      <div v-if="menuOpen" class="overflow-menu-dropdown" role="menu">
        <button
          class="overflow-menu-item"
          type="button"
          role="menuitem"
          @click.stop="onOpenRemote"
        >
          Open remote
        </button>
        <button
          class="overflow-menu-item"
          type="button"
          role="menuitem"
          @click.stop="onOpenInFinder"
        >
          Open in Finder
        </button>
        <button
          v-if="flush"
          class="overflow-menu-item"
          type="button"
          role="menuitem"
          @click.stop="startEdit"
        >
          Edit repository
        </button>
        <button
          class="overflow-menu-item danger"
          type="button"
          role="menuitem"
          @click.stop="onRemove"
        >
          Remove repository
        </button>
      </div>
    </div>
  </div>
</template>
