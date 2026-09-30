<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { rangeIds, toggleId } from "../selection";
import type { WorkingTreeFile } from "../types";
import { useApp } from "../composables/useApp";
import { isConflicted, openInEditorLabel, type IgnoreKind } from "../gitOperation";
import FileContextMenu from "./FileContextMenu.vue";
import FileStatusIcon from "./FileStatusIcon.vue";
import PathLabel from "./PathLabel.vue";

const props = defineProps<{
  files: WorkingTreeFile[];
  selectedPath: string;
  selectedStaged: boolean;
  operation?: string;
  hasDraft?: boolean;
}>();

const { editor } = useApp();
const openEditorLabel = computed(() => openInEditorLabel(editor.value));
const menu = ref<{ files: WorkingTreeFile[]; x: number; y: number } | null>(null);
const selection = ref<{ staged: boolean; paths: string[] }>({ staged: false, paths: [] });
const anchor = ref<string | null>(null);

const emit = defineEmits<{
  select: [file: WorkingTreeFile, options?: { toggle?: boolean }];
  stage: [files: WorkingTreeFile[]];
  unstage: [files: WorkingTreeFile[]];
  stageAll: [];
  unstageAll: [];
  discard: [];
  stash: [];
  stashFiles: [files: WorkingTreeFile[]];
  ignore: [files: WorkingTreeFile[], kind: IgnoreKind];
  reveal: [file: WorkingTreeFile];
  copyPath: [file: WorkingTreeFile];
  discardFiles: [files: WorkingTreeFile[]];
  deleteFiles: [files: WorkingTreeFile[]];
  commit: [];
  openEditor: [file: WorkingTreeFile];
}>();

const conflicted = computed(() => props.files.filter(isConflicted));
const unstaged = computed(() =>
  props.files.filter((file) => !file.staged && !isConflicted(file)),
);
const staged = computed(() =>
  props.files.filter((file) => file.staged && !isConflicted(file)),
);
const resolving = computed(() => Boolean(props.operation || conflicted.value.length));
const canCommit = computed(
  () => props.files.length > 0 && (!props.operation || props.operation === "merge"),
);

function isMac() {
  return /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
}

function isMenuClick(event: MouseEvent) {
  return event.ctrlKey && !event.metaKey && isMac();
}

function isActive(file: WorkingTreeFile) {
  return props.selectedPath === file.path && props.selectedStaged === file.staged;
}

function isPicked(file: WorkingTreeFile) {
  return selection.value.staged === file.staged && selection.value.paths.includes(file.path);
}

function listFor(isStaged: boolean) {
  return isStaged ? staged.value : unstaged.value;
}

function pickedFiles() {
  const paths = selection.value.paths;
  return listFor(selection.value.staged).filter((file) => paths.includes(file.path));
}

function pick(file: WorkingTreeFile, paths: string[]) {
  selection.value = { staged: file.staged, paths };
}

function clearSelection() {
  selection.value = { staged: false, paths: [] };
  anchor.value = null;
}

function openFileMenu(event: MouseEvent, file: WorkingTreeFile) {
  event.preventDefault();
  if (!isPicked(file)) {
    pick(file, [file.path]);
    anchor.value = file.path;
    emit("select", file, { toggle: false });
  }
  menu.value = { files: pickedFiles(), x: event.clientX, y: event.clientY };
}

function onRowClick(event: MouseEvent, file: WorkingTreeFile) {
  if (isMenuClick(event)) {
    openFileMenu(event, file);
  }
}

function onFileClick(event: MouseEvent, file: WorkingTreeFile) {
  if (isMenuClick(event)) {
    return;
  }
  const sameList = selection.value.staged === file.staged && selection.value.paths.length > 0;
  if (event.shiftKey) {
    const activeHere = props.selectedPath && props.selectedStaged === file.staged;
    const from = sameList ? anchor.value : activeHere ? props.selectedPath : null;
    const paths = listFor(file.staged).map((entry) => entry.path);
    pick(file, from ? rangeIds(paths, from, file.path) : [file.path]);
    anchor.value = from ?? file.path;
    emit("select", file, { toggle: false });
    return;
  }
  if (event.metaKey || (event.ctrlKey && !isMac())) {
    const paths = sameList ? toggleId(selection.value.paths, file.path) : [file.path];
    pick(file, paths);
    anchor.value = file.path;
    if (paths.includes(file.path)) {
      emit("select", file, { toggle: false });
    }
    return;
  }
  if (isActive(file) && selection.value.paths.length <= 1) {
    clearSelection();
    emit("select", file);
    return;
  }
  pick(file, [file.path]);
  anchor.value = file.path;
  emit("select", file, { toggle: false });
}

function closeFileMenu() {
  menu.value = null;
}

function onKeydown(event: KeyboardEvent) {
  const target = event.target;
  if (
    event.key !== "Escape" ||
    menu.value ||
    !selection.value.paths.length ||
    (target instanceof HTMLElement && target.closest("input, textarea, [contenteditable]"))
  ) {
    return;
  }
  clearSelection();
}

