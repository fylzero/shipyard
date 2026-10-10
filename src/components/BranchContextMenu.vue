<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useApp } from "../composables/useApp";

const props = withDefaults(
  defineProps<{
    branch: string;
    x: number;
    y: number;
    current?: string;
    behind?: number;
    ahead?: number;
    upstream?: string;
    busy?: boolean;
    copyLabel?: string;
    copiedMessage?: string;
  }>(),
  {
    current: "",
    behind: 0,
    ahead: 0,
    upstream: "",
    busy: false,
    copyLabel: "Copy branch name",
    copiedMessage: "Copied branch name",
  },
);

const emit = defineEmits<{
  pull: [];
  mergeIntoCurrent: [];
  createPullRequest: [];
  close: [];
}>();

const { showToast } = useApp();

const menuEl = ref<HTMLElement | null>(null);
const left = ref(props.x);
const top = ref(props.y);

const isCurrent = computed(() => props.branch === props.current);
const showPull = computed(() => Boolean(props.upstream) && props.behind > 0);
const showMerge = computed(() => Boolean(props.current) && !isCurrent.value);
const showPullRequest = computed(() => props.upstream.startsWith("origin/"));

// A branch that isn't checked out can only be fast-forwarded in place, so local commits block it.
const pullBlocked = computed(() => !isCurrent.value && props.ahead > 0);
const pullTitle = computed(() => {
  if (props.busy) {
    return "Wait for the current action to finish.";
  }
  if (pullBlocked.value) {
    return `${props.branch} has commits that aren't on ${props.upstream}, so it can't fast-forward. Check it out and pull to merge them.`;
  }
  const count = props.behind === 1 ? "1 commit" : `${props.behind} commits`;
  return `Bring in ${count} from ${props.upstream}`;
});

const mergeTitle = computed(() =>
  props.busy ? "Wait for the current action to finish." : `Merge ${props.branch} into ${props.current}`,
);

function menuItems() {
  return Array.from(
    menuEl.value?.querySelectorAll<HTMLButtonElement>(".context-menu-item:not(:disabled)") ?? [],
  );
}

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
  menuItems()[0]?.focus();
}

function onDocumentPointerDown(event: PointerEvent) {
  const target = event.target;
  if (target instanceof Element && target.closest(".branch-context-menu")) {
    return;
  }
  emit("close");
}

// Captured on window so Escape and arrows stay inside this menu instead of reaching the branch dropdown.
function onWindowKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    emit("close");
    return;
  }
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    event.stopPropagation();
    const items = menuItems();
    if (!items.length) {
      return;
    }
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    const delta = event.key === "ArrowDown" ? 1 : -1;
    items[(index + delta + items.length) % items.length]?.focus();
    return;
  }
  if (event.key === "Tab") {
    event.preventDefault();
    event.stopPropagation();
  }
}

function onContextMenu(event: MouseEvent) {
  event.preventDefault();
}

async function copyName() {
  try {
    await navigator.clipboard.writeText(props.branch);
    showToast(props.copiedMessage);
    emit("close");
  } catch (err) {
    showToast(String(err), "error");
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", onDocumentPointerDown);
  window.addEventListener("keydown", onWindowKeydown, true);
  void placeMenu();
});

onUnmounted(() => {
  document.removeEventListener("pointerdown", onDocumentPointerDown);
  window.removeEventListener("keydown", onWindowKeydown, true);
});

watch(
  () => [props.x, props.y, props.branch],
  () => {
    left.value = props.x;
    top.value = props.y;
    void placeMenu();
  },
);
</script>

<template>
  <Teleport to="body">
    <!-- Stops pointerdown so the branch dropdown underneath stays open while an item is clicked. -->
    <div
      ref="menuEl"
      class="file-context-menu branch-context-menu"
      role="menu"
      :aria-label="`Actions for ${branch}`"
      :style="{ left: `${left}px`, top: `${top}px` }"
      @pointerdown.stop
      @contextmenu="onContextMenu"
    >
      <button
        v-if="showPull"
        class="context-menu-item"
        type="button"
        role="menuitem"
        :disabled="busy || pullBlocked"
        :title="pullTitle"
        @click="emit('pull')"
      >
        Pull
        <span class="sync-count behind">↓{{ behind }}</span>
      </button>
      <button
        v-if="showMerge"
        class="context-menu-item"
        type="button"
        role="menuitem"
        :disabled="busy"
        :title="mergeTitle"
        @click="emit('mergeIntoCurrent')"
      >
        Merge into current branch
      </button>
      <button
        v-if="showPullRequest"
        class="context-menu-item"
        type="button"
        role="menuitem"
        :title="`Open a new pull request for ${upstream} in your browser`"
        @click="emit('createPullRequest')"
      >
        Create pull request…
      </button>
      <div v-if="showPull || showMerge || showPullRequest" class="context-menu-sep" />
      <button class="context-menu-item" type="button" role="menuitem" @click="copyName">
        {{ copyLabel }}
      </button>
    </div>
  </Teleport>
</template>
