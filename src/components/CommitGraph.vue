<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { rangeIds, toggleId } from "../selection";
import type { CommitNode } from "../types";
import {
  GRAPH_COLORS,
  GRAPH_ROW_HEIGHT,
  formatCommitDate,
  formatFullCommitDate,
  laneX,
  layoutGraph,
  parseRefs,
  pipePath,
} from "../graphLayout";

const props = defineProps<{
  commits: CommitNode[];
  selectedHash?: string;
  matches?: Set<string>;
  currentHash?: string;
}>();

const root = ref<HTMLElement | null>(null);

function revealRow(hash: string | undefined) {
  if (!hash) {
    return;
  }
  const row = root.value?.querySelector<HTMLElement>(`[data-hash="${hash}"]`);
  const viewport = row?.closest<HTMLElement>(".graph-scroll");
  if (!row || !viewport) {
    return;
  }
  const rowRect = row.getBoundingClientRect();
  const viewRect = viewport.getBoundingClientRect();
  if (rowRect.top < viewRect.top || rowRect.bottom > viewRect.bottom) {
    row.scrollIntoView({ block: "center" });
  }
}

const emit = defineEmits<{
  select: [commit: CommitNode];
  menu: [commit: CommitNode, hashes: string[], x: number, y: number];
}>();

const selected = ref<string[]>([]);
const anchor = ref<string | null>(null);
const hashes = computed(() => props.commits.map((commit) => commit.hash));

function isMac() {
  return /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
}

function isSelected(hash: string) {
  return selected.value.includes(hash);
}

function onRowClick(event: MouseEvent, commit: CommitNode) {
  if (event.button !== 0) {
    return;
  }
  if (event.ctrlKey && !event.metaKey && isMac()) {
    return;
  }
  if (event.shiftKey && anchor.value) {
    selected.value = rangeIds(hashes.value, anchor.value, commit.hash);
    return;
  }
  if (event.metaKey || (event.ctrlKey && !isMac())) {
    selected.value = toggleId(selected.value, commit.hash);
    anchor.value = commit.hash;
    return;
  }
  selected.value = [commit.hash];
  anchor.value = commit.hash;
  emit("select", commit);
}

function onRowMenu(event: MouseEvent, commit: CommitNode) {
  event.preventDefault();
  if (!selected.value.includes(commit.hash)) {
    selected.value = [commit.hash];
    anchor.value = commit.hash;
  }
  emit("menu", commit, [...selected.value], event.clientX, event.clientY);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && selected.value.length) {
    selected.value = [];
    anchor.value = null;
  }
}

watch(hashes, (next) => {
  const live = new Set(next);
  selected.value = selected.value.filter((hash) => live.has(hash));
  if (anchor.value && !live.has(anchor.value)) {
    anchor.value = selected.value[selected.value.length - 1] ?? null;
  }
});

const layout = computed(() => layoutGraph(props.commits));
const midY = GRAPH_ROW_HEIGHT / 2;

type DatePopover = {
  text: string;
  right: number;
  y: number;
  below: boolean;
};

const datePopover = ref<DatePopover | null>(null);

function color(column: number) {
  return GRAPH_COLORS[column % GRAPH_COLORS.length];
}

function linkPath(from: number, y1: number, to: number, y2: number) {
  return pipePath(from, y1, to, y2, layout.value.colWidth, layout.value.padX);
}

function hideDatePopover() {
  datePopover.value = null;
}

function showDatePopover(event: MouseEvent, value: string) {
  const target = event.currentTarget;
  const text = formatFullCommitDate(value);
  if (!(target instanceof HTMLElement) || !text) {
    hideDatePopover();
    return;
  }
  const rect = target.getBoundingClientRect();
  const gap = 8;
  const below = rect.top < 72;
  datePopover.value = {
    text,
    right: Math.max(gap, window.innerWidth - rect.right),
    y: below ? rect.bottom + gap : rect.top - gap,
    below,
  };
}

