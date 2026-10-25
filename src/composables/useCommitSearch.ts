import { computed, onUnmounted, ref, watch, type Ref } from "vue";
import * as api from "../api";
import type { CommitNode } from "../types";

const SEARCH_DELAY_MS = 250;

export function useCommitSearch(path: () => string | undefined, commits: Ref<CommitNode[]>) {
  const query = ref("");
  const results = ref<CommitNode[]>([]);
  const truncated = ref(false);
  const searching = ref(false);
  const error = ref("");
  const index = ref(0);
  let generation = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const active = computed(() => query.value.trim().length > 0);

  const graphOrder = computed(
    () => new Map(commits.value.map((commit, position) => [commit.hash, position])),
  );

  const inGraph = computed(() =>
    results.value
      .filter((commit) => graphOrder.value.has(commit.hash))
      .sort((a, b) => graphOrder.value.get(a.hash)! - graphOrder.value.get(b.hash)!),
  );

  const older = computed(() =>
    results.value.filter((commit) => !graphOrder.value.has(commit.hash)),
  );

  const matchHashes = computed(() => new Set(inGraph.value.map((commit) => commit.hash)));

  const current = computed(() => inGraph.value[index.value] ?? null);

  function reset() {
    generation += 1;
    clearTimeout(timer);
    results.value = [];
    truncated.value = false;
    searching.value = false;
    error.value = "";
    index.value = 0;
  }

  async function run(keepPosition: boolean) {
    const repo = path();
    const needle = query.value.trim();
    if (!repo || !needle) {
      reset();
      return;
    }
    const run = ++generation;
    const previous = current.value?.hash;
    searching.value = true;
    try {
      const next = await api.searchCommits(repo, needle);
      if (run !== generation) {
        return;
      }
      results.value = next.commits;
      truncated.value = next.truncated;
      error.value = "";
      const kept = keepPosition && previous
        ? inGraph.value.findIndex((commit) => commit.hash === previous)
        : -1;
      index.value = Math.max(kept, 0);
    } catch (err) {
      if (run !== generation) {
        return;
      }
      results.value = [];
      truncated.value = false;
      error.value = String(err);
    } finally {
      if (run === generation) {
        searching.value = false;
      }
    }
  }

  function schedule(keepPosition: boolean) {
    clearTimeout(timer);
    if (!active.value) {
      reset();
      return;
    }
    searching.value = true;
    timer = setTimeout(() => void run(keepPosition), SEARCH_DELAY_MS);
  }

  function step(delta: number) {
    const total = inGraph.value.length;
    if (!total) {
      return;
    }
    index.value = (index.value + delta + total) % total;
  }

  function clear() {
    query.value = "";
  }

  watch(query, () => schedule(false));
  watch(path, () => {
    query.value = "";
    reset();
  });
  watch(commits, () => {
    if (active.value) {
      void run(true);
    }
  });

  onUnmounted(() => clearTimeout(timer));

  return {
    query,
    active,
    searching,
    error,
    truncated,
    results,
    inGraph,
    older,
    matchHashes,
    current,
    index,
    step,
    clear,
  };
}
