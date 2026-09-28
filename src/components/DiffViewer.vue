<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import * as api from "../api";
import { diffNotes, parseDiff, toSplitRows, type DiffKind } from "../diff";
import { formatCommitDate } from "../graphLayout";
import type { BlameLine, FileBlame } from "../types";

const props = defineProps<{
  raw: string;
  mode: "inline" | "split";
  repoPath?: string;
  file?: string;
  rev?: string;
  staged?: boolean;
  oldPath?: string;
}>();

const parsed = computed(() => parseDiff(props.raw));
const lines = computed(() => parsed.value.filter((line) => line.kind !== "meta"));
const notes = computed(() => diffNotes(parsed.value));
const splitRows = computed(() => toSplitRows(lines.value));

const emptyBlame = (): FileBlame => ({ current: [], previous: [] });
const blame = ref<FileBlame>(emptyBlame());
const blameLoading = ref(false);
let blameGeneration = 0;

const currentByLine = computed(() => mapBlame(blame.value.current));
const previousByLine = computed(() => mapBlame(blame.value.previous));

type HoverSide = "current" | "previous";
type HoverState = {
  key: string;
  side: HoverSide;
  lineNo: number;
  right: number;
  y: number;
  below: boolean;
};

const hover = ref<HoverState | null>(null);
const tooltipEl = ref<HTMLElement | null>(null);
const rootEl = ref<HTMLElement | null>(null);

const hoveredBlame = computed(() => {
  const next = hover.value;
  if (!next) {
    return null;
  }
  const table = next.side === "previous" ? previousByLine.value : currentByLine.value;
  return table.get(next.lineNo) ?? null;
});

const tooltip = computed(() => {
  if (!hover.value) {
    return null;
  }
  if (hoveredBlame.value) {
    return formatBlame(hoveredBlame.value);
  }
  if (blameLoading.value) {
    return {
      author: "Loading blame…",
      meta: "",
      summary: "",
    };
  }
  return null;
});

function mapBlame(entries: BlameLine[]) {
  return new Map(entries.map((entry) => [entry.line, entry]));
}

function isUncommitted(hash: string) {
  return !hash || hash.replace(/0/g, "") === "";
}

function formatBlame(entry: BlameLine) {
  if (isUncommitted(entry.hash)) {
    return {
      author: entry.author || "You",
      meta: "Not Committed Yet",
      summary: "",
    };
  }
  const hash = entry.hash.slice(0, 7);
  const date = entry.timestamp ? formatCommitDate(new Date(entry.timestamp * 1000).toISOString()) : "";
  return {
    author: entry.author || hash,
    meta: [hash, date].filter(Boolean).join(" · "),
    summary: entry.summary,
  };
}

function hoverRect(target: HTMLElement): DOMRect {
  const gutter = target.classList.contains("diff-gutter")
    ? target
    : target.previousElementSibling instanceof HTMLElement
      && target.previousElementSibling.classList.contains("diff-gutter")
      ? target.previousElementSibling
      : null;
  const code = target.classList.contains("diff-code")
    ? target
    : target.nextElementSibling instanceof HTMLElement
      && target.nextElementSibling.classList.contains("diff-code")
      ? target.nextElementSibling
      : null;
  if (gutter && code) {
    const left = gutter.getBoundingClientRect();
    const right = code.getBoundingClientRect();
    return new DOMRect(left.left, left.top, right.right - left.left, Math.max(left.height, right.height));
  }
  return target.getBoundingClientRect();
}

function canBlame(kind: DiffKind, lineNo?: number) {
  return (kind === "add" || kind === "del" || kind === "ctx") && lineNo != null;
}

function blameSide(kind: DiffKind, column?: HoverSide): HoverSide {
  if (column) {
    return column;
  }
  return kind === "del" ? "previous" : "current";
}

async function loadBlame() {
  const repoPath = props.repoPath?.trim() ?? "";
  const file = props.file?.trim() ?? "";
  if (!repoPath || !file) {
    blameGeneration += 1;
    blame.value = emptyBlame();
    blameLoading.value = false;
    return;
  }
  const generation = ++blameGeneration;
  blameLoading.value = true;
  try {
    const next = await api.fileBlame(repoPath, file, props.rev, props.staged ?? false, props.oldPath);
    if (generation !== blameGeneration) {
      return;
    }
    blame.value = next;
  } catch {
    if (generation === blameGeneration) {
      blame.value = emptyBlame();
    }
  } finally {
    if (generation === blameGeneration) {
      blameLoading.value = false;
    }
  }
}

function placeTooltip(rect: DOMRect) {
  const gap = 8;
  const height = tooltipEl.value?.offsetHeight ?? 84;
  const right = Math.max(gap, window.innerWidth - rect.right + gap);
  const below = rect.top < height + gap + 12;
  let y = below ? rect.bottom + gap : rect.top - gap;
  if (below) {
    y = Math.min(y, window.innerHeight - height - gap);
  } else {
    y = Math.max(height + gap, y);
  }
  return { right, y, below };
}

