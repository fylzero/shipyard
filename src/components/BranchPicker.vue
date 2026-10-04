<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import type { BranchTracking } from "../types";

const props = defineProps<{
  branches: string[];
  branchTracking?: BranchTracking[];
  selected?: string;
  current?: string;
  initialQuery?: string;
  emptyText?: string;
}>();

const emit = defineEmits<{
  select: [branch: string];
  close: [];
  branchMenu: [branch: string, x: number, y: number];
}>();

const query = ref(props.initialQuery ?? "");
const searchInput = ref<HTMLInputElement | null>(null);
const listEl = ref<HTMLElement | null>(null);
const activeIndex = ref(0);
const moved = ref(Boolean(props.initialQuery));

const items = computed(() => {
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

const filteredItems = computed(() => {
  const needle = query.value.trim().toLowerCase();
  if (!needle) {
    return items.value;
  }
  return items.value.filter((item) => item.name.toLowerCase().includes(needle));
});

function selectedIndex() {
  const index = items.value.findIndex((item) => item.name === props.selected);
  return index >= 0 ? index : 0;
}

function scrollIntoView(selector: string) {
  void nextTick(() => {
    const list = listEl.value;
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

function clearQuery() {
  query.value = "";
  searchInput.value?.focus();
}

function onHover(index: number) {
  moved.value = true;
  activeIndex.value = index;
}

function onBranchContextMenu(event: MouseEvent, index: number) {
  event.preventDefault();
  const item = filteredItems.value[index];
  if (!item) {
    return;
  }
  onHover(index);
  emit("branchMenu", item.name, event.clientX, event.clientY);
}

function openActiveBranchMenu() {
  const item = filteredItems.value[activeIndex.value];
  const row = listEl.value?.querySelector<HTMLElement>(`[data-branch-index="${activeIndex.value}"]`);
  if (!item || !row) {
    return;
  }
  const rect = row.getBoundingClientRect();
  emit("branchMenu", item.name, rect.left + 16, rect.bottom);
}

function onKeydown(event: KeyboardEvent) {
  const list = filteredItems.value;
  if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
    event.preventDefault();
    openActiveBranchMenu();
    return;
  }
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (!list.length) {
      return;
    }
    const delta = event.key === "ArrowDown" ? 1 : -1;
    moved.value = true;
    activeIndex.value = Math.min(list.length - 1, Math.max(0, activeIndex.value + delta));
    scrollIntoView(`[data-branch-index="${activeIndex.value}"]`);
    return;
  }
  if (event.key === "Enter") {
    const item = list[activeIndex.value];
    event.preventDefault();
    if (!item || (!query.value.trim() && !moved.value)) {
      emit("close");
      return;
    }
    emit("select", item.name);
    return;
  }
  if (event.key === "Escape" && query.value) {
    event.stopPropagation();
    event.preventDefault();
    query.value = "";
  }
}

watch(query, async (value) => {
  if (!value.trim()) {
    activeIndex.value = selectedIndex();
    return;
  }
  activeIndex.value = 0;
  await nextTick();
  listEl.value?.scrollTo({ top: 0 });
});

watch(filteredItems, (list) => {
  if (activeIndex.value >= list.length) {
    activeIndex.value = Math.max(0, list.length - 1);
  }
});

onMounted(() => {
  activeIndex.value = query.value.trim() ? 0 : selectedIndex();
  searchInput.value?.focus();
  scrollIntoView(".branch-menu-branch.active");
});

defineExpose({
  focus: () => searchInput.value?.focus(),
});
</script>

<template>
  <div class="branch-picker">
    <label class="branch-menu-search">
      <span class="sr-only">Filter branches</span>
      <svg class="branch-menu-search-icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
      </svg>
      <input
        ref="searchInput"
        v-model="query"
        type="text"
        placeholder="Filter branches"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown="onKeydown"
      />
      <button
        v-if="query"
        class="branch-menu-search-clear"
        type="button"
        title="Clear search"
        @mousedown.prevent
        @click="clearQuery"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 18 18 6M6 6l12 12" />
        </svg>
      </button>
    </label>
    <div ref="listEl" class="branch-menu-list" @contextmenu.prevent>
      <p v-if="!branches.length" class="muted tiny empty-branches">
        {{ emptyText ?? "No local branches." }}
      </p>
      <p v-else-if="!filteredItems.length" class="muted tiny empty-branches">
        No branches match that search.
      </p>
      <button
        v-for="(item, index) in filteredItems"
        :key="item.name"
        class="overflow-menu-item branch-menu-branch"
        :class="{
          active: item.name === selected,
          highlighted: index === activeIndex && item.name !== selected,
        }"
        :data-branch-index="index"
        :title="item.name"
        type="button"
        role="menuitem"
        @mousedown.prevent
        @mouseenter="onHover(index)"
        @click="emit('select', item.name)"
        @contextmenu="onBranchContextMenu($event, index)"
      >
        <span class="branch-menu-name">{{ item.name }}</span>
        <span v-if="current && item.name === current" class="branch-pill">Current</span>
        <span v-if="item.localOnly" class="branch-pill" title="Local only — no remote counterpart">
          Local
        </span>
        <span v-else-if="item.behind || item.ahead" class="sync-counts">
          <span v-if="item.behind" class="sync-count behind" :title="item.behindTitle">
            ↓{{ item.behind }}
          </span>
          <span v-if="item.ahead" class="sync-count ahead" :title="item.aheadTitle">
            ↑{{ item.ahead }}
          </span>
        </span>
      </button>
    </div>
  </div>
</template>