watch(
  () => props.files,
  () => {
    closeFileMenu();
    const live = new Set(listFor(selection.value.staged).map((file) => file.path));
    selection.value.paths = selection.value.paths.filter((path) => live.has(path));
    if (anchor.value && !live.has(anchor.value)) {
      anchor.value = selection.value.paths[selection.value.paths.length - 1] ?? null;
    }
  },
);

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="file-pane">
    <section v-if="conflicted.length" class="file-section conflicted">
      <div class="pane-header">
        <div class="file-heading">
          <strong>Conflicted files</strong>
          <span class="file-count-badge conflicted">{{ conflicted.length }}</span>
        </div>
      </div>
      <div class="file-list">
        <div
          v-for="file in conflicted"
          :key="`conflicted:${file.path}`"
          class="file-item"
          :class="{ active: isActive(file) }"
        >
          <button class="file-item-main" type="button" @click="emit('select', file)">
            <FileStatusIcon :status="file.status" />
            <PathLabel class="file-item-path" :path="file.path" />
          </button>
          <div class="file-item-actions">
            <button
              class="tiny file-item-action editor"
              type="button"
              @click.stop="emit('openEditor', file)"
            >
              {{ openEditorLabel }}
            </button>
            <button
              class="tiny file-item-action stage"
              type="button"
              @click.stop="emit('stage', [file])"
            >
              Mark resolved
            </button>
          </div>
        </div>
      </div>
    </section>
    <section class="file-section">
      <div class="pane-header">
        <div class="file-heading">
          <strong>Unstaged files</strong>
          <span class="file-count-badge">{{ unstaged.length }}</span>
        </div>
        <button
          class="ghost tiny file-bulk-action stage"
          type="button"
          :disabled="!unstaged.length"
          @click="emit('stageAll')"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          Stage all files
        </button>
      </div>
      <div class="file-list">
        <p v-if="!unstaged.length" class="muted tiny empty-files">No unstaged files.</p>
        <div
          v-for="file in unstaged"
          :key="`unstaged:${file.path}`"
          class="file-item"
          :class="{ active: isActive(file), selected: isPicked(file) }"
          @contextmenu="openFileMenu($event, file)"
          @click="onRowClick($event, file)"
        >
          <button class="file-item-main" type="button" @click="onFileClick($event, file)">
            <FileStatusIcon :status="file.status" />
            <PathLabel class="file-item-path" :path="file.path" />
          </button>
          <button
            class="tiny file-item-action stage"
            type="button"
            @click.stop="emit('stage', [file])"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
            Stage file
          </button>
        </div>
      </div>
    </section>
    <section class="file-section">
      <div class="pane-header">
        <div class="file-heading">
          <strong>Staged files</strong>
          <span class="file-count-badge">{{ staged.length }}</span>
        </div>
        <button
          class="ghost tiny file-bulk-action unstage"
          type="button"
          :disabled="!staged.length"
          @click="emit('unstageAll')"
        >
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 12h14" />
          </svg>
          Unstage all files
        </button>
      </div>
      <div class="file-list">
        <p v-if="!staged.length" class="muted tiny empty-files">No staged files.</p>
        <div
          v-for="file in staged"
          :key="`staged:${file.path}`"
          class="file-item"
          :class="{ active: isActive(file), selected: isPicked(file) }"
          @contextmenu="openFileMenu($event, file)"
          @click="onRowClick($event, file)"
        >
          <button class="file-item-main" type="button" @click="onFileClick($event, file)">
            <FileStatusIcon :status="file.status" />
            <PathLabel class="file-item-path" :path="file.path" />
          </button>
          <button
            class="tiny file-item-action unstage"
            type="button"
            @click.stop="emit('unstage', [file])"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M5 12h14" />
            </svg>
            Unstage file
          </button>
        </div>
      </div>
    </section>
    <div class="file-footer">
      <button
        class="ghost tiny danger"
        type="button"
        :disabled="!files.length || resolving"
        :title="resolving ? 'Abort the merge or rebase instead of discarding everything.' : undefined"
        @click="emit('discard')"
      >
        <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0"
          />
        </svg>
        Discard
      </button>
      <button
        class="ghost tiny"
        type="button"
        :disabled="!files.length || resolving"
        :title="resolving ? 'Finish or abort the merge or rebase before stashing.' : undefined"
        @click="emit('stash')"
      >
        <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M20.25 7.5l-.625 10.632a2.25 2.25 0 01-2.247 2.118H6.622a2.25 2.25 0 01-2.247-2.118L3.75 7.5M10 11.25h4M3.375 7.5h17.25c.621 0 1.125-.504 1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125Z"
          />
        </svg>
        Stash
      </button>
      <button
        class="ghost tiny commit"
        type="button"
        :class="{ 'has-draft': hasDraft }"
        :disabled="!canCommit"
        :title="hasDraft ? 'Draft commit message saved' : undefined"
        @click="emit('commit')"
      >
        <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4.5 12.75l6 6 9-13.5" />
        </svg>
        Commit
      </button>
    </div>
    <FileContextMenu
      v-if="menu"
      :files="menu.files"
      :x="menu.x"
      :y="menu.y"
      :editor-label="openEditorLabel"
      :can-stash="!resolving"
      :can-discard="!resolving"
      @stage="emit('stage', menu.files); closeFileMenu()"
      @unstage="emit('unstage', menu.files); closeFileMenu()"
      @ignore="emit('ignore', menu.files, $event); closeFileMenu()"
      @stash="emit('stashFiles', menu.files); closeFileMenu()"
      @open-editor="emit('openEditor', menu.files[0]); closeFileMenu()"
      @reveal="emit('reveal', menu.files[0]); closeFileMenu()"
      @copy-path="emit('copyPath', menu.files[0]); closeFileMenu()"
      @discard="emit('discardFiles', menu.files); closeFileMenu()"
      @delete="emit('deleteFiles', menu.files); closeFileMenu()"
      @close="closeFileMenu"
    />
  </div>
</template>
