<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { WorkingTreeFile } from "../types";
import {
  fileBasename,
  fileExtension,
  fileParentFolder,
  type IgnoreKind,
} from "../gitOperation";

const props = defineProps<{
  files: WorkingTreeFile[];
  x: number;
  y: number;
  editorLabel: string;
  canStash: boolean;
  canDiscard: boolean;
}>();

const emit = defineEmits<{
  stage: [];
  unstage: [];
  ignore: [kind: IgnoreKind];
  stash: [];
  openEditor: [];
  reveal: [];
  copyPath: [];
  discard: [];
  delete: [];
  close: [];
}>();

const menuEl = ref<HTMLElement | null>(null);
const left = ref(props.x);
const top = ref(props.y);
const ignoreOpen = ref(false);
const submenuLeft = ref(false);

const file = computed(() => props.files[0]);
const count = computed(() => props.files.length);
const multiple = computed(() => count.value > 1);
const name = computed(() => fileBasename(file.value.path));
const extension = computed(() => fileExtension(file.value.path));
const folder = computed(() => fileParentFolder(file.value.path));
const onDisk = computed(() =>
  props.files.every((entry) => entry.status.trim().toLowerCase() !== "deleted"),
);

async function placeMenu() {
  await nextTick();
  const el = menuEl.value;
  if (!el) {
    return;
  }
  const rect = el.getBoundingClientRect();
  const pad = 8;
  let nextLeft = props.x;
  let nextTop = props.y;
  if (nextLeft + rect.width > window.innerWidth - pad) {
    nextLeft = window.innerWidth - rect.width - pad;
  }
  if (nextTop + rect.height > window.innerHeight - pad) {
    nextTop = window.innerHeight - rect.height - pad;
  }
  left.value = Math.max(pad, nextLeft);
  top.value = Math.max(pad, nextTop);
  submenuLeft.value = left.value + rect.width + 220 > window.innerWidth - pad;
}

function onDocumentPointerDown(event: PointerEvent) {
  const target = event.target;
  if (target instanceof Element && target.closest(".file-context-menu")) {
    return;
  }
  emit("close");
}

function onDocumentKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    emit("close");
  }
}

function onContextMenu(event: MouseEvent) {
  event.preventDefault();
}

onMounted(() => {
  document.addEventListener("pointerdown", onDocumentPointerDown);
  document.addEventListener("keydown", onDocumentKeydown);
  void placeMenu();
});

onUnmounted(() => {
  document.removeEventListener("pointerdown", onDocumentPointerDown);
  document.removeEventListener("keydown", onDocumentKeydown);
});

watch(
  () => [props.x, props.y, props.files.map((entry) => entry.path).join("\0")],
  () => {
    ignoreOpen.value = false;
    left.value = props.x;
    top.value = props.y;
    void placeMenu();
  },
);
</script>

<template>
  <Teleport to="body">
    <div
      ref="menuEl"
      class="file-context-menu"
      role="menu"
      :style="{ left: `${left}px`, top: `${top}px` }"
      @contextmenu="onContextMenu"
    >
      <button
        class="context-menu-item"
        type="button"
        role="menuitem"
        @click="file.staged ? emit('unstage') : emit('stage')"
      >
        {{ file.staged ? "Unstage" : "Stage" }}{{ multiple ? ` ${count} files` : "" }}
      </button>
      <button
        v-if="multiple"
        class="context-menu-item"
        type="button"
        role="menuitem"
        @click="emit('ignore', 'file')"
      >
        Ignore {{ count }} files
      </button>
      <div
        v-else
        class="context-menu-subwrap"
        @mouseenter="ignoreOpen = true"
        @mouseleave="ignoreOpen = false"
      >
        <button
          class="context-menu-item has-sub"
          type="button"
          role="menuitem"
          aria-haspopup="menu"
          :aria-expanded="ignoreOpen"
          @click="ignoreOpen = !ignoreOpen"
        >
          <span>Ignore</span>
          <svg class="context-menu-chevron" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M6.25 3.5 10.75 8l-4.5 4.5" />
          </svg>
        </button>
        <div
          v-if="ignoreOpen"
          class="context-menu-sub"
          :class="{ left: submenuLeft }"
          role="menu"
        >
          <button
            class="context-menu-item"
            type="button"
            role="menuitem"
            @click="emit('ignore', 'file')"
          >
            Ignore '{{ name }}'
          </button>
          <button
            v-if="extension"
            class="context-menu-item"
            type="button"
            role="menuitem"
            @click="emit('ignore', 'extension')"
          >
            All files with the extension '{{ extension }}'
          </button>
          <button
            v-if="folder"
            class="context-menu-item"
            type="button"
            role="menuitem"
            @click="emit('ignore', 'folder')"
          >
            All files in '{{ folder }}'
          </button>
        </div>
      </div>
      <button
        class="context-menu-item"
        type="button"
        role="menuitem"
        :disabled="!canStash"
        :title="canStash ? undefined : 'Finish or abort the merge or rebase before stashing.'"
        @click="emit('stash')"
      >
        {{ multiple ? `Stash ${count} files` : "Stash file" }}
      </button>
      <template v-if="!multiple">
        <div class="context-menu-sep" />
        <button
          class="context-menu-item"
          type="button"
          role="menuitem"
          :disabled="!onDisk"
          @click="emit('openEditor')"
        >
          {{ editorLabel }}
        </button>
        <button
          class="context-menu-item"
          type="button"
          role="menuitem"
          :disabled="!onDisk"
          @click="emit('reveal')"
        >
          Show in Finder
        </button>
        <div class="context-menu-sep" />
        <button
          class="context-menu-item"
          type="button"
          role="menuitem"
          @click="emit('copyPath')"
        >
          Copy file path
        </button>
      </template>
      <div class="context-menu-sep" />
      <button
        class="context-menu-item danger"
        type="button"
        role="menuitem"
        :disabled="!canDiscard"
        :title="canDiscard ? undefined : 'Abort the merge or rebase instead of discarding changes.'"
        @click="emit('discard')"
      >
        {{ multiple ? `Discard changes in ${count} files` : "Discard changes" }}
      </button>
      <button
        class="context-menu-item danger"
        type="button"
        role="menuitem"
        :disabled="!onDisk"
        :title="onDisk || !multiple ? undefined : 'Some of these files are already deleted.'"
        @click="emit('delete')"
      >
        {{ multiple ? `Delete ${count} files` : "Delete file" }}
      </button>
    </div>
  </Teleport>
</template>
