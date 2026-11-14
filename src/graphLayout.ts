import type { CommitNode, ResolvedTheme } from "./types";

export const GRAPH_ROW_HEIGHT = 28;
export const GRAPH_COL_WIDTH = 14;
export const GRAPH_MIN_COL_WIDTH = 6;
export const GRAPH_PAD_X = 10;
export const GRAPH_MAX_WIDTH = 156;
export const GRAPH_COLORS: Record<ResolvedTheme, string[]> = {
  dark: ["#5b8def", "#3dd68c", "#e6c07b", "#c678dd", "#56b6c2", "#f07178", "#abb2bf", "#d19a66"],
  light: ["#2f6fdb", "#178a52", "#b07a12", "#9333ea", "#0e7490", "#d1434b", "#6b7280", "#c2621d"],
};

export interface GraphPipe {
  column: number;
  hash: string;
}

export interface GraphLink {
  from: number;
  to: number;
}

export interface GraphRow {
  commit: CommitNode;
  column: number;
  incoming: GraphLink[];
  outgoing: GraphLink[];
  through: GraphLink[];
}

export interface GraphMetrics {
  colWidth: number;
  padX: number;
  width: number;
  nodeRadius: number;
  strokeWidth: number;
  packed: boolean;
}

export interface GraphLayout extends GraphMetrics {
  rows: GraphRow[];
  laneCount: number;
}

export function graphColumnMetrics(laneCount: number): GraphMetrics {
  const lanes = Math.max(1, laneCount);
  const padX = lanes > 14 ? 6 : GRAPH_PAD_X;
  const inner = GRAPH_MAX_WIDTH - padX * 2;
  const colWidth = Math.min(
    GRAPH_COL_WIDTH,
    Math.max(GRAPH_MIN_COL_WIDTH, inner / lanes),
  );
  const width = Math.min(GRAPH_MAX_WIDTH, padX * 2 + lanes * colWidth);
  return {
    colWidth,
    padX,
    width,
    nodeRadius: Math.min(4.5, Math.max(2, colWidth * 0.36)),
    strokeWidth: colWidth >= 11 ? 1.6 : colWidth >= 8 ? 1.3 : 1.05,
    packed: colWidth < GRAPH_COL_WIDTH,
  };
}

function firstEmpty(lanes: Array<string | null>): number {
  const hole = lanes.findIndex((lane) => lane === null);
  return hole === -1 ? lanes.length : hole;
}

export function layoutGraph(commits: CommitNode[]): GraphLayout {
  const lanes: Array<string | null> = [];
  const rows: GraphRow[] = [];
  let laneCount = 1;
  let previous: GraphPipe[] = [];

  for (const commit of commits) {
    let column = lanes.indexOf(commit.hash);
    if (column === -1) {
      column = firstEmpty(lanes);
      if (column === lanes.length) {
        lanes.push(commit.hash);
      } else {
        lanes[column] = commit.hash;
      }
    }

    const incoming: GraphLink[] = [];
    for (const pipe of previous) {
      if (pipe.hash === commit.hash) {
        incoming.push({ from: pipe.column, to: column });
      }
    }

    const nextLanes = lanes.map((hash) => (hash === commit.hash ? null : hash));
    const parents = commit.parents;
    const outgoing: GraphLink[] = [];

    if (parents[0]) {
      const existing = nextLanes.indexOf(parents[0]);
      if (existing !== -1) {
        outgoing.push({ from: column, to: existing });
      } else {
        nextLanes[column] = parents[0];
        outgoing.push({ from: column, to: column });
      }
    }

    for (const parent of parents.slice(1)) {
      let parentColumn = nextLanes.indexOf(parent);
      if (parentColumn === -1) {
        parentColumn = firstEmpty(nextLanes);
        if (parentColumn === nextLanes.length) {
          nextLanes.push(parent);
        } else {
          nextLanes[parentColumn] = parent;
        }
      }
      outgoing.push({ from: column, to: parentColumn });
    }

    while (nextLanes.length > 0 && nextLanes[nextLanes.length - 1] === null) {
      nextLanes.pop();
    }

    const through: GraphLink[] = [];
    for (const pipe of previous) {
      if (pipe.hash === commit.hash) {
        continue;
      }
      const nextColumn = nextLanes.indexOf(pipe.hash);
      if (nextColumn !== -1) {
        through.push({ from: pipe.column, to: nextColumn });
      }
    }

    rows.push({ commit, column, incoming, outgoing, through });
    laneCount = Math.max(laneCount, column + 1, nextLanes.length, lanes.length);
    lanes.splice(0, lanes.length, ...nextLanes);
    previous = nextLanes
      .map((hash, index) => (hash ? { column: index, hash } : null))
      .filter((pipe): pipe is GraphPipe => pipe !== null);
  }

  return {
    rows,
    laneCount,
    ...graphColumnMetrics(laneCount),
  };
}

export function laneX(
  column: number,
  colWidth = GRAPH_COL_WIDTH,
  padX = GRAPH_PAD_X,
): number {
  return padX + column * colWidth + colWidth / 2;
}

export function pipePath(
  from: number,
  y1: number,
  to: number,
  y2: number,
  colWidth = GRAPH_COL_WIDTH,
  padX = GRAPH_PAD_X,
): string {
  const x1 = laneX(from, colWidth, padX);
  const x2 = laneX(to, colWidth, padX);
  if (from === to) {
    return `M ${x1} ${y1} L ${x2} ${y2}`;
  }
  const mid = (y1 + y2) / 2;
  return `M ${x1} ${y1} C ${x1} ${mid}, ${x2} ${mid}, ${x2} ${y2}`;
}

export interface RefChip {
  kind: "head" | "tag" | "branch";
  name: string;
}

export function parseRefs(raw: string): RefChip[] {
  if (!raw.trim()) {
    return [];
  }

  const chips: RefChip[] = [];
  for (const part of raw.split(",")) {
    const item = part.trim();
    if (!item || item === "origin/HEAD" || item === "refs/stash") {
      continue;
    }
    if (item.startsWith("HEAD -> ")) {
      chips.push({ kind: "head", name: item.slice(8) });
      continue;
    }
    if (item === "HEAD") {
      chips.push({ kind: "head", name: "HEAD" });
      continue;
    }
    if (item.startsWith("tag: ")) {
      chips.push({ kind: "tag", name: item.slice(5) });
      continue;
    }
    chips.push({ kind: "branch", name: item.replace(/^refs\/heads\//, "") });
  }
  return chips.slice(0, 3);
}

export function formatCommitDate(value: string): string {
  if (!value) {
    return "";
  }
  const date = parseCommitDate(value);
  if (!date) {
    return value;
  }
  const days = Math.floor((Date.now() - date.getTime()) / 86_400_000);
  if (days < 1) {
    return "today";
  }
  if (days < 30) {
    return `${days}d ago`;
  }
  if (days < 365) {
    return `${Math.floor(days / 30)}mo ago`;
  }
  return `${Math.floor(days / 365)}y ago`;
}

export function formatFullCommitDate(value: string): string {
  if (!value) {
    return "";
  }
  const date = parseCommitDate(value);
  if (!date) {
    return value;
  }
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
    timeZoneName: "short",
  }).format(date);
}

function parseCommitDate(value: string): Date | null {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}