watch(() => [props.commits, props.selectedHash], hideDatePopover);

watch(
  () => props.currentHash,
  (hash) => void nextTick(() => revealRow(hash)),
);

watch(
  () => props.selectedHash,
  (hash) => void nextTick(() => revealRow(hash)),
);

onMounted(() => {
  window.addEventListener("scroll", hideDatePopover, true);
  window.addEventListener("resize", hideDatePopover);
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("scroll", hideDatePopover, true);
  window.removeEventListener("resize", hideDatePopover);
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div v-if="!commits.length" class="muted" style="padding: 1rem">No commits yet.</div>
  <div v-else ref="root" class="commit-graph" :class="{ searching: matches }">
    <div
      v-for="row in layout.rows"
      :key="row.commit.hash"
      class="commit-graph-row"
      :data-hash="row.commit.hash"
      :class="{
        active: selectedHash === row.commit.hash,
        selected: isSelected(row.commit.hash),
        match: matches?.has(row.commit.hash),
        'current-match': currentHash === row.commit.hash,
      }"
      role="button"
      tabindex="0"
      :aria-pressed="selectedHash === row.commit.hash || isSelected(row.commit.hash)"
      @click="onRowClick($event, row.commit)"
      @contextmenu="onRowMenu($event, row.commit)"
      @keydown.enter.prevent="emit('select', row.commit)"
      @keydown.space.prevent="emit('select', row.commit)"
    >
      <div class="commit-graph-cell" :class="{ packed: layout.packed }">
        <svg
          class="commit-graph-svg"
          :width="layout.width"
          :height="GRAPH_ROW_HEIGHT"
          :viewBox="`0 0 ${layout.width} ${GRAPH_ROW_HEIGHT}`"
        >
          <path
            v-for="(link, index) in row.through"
            :key="`t-${index}`"
            :d="linkPath(link.from, 0, link.to, GRAPH_ROW_HEIGHT)"
            fill="none"
            :stroke="color(link.to)"
            :stroke-width="layout.strokeWidth"
          />
          <path
            v-for="(link, index) in row.incoming"
            :key="`i-${index}`"
            :d="linkPath(link.from, 0, link.to, midY)"
            fill="none"
            :stroke="color(link.to)"
            :stroke-width="layout.strokeWidth"
          />
          <path
            v-for="(link, index) in row.outgoing"
            :key="`o-${index}`"
            :d="linkPath(link.from, midY, link.to, GRAPH_ROW_HEIGHT)"
            fill="none"
            :stroke="color(link.to)"
            :stroke-width="layout.strokeWidth"
          />
          <circle
            :cx="laneX(row.column, layout.colWidth, layout.padX)"
            :cy="midY"
            :r="layout.nodeRadius"
            :fill="color(row.column)"
          />
        </svg>
      </div>
      <div class="commit-subject" :title="row.commit.subject">
        <span
          v-for="chip in parseRefs(row.commit.refs)"
          :key="`${chip.kind}-${chip.name}`"
          class="ref-chip"
          :class="chip.kind"
        >{{ chip.name }}</span>
        {{ row.commit.subject }}
      </div>
      <div class="commit-author">{{ row.commit.author }}</div>
      <div
        class="commit-date"
        :aria-label="formatFullCommitDate(row.commit.date)"
        @mouseenter="showDatePopover($event, row.commit.date)"
        @mouseleave="hideDatePopover"
      >{{ formatCommitDate(row.commit.date) }}</div>
    </div>
    <Teleport to="body">
      <div
        v-if="datePopover"
        class="date-popover"
        :class="{ below: datePopover.below }"
        role="tooltip"
        :style="{ right: `${datePopover.right}px`, top: `${datePopover.y}px` }"
      >
        {{ datePopover.text }}
      </div>
    </Teleport>
  </div>
</template>