function showHover(
  target: EventTarget | null,
  key: string,
  kind: DiffKind,
  lineNo?: number,
  column?: HoverSide,
) {
  if (!(target instanceof HTMLElement) || !canBlame(kind, lineNo) || lineNo == null) {
    hideHover();
    return;
  }
  const rect = hoverRect(target);
  const placed = placeTooltip(rect);
  hover.value = {
    key,
    side: blameSide(kind, column),
    lineNo,
    ...placed,
  };
  void nextTick(() => {
    if (!hover.value || hover.value.key !== key) {
      return;
    }
    const next = placeTooltip(rect);
    hover.value = { ...hover.value, ...next };
  });
}

function hideHover() {
  hover.value = null;
}

function onRowLeave(event: MouseEvent) {
  const next = event.relatedTarget;
  if (next instanceof Node && event.currentTarget instanceof Node && event.currentTarget.contains(next)) {
    return;
  }
  hideHover();
}

function isHovered(key: string) {
  return hover.value?.key === key;
}

watch(
  () => [props.repoPath, props.file, props.rev, props.staged, props.oldPath, props.raw],
  () => {
    hideHover();
    void loadBlame();
  },
);

onMounted(() => {
  void loadBlame();
  const scroller = rootEl.value?.closest(".diff-scroll");
  scroller?.addEventListener("scroll", hideHover, { passive: true });
  window.addEventListener("resize", hideHover);
});

onUnmounted(() => {
  blameGeneration += 1;
  const scroller = rootEl.value?.closest(".diff-scroll");
  scroller?.removeEventListener("scroll", hideHover);
  window.removeEventListener("resize", hideHover);
});
</script>

<template>
  <div ref="rootEl">
    <div v-if="!raw" class="muted" style="padding: 0.85rem">No diff available.</div>
    <div v-if="raw && notes.length" class="diff-notes muted">
      <div v-for="note in notes" :key="note">{{ note }}</div>
    </div>
    <div v-if="raw && mode === 'inline'">
      <template v-for="(line, index) in lines" :key="index">
        <template v-if="line.kind === 'hunk'">
          <div v-if="index > 0" class="diff-hunk" @mouseenter="hideHover" />
        </template>
        <div
          v-else
          class="diff-line"
          :class="{
            'diff-add': line.kind === 'add',
            'diff-del': line.kind === 'del',
            'is-hovered': isHovered(`inline-${index}`),
          }"
          @mouseenter="showHover($event.currentTarget, `inline-${index}`, line.kind, line.kind === 'del' ? line.oldNo : line.newNo)"
          @mouseleave="onRowLeave"
        >
          <span class="diff-gutter">{{ line.kind === "add" ? "" : line.oldNo ?? "" }}</span>
          <span class="diff-gutter">{{ line.kind === "del" ? "" : line.newNo ?? "" }}</span>
          <span class="diff-code">{{
            line.kind === "add" ? `+${line.text}` : line.kind === "del" ? `-${line.text}` : line.text
          }}</span>
        </div>
      </template>
    </div>
    <div v-else-if="raw">
      <template v-for="(row, index) in splitRows" :key="index">
        <template v-if="row.leftKind === 'hunk'">
          <div v-if="index > 0" class="diff-hunk" @mouseenter="hideHover" />
        </template>
        <div
          v-else
          class="diff-line side"
          @mouseleave="onRowLeave"
        >
          <span
            class="diff-gutter"
            :class="{ 'is-hovered': isHovered(`split-${index}-left`) }"
            @mouseenter="showHover($event.currentTarget, `split-${index}-left`, row.leftKind, row.leftNo, 'previous')"
          >{{ row.leftNo ?? "" }}</span>
          <span
            class="diff-code"
            :class="{
              'diff-del': row.leftKind === 'del',
              'is-hovered': isHovered(`split-${index}-left`),
            }"
            @mouseenter="showHover($event.currentTarget, `split-${index}-left`, row.leftKind, row.leftNo, 'previous')"
          >{{ row.leftText }}</span>
          <span
            class="diff-gutter"
            :class="{ 'is-hovered': isHovered(`split-${index}-right`) }"
            @mouseenter="showHover($event.currentTarget, `split-${index}-right`, row.rightKind, row.rightNo, 'current')"
          >{{ row.rightNo ?? "" }}</span>
          <span
            class="diff-code"
            :class="{
              'diff-add': row.rightKind === 'add',
              'is-hovered': isHovered(`split-${index}-right`),
            }"
            @mouseenter="showHover($event.currentTarget, `split-${index}-right`, row.rightKind, row.rightNo, 'current')"
          >{{ row.rightText }}</span>
        </div>
      </template>
    </div>
    <Teleport to="body">
      <div
        v-if="hover && tooltip"
        ref="tooltipEl"
        class="blame-tooltip"
        :class="{ below: hover.below }"
        role="tooltip"
        :style="{ right: `${hover.right}px`, top: `${hover.y}px` }"
      >
        <strong>{{ tooltip.author }}</strong>
        <span v-if="tooltip.meta" class="muted tiny">{{ tooltip.meta }}</span>
        <span v-if="tooltip.summary">{{ tooltip.summary }}</span>
      </div>
    </Teleport>
  </div>
</template>
